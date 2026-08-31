use std::fs::{self, File};
use std::io::Write;

use crate::model::AppSettings;
use crate::paths::AppPaths;

#[derive(Debug, Clone)]
pub struct SettingsStore {
    paths: AppPaths,
}

impl SettingsStore {
    pub fn new(paths: AppPaths) -> Self {
        Self { paths }
    }

    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    pub fn load(&self) -> anyhow::Result<AppSettings> {
        let bytes = match fs::read(self.paths.settings_file()) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(AppSettings::default());
            }
            Err(error) => return Err(error.into()),
        };
        let settings = serde_json::from_slice::<AppSettings>(&bytes).unwrap_or_default();
        Ok(normalize(settings))
    }

    pub fn save(&self, settings: &AppSettings) -> anyhow::Result<()> {
        fs::create_dir_all(self.paths.root())?;
        let normalized = normalize(settings.clone());
        let bytes = serde_json::to_vec_pretty(&normalized)?;
        let temporary = self.paths.settings_temp_file();
        let mut file = File::create(&temporary)?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        fs::rename(&temporary, self.paths.settings_file())?;
        Ok(())
    }
}

fn normalize(mut settings: AppSettings) -> AppSettings {
    if settings.debug_port == 0 {
        settings.debug_port = 9222;
    }
    settings.conversation_max_width = settings.conversation_max_width.clamp(320, 4000);
    settings.active_theme_id = settings
        .active_theme_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    settings
}
