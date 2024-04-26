use clap::{command, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    // Create a new chat
    New {
        #[clap(long = "name", short = 'n')]
        maybe_name: Option<String>,
    },
    // List all chats
    Ls,
    // Continue a chat
    Cont {
        #[clap(long, short)]
        name: String,
    },
}
