use std::collections::HashMap;

use reqwest::Client;
use serde::Deserialize;

use crate::{auth::Auth, error::ValoTrackerError};

/// Resolved display name for a player.
#[derive(Debug, Clone)]
pub struct PlayerName {
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
    /// Pre-formatted `"name#tag"` string.
    pub display: String,
}

impl PlayerName {
    fn new(puuid: String, game_name: String, tag_line: String) -> Self {
        let display = format!("{}#{}", game_name, tag_line);
        Self {
            puuid,
            game_name,
            tag_line,
            display,
        }
    }

    /// Returns `"[S]"` placeholder when the player has streamer mode enabled
    /// (i.e. their name is empty / hidden). Actual incognito detection happens
    /// in pregame/coregame modules.
    pub fn display_or_hidden(display: &str) -> &str {
        if display.trim_matches('#').is_empty() {
            "[S]"
        } else {
            display
        }
    }
}

#[derive(Debug, Deserialize)]
struct NameEntry {
    #[serde(rename = "Subject")]
    subject: String,
    #[serde(rename = "GameName")]
    game_name: String,
    #[serde(rename = "TagLine")]
    tag_line: String,
}

/// Resolve display names for a batch of PUUIDs.
///
/// Endpoint: `PUT https://pd.{shard}.a.pvp.net/name-service/v2/players`
/// Body: JSON array of PUUID strings.
pub async fn fetch_names(
    client: &Client,
    auth: &Auth,
    puuids: &[String],
) -> Result<HashMap<String, PlayerName>, ValoTrackerError> {
    if puuids.is_empty() {
        return Ok(HashMap::new());
    }

    let url = auth.pvp_url("/name-service/v2/players");
    let entries: Vec<NameEntry> = client
        .put(&url)
        .json(puuids)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let map = entries
        .into_iter()
        .map(|e| {
            let name = PlayerName::new(e.subject.clone(), e.game_name, e.tag_line);
            (e.subject, name)
        })
        .collect();

    Ok(map)
}
