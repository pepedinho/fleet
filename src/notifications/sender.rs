use anyhow::Result;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;

use crate::{
    core::watcher::WatchContext,
    exec::metrics::ExecMetrics,
    notifications::{DiscordEmbed, DiscordField, DiscordFooter, DiscordImage},
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
    if ctx.config.pipeline.notifications.is_none() {
        return Ok(());
    }

    let notification_config = ctx.config.pipeline.notifications.as_ref().unwrap();

    let embed = DiscordEmbed {
        title: "✅Pipeline finish".into(),
        description: format!("Pipeline **{}** executed successfully", ctx.repo.name),
        color: 0x2ECC71,
        thumbnail: DiscordImage::load(notification_config.thumbnail.clone()),
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

    for c in notification_config.channels.iter() {
        if c.service == "discord" {
            discord_sender(&c.url, &embed).await?;
        }
    }
    Ok(())
}

/// this function take ctx and msg
/// msg will be split on ":/:" pattern and divide in field
pub async fn discord_send_failure(ctx: &WatchContext, msg: &str, m: &ExecMetrics) -> Result<()> {
    if ctx.config.pipeline.notifications.is_none() {
        return Ok(());
    }

    let notification_config = ctx.config.pipeline.notifications.as_ref().unwrap();

    let embed = DiscordEmbed {
        title: "❌ Pipeline failed".into(),
        description: String::from(msg),
        color: 0xE74C3C,
        thumbnail: DiscordImage::load(notification_config.thumbnail.clone()),
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
        timestamp: Some(m.finished_at.unwrap_or(Utc::now())),
    };
    for c in notification_config.channels.iter() {
        if c.service == "discord" {
            discord_sender(&c.url, &embed).await?;
        }
    }
    Ok(())
}
