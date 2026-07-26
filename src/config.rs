use anyhow::{ Context, bail };
use std::io::{ self, Write };

pub fn set_api_key() -> anyhow::Result<()> {
    println!("Which provider are you using? (gemini, openai, anthropic)");
    print!("> ");
    io::stdout().flush()?;

    let mut input = String::new();

    io::stdin().read_line(&mut input).context("Failed to read input")?;

    let mut provider = input.trim().to_lowercase();

    if !["gemini", "openai", "anthropic"].contains(&provider.as_str()) {
        bail!("Invalid provider. Please choose from [gemini, openai, anthropic].");
    }

    println!("\nPaste your {} API key:", provider);
    print!("> ");
    io::stdout().flush()?;

    input.clear();
    io::stdin().read_line(&mut input).context("Failed to read input")?;

    let api_key = input.trim().to_string();

    let provider_line = format!("AI_PROVIDER={}", provider);
    let key_line = format!("{}_API_KEY={}", provider.to_uppercase(), api_key);

    let file_content = format!("{}\n{}\n", provider_line, key_line);
    std::fs::write(".env", file_content).context("Failed to write .env file")?;

    println!("\nSuccess! Configuration saved to .env");
    Ok(())
}
