use anyhow::Result;
use reqwest::Client;
use serde_json::json;

pub async fn discord_sender(url: &str, content: &str) -> Result<()> {
    let payload = json!({"content": content});

    let client = Client::new();
    let resp = client.post(url).json(&payload).send().await?;

    if resp.status().is_success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Discord error: {}", resp.status()).into())
    }
}
