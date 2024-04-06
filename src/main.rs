use std::path::PathBuf;

use chromadb::v1::collection::CollectionEntries;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

use blossom::{Config, State};

enum Command {
    Respond,
    Embed,
    Search,
    Image,
}

impl TryFrom<&str> for Command {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "respond" => Ok(Self::Respond),
            "embed" => Ok(Self::Embed),
            "search" => Ok(Self::Search),
            "image" => Ok(Self::Image),
            _ => Err("Invalid command"),
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

    // Get the input from the command line
    let args: Vec<String> = std::env::args().collect();
    // Get the command
    let cmd = args[1].clone();
    println!("{:?}", cmd);
    let command = Command::try_from(cmd.as_str()).unwrap();
    match command {
        Command::Respond => {
            let input = args[2..].join(" ");
            let engine = state.engine();
            let response = engine.respond(&input).await.unwrap();
            println!("{:?}", response);
        }
        Command::Embed => {
            let input = args[2].clone();
            let _input_str = input.clone();
            let path = PathBuf::from(input.clone());
            let chroma_client = state.chroma_database();
            let chroma_collection = chroma_client
                .create_collection("testing", None, true)
                .unwrap();

            // Read the data and chunk the file among paragraphs
            let data = std::fs::read_to_string(path).unwrap();
            let paragraphs = data.split("\n\n").collect::<Vec<&str>>();
            // Embed the paragraphs
            let engine = state.engine();
            let mut ids = Vec::new();
            let mut documents = Vec::new();
            let mut embeddings = Vec::new();
            for paragraph in paragraphs {
                let response = engine.embed(paragraph).await.unwrap();
                // Map the response to f32
                let response = response
                    .iter()
                    .map(|x| *x as f32)
                    .collect::<Vec<f32>>();
                let len = ids.len();
                let i = input.clone();
                let id = format!("{}-{}", i, len);
                let id_str = id;
                ids.push(id_str);
                documents.push(paragraph);
                embeddings.push(response);
            }
            let collection_entries = CollectionEntries {
                // Map Ids to vec[&str]
                //
                ids: ids.iter().map(|x| x.as_str()).collect::<Vec<&str>>(),
                embeddings: Some(embeddings),
                metadatas: None,
                documents: Some(documents),
            };

            let value = chroma_collection.upsert(collection_entries, None).unwrap();
            println!("{:?}", value);
        }
        Command::Image => {
            let input = args[2].clone();
            let path = PathBuf::from(input);
            let engine = state.engine();
            let response = engine.image(&path).await.unwrap();
            println!("{:?}", response);
        }
        Command::Search => {
            println!("todo");
        }
    }
}
