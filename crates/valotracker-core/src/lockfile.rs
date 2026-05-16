use std::path::PathBuf;

use crate::error::ValoTrackerError;

/// Parsed contents of the Riot lockfile.
///
/// Located at:
///   `%LOCALAPPDATA%\Riot Games\Riot Client\Config\lockfile`
///
/// Format (colon-separated):
///   `name:pid:port:password:protocol`
#[derive(Debug, Clone)]
pub struct Lockfile {
    pub port: u16,
    pub password: String,
    pub protocol: String,
}

impl Lockfile {
    /// Read and parse the Riot lockfile from its default Windows location.
    pub fn read() -> Result<Self, ValoTrackerError> {
        let path = Self::path()?;
        let raw = std::fs::read_to_string(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ValoTrackerError::LockfileNotFound
            } else {
                ValoTrackerError::Io(e)
            }
        })?;
        Self::parse(&raw)
    }

    /// Read the lockfile from an explicit path (useful for testing).
    pub fn read_from(path: &std::path::Path) -> Result<Self, ValoTrackerError> {
        let raw = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ValoTrackerError::LockfileNotFound
            } else {
                ValoTrackerError::Io(e)
            }
        })?;
        Self::parse(&raw)
    }

    /// Parse a raw lockfile string.
    fn parse(raw: &str) -> Result<Self, ValoTrackerError> {
        let parts: Vec<&str> = raw.trim().splitn(5, ':').collect();
        if parts.len() < 5 {
            return Err(ValoTrackerError::other(format!(
                "Unexpected lockfile format (got {} fields, expected 5): {raw}",
                parts.len()
            )));
        }
        let port = parts[2]
            .parse::<u16>()
            .map_err(|e| ValoTrackerError::other(format!("Invalid port in lockfile: {e}")))?;
        Ok(Lockfile {
            port,
            password: parts[3].to_owned(),
            protocol: parts[4].to_owned(),
        })
    }

    /// `Authorization: Basic <base64("riot:{password}")>` header value.
    pub fn auth_header(&self) -> String {
        use base64::Engine as _;
        let credentials = format!("riot:{}", self.password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
        format!("Basic {encoded}")
    }

    /// Build a local API URL: `https://127.0.0.1:{port}{path}`
    pub fn local_url(&self, path: &str) -> String {
        format!("{}://127.0.0.1:{}{}", self.protocol, self.port, path)
    }

    /// Canonical path to the lockfile on Windows.
    fn path() -> Result<PathBuf, ValoTrackerError> {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .map_err(|_| ValoTrackerError::other("LOCALAPPDATA environment variable not set"))?;
        Ok(PathBuf::from(local_app_data)
            .join("Riot Games")
            .join("Riot Client")
            .join("Config")
            .join("lockfile"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_lockfile() {
        let raw = "RiotClient:12345:54321:supersecret:https";
        let lf = Lockfile::parse(raw).unwrap();
        assert_eq!(lf.port, 54321);
        assert_eq!(lf.password, "supersecret");
        assert_eq!(lf.protocol, "https");
    }

    #[test]
    fn auth_header_format() {
        let lf = Lockfile {
            port: 1234,
            password: "pass".into(),
            protocol: "https".into(),
        };
        // "riot:pass" base64-encoded is "cmlvdDpwYXNz"
        assert_eq!(lf.auth_header(), "Basic cmlvdDpwYXNz");
    }

    #[test]
    fn local_url_format() {
        let lf = Lockfile {
            port: 54321,
            password: "x".into(),
            protocol: "https".into(),
        };
        assert_eq!(lf.local_url("/foo/bar"), "https://127.0.0.1:54321/foo/bar");
    }
}
