use chrono::{DateTime, Utc};
use serde::Serialize;

pub mod sender;

#[derive(Serialize)]
pub struct DiscordField {
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub inline: bool,
}

#[derive(Serialize)]
pub struct DiscordFooter {
    pub text: String,
}

#[derive(Serialize)]
pub struct DiscordEmbed {
    pub title: String,
    pub description: String,
    pub color: u32,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fields: Vec<DiscordField>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub footer: Option<DiscordFooter>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timestamp: Option<DateTime<Utc>>,
}
