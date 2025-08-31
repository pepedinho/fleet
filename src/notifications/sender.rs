use anyhow::Result;
use reqwest::Client;
use serde_json::json;

use crate::{
    core::watcher::WatchContext,
    exec::metrics::ExecMetrics,
    notifications::{DiscordEmbed, DiscordField, DiscordFooter},
};

pub async fn discord_sender(url: &str, embed: &DiscordEmbed) -> Result<()> {
    let payload = json!({
        "embeds": [embed]
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

pub async fn discord_send_succes(ctx: &WatchContext, m: &ExecMetrics) -> Result<()> {
    let embed = DiscordEmbed {
        title: "✅Pipeline finish".into(),
        description: format!("Pipeline **{}** executed successfully", ctx.repo.name),
        color: 0x2ECC71,
        fields: vec![
            DiscordField {
                name: "Service name".into(),
                value: format!("`{}`", ctx.repo.name.clone()),
                inline: false,
            },
            DiscordField {
                name: "Duration".into(),
                value: format!("`{:.2}s`", (m.duration_ms.unwrap_or(1) as f64) / 1000.0),
                inline: true,
            },
            DiscordField {
                name: "CPU".into(),
                value: format!("`{:.2}%`", m.cpu_usage),
                inline: true,
            },
            DiscordField {
                name: "Mem (%)".into(),
                value: format!("`{:.2}%`", m.mem_usage),
                inline: true,
            },
            DiscordField {
                name: "Mem (Mb)".into(),
                value: format!("`{}Mb`", m.mem_usage_kb / (1024 * 1024)),
                inline: true,
            },
        ],
        footer: Some(DiscordFooter {
            text: "Fleet CI/CD Pipeline".into(),
        }),
        timestamp: Some(m.finished_at.unwrap()),
    };
    for c in ctx.config.pipeline.notifications.channels.iter() {
        if c.service == "discord" {
            discord_sender(&c.url, &embed).await?;
        }
    }
    Ok(())
}

pub async fn discord_send_failure<E: std::error::Error>(ctx: &WatchContext, e: E) -> Result<()> {
    let embed = DiscordEmbed {
        title: "❌ Pipeline failed".into(),
        description: format!("Error: {}", &e.to_string()),
        color: 0xE74C3C,
        fields: vec![],
        footer: None,
        timestamp: Some(chrono::Utc::now()),
    };
    for c in ctx.config.pipeline.notifications.channels.iter() {
        if c.service == "discord" {
            discord_sender(&c.url, &embed).await?;
        }
    }
    Ok(())
}
