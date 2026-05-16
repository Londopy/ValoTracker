use reqwest::{header, Client};
use serde::Deserialize;

use crate::{error::ValoTrackerError, lockfile::Lockfile};

/// Authenticated session credentials obtained from the local Riot Client.
///
/// No username/password is required — everything is read from the running
/// VALORANT process via the local entitlements endpoint.
#[derive(Debug, Clone)]
pub struct Auth {
    /// The player's unique PUUID.
    pub puuid: String,
    /// OAuth2 access token.
    pub access_token: String,
    /// Riot entitlements JWT.
    pub entitlements_token: String,
    /// Region code, e.g. "na", "eu", "ap", "kr", "br", "latam".
    pub region: String,
    /// Shard (PD/GLZ routing), usually mirrors `region`.
    pub shard: String,
    /// Current Riot Client version string (sent as `X-Riot-ClientVersion`).
    pub client_version: String,
}

// ── Internal response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct EntitlementsResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    token: String,
    subject: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RegionResponse {
    affinities: RegionAffinities,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RegionAffinities {
    live: String,
}

#[derive(Debug, Deserialize)]
struct ClientVersionResponse {
    data: ClientVersionData,
}

#[derive(Debug, Deserialize)]
struct ClientVersionData {
    #[serde(rename = "riotClientVersion")]
    riot_client_version: String,
}

// ── Impl ─────────────────────────────────────────────────────────────────────

impl Auth {
    /// Fetch auth credentials from the local Riot Client entitlements endpoint.
    ///
    /// Requires VALORANT to be running. Reads from:
    ///   `GET https://127.0.0.1:{port}/entitlements/v1/token`
    pub async fn from_lockfile(
        lockfile: &Lockfile,
        client: &Client,
    ) -> Result<Self, ValoTrackerError> {
        // 1. Entitlements token + access token + PUUID
        let ent_url = lockfile.local_url("/entitlements/v1/token");
        let ent: EntitlementsResponse = client
            .get(&ent_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        // 2. Region from product-session endpoint
        let region_url = lockfile.local_url("/product-session/v1/external-sessions");
        let region_body: serde_json::Value = client
            .get(&region_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        // The response is a map — grab the first session entry's launchConfiguration
        let shard = region_body
            .as_object()
            .and_then(|m| m.values().next())
            .and_then(|v| v.get("launchConfiguration"))
            .and_then(|v| v.get("arguments"))
            .and_then(|v| v.as_array())
            .and_then(|args| {
                args.iter().find_map(|a| {
                    let s = a.as_str()?;
                    s.strip_prefix("-ares-deployment=").map(|r| r.to_owned())
                })
            })
            .unwrap_or_else(|| "na".to_owned());

        // Region and shard are the same for most regions; ap/kr share pd endpoint
        let region = shard.clone();

        // 3. Client version from valorant-version API
        let client_version = Self::fetch_client_version().await.unwrap_or_else(|_| {
            // Fallback: Riot accepts an empty version (may get 403 on some endpoints)
            "release-09.00-shipping-10-2478706".to_owned()
        });

        Ok(Auth {
            puuid: ent.subject,
            access_token: ent.access_token,
            entitlements_token: ent.token,
            region,
            shard,
            client_version,
        })
    }

    /// Fetch the current client version from the unofficial valorant-version API.
    async fn fetch_client_version() -> Result<String, ValoTrackerError> {
        let resp: ClientVersionResponse = reqwest::get("https://valorant-api.com/v1/version")
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(resp.data.riot_client_version)
    }

    // ── URL builders ─────────────────────────────────────────────────────────

    /// Player data (PD) base URL: `https://pd.{shard}.a.pvp.net`
    pub fn pvp_url(&self, path: &str) -> String {
        format!("https://pd.{}.a.pvp.net{}", self.shard, path)
    }

    /// GLZ (match) base URL: `https://glz-{shard}-1.{region}.a.pvp.net`
    pub fn glz_url(&self, path: &str) -> String {
        format!(
            "https://glz-{}-1.{}.a.pvp.net{}",
            self.shard, self.region, path
        )
    }

    // ── Header builder ───────────────────────────────────────────────────────

    /// Standard Riot API headers required on all remote requests.
    pub fn riot_headers(&self) -> reqwest::header::HeaderMap {
        let mut map = header::HeaderMap::new();

        let insert = |map: &mut header::HeaderMap, key: &str, val: &str| {
            if let (Ok(k), Ok(v)) = (
                header::HeaderName::from_bytes(key.as_bytes()),
                header::HeaderValue::from_str(val),
            ) {
                map.insert(k, v);
            }
        };

        insert(
            &mut map,
            "Authorization",
            &format!("Bearer {}", self.access_token),
        );
        insert(
            &mut map,
            "X-Riot-Entitlements-JWT",
            &self.entitlements_token,
        );
        insert(&mut map, "X-Riot-ClientVersion", &self.client_version);
        insert(&mut map, "X-Riot-ClientPlatform", PLATFORM_TOKEN);

        map
    }
}

/// Static base-64 encoded platform token (tells Riot we're on Windows PC).
///
/// This is the same value used by most open-source VALORANT API clients.
const PLATFORM_TOKEN: &str =
    "ew0KCSJwbGF0Zm9ybVR5cGUiOiAiUEMiLA0KCSJwbGF0Zm9ybU9TIjogIldpbmRvd3MiLA0KCSJwbGF0Zm9ybU9TVmVyc2lvbiI6ICIxMC4wLjE5MDQyLjEuMjU2LjY0Yml0IiwNCgkicGxhdGZvcm1DaGlwc2V0IjogIlVua25vd24iDQp9";
