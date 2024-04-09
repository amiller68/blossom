use std::path::PathBuf;

use chromadb::v1::collection::{CollectionEntries, QueryOptions, QueryResult};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

use blossom::{Config, State};

use std::io::{self, Write};

enum Command {
    Respond,
    Embed,
    Search,
    Image,
    Exit,
}

impl From<&str> for Command {
    fn from(value: &str) -> Self {
        match value {
            "/embed" => Self::Embed,
            "/search" => Self::Search,
            "/image" => Self::Image,
            "/exit" => Self::Exit,
            _ => Self::Respond,
        }
    }
}

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

    loop {
        print!("You: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input == "exit" {
            println!("Goodbye!");
            break;
        }

        println!("Bot: {:?}", input);
        handle_input(input.to_string(), &state).await;
    }
}

async fn handle_input(input: String, state: &State) {
    let engine = state.engine();
    let chroma = state.chroma_database();

    // Split off the command from the input
    let command_str = input.split_whitespace().next().unwrap_or("");
    let command = Command::from(command_str);
    match command {
        Command::Respond => {
            let response = engine.respond(&input).await.unwrap();
            println!("{:?}", response);
        }
        Command::Embed => {
            let paths = input.split_whitespace().skip(1);
            for path in paths {
                println!("Embedding: {:?}", path);
                let path = PathBuf::from(path);
                embed_path(path, state).await;
            }
        }
        Command::Image => {
            let paths = input.split_whitespace().skip(1);
            for path in paths {
                println!("Analzying image: {:?}", path);
                let path = PathBuf::from(path);
                let engine = state.engine();
                let response = engine.image(&path).await.unwrap();
                println!("{:?}", response);
            }
        }
        Command::Search => {
            // Take the rest of the input as the query
            let input = input
                .split_whitespace()
                .skip(1)
                .collect::<Vec<&str>>()
                .join(" ");
            let collection = chroma.create_collection("testing", None, true).unwrap();

            let query_embedding = engine.embed(&input).await.unwrap();
            // Map the response to f32
            let query_embedding = query_embedding
                .iter()
                .map(|x| *x as f32)
                .collect::<Vec<f32>>();

            let query = QueryOptions {
                query_texts: None,
                query_embeddings: Some(vec![query_embedding]),
                where_metadata: None,
                where_document: None,
                n_results: Some(5),
                include: None,
            };

            let query_result: QueryResult = collection.query(query, None).unwrap();
            println!("Query result: {:?}", query_result);
        }
        Command::Exit => {
            println!("Goodbye!");
            std::process::exit(0);
        }
    }
}

async fn embed_path(path: PathBuf, state: &State) {
    let engine = state.engine();
    let chroma_client = state.chroma_database();
    let chroma_collection = chroma_client
        .create_collection("testing", None, true)
        .unwrap();

    // If the path is a directory, panic
    if path.is_dir() {
        println!("Sorry, i can't embed directories yet");
        return;
    }

    // Check if the extension is not a text file
    if path.extension().unwrap_or_default() != "txt" {
        println!("Sorry, i can only embed text files");
        return;
    }

    // Read the data and chunk the file among paragraphs
    let data = std::fs::read_to_string(path.clone()).unwrap();
    let paragraphs = data.split("\n\n").collect::<Vec<&str>>();

    // Embed the paragraphs in batches
    let mut batch_index = 0;
    let batch_size = 5;
    let mut batch_ids = Vec::new();
    let mut batch_documents = Vec::new();
    let mut batch_embeddings = Vec::new();
    for (id_index, paragraph) in paragraphs.into_iter().enumerate() {
        let response = engine.embed(paragraph).await.unwrap();
        println!("embedding:  {:?}", response);
        // Map the response to f32
        let response = response.iter().map(|x| *x as f32).collect::<Vec<f32>>();
        let i = path.clone();
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
            let value = chroma_collection.upsert(collection_entries, None).unwrap();
            println!("{:?}", value);
            batch_ids.clear();
            batch_documents.clear();
            batch_embeddings.clear();
            batch_index = 0;
        }
        batch_index += 1;
    }
}
