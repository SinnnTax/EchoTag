use std::path::PathBuf;
use clap::{ Parser, Subcommand };

#[derive(Parser)]
#[command(name = "echotag")]
#[command(about = "Downloads YouTube audio and tags it with official metadata", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Download and tag audio from YouTube
    Download {
        /// YouTube URLs to download (you can pass multiple)
        #[arg(num_args = 1.., required = true)]
        urls: Vec<String>,

        /// Path to youtube cookies.txt file
        #[arg(short, long)]
        cookies: PathBuf,
    },
    /// Update tags for existing audio files
    Update {
        /// Paths to audio files to update
        #[arg(num_args = 1.., required = true)]
        paths: Vec<PathBuf>,
    },
    /// Configure AI provider and cache server settings
    Config {
        /// Set up or change the AI provider (Gemini, OpenAI, Anthropic, Ollama) and API key/model
        #[arg(long)]
        setup_model: bool,

        /// Set up or change the cache server IP address
        #[arg(long)]
        setup_cache_server: bool,
    },
    /// Chat with the AI to get music recommendations
    Chat,
}
