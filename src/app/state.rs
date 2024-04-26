use chromadb::v1::{client::ChromaClientOptions, ChromaClient};

use crate::agent::LlmEngine;
use crate::app::Config;
use crate::database::Database;

pub struct State {
    sqlite_database: Database,
    chroma_database: ChromaClient,
    llm_engine: LlmEngine,
}

#[allow(dead_code)]
impl State {
    pub fn sqlite_database(&self) -> &Database {
        &self.sqlite_database
    }

    pub fn chroma_database(&self) -> &ChromaClient {
        &self.chroma_database
    }

    pub fn llm_engine(&self) -> &LlmEngine {
        &self.llm_engine
    }

    pub async fn from_config(config: &Config) -> Result<Self, StateSetupError> {
        let sqlite_database = Database::connect(config.sqlite_database_url()).await?;
        // TODO: Add Chroma configuration
        let chroma_connection_options = ChromaClientOptions::default();
        let chroma_database = ChromaClient::new(chroma_connection_options);

        let llm_engine = LlmEngine::new(
            config.ollama_server_url(),
            config.ollama_supervisor_model().to_string(),
            config.ollama_conversational_model().to_string(),
            config.ollama_image_model().to_string(),
            config.ollama_embedding_model().to_string(),
        );

        Ok(Self {
            sqlite_database,
            chroma_database,
            llm_engine,
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
    EngineSetup(#[from] crate::agent::LlmEngineError),
}
