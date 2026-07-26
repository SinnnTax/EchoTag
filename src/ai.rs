use anyhow::{ Context, bail };
use rig::client::{ CompletionClient, ProviderClient };
use rig::completion::{ CompletionModel, Prompt };
use rig::agent::Agent;
use rig::providers;
use std::env;
use std::io::{ self, Write };
use tokio::io::{ AsyncBufReadExt, BufReader };
use termimad::print_text;
use crate::history;

pub async fn start_chat() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let provider_str = env
        ::var("AI_PROVIDER")
        .context("AI_PROVIDER not found in .env. Run `echotag config --set-api-key` first.")?;

    let preamble_text = history::get_history_prompt().await?;

    match provider_str.as_str() {
        "openai" => {
            let client = providers::openai::Client
                ::from_env()
                .context("OPENAI_API_KEY not found in environment")?;
            let agent = client.agent("gpt-5-mini").preamble(&preamble_text).build();
            chat(agent).await
        }
        "gemini" => {
            let client = providers::gemini::Client
                ::from_env()
                .context("GEMINI_API_KEY not found in environment")?;
            let agent = client.agent("gemini-3.6-flash").preamble(&preamble_text).build();
            chat(agent).await
        }
        "anthropic" => {
            let client = providers::anthropic::Client
                ::from_env()
                .context("ANTHROPIC_API_KEY not found in environment")?;
            let agent = client.agent("claude-haiku-4-5").preamble(&preamble_text).build();
            chat(agent).await
        }
        _ => bail!("Unsupported AI_PROVIDER: {}", provider_str),
    }
}

async fn chat<M: CompletionModel + 'static>(agent: Agent<M>) -> anyhow::Result<()> {
    println!("          (Type 'exit' or 'quit' to stop)          ");
    println!("/--------------------------------------------------\\\n");

    let initial_prompt =
        "Please introduce yourself briefly and give me my 3 song recommendations based on my history.";

    match agent.prompt(initial_prompt).await {
        Ok(response) => {
            print_text(&response); // Renders the markdown!
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
                print_text(&response); // Renders the markdown!
                println!();
            }
            Err(e) => {
                eprintln!("\n[AI Error]: {:?}\n", e);
            }
        }
    }

    Ok(())
}
