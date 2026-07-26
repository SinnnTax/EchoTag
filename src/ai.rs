use anyhow::{ Context, bail };
use rig::client::{ CompletionClient, ProviderClient };
use rig::completion::CompletionModel;
use rig::agent::Agent;
use rig::providers;
use std::env;

pub async fn start_chat() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let provider_str = env
        ::var("AI_PROVIDER")
        .context("AI_PROVIDER not found in .env. Run `echotag config --set-api-key` first.")?;

    match provider_str.as_str() {
        "openai" => {
            let client = providers::openai::Client
                ::from_env()
                .context("OPENAI_API_KEY not found in environment")?;

            let agent = client
                .agent("gpt-5-mini")
                .preamble("You are a helpful music recommendation engine.")
                .build();

            run_chat_loop(agent).await
        }
        "gemini" => {
            let client = providers::gemini::Client
                ::from_env()
                .context("GEMINI_API_KEY not found in environment")?;

            let agent = client
                .agent("gemini-3.6-flash")
                .preamble("You are a helpful music recommendation engine.")
                .build();

            run_chat_loop(agent).await
        }
        "anthropic" => {
            let client = providers::anthropic::Client
                ::from_env()
                .context("ANTHROPIC_API_KEY not found in environment")?;

            let agent = client
                .agent("claude-haiku-4-5")
                .preamble("You are a helpful music recommendation engine.")
                .build();

            run_chat_loop(agent).await
        }
        _ => bail!("Unsupported AI_PROVIDER: {}", provider_str),
    }
}

async fn run_chat_loop<M: CompletionModel>(agent: Agent<M>) -> anyhow::Result<()> {
    println!("Successfully connected to the model");

    Ok(())
}
