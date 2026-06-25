// pull each players equipped primary-weapon skin from the in-game loadouts
// endpoint. returns puuid -> skin uuid (resolve the name with `content`).
//
// best-effort: any failure (wrong phase, riot hiccup, schema drift) just yields
// an empty map, so skins show blank and nothing else breaks.

use std::collections::HashMap;

use reqwest::Client;
use serde::Deserialize;

use crate::auth::Auth;

// the "skin" socket inside a weapon (separate from the level / chroma sockets)
const SKIN_SOCKET: &str = "bcef87d6-209b-46c6-8b19-fbe40bd95abc";
// the rifle we show as the "primary" skin (vandal first, then phantom)
const VANDAL: &str = "9c82e19d-4575-0200-1a81-3eacf00cf872";
const PHANTOM: &str = "ee8e8d15-496b-07ac-e5f6-8fae5d4c7b1a";

#[derive(Deserialize, Default)]
struct LoadoutsResponse {
    #[serde(rename = "Loadouts", default)]
    loadouts: Vec<Entry>,
}

#[derive(Deserialize, Default)]
struct Entry {
    #[serde(rename = "Subject", default)]
    subject: String,
    #[serde(rename = "Loadout", default)]
    loadout: Loadout,
}

#[derive(Deserialize, Default)]
struct Loadout {
    #[serde(rename = "Subject", default)]
    subject: String,
    #[serde(rename = "Items", default)]
    items: HashMap<String, Item>,
}

#[derive(Deserialize, Default)]
struct Item {
    #[serde(rename = "Sockets", default)]
    sockets: HashMap<String, Socket>,
}

#[derive(Deserialize, Default)]
struct Socket {
    #[serde(rename = "Item", default)]
    item: SocketItem,
}

#[derive(Deserialize, Default)]
struct SocketItem {
    #[serde(rename = "ID", default)]
    id: String,
}

// returns puuid -> primary weapon skin uuid for everyone in the match.
pub async fn get_loadouts(client: &Client, auth: &Auth, match_id: &str) -> HashMap<String, String> {
    let url = auth.glz_url(&format!("/core-game/v1/matches/{match_id}/loadouts"));
    let resp = match client.get(&url).send().await {
        Ok(r) => match r.error_for_status() {
            Ok(r) => r,
            Err(_) => return HashMap::new(),
        },
        Err(_) => return HashMap::new(),
    };
    let parsed: LoadoutsResponse = match resp.json().await {
        Ok(j) => j,
        Err(_) => return HashMap::new(),
    };

    let mut out = HashMap::new();
    for entry in parsed.loadouts {
        // newer responses carry Subject on the entry; older ones on the loadout
        let puuid = if !entry.subject.is_empty() {
            entry.subject.clone()
        } else {
            entry.loadout.subject.clone()
        };
        if puuid.is_empty() {
            continue;
        }
        let skin = entry
            .loadout
            .items
            .get(VANDAL)
            .or_else(|| entry.loadout.items.get(PHANTOM))
            .and_then(|w| w.sockets.get(SKIN_SOCKET))
            .map(|s| s.item.id.clone())
            .unwrap_or_default();
        if !skin.is_empty() {
            out.insert(puuid, skin);
        }
    }
    out
}
