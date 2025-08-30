use anyhow::Result;
use reqwest::Client;
use serde_json::json;

pub async fn discord_sender(url: &str, title: &str, description: &str, color: u32) -> Result<()> {
    let payload = json!({
        "embeds": [
            {
                "title": title,
                "description": description,
                "color": color
            }
        ]
    });

    let client = Client::new();
    let resp = client.post(url).json(&payload).send().await?;

    if resp.status().is_success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Discord error: {} - {:?}",
            resp.status(),
            resp.text().await?
        ))
    }
}
