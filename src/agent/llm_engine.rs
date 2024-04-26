use std::io::Cursor;
use std::ops::Deref;
use std::path::PathBuf;

use base64::prelude::*;
use futures::StreamExt;
use image::{io::Reader as ImageReader, ImageFormat};
use ollama_rs::{
    generation::{
        chat::{request::ChatMessageRequest, ChatMessage, MessageRole},
        completion::{request::GenerationRequest, GenerationContext, GenerationResponseStream},
        images::Image,
        options::GenerationOptions,
    },
    Ollama,
};
use url::Url;

use super::tool_call::ToolCall;

lazy_static::lazy_static! {
    static ref SUPERVISOR_SYSTEM_PROMPT: String = include_str!("../../supervisor.txt").to_string();
    static ref CONVERSATIONAL_SYSTEM_PROMPT: String = include_str!("../../conversational.txt").to_string();
}

#[derive(Debug, Clone)]
pub struct LlmEngine {
    // model_map: HashMap<String, String>,
    ollama: Ollama,

    supervisor_model: String,
    conversational_model: String,
    image_model: String,
    embedding_model: String,
}

/// Mulitpuropse engine with access to various models
impl LlmEngine {
    pub fn new(
        url: &Url,
        supervisor_model: String,
        conversational_model: String,
        image_model: String,
        embedding_model: String,
    ) -> Self {
        let scheme = url.scheme();
        let host = url.host_str().unwrap_or("localhost");
        let port = url.port().unwrap_or(11434);
        let host = format!("{}://{}", scheme, host);
        Self {
            ollama: Ollama::new(host, port),

            supervisor_model,
            conversational_model,
            image_model,
            embedding_model,
        }
    }

    pub async fn embed(&self, input: &str) -> Result<Vec<f64>, LlmEngineError> {
        let response = self
            .generate_embeddings(self.embedding_model.clone(), input.to_string(), None)
            .await?;
        Ok(response.embeddings)
    }

    pub async fn handle(&self, input: &str) -> Result<ToolCall, LlmEngineError> {
        // Build a new chat message request
        let system_prompt_message =
            ChatMessage::new(MessageRole::System, SUPERVISOR_SYSTEM_PROMPT.to_string());
        let chat_message = ChatMessage::new(MessageRole::User, input.to_string());
        let request = ChatMessageRequest::new(
            self.supervisor_model.clone(),
            vec![system_prompt_message, chat_message],
        );

        let chat_message_response = self.send_chat_messages(request).await?;
        let response = match chat_message_response.message {
            None => return Err(LlmEngineError::NoMessageError),
            Some(response) => response,
        };
        let tool_call = match ToolCall::try_from(response.content.as_str()) {
            Ok(tool_call) => tool_call,
            Err(e) => {
                tracing::error!("Received unparsable tool call: {}", response.content);
                tracing::error!("Failed to parse tool call: {}", e);
                return Err(LlmEngineError::ToolCallError(e));
            }
        };
        Ok(tool_call)
    }

    // TODO: Streaming
    pub async fn image(
        &self,
        // Eventually just make this a URL
        image_path: &PathBuf,
    ) -> Result<String, LlmEngineError> {
        let image = ImageReader::open(image_path).unwrap().decode().unwrap();
        let mut buf = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .map_err(|e| LlmEngineError::DefaultError(e.into()))?;
        let base64_image = BASE64_STANDARD.encode(&buf);

        let image = Image::from_base64(&base64_image);
        let request = GenerationRequest::new(
            self.image_model.clone(),
            "Please analyze this image".to_string(),
        )
        .add_image(image);

        let mut stream: GenerationResponseStream =
            self.generate_stream(request.clone()).await.unwrap();
        let mut response_buffer = Vec::new();
        while let Some(Ok(response)) = stream.next().await {
            for ele in response.clone() {
                response_buffer.extend(ele.response.as_bytes());
            }
        }
        let response = String::from_utf8(response_buffer).unwrap();
        Ok(response)
    }

    pub async fn converse(
        &self,
        input: &str,
        context: Option<GenerationContext>,
    ) -> Result<GenerationResponseStream, LlmEngineError> {
        let input = input.trim();
        let options = GenerationOptions::default();
        let request = GenerationRequest::new(self.conversational_model.clone(), input.to_string())
            .options(options);
        if let Some(context) = context.clone() {
            request.clone().context(context);
        }
        let stream: GenerationResponseStream = self.generate_stream(request.clone()).await?;
        Ok(stream)
    }
}

impl Deref for LlmEngine {
    type Target = Ollama;
    fn deref(&self) -> &Self::Target {
        &self.ollama
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LlmEngineError {
    #[error("default error: {0}")]
    DefaultError(anyhow::Error),
    #[error("tool call error: {0}")]
    ToolCallError(#[from] super::tool_call::ToolCallError),
    #[error("ollama error: {0}")]
    Ollam(#[from] ollama_rs::error::OllamaError),
    #[error("image error: {0}")]
    ImageError(#[from] image::ImageError),
    #[error("no message error")]
    NoMessageError,
}
