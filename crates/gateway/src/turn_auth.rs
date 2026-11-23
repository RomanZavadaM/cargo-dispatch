use std::time::{SystemTime, UNIX_EPOCH};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use cargo_connectivity_protocol::{IceServerConfig, TripSessionId};
use hmac::{Hmac, Mac};
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;

#[derive(Clone)]
pub struct IceSettings {
    stun_url: String,
    turn_urls: Vec<String>,
    turn_secret: Option<String>,
    ttl_secs: u64,
}

impl IceSettings {
    pub fn from_env() -> Self {
        let stun_url = std::env::var("CARGO_STUN_URL")
            .unwrap_or_else(|_| "stun:stun.l.google.com:19302".into());
        let turn_urls = std::env::var("CARGO_TURN_URLS")
            .ok()
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let turn_secret = std::env::var("CARGO_TURN_SECRET").ok();
        let ttl_secs = std::env::var("CARGO_TURN_TTL_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(3600);

        Self {
            stun_url,
            turn_urls,
            turn_secret,
            ttl_secs,
        }
    }

    pub fn for_trip(&self, trip_session_id: TripSessionId) -> (Vec<IceServerConfig>, Option<u64>) {
        let mut servers = vec![IceServerConfig {
            urls: vec![self.stun_url.clone()],
            username: String::new(),
            credential: String::new(),
        }];

        let Some(secret) = self.turn_secret.as_deref() else {
            return (servers, None);
        };
        if self.turn_urls.is_empty() {
            return (servers, None);
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_secs();
        let expires_at = now.saturating_add(self.ttl_secs);
        let username = format!("{expires_at}:{trip_session_id}");
        let credential = make_turn_credential(secret, &username);

        servers.push(IceServerConfig {
            urls: self.turn_urls.clone(),
            username,
            credential,
        });

        (servers, Some(expires_at))
    }
}

fn make_turn_credential(secret: &str, username: &str) -> String {
    let mut mac = HmacSha1::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(username.as_bytes());
    STANDARD.encode(mac.finalize().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_credential_is_stable_for_same_input() {
        let a = make_turn_credential("secret", "1700000000:trip");
        let b = make_turn_credential("secret", "1700000000:trip");
        assert_eq!(a, b);
        assert!(!a.is_empty());
    }

    #[test]
    fn turn_credential_changes_with_username() {
        let a = make_turn_credential("secret", "1700000000:trip-a");
        let b = make_turn_credential("secret", "1700000000:trip-b");
        assert_ne!(a, b);
    }
}
