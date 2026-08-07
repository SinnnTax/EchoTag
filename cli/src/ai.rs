use anyhow::{ Context, bail };
use rig::client::{ CompletionClient, ProviderClient, Nothing };
use rig::completion::{ CompletionModel, Prompt };
use rig::agent::Agent;
use rig::providers;
use std::env;
use std::io::{ self, Write };
use tokio::io::{ AsyncBufReadExt, BufReader };
use termimad::print_text;
use std::path::PathBuf;
use shared::models::YTSearchResult;
use crate::history;
use crate::youtube;

#[rig::tool_macro(
    description = "Searches YouTube for a specific song. 
Use this tool when the user asks to download, find, play, or get a link for a song. 
Returns a compact list of up to 3 matching videos. 
You must analyze the 'title' and 'channel' fields to select the official version, avoiding 'slowed', 'sped up', or 'cover' versions unless explicitly requested.

After analyzing the results, your next step depends on what the user asked:
- If the user ONLY asked for the URL or link (e.g., 'give me the url'), simply reply with the URL. Do NOT call the download tool.
- If the user asked to download, play, or save the song (e.g., 'download song 1'), extract the 'url' of the best result and call the 'download_url' tool.

CRITICAL ERROR HANDLING: If this tool returns an error, or returns 'No YouTube results found', you MUST stop and inform the user of the exact error. Do NOT try to guess or hallucinate URLs. Do NOT call the download tool if the search fails.

Parameters:
- artist_name: The official name of the artist or band (string).
- track_name: The exact title of the song (string).",
    required(artist_name, track_name)
)]
async fn search_youtube(
    artist_name: String,
    track_name: String
) -> Result<Vec<YTSearchResult>, rig::tool::ToolError> {
    let query = format!("{} - {}", artist_name, track_name);
    let cookies_path = Some(PathBuf::from("cookies.txt"));

    let results = youtube
        ::search(&query, cookies_path, None).await
        .map_err(|e| rig::tool::ToolError::ToolCallError(e.to_string().into()))?;

    Ok(results)
}

#[rig::tool_macro(
    description = "Downloads a specific YouTube video as an MP3 in the background. 
Use this tool ONLY after you have called 'search_youtube' and selected the best URL from the results. 
Do not guess URLs; only pass URLs that were returned by the search tool.

CRITICAL RULE: You must copy the EXACT URL string that was returned by the 'search_youtube' tool. Do NOT use URLs from your memory, do NOT guess, and do NOT use URLs from previous turns. Copy-paste the exact URL string from the most recent search results.

Parameters:
- url: The exact YouTube video URL string returned by the search tool.",
    required(url)
)]
async fn download_url(url: String) -> Result<String, rig::tool::ToolError> {
    let exe_path = std::env
        ::current_exe()
        .map_err(|e| rig::tool::ToolError::ToolCallError(e.to_string().into()))?;

    println!("\n");

    let status = tokio::process::Command
        ::new(exe_path)
        .arg("download")
        .arg("-c")
        .arg("cookies.txt")
        .arg(&url)
        .status().await
        .map_err(|e| rig::tool::ToolError::ToolCallError(e.to_string().into()))?;

    if status.success() {
        Ok(format!("Successfully finished downloading {}.", url))
    } else {
        Err(rig::tool::ToolError::ToolCallError("The download process failed.".to_string().into()))
    }
}

pub async fn start_chat() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let provider_str = env
        ::var("AI_PROVIDER")
        .context("AI_PROVIDER not found in .env. Run `echotag config --set-api-key` first.")?;

    let preamble_text = history::get_history_prompt(None).await?;

    match provider_str.as_str() {
        "openai" => {
            let client = providers::openai::Client
                ::from_env()
                .context("OPENAI_API_KEY not found in environment")?;
            let agent = client
                .agent("gpt-5-mini")
                .preamble(&preamble_text)
                .tool(SearchYoutube)
                .tool(DownloadUrl)
                .default_max_turns(5)
                .build();
            chat(agent).await
        }
        "gemini" => {
            let client = providers::gemini::Client
                ::from_env()
                .context("GEMINI_API_KEY not found in environment")?;
            let agent = client
                .agent("gemini-3.6-flash")
                .preamble(&preamble_text)
                .tool(SearchYoutube)
                .tool(DownloadUrl)
                .default_max_turns(5)
                .build();
            chat(agent).await
        }
        "anthropic" => {
            let client = providers::anthropic::Client
                ::from_env()
                .context("ANTHROPIC_API_KEY not found in environment")?;
            let agent = client
                .agent("claude-haiku-4-5")
                .preamble(&preamble_text)
                .tool(SearchYoutube)
                .tool(DownloadUrl)
                .default_max_turns(5)
                .build();
            chat(agent).await
        }
        "ollama" => {
            let model_name = env
                ::var("OLLAMA_MODEL")
                .context("OLLAMA_MODEL not found in .env. Run `echotag config --setup` first.")?;

            let client = providers::ollama::Client
                ::new(Nothing)
                .context(
                    "Failed to initialize Ollama client. Make sure Ollama is running at http://localhost:11434"
                )?;

            let agent = client
                .agent(&model_name)
                .preamble(&preamble_text)
                .tool(SearchYoutube)
                .tool(DownloadUrl)
                .default_max_turns(5)
                .build();
            chat(agent).await
        }
        _ => bail!("Unsupported AI_PROVIDER: {}", provider_str),
    }
}

async fn chat<M: CompletionModel + 'static>(agent: Agent<M>) -> anyhow::Result<()> {
    println!("          (Type 'exit' or 'quit' to stop)          ");
    println!("/--------------------------------------------------\\\n");

    let initial_prompt =
        "Please introduce yourself briefly and give me my 3 song recommendations based on my history. \
        Tell the user they can ask you to search for and download any of them.";

    match agent.prompt(initial_prompt).await {
        Ok(response) => {
            print_text(&response);
            println!();
        }
        Err(e) => {
            eprintln!("\n[AI Error]: {:?}\n", e);
        }
    }

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut input = String::new();

    loop {
        print!("-> ");
        io::stdout().flush()?;

        input.clear();
        if reader.read_line(&mut input).await.is_err() || input.is_empty() {
            break;
        }

        let trimmed = input.trim();

        if trimmed.eq_ignore_ascii_case("exit") || trimmed.eq_ignore_ascii_case("quit") {
            println!("Goodbye!");
            break;
        }

        if trimmed.is_empty() {
            continue;
        }

        match agent.prompt(trimmed).await {
            Ok(response) => {
                print_text(&response);
                println!();
            }
            Err(e) => {
                eprintln!("\n[AI Error]: {:?}\n", e);
            }
        }
    }

    Ok(())
}
