use std::path::PathBuf;

pub enum Command {
    Chat { message: String },
    Attach { paths: Vec<PathBuf> },
    Exit,
}

impl From<&str> for Command {
    fn from(value: &str) -> Self {
        // Get the first word
        let cmd = value.split_whitespace().next().unwrap();

        match cmd.to_lowercase().as_str() {
            "/attach" => Command::Attach {
                paths: value
                    .split_whitespace()
                    .skip(1)
                    .map(PathBuf::from)
                    .collect(),
            },
            "/exit" => Command::Exit,
            "exit" => Command::Exit,
            "bye" => Command::Exit,
            _ => Command::Chat {
                message: value.to_string(),
            },
        }
    }
}
