use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, ValueEnum)]
pub enum Logs {
    Off,
    Stdout,
    File,
}

#[derive(Parser, Serialize, Deserialize, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Enable app logs
    #[arg(short, long)]
    pub logs: Option<Logs>,
}
