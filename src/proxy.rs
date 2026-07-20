use std::path::Path;
use anyhow::{ bail, Context, Result };
use tokio::io::AsyncWriteExt;
use std::time::{ Duration, Instant };
use tokio::task::JoinSet;

#[allow(dead_code)]
#[derive(Clone)]
pub enum ProxyType {
    HTTP,
    SOCKS5,
    SOCKS4,
}

#[allow(dead_code)]
const TEST_URL: &str = "https://www.youtube.com";
#[allow(dead_code)]
const TIMEOUT: Duration = Duration::from_secs(5);

#[allow(dead_code)]
pub async fn get_proxy(url: &str, path: &Path) -> anyhow::Result<()> {
    let bytes = reqwest
        ::get(url).await
        .with_context(|| format!("Couldn't connect to {}!", &url))?
        .bytes().await
        .with_context(|| format!("Failed to fetch proxy from {}", url))?;

    let mut file = tokio::fs::File
        ::create(path).await
        .with_context(|| format!("Couldn't create {:?}", path))?;

    file.write_all(&bytes).await.context("Failed to write bytes to file")?;

    Ok(())
}

#[allow(dead_code)]
pub async fn filter_proxy(from: &Path, to: &Path, proxy_type: ProxyType) -> Result<()> {
    let content = tokio::fs
        ::read_to_string(from).await
        .with_context(|| format!("Failed to read {}", from.display()))?;

    let proxies: Vec<String> = content
        .lines()
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect();

    // JoinSet lets us collect results in completion order not input order
    let mut tasks = JoinSet::new();

    for proxy in proxies {
        let proxy_type = proxy_type.clone();
        tasks.spawn(async move {
            let result = test_proxy(&proxy, proxy_type).await;
            (proxy, result)
        });
    }

    // join_next returns the next completed task regardless of
    // which one started first so a slow proxy won't block results from fast proxies
    let mut results = Vec::new();

    while let Some(joined) = tasks.join_next().await {
        match joined {
            Ok((proxy, Ok(_))) => results.push(format!("{}", proxy)),

            Ok((_, Err(_))) => (),

            Err(join_err) => eprintln!("Task panicked: {}", join_err),
        }
    }

    let output = results.join("\n");
    tokio::fs
        ::write(to, &output).await
        .with_context(|| format!("Failed to write {}", to.display()))?;

    Ok(())
}

#[allow(dead_code)]
async fn test_proxy(proxy: &str, proxy_type: ProxyType) -> Result<u64> {
    let mut proxy_url = String::from(proxy);
    match proxy_type {
        ProxyType::HTTP => {
            if !proxy.starts_with("http://") {
                proxy_url = format!("http://{}", proxy);
            }
        }
        ProxyType::SOCKS4 => {
            if !proxy.starts_with("socks4://") {
                proxy_url = format!("socks4://{}", proxy);
            }
        }
        ProxyType::SOCKS5 => {
            if !proxy.starts_with("socks5://") {
                proxy_url = format!("socks5://{}", proxy);
            }
        }
    }

    let client = reqwest::Client
        ::builder()
        .proxy(reqwest::Proxy::all(&proxy_url)?)
        .timeout(TIMEOUT)
        .build()?;

    let start = Instant::now();

    let response = client.get(TEST_URL).send().await?;

    if !response.status().is_success() {
        bail!("HTTP {}", response.status());
    }

    Ok(start.elapsed().as_millis() as u64)
}
