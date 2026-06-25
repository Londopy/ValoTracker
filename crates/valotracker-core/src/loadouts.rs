// pull each players equipped primary-weapon skin from the in-game loadouts
// endpoint. returns puuid -> skin uuid (resolve the name with `content`).
//
// best-effort: any failure (wrong phase, riot hiccup, schema drift) just yields
// an empty map, so skins show blank and nothing else breaks. we parse straight
// off a json Value on purpose so a renamed field cant nuke the whole thing.

use std::collections::HashMap;

use reqwest::Client;

use crate::auth::Auth;

// the "skin" socket inside a weapon (separate from the level / chroma sockets)
const SKIN_SOCKET: &str = "bcef87d6-209b-46c6-8b19-fbe40bd95abc";
// the rifle we show as the "primary" skin (vandal first, then phantom)
const VANDAL: &str = "9c82e19d-4575-0200-1a81-3eacf00cf872";
const PHANTOM: &str = "ee8e8d15-496b-07ac-e5f6-8fae5d4c7b1a";

// returns puuid -> primary weapon skin uuid for everyone in the match.
pub async fn get_loadouts(client: &Client, auth: &Auth, match_id: &str) -> HashMap<String, String> {
    let url = auth.glz_url(&format!("/core-game/v1/matches/{match_id}/loadouts"));
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(_) => return HashMap::new(),
    };
    if !resp.status().is_success() {
        return HashMap::new();
    }
    let body = resp.text().await.unwrap_or_default();
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);

    let mut out = HashMap::new();
    let Some(loadouts) = json["Loadouts"].as_array() else {
        return out;
    };

    for entry in loadouts {
        // newer responses nest everything under "Loadout"; older ones are flat.
        let loadout = if entry["Loadout"].is_object() {
            &entry["Loadout"]
        } else {
            entry
        };
        // subject (puuid) can live on the entry or the inner loadout
        let puuid = entry["Subject"]
            .as_str()
            .filter(|s| !s.is_empty())
            .or_else(|| loadout["Subject"].as_str())
            .unwrap_or("");
        if puuid.is_empty() {
            continue;
        }

        // vandal first, fall back to phantom
        let items = &loadout["Items"];
        let weapon = if items[VANDAL].is_object() {
            &items[VANDAL]
        } else {
            &items[PHANTOM]
        };
        let skin = weapon["Sockets"][SKIN_SOCKET]["Item"]["ID"]
            .as_str()
            .unwrap_or("");
        if !skin.is_empty() {
            out.insert(puuid.to_string(), skin.to_string());
        }
    }
    out
}
