/// Runtime configuration, read once at startup.
///
/// Subsonic credentials: env vars take priority, then hardcoded defaults.
/// Keybinds are hardcoded in Phase 1.
#[derive(Debug, Clone)]
pub struct Config {
    pub subsonic_url: String,
    pub subsonic_user: String,
    pub subsonic_pass: String,
}

impl Config {
    pub fn from_env() -> Self {
        let url = std::env::var("TERMUSIC_SUBSONIC_URL")
            .or_else(|_| std::env::var("SUBSONIC_URL"))
            .unwrap_or_else(|_| "http://192.168.68.122:4533".to_string());
        let user = std::env::var("TERMUSIC_SUBSONIC_USER")
            .or_else(|_| std::env::var("SUBSONIC_USER"))
            .unwrap_or_else(|_| "admin".to_string());
        let pass = std::env::var("TERMUSIC_SUBSONIC_PASS")
            .or_else(|_| std::env::var("SUBSONIC_PASS"))
            .unwrap_or_else(|_| "REDACTED".to_string());
        Self {
            subsonic_url: url,
            subsonic_user: user,
            subsonic_pass: pass,
        }
    }
}
