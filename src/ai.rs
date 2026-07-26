use anyhow::{ Context, bail };
use rig::client::{ CompletionClient, ProviderClient };
use rig::completion::CompletionModel;
use rig::agent::Agent;
use rig::providers;
use std::env;
use crate::history;

pub async fn start_chat() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let provider_str = env
        ::var("AI_PROVIDER")
        .context("AI_PROVIDER not found in .env. Run `echotag config --set-api-key` first.")?;

    let preamble_text = history::get_history_prompt().await?;
    println!("{preamble_text}");

    match provider_str.as_str() {
        "openai" => {
            let client = providers::openai::Client
                ::from_env()
                .context("OPENAI_API_KEY not found in environment")?;
            let agent = client.agent("gpt-4o-mini").preamble(&preamble_text).build();
            chat(agent).await
        }
        "gemini" => {
            let client = providers::gemini::Client
                ::from_env()
                .context("GEMINI_API_KEY not found in environment")?;
            let agent = client.agent("gemini-1.5-flash").preamble(&preamble_text).build();
            chat(agent).await
        }
        "anthropic" => {
            let client = providers::anthropic::Client
                ::from_env()
                .context("ANTHROPIC_API_KEY not found in environment")?;
            let agent = client.agent("claude-3-5-sonnet-20240620").preamble(&preamble_text).build();
            chat(agent).await
        }
        _ => bail!("Unsupported AI_PROVIDER: {}", provider_str),
    }
}

async fn chat<M: CompletionModel>(agent: Agent<M>) -> anyhow::Result<()> {
    println!("Successfully connected to the model");
    println!("History prompt generated");

    Ok(())
}
