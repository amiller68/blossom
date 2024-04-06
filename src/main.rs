use std::io::Cursor;

use base64::prelude::*;
use image::{io::Reader as ImageReader, ImageFormat};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

use blossom::{Config, State};

#[tokio::main]
async fn main() {
    // Get the input from the command line
    let args: Vec<String> = std::env::args().collect();
    // split off the first argument (the program name)
    let input = args[1..].join(" ");

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

    let engine = state.engine();

    let image = ImageReader::open("image.png").unwrap().decode().unwrap();
    let mut buf = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
        .unwrap();
    let base64_image = BASE64_STANDARD.encode(&buf);

    let r = engine.respond(&input).await.unwrap();
    println!("{:?}", r);
}
