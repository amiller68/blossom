use std::io::Cursor;
use std::ops::Deref;
use std::path::PathBuf;

use base64::prelude::*;
use futures::StreamExt;
use image::{io::Reader as ImageReader, ImageFormat};
use ollama_rs::{
    generation::{
        chat::{request::ChatMessageRequest, ChatMessage, MessageRole},
        completion::{request::GenerationRequest, GenerationResponseStream},
        images::Image,
    },
    Ollama,
};
use url::Url;



lazy_static::lazy_static! {
    static ref SUPERVISOR_SYSTEM_PROMPT: String = include_str!("../supervisor.txt").to_string();
    static ref CONVERSATIONAL_SYSTEM_PROMPT: String = include_str!("../conversational.txt").to_string();
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
            ollama: Ollama::new(host, port),
        }
    }

    pub async fn embed(&self, input: &str) -> Result<Vec<f64>, OllamaEngineError> {
        tracing::info!("embedding input: {}", input);
        let response = self
            .generate_embeddings(
                "blossom-conversational".to_string(),
                input.to_string(),
                None,
            )
            .await?;
        Ok(response.embeddings)
    }

    pub async fn respond(&self, input: &str) -> Result<String, OllamaEngineError> {
        tracing::info!("responding to input: {}", input);
        // Build a new chat message request
        let system_prompt_message = ChatMessage::new(
            MessageRole::System,
            CONVERSATIONAL_SYSTEM_PROMPT.to_string(),
        );
        let chat_message = ChatMessage::new(MessageRole::User, input.to_string());

        let request = ChatMessageRequest::new(
            "blossom-conversational".to_string(),
            vec![system_prompt_message, chat_message],
        );

        let chat_message_response = self.send_chat_messages(request).await?;
        let response = chat_message_response.message.unwrap();
        tracing::info!("response: {}", response.content);
        Ok(response.content)
    }

    // TODO: Streaming
    pub async fn image(
        &self,
        // Eventually just make this a URL
        image_path: &PathBuf,
    ) -> Result<String, OllamaEngineError> {
        tracing::info!("analysing image path: {:?}", image_path);
        let image = ImageReader::open(image_path).unwrap().decode().unwrap();
        let mut buf = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
            .unwrap();
        let base64_image = BASE64_STANDARD.encode(&buf);

        let image = Image::from_base64(&base64_image);
        let request = GenerationRequest::new(
            "blossom-image".to_string(),
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

    /*
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
    */
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
