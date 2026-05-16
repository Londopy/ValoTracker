use reqwest::{header, Client, ClientBuilder};

use crate::{auth::Auth, error::VtError, lockfile::Lockfile};

/// Build a `reqwest::Client` for the **local** Riot API (127.0.0.1:{port}).
///
/// TLS certificate verification is intentionally disabled because the Riot
/// Client uses a self-signed certificate on localhost.
pub fn build_local_client(lockfile: &Lockfile) -> Result<Client, VtError> {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(&lockfile.auth_header())
            .map_err(|e| VtError::other(format!("Invalid auth header: {e}")))?,
    );

    let client = ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .default_headers(headers)
        .build()?;

    Ok(client)
}

/// Build a `reqwest::Client` for **remote** Riot PD / GLZ endpoints.
///
/// Uses the authenticated headers derived from the `Auth` struct.
pub fn build_remote_client(auth: &Auth) -> Result<Client, VtError> {
    let client = ClientBuilder::new()
        .default_headers(auth.riot_headers())
        .build()?;

    Ok(client)
}
