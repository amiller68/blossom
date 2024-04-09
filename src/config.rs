use dotenvy::dotenv;
use std::env;

use url::Url;

#[derive(Debug)]
pub struct Config {
    // Database Config
    sqlite_database_url: Url,

    // Chroma Config
    chroma_database_url: Url,
    chroma_collection_name: String,

    // Ollama Config
    ollama_server_url: Url,
    ollama_supervisor_model: String,
    ollama_conversational_model: String,
    ollama_image_model: String,
    ollama_embedding_model: String,
}

// TODO: arg parsing
impl Config {
    pub fn parse_env() -> Result<Config, ConfigError> {
        if dotenv().is_err() {
            tracing::warn!("No .env file found");
        }

        let sqlite_database_url_str = match env::var("SQLITE_DATABASE_URL") {
            Ok(url) => url,
            Err(e) => {
                tracing::warn!("No SQLITE_DATABASE_URL found in .env");
                return Err(ConfigError::InvalidEnv(e));
            }
        };
        let sqlite_database_url = Url::parse(&sqlite_database_url_str)?;

        let chroma_database_url_str = match env::var("CHROMA_DATABASE_URL") {
            Ok(url) => url,
            Err(_) => {
                tracing::warn!("No CHROMA_DATABASE_URL found in .env, using default");
                "http://localhost:8000".to_string()
            }
        };
        let chroma_database_url = Url::parse(&chroma_database_url_str)?;
        let chroma_collection_name = match env::var("CHROMA_COLLECTION_NAME") {
            Ok(name) => name,
            Err(_) => {
                tracing::warn!("No CHROMA_COLLECTION_NAME found in .env, using default");
                "blossom-embeddings".to_string()
            }
        };

        let ollama_server_url_str = match env::var("OLLAMA_SERVER_URL") {
            Ok(url) => url,
            Err(_) => {
                tracing::warn!("No OLLAMA_SERVER_URL found in .env, using default");
                "http://localhost:11434".to_string()
            }
        };
        let ollama_server_url = Url::parse(&ollama_server_url_str)?;

        let ollama_supervisor_model = match env::var("OLLAMA_SUPERVISOR_MODEL") {
            Ok(model) => model,
            Err(_) => {
                tracing::warn!("No OLLAMA_SUPERVISOR_MODEL found in .env, using default");
                "blossom-supervisor".to_string()
            }
        };

        let ollama_conversational_model = match env::var("OLLAMA_CONVERSATIONAL_MODEL") {
            Ok(model) => model,
            Err(_) => {
                tracing::warn!("No OLLAMA_CONVERSATIONAL_MODEL found in .env, using default");
                "blossom-conversational".to_string()
            }
        };

        let ollama_image_model = match env::var("OLLAMA_IMAGE_MODEL") {
            Ok(model) => model,
            Err(_) => {
                tracing::warn!("No OLLAMA_IMAGE_MODEL found in .env, using default");
                "blossom-image".to_string()
            }
        };

        let ollama_embedding_model = match env::var("OLLAMA_EMBEDDING_MODEL") {
            Ok(model) => model,
            Err(_) => {
                tracing::warn!("No OLLAMA_EMBEDDING_MODEL found in .env, using default");
                "blossom-embedding".to_string()
            }
        };

        Ok(Config {
            sqlite_database_url,
            chroma_database_url,
            chroma_collection_name,
            ollama_server_url,
            ollama_supervisor_model,
            ollama_conversational_model,
            ollama_image_model,
            ollama_embedding_model,
        })
    }

    pub fn sqlite_database_url(&self) -> &Url {
        &self.sqlite_database_url
    }

    pub fn chroma_database_url(&self) -> &Url {
        &self.chroma_database_url
    }

    pub fn chroma_collection_name(&self) -> &str {
        &self.chroma_collection_name
    }

    pub fn ollama_server_url(&self) -> &Url {
        &self.ollama_server_url
    }

    pub fn ollama_supervisor_model(&self) -> &str {
        &self.ollama_supervisor_model
    }

    pub fn ollama_conversational_model(&self) -> &str {
        &self.ollama_conversational_model
    }

    pub fn ollama_image_model(&self) -> &str {
        &self.ollama_image_model
    }

    pub fn ollama_embedding_model(&self) -> &str {
        &self.ollama_embedding_model
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("Missing Env: {0}")]
    InvalidEnv(#[from] env::VarError),
}
