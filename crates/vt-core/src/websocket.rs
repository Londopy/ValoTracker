use futures::{SinkExt, StreamExt};
use tokio::sync::watch;
use tokio_tungstenite::{connect_async_tls_with_config, tungstenite::Message};

use crate::{error::VtError, lockfile::Lockfile, state::GameState};

const EVENT_PRESENCES: &str = "OnJsonApiEvent_chat_v4_presences";

/// Subscribe to the Riot local WebSocket for instant game-state change events.
///
/// Sends `GameState` updates through `tx` whenever a presence event fires.
/// The WebSocket uses `wss://riot:{password}@127.0.0.1:{port}`.
///
/// This runs as a long-lived async task — spawn it with `tokio::spawn`.
pub async fn run_websocket(
    lockfile: &Lockfile,
    tx: watch::Sender<GameState>,
    my_puuid: String,
) -> Result<(), VtError> {
    use base64::Engine as _;

    let credentials = format!("riot:{}", lockfile.password);
    let b64 = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
    let auth_header = format!("Basic {b64}");

    let url = format!("wss://127.0.0.1:{}", lockfile.port);
    tracing::info!("Connecting to Riot WebSocket at {url}");

    // Build a custom TLS connector that accepts self-signed certs
    let tls_config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(std::sync::Arc::new(AcceptAnyCert))
        .with_no_client_auth();

    let connector = tokio_tungstenite::Connector::Rustls(std::sync::Arc::new(tls_config));

    let request = tokio_tungstenite::tungstenite::handshake::client::Request::builder()
        .uri(&url)
        .header("Authorization", &auth_header)
        .body(())
        .map_err(|e| VtError::other(format!("WebSocket request build error: {e}")))?;

    let (mut ws, _) =
        connect_async_tls_with_config(request, None, false, Some(connector)).await?;

    // Subscribe to presence events
    let subscribe_msg = serde_json::json!([5, EVENT_PRESENCES]).to_string();
    ws.send(Message::Text(subscribe_msg)).await?;
    tracing::info!("Subscribed to {EVENT_PRESENCES}");

    while let Some(msg) = ws.next().await {
        let msg = msg?;
        if let Message::Text(text) = msg {
            if let Some(state) = parse_presence_event(&text, &my_puuid) {
                let _ = tx.send(state);
            }
        }
    }

    Ok(())
}

/// Parse a raw WebSocket presence event message into a `GameState`.
fn parse_presence_event(raw: &str, puuid: &str) -> Option<GameState> {
    use base64::Engine as _;

    // Events come as: [8, "OnJsonApiEvent_...", { "data": { "presences": [...] } }]
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    let arr = value.as_array()?;
    if arr.len() < 3 {
        return None;
    }

    let presences = arr[2]
        .get("data")?
        .get("presences")?
        .as_array()?;

    let player = presences.iter().find(|p| {
        p.get("puuid")
            .and_then(|v| v.as_str())
            .map(|s| s == puuid)
            .unwrap_or(false)
    })?;

    let private_b64 = player.get("private")?.as_str()?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(private_b64)
        .ok()?;
    let decoded: serde_json::Value = serde_json::from_slice(&bytes).ok()?;

    let state = decoded
        .get("sessionLoopState")
        .and_then(|v| v.as_str())
        .unwrap_or("MENUS");

    Some(match state {
        "PREGAME" => GameState::Pregame {
            match_id: String::new(),
        },
        "INGAME" => GameState::Ingame {
            match_id: String::new(),
        },
        _ => GameState::Menu,
    })
}

// ── Custom TLS verifier (accept self-signed Riot cert) ────────────────────────

#[derive(Debug)]
struct AcceptAnyCert;

impl rustls::client::danger::ServerCertVerifier for AcceptAnyCert {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}
