//! What Control remembers between starts: connected trips with their driver
//! links and the routing settings. Sharing is never switched on by itself, and
//! the Gateway secret is not stored.

use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct StoredSession {
    token: String,
    folder_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct StoredControl {
    #[serde(default)]
    sessions: Vec<StoredSession>,
    #[serde(default)]
    routing: Option<ConfigureRoutingInput>,
}

fn read(path: &Path) -> StoredControl {
    let Ok(text) = fs::read_to_string(path) else {
        return StoredControl::default();
    };
    serde_json::from_str(&text).unwrap_or_else(|_| {
        // A damaged file is set aside, never overwritten.
        let _ = fs::rename(path, path.with_extension("damaged"));
        StoredControl::default()
    })
}

fn write(path: &Path, stored: &StoredControl) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(stored)?)?;
    fs::rename(&temporary, path)
}

impl DesktopControlService {
    pub(crate) fn with_memory(path: PathBuf) -> Self {
        Self {
            memory: Some(path),
            ..Self::default()
        }
    }

    fn change_memory(&self, change: impl FnOnce(&mut StoredControl)) -> Result<(), String> {
        let Some(path) = &self.memory else {
            return Ok(());
        };
        let _guard = self
            .memory_lock
            .lock()
            .map_err(|_| "Не вдалося запам'ятати рейси Control".to_owned())?;
        let mut stored = read(path);
        change(&mut stored);
        write(path, &stored)
            .map_err(|error| format!("Не вдалося запам'ятати рейси Control: {error}"))
    }

    /// Keeps one entry per trip folder: a new link replaces the older one.
    pub(crate) fn remember_session(&self, token: &str, folder_path: &str) -> Result<(), String> {
        let session = StoredSession {
            token: token.trim().to_owned(),
            folder_path: folder_path.trim().to_owned(),
        };
        self.change_memory(|stored| {
            stored.sessions.retain(|known| {
                known.folder_path != session.folder_path && known.token != session.token
            });
            stored.sessions.push(session);
        })
    }

    pub(crate) fn remember_routing(
        &self,
        routing: Option<ConfigureRoutingInput>,
    ) -> Result<(), String> {
        self.change_memory(|stored| stored.routing = routing)
    }

    /// Connects the trips of the previous session with the same driver links. A
    /// folder that cannot be opened now stays remembered for the next start.
    pub(crate) async fn restore(&self) {
        let Some(path) = &self.memory else {
            return;
        };
        let stored = read(path);
        for session in stored.sessions {
            let _ = self
                .register_trip(RegisterTripInput {
                    folder_path: session.folder_path,
                    token: session.token,
                })
                .await;
        }
        if let Some(routing) = stored.routing {
            let _ = self.configure_routing(routing).await;
        }
    }
}
