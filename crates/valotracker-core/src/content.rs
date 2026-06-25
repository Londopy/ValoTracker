// dynamic content lookups (agents / maps / skins).
//
// we pull these tables from valorant-api.com ONCE and cache them, so when riot
// ships a new agent/map/skin it just works with no app update. anything we cant
// resolve falls back to a raw id / codename so nothing ever breaks.
//
// valorant-api.com is a public community cdn (NOT riot), so theres no ban risk
// here - but we cache anyway so its only hit once per run.

use std::collections::HashMap;
use std::sync::OnceLock;

use reqwest::Client;

#[derive(Default)]
struct Content {
    agents: HashMap<String, String>, // character uuid -> name
    maps: HashMap<String, String>,   // map asset path -> name
    skins: HashMap<String, String>,  // skin uuid -> name
}

static CONTENT: OnceLock<Content> = OnceLock::new();

// load + cache the tables. cheap to call every snapshot: it only does network
// the first time it succeeds. if it fails it just doesnt cache and tries again
// next time.
pub async fn ensure_loaded(client: &Client) {
    if CONTENT.get().is_some() {
        return;
    }
    if let Some(c) = fetch_all(client).await {
        let _ = CONTENT.set(c);
    }
}

async fn fetch_all(client: &Client) -> Option<Content> {
    let agents = fetch_map(
        client,
        "https://valorant-api.com/v1/agents?isPlayableCharacter=true",
        "uuid",
        "displayName",
    )
    .await?;
    let maps = fetch_map(
        client,
        "https://valorant-api.com/v1/maps",
        "mapUrl",
        "displayName",
    )
    .await?;
    let skins = fetch_map(
        client,
        "https://valorant-api.com/v1/weapons/skins",
        "uuid",
        "displayName",
    )
    .await?;
    Some(Content {
        agents,
        maps,
        skins,
    })
}

// pull a `{ "data": [ { <key>, <name> }, ... ] }` list into a lowercased map
async fn fetch_map(
    client: &Client,
    url: &str,
    key: &str,
    name: &str,
) -> Option<HashMap<String, String>> {
    let v: serde_json::Value = client.get(url).send().await.ok()?.json().await.ok()?;
    let arr = v.get("data")?.as_array()?;
    let mut m = HashMap::new();
    for item in arr {
        if let (Some(k), Some(n)) = (
            item.get(key).and_then(|x| x.as_str()),
            item.get(name).and_then(|x| x.as_str()),
        ) {
            m.insert(k.to_lowercase(), n.to_owned());
        }
    }
    Some(m)
}

// agent display name for a character uuid, if we know it.
pub fn agent_name(uuid: &str) -> Option<String> {
    CONTENT.get()?.agents.get(&uuid.to_lowercase()).cloned()
}

// skin display name for a skin uuid, if we know it.
pub fn skin_name(uuid: &str) -> Option<String> {
    CONTENT.get()?.skins.get(&uuid.to_lowercase()).cloned()
}

// map name from its asset path (e.g. "/Game/Maps/Triad/Triad" -> "Haven").
// falls back to the last path segment if the live table isnt loaded / lacks it.
pub fn map_name(map_url: &str) -> String {
    if let Some(c) = CONTENT.get() {
        if let Some(n) = c.maps.get(&map_url.to_lowercase()) {
            return n.clone();
        }
    }
    map_url
        .split('/')
        .rfind(|s| !s.is_empty())
        .unwrap_or("Unknown Map")
        .to_owned()
}
