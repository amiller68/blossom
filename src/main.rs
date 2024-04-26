use clap::Parser;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

use blossom::{Chat, Config, State};

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
    match args.command {
        Command::New => {
            Chat::create(&state).await.expect("Failed to create chat");
        }
        Command::Ls => {
            Chat::list(&state)
                .await
                .expect("Failed to list collections");
        }
        Command::Cont { name } => {
            let mut chat = Chat::load(&name, &state)
                .await
                .expect("Failed to load chat");
            chat.run(&state).await.expect("Failed to run chat");
        }
        _ => {}
    }
}
