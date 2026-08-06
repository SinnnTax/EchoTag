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

    let mut cache_server_ip = String::new();
    if let Ok(content) = std::fs::read_to_string(".env") {
        let splitted: Vec<&str> = content.split("CACHE_SERVER_IP=").collect();

        if splitted.len() == 2 {
            cache_server_ip = splitted[1].to_string();
        }
    }

    let file_content = if cache_server_ip.is_empty() {
        format!("{}\n{}", provider_line, key_line)
    } else {
        format!("{}\n{}\nCACHE_SERVER_IP={}", provider_line, key_line, cache_server_ip)
    };

    std::fs::write(".env", file_content).context("Failed to write .env file")?;

    println!("\nSuccess! Configuration saved to .env");
    Ok(())
}

pub fn setup_cache_server() -> anyhow::Result<()> {
    let env_file = match std::fs::read_to_string("./.env") {
        Ok(s) => s,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                String::new()
            } else {
                bail!("Couldn't read .env file")
            }
        }
    };

    let mut splitted_env: Vec<&str> = env_file.split("CACHE_SERVER_IP=").collect();
    if splitted_env.len() != 2 {
        println!("No cache server IP configured yet.");
        print!("Cache server ip: ");
    } else {
        println!("Current cache server IP: {}", splitted_env[1]);
        splitted_env.pop();
        print!("New cache server IP: ");
    }

    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let new_ip = input.trim();

    if new_ip.is_empty() {
        bail!("Cache server IP cannot be empty.");
    }

    splitted_env.push("CACHE_SERVER_IP=");
    splitted_env.push(&input.trim());

    let env_content = splitted_env.join("");
    std::fs::write(".env", env_content).context("Couldn't write to env file")?;

    Ok(())
}
