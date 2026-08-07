use std::path::PathBuf;
use clap::{ Parser, Subcommand };

#[derive(Parser, Debug)]
#[command(name = "echotag")]
#[command(about = "Downloads YouTube audio and tags it with official metadata", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Download and tag audio from YouTube
    Download {
        /// YouTube URLs to download (you can pass multiple)
        #[arg(num_args = 1.., required = true)]
        urls: Vec<String>,

        /// Path to youtube cookies.txt file
        #[arg(short, long)]
        cookies: PathBuf,

        /// Skips metadata verification
        #[arg(short, long)]
        skip_metadata_verification: bool,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_download_command_with_urls_and_cookies() {
        let args = vec![
            "echotag",
            "download",
            "-c",
            "cookies.txt",
            "https://youtube.com/watch?v=123",
            "https://youtube.com/watch?v=456"
        ];

        let cli = Cli::try_parse_from(args).expect("Parsing valid download command should succeed");

        match cli.command {
            Command::Download { urls, cookies, skip_metadata_verification } => {
                assert_eq!(urls.len(), 2, "Should have parsed 2 URLs");
                assert_eq!(urls[0], "https://youtube.com/watch?v=123");
                assert_eq!(cookies, PathBuf::from("cookies.txt"));
                assert!(!skip_metadata_verification);
            }
            _ => panic!("Expected Download command, got something else"),
        }
    }

    #[test]
    fn parse_config_command_with_flags() {
        let args = vec!["echotag", "config", "--setup-model"];

        let cli = Cli::try_parse_from(args).expect("Parsing valid config command should succeed");

        match cli.command {
            Command::Config { setup_model, setup_cache_server } => {
                assert!(setup_model, "setup_model should be true");
                assert!(!setup_cache_server, "setup_cache_server should be false");
            }
            _ => panic!("Expected Config command, got something else"),
        }
    }

    #[test]
    fn parse_download_fails_without_urls() {
        let args = vec!["echotag", "download", "-c", "cookies.txt"];

        let result = Cli::try_parse_from(args);

        assert!(result.is_err(), "Parsing should fail when required URLs are missing");
    }
}
