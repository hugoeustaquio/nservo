use std::path::PathBuf;

/// Resolves configuration and data paths across Linux, Windows, macOS and Android.
#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub base_dir: PathBuf,
}

impl ConfigPaths {
    pub fn new() -> Self {
        Self {
            base_dir: Self::detect_config_dir(),
        }
    }

    pub fn from_base(base: PathBuf) -> Self {
        Self { base_dir: base }
    }

    fn detect_config_dir() -> PathBuf {
        #[cfg(target_os = "android")]
        {
            let p = PathBuf::from("/data/data/org.nservo.browser/files");
            if std::fs::create_dir_all(&p).is_ok() {
                return p;
            }
        }

        #[cfg(not(target_os = "android"))]
        {
            if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
                if !xdg.trim().is_empty() {
                    let p = PathBuf::from(xdg).join("nservo");
                    if std::fs::create_dir_all(&p).is_ok() {
                        return p;
                    }
                }
            }

            if let Ok(home) = std::env::var("HOME") {
                let candidate = PathBuf::from(home).join(".config").join("nservo");
                if std::fs::create_dir_all(&candidate).is_ok() {
                    return candidate;
                }
            }

            if let Ok(appdata) = std::env::var("APPDATA") {
                let candidate = PathBuf::from(appdata).join("nservo");
                if std::fs::create_dir_all(&candidate).is_ok() {
                    return candidate;
                }
            }
        }

        let local = PathBuf::from(".nservo");
        let _ = std::fs::create_dir_all(&local);
        local
    }

    pub fn init_lua(&self) -> PathBuf {
        self.base_dir.join("init.lua")
    }

    pub fn history_db(&self) -> PathBuf {
        self.base_dir.join("history.db")
    }

    pub fn bookmarks_db(&self) -> PathBuf {
        self.base_dir.join("bookmarks.db")
    }

    pub fn session_file(&self) -> PathBuf {
        self.base_dir.join("session.json")
    }

    pub fn trusted_plugins_file(&self) -> PathBuf {
        self.base_dir.join("trusted_plugins.json")
    }

    pub fn filters_dir(&self) -> PathBuf {
        self.base_dir.join("filters")
    }

    pub fn ads_file(&self) -> PathBuf {
        self.filters_dir().join("ads.txt")
    }

    pub fn plugins_dir(&self) -> PathBuf {
        self.base_dir.join("plugins")
    }

    pub fn greasemonkey_dir(&self) -> PathBuf {
        self.base_dir.join("greasemonkey")
    }
}

impl Default for ConfigPaths {
    fn default() -> Self {
        Self::new()
    }
}
