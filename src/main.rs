use clap::Parser;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

use blossom::agent::ChatCommand;
use blossom::{ChatModel, Config, State};

mod cli;

use cli::{Cli, Command};

#[tokio::main]
async fn main() {
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(std::io::stdout());
    let env_filter = EnvFilter::builder()
        .with_default_directive(tracing::Level::INFO.into())
        .from_env_lossy();

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_writer)
        .with_filter(env_filter);

    tracing_subscriber::registry().with(stderr_layer).init();

    blossom::register_panic_logger();
    blossom::report_version();

    let config = Config::parse_env().expect("Failed to load configuration");
    let state = State::from_config(&config)
        .await
        .expect("Failed to create state");
    let args = Cli::parse();
    handle_command(state, args.command)
        .await
        .expect("Failed to handle command");
}

fn pretty_message(message: &str) {
    println!("🌸 {}", message);
}

fn pretty_warn(message: &str) {
    println!("⚠️  Oops! {}", message);
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::error::Error),
    #[error("engine error: {0}")]
    Engine(#[from] blossom::agent::LlmEngineError),
    #[error("chroma error: {0}")]
    Chroma(#[from] anyhow::Error),
}

/* App scripting */

use std::io::{self, Write};
use std::path::Path;

use chromadb::v1::collection::CollectionEntries;
use chromadb::v1::{ChromaClient, ChromaCollection};
use names::Generator;
use tokio::io::{stdout, AsyncWriteExt};
use tokio_stream::StreamExt;

use blossom::agent::LlmEngine;

async fn handle_command(state: State, command: Command) -> Result<(), AppError> {
    match command {
        Command::New { maybe_name } => {
            let name = maybe_name.unwrap_or_else(|| Generator::default().next().unwrap());
            let mut conn = state.sqlite_database().begin().await?;
            let id = ChatModel::create(&name, &mut conn).await?;
            conn.commit().await?;
            pretty_message(&format!("Created new chat named '{}' with ID {}", name, id));
        }
        Command::Ls => {
            let mut conn = state.sqlite_database().acquire().await?;
            let chats = ChatModel::read_all(&mut conn).await?;
            for chat in chats {
                pretty_message(&format!("ID: {} | Name: {}", chat.id(), chat.name()));
            }
        }
        Command::Cont { name } => {
            let mut conn = state.sqlite_database().acquire().await?;
            let maybe_chat = ChatModel::read_by_name(&name, &mut conn).await;
            let chat = match maybe_chat {
                Ok(chat) => chat,
                Err(_) => {
                    pretty_warn(&format!("Chat '{}' not found", name));
                    return Ok(());
                }
            };
            run(&chat, &state).await?;
        }
    }
    Ok(())
}

async fn run(chat: &ChatModel, state: &State) -> Result<(), AppError> {
    // let _chat_id = chat.id();
    let chat_name = chat.name();
    let engine = state.llm_engine();
    let chroma_database = state.chroma_database();
    // let _sqlite_database = state.sqlite_database();
    let mut context = None;
    pretty_message(&format!("Running chat '{}'", chat_name));

    loop {
        print!(">>> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        let chat_command = ChatCommand::from(input);

        match chat_command {
            ChatCommand::Attach { paths } => {
                let collection = get_collection(chat_name, chroma_database)?;
                for path in paths {
                    embed_path(&path, &collection, engine).await;
                }
            }
            ChatCommand::Chat { message } => {
                pretty_message("Thinking about your message...");
                let maybe_tool_call = engine.handle(&message).await;
                let tool_call = match maybe_tool_call {
                    Ok(tool_call) => tool_call,
                    Err(e) => {
                        pretty_warn(&format!("Failed to handle message: {}", e));
                        continue;
                    }
                };
                match tool_call.name() {
                    "converse" => {
                        pretty_message("Crafting a response...");
                        // Validate the tool call
                        let args = tool_call.args();
                        if args.len() != 1 {
                            pretty_warn(&format!(
                                "Accidentally called `converse` with more than one argument: {:?}",
                                tool_call
                            ));
                            continue;
                        }
                        let input = args.first().unwrap();
                        assert_eq!(input.name(), "input");
                        assert_eq!(input.r#type(), "String");

                        // Complete on the response to std out
                        let input = input.value();
                        let mut stdout = stdout();
                        let mut stream = engine.converse(input, context.clone()).await?;
                        while let Some(Ok(res)) = stream.next().await {
                            for ele in res {
                                stdout.write_all(ele.response.as_bytes()).await.unwrap();
                                stdout.flush().await.unwrap();

                                if let Some(final_data) = ele.final_data {
                                    context = Some(final_data.context);
                                }
                            }
                        }
                        println!();
                    }
                    _ => pretty_warn(&format!("Unknown tool call: {:?}", tool_call)),
                }
            }
            ChatCommand::Exit => {
                pretty_message("Exiting chat");
                break;
            }
        }
    }
    Ok(())
}

fn get_collection(
    name: &str,
    chroma_database: &ChromaClient,
) -> Result<ChromaCollection, AppError> {
    let collection = chroma_database.create_collection(name, None, true)?;
    Ok(collection)
}

async fn embed_path(path: &Path, collection: &ChromaCollection, engine: &LlmEngine) {
    // If the path is a directory, panic
    if path.is_dir() {
        pretty_warn(&format!(
            "This is a directory, not a file: {}",
            path.display()
        ));
        return;
    }

    // Check if the extension is not a text file
    if path.extension().unwrap_or_default() != "txt" {
        pretty_warn(&format!("This is not a text file: {}", path.display()));
        return;
    }

    // TODO: this is janky bad chunking
    // Read the data and chunk the file among paragraphs
    let maybe_data = std::fs::read_to_string(path);
    if let Err(e) = maybe_data {
        pretty_warn(&format!("Failed to read the file: {}", e));
        return;
    }
    let data = maybe_data.unwrap();
    let paragraphs = data.split("\n\n").collect::<Vec<&str>>();

    // Embed the paragraphs in batches
    let mut batch_index = 0;
    let batch_size = 5;
    let mut batch_ids = Vec::new();
    let mut batch_documents = Vec::new();
    let mut batch_embeddings = Vec::new();
    for (id_index, paragraph) in paragraphs.into_iter().enumerate() {
        let response = engine.embed(paragraph).await.unwrap();
        // Map the response to f32
        let response = response.iter().map(|x| *x as f32).collect::<Vec<f32>>();
        let i = path;
        let id = format!("{}-{}", i.to_str().unwrap(), id_index);
        let id_str = id;
        batch_ids.push(id_str);
        batch_documents.push(paragraph);
        batch_embeddings.push(response);
        if batch_index == batch_size {
            let collection_entries = CollectionEntries {
                ids: batch_ids.iter().map(|x| x.as_str()).collect::<Vec<&str>>(),
                embeddings: Some(batch_embeddings.clone()),
                metadatas: None,
                documents: Some(batch_documents.clone()),
            };
            collection.upsert(collection_entries, None).unwrap();
            batch_ids.clear();
            batch_documents.clear();
            batch_embeddings.clear();
            batch_index = 0;
        }
        batch_index += 1;
    }

    pretty_message(&format!("Embedded the file: {}", path.display()));
}
