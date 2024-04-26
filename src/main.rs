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

fn print_message(message: &str) {
    println!("🌸 {}", message);
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::error::Error),
    #[error("engine error: {0}")]
    Engine(#[from] blossom::agent::LlmEngineError),
    #[error("chroma error: {0}")]
    Chroma(#[from] chromadb::v1::Error),
}

/* App scripting */

use names::Generator;
use ollama_rs::generation::completion::GenerationContext;
use tokio::io::{stdout, AsyncWriteExt};
use tokio_stream::StreamExt;

use std::io::{self, Write};

async fn handle_command(state: State, command: Command) -> Result<(), AppError> {
    match command {
        Command::New { maybe_name } => {
            let name = maybe_name.unwrap_or_else(|| Generator::default().next().unwrap());
            let mut conn = state.sqlite_database().begin().await?;
            let id = ChatModel::create(&name, &mut conn).await?;
            conn.commit().await?;
            print_message(&format!("Created new chat named '{}' with ID {}", name, id));
        }
        Command::Ls => {
            let mut conn = state.sqlite_database().acquire().await?;
            let chats = ChatModel::read_all(&mut conn).await?;
            for chat in chats {
                print_message(&format!("ID: {} | Name: {}", chat.id(), chat.name()));
            }
        }
        Command::Cont { name } => {
            let mut conn = state.sqlite_database().acquire().await?;
            let chat = ChatModel::read_by_name(&name, &mut conn).await?;
            run(&chat, &state).await?;
        }
    }
    Ok(())
}

async fn run(chat: &ChatModel, state: &State) -> Result<(), AppError> {
    let chat_id = chat.id();
    let chat_name = chat.name();
    let engine = state.llm_engine();
    let chroma_database = state.chroma_database();
    let sqlite_database = state.sqlite_database();
    let mut generation_context = None;

    loop {
        print!(">>> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        let chat_command = ChatCommand::from(input);

        match chat_command {
            ChatCommand::Attach { paths } => {
                let collection = get_collection(chat_name, chroma_database.clone())?;
                for path in paths {}
            }
            ChatCommand::Respond { input } => {
                let tool_call = engine.handle(&input).await.unwrap();
                match tool_call.name() {
                    "converse" => {
                        let args = tool_call.args();
                        if args.len() != 1 {
                            println!("[ERROR]: Expected 1 argument, got {}", args.len());
                            continue;
                        }
                        let input = args.first().unwrap();
                        assert_eq!(input.name(), "input");
                        assert_eq!(input.r#type(), "String");
                        let input = input.value();
                        let mut stdout = stdout();
                        let mut stream = engine.converse(input, self.context.clone()).await?;
                        while let Some(Ok(res)) = stream.next().await {
                            for ele in res {
                                stdout.write_all(ele.response.as_bytes()).await.unwrap();
                                stdout.flush().await.unwrap();

                                if let Some(final_data) = ele.final_data {
                                    self.context = Some(final_data.context);
                                }
                            }
                        }
                        println!();
                    }
                    _ => {
                        print!("Unknown tool call: {:?}", tool_call.name());
                    }
                }
            }
            ChatCommand::Exit => {
                print!("Goodbye!");
                break;
            }
        }
    }
    Ok(())
}

fn get_collection(
    name: &str,
    chroma_database: chromadb::v1::ChromaClient,
) -> Result<chromadb::v1::Collection, AppError> {
    let collection = chroma_database.create_collection(name, None, true)?;
    Ok(collection)
}
