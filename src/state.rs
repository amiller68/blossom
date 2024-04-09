use chromadb::v1::{client::ChromaClientOptions, ChromaClient, ChromaCollection};

use crate::config::Config;
use crate::database::Database;
use crate::engine::OllamaEngine;

pub struct State {
    sqlite_database: Database,
    chroma_collection: ChromaCollection,
    engine: OllamaEngine,
}

#[allow(dead_code)]
impl State {
    pub fn sqlite_database(&self) -> &Database {
        &self.sqlite_database
    }

    pub fn chroma_collection(&self) -> &ChromaCollection {
        &self.chroma_collection
    }

    pub fn engine(&self) -> &OllamaEngine {
        &self.engine
    }

    pub async fn from_config(config: &Config) -> Result<Self, StateSetupError> {
        let sqlite_database = Database::connect(config.sqlite_database_url()).await?;
        let chroma_database = ChromaClient::new(ChromaClientOptions::default());
        let chroma_collection =
            chroma_database.create_collection(config.chroma_collection_name(), None, true)?;

        let engine = OllamaEngine::new(
            config.ollama_server_url(),
            config.ollama_supervisor_model().to_string(),
            config.ollama_conversational_model().to_string(),
            config.ollama_image_model().to_string(),
            config.ollama_embedding_model().to_string(),
        );

        Ok(Self {
            sqlite_database,
            chroma_collection,
            engine,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StateSetupError {
    #[error("failed to connect chroma database: {0}")]
    ChromaDatabase(#[from] anyhow::Error),
    #[error("failed to setup the database: {0}")]
    DatabaseSetup(#[from] crate::database::DatabaseSetupError),
    #[error("failed to setup the Chroma database: {0}")]
    EngineSetup(#[from] crate::engine::OllamaEngineError),
}
