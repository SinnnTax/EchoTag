use anyhow::{ Context, bail };
use std::io::{ self, Write };

pub fn setup_provider() -> anyhow::Result<()> {
    println!("Which provider are you using? (gemini, openai, anthropic, ollama)");
    print!("> ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).context("Failed to read input")?;
    let provider = input.trim().to_lowercase();

    if !["gemini", "openai", "anthropic", "ollama"].contains(&provider.as_str()) {
        bail!("Invalid provider. Please choose from [gemini, openai, anthropic, ollama].");
    }

    let provider_line = format!("AI_PROVIDER={}", provider);
    let key_line;

    if provider == "ollama" {
        println!("\nEnter your local Ollama model name (e.g., llama3, mistral, qwen:7b):");
        print!("> ");
        io::stdout().flush()?;

        input.clear();
        io::stdin().read_line(&mut input).context("Failed to read input")?;
        let model_name = input.trim();

        if model_name.is_empty() {
            bail!("Ollama model name cannot be empty.");
        }
        key_line = format!("OLLAMA_MODEL={}", model_name);
    } else {
        println!("\nPaste your {} API key:", provider);
        print!("> ");
        io::stdout().flush()?;

        input.clear();
        io::stdin().read_line(&mut input).context("Failed to read input")?;
        let api_key = input.trim();

        key_line = format!("{}_API_KEY={}", provider.to_uppercase(), api_key);
    }

    let file_content = format!("{}\n{}\n", provider_line, key_line);
    std::fs::write(".env", file_content).context("Failed to write .env file")?;

    println!("\nSuccess! Configuration saved to .env");
    Ok(())
}
