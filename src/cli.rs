use std::path::PathBuf;

use clap::{command, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    // Add a new document to a collection
    Add {
        #[clap(long, short)]
        path: PathBuf,
        #[clap(long, short)]
        collection: String,
    },
    // Create a new chat
    New,
    // List all chats
    Ls,
    // Continue a chat
    Cont {
        #[clap(long, short)]
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum AddCommand {
    Image {
        #[clap(long, short)]
        path: PathBuf,
        #[clap(long, short)]
        collection: String,
    },
    Text {
        #[clap(long, short)]
        path: PathBuf,
        #[clap(long, short)]
        collection: String,
    },
}
