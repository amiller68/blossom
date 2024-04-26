use ollama_rs::generation::completion::GenerationContext;
use tokio::io::{stdout, AsyncWriteExt};
use tokio_stream::StreamExt;
// use uuid::Uuid;

use crate::database::models::Chat as ChatModel;
use crate::state::State;

use std::io::{self, Write};

pub struct Chat {
    //    id: Uuid,
    //    name: String,
    context: Option<GenerationContext>,
}

enum ChatCommand {
    Respond { input: String },
    Exit,
}

impl From<&str> for ChatCommand {
    fn from(value: &str) -> Self {
        match value {
            "/exit" => Self::Exit,
            "exit" => Self::Exit,
            _ => Self::Respond {
                input: value.to_string(),
            },
        }
    }
}

impl Chat {
    pub async fn create(state: &State) -> Result<(), ChatError> {
        let mut conn = state.sqlite_database().begin().await?;
        let id = ChatModel::create(&mut conn).await?;
        println!("Chat ID: {}", id);
        Ok(())
    }

    pub async fn list(state: &State) -> Result<(), ChatError> {
        let mut conn = state.sqlite_database().acquire().await?;
        let chats = ChatModel::read_all(&mut conn).await?;
        for chat in chats {
            println!("ID: {} | Name: {}", chat.id(), chat.name());
        }
        Ok(())
    }

    pub async fn load(_name: &str, _state: &State) -> Result<Self, ChatError> {
        // let mut conn = state.sqlite_database().acquire().await?;
        // let chat = ChatModel::read_by_name(name, &mut conn).await?;
        Ok(Self {
            //       id: chat.id(),
            //       name: chat.name().to_string(),
            context: None,
        })
    }

    pub async fn run(&mut self, state: &State) -> Result<(), ChatError> {
        let engine = state.engine();
        loop {
            print!("You: ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let input = input.trim();
            let chat_command = ChatCommand::from(input);
            println!("Bot: ");
            match chat_command {
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
}

#[derive(Debug, thiserror::Error)]
pub enum ChatError {
    #[error("engine failed: {0}")]
    Ollama(#[from] crate::engine::OllamaEngineError),
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::error::Error),
}
