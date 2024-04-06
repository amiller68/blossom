use std::ops::Deref;

use futures::StreamExt;
use ollama_rs::{
    generation::{
        chat::{request::ChatMessageRequest, ChatMessage, ChatMessageResponse, MessageRole},
        completion::{request::GenerationRequest, GenerationResponseStream},
        images::Image,
        options::GenerationOptions,
    },
    Ollama,
};
use url::Url;

pub use ollama_rs::generation::completion::GenerationContext;

lazy_static::lazy_static! {
    static ref SUPERVISOR_SYSTEM_PROMPT: String = include_str!("../supervisor.txt").to_string();
}

#[derive(Debug, Clone)]
pub struct OllamaEngine {
    // model_map: HashMap<String, String>,
    ollama: Ollama,
}

/// Mulitpuropse engine with access to various models
impl OllamaEngine {
    pub fn new(url: &Url) -> Self {
        let scheme = url.scheme();
        let host = url.host_str().unwrap_or("localhost");
        let port = url.port().unwrap_or(11434);
        let host = format!("{}://{}", scheme, host);
        Self {
            //   model_map,
            ollama: Ollama::new(host, port),
        }
    }

    pub async fn respond(&self, input: &str) -> Result<String, OllamaEngineError> {
        tracing::info!("responding to input: {}", input);
        // Build a new chat message request
        let system_prompt_message =
            ChatMessage::new(MessageRole::System, SUPERVISOR_SYSTEM_PROMPT.to_string());
        let chat_message = ChatMessage::new(MessageRole::User, input.to_string());

        let request = ChatMessageRequest::new(
            "blossom-supervisor".to_string(),
            vec![system_prompt_message, chat_message],
        );

        let chat_message_response = self.send_chat_messages(request).await?;
        let response = chat_message_response.message.unwrap();
        tracing::info!("response: {}", response.content);
        Ok(response.content)
    }

    pub async fn analyze_image(
        &self,
        base64_image: &str,
        context: Option<GenerationContext>,
    ) -> Result<(String, Option<GenerationContext>), OllamaEngineError> {
        let image = Image::from_base64(base64_image);

        let mut request = GenerationRequest::new(
            "blossom-image".to_string(),
            "Please analyze this image".to_string(),
        )
        .add_image(image);

        if let Some(context) = context.clone() {
            request = request.clone().context(context);
        }
        let mut stream: GenerationResponseStream =
            self.generate_stream(request.clone()).await.unwrap();
        let mut response_buffer = Vec::new();
        let mut next_context = None;
        while let Some(Ok(response)) = stream.next().await {
            for ele in response.clone() {
                response_buffer.extend(ele.response.as_bytes());

                if let Some(final_data) = ele.final_data {
                    next_context = Some(final_data.context);
                }
            }
        }
        let response = String::from_utf8(response_buffer).unwrap();
        Ok((response, next_context))
    }

    pub async fn complete(
        &self,
        input: &str,
        context: Option<GenerationContext>,
    ) -> Result<(String, Option<GenerationContext>), OllamaEngineError> {
        let input = input.trim();
        let options = GenerationOptions::default();
        let options = options.stop(vec!["<|im_end|>".to_string()]);

        let mut request =
            GenerationRequest::new("blossom-conversational".into(), input.to_string())
                .options(options);
        if let Some(context) = context.clone() {
            request = request.clone().context(context);
        }
        let mut stream: GenerationResponseStream =
            self.generate_stream(request.clone()).await.unwrap();
        let mut response_buffer = Vec::new();
        let mut next_context = None;
        while let Some(Ok(response)) = stream.next().await {
            for ele in response.clone() {
                response_buffer.extend(ele.response.as_bytes());

                if let Some(final_data) = ele.final_data {
                    next_context = Some(final_data.context);
                }
            }
        }
        let response = String::from_utf8(response_buffer).unwrap();
        Ok((response, next_context))
    }

    pub async fn complete_stream(
        &self,
        input: &str,
        context: Option<GenerationContext>,
    ) -> Result<GenerationResponseStream, OllamaEngineError> {
        let input = input.trim();
        let options = GenerationOptions::default();
        let options = options.stop(vec!["<|im_end|>".to_string()]);
        let request =
            GenerationRequest::new("nous-hermes-2-pro".into(), input.to_string()).options(options);
        if let Some(context) = context.clone() {
            request.clone().context(context);
        }
        let stream: GenerationResponseStream = self.generate_stream(request.clone()).await.unwrap();
        Ok(stream)
    }
}

impl Deref for OllamaEngine {
    type Target = Ollama;
    fn deref(&self) -> &Self::Target {
        &self.ollama
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OllamaEngineError {
    #[error("default error: {0}")]
    DefaultError(anyhow::Error),
    #[error("ollama error: {0}")]
    Ollam(#[from] ollama_rs::error::OllamaError),
}
