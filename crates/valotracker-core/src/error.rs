use thiserror::Error;

/// All errors that can occur in valotracker-core.
#[derive(Debug, Error)]
pub enum ValoTrackerError {
    #[error("Lockfile not found — is VALORANT running?")]
    LockfileNotFound,

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("Auth token expired — restart VALORANT")]
    AuthExpired,

    #[error("Player not in match")]
    NotInMatch,

    #[error("Rate limited — retrying in {0}s")]
    RateLimited(u64),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("{0}")]
    Other(String),
}

impl ValoTrackerError {
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}
