use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    root: PathBuf,
    settings_file: PathBuf,
    themes_dir: PathBuf,
    logs_dir: PathBuf,
}

impl AppPaths {
    pub fn discover() -> anyhow::Result<Self> {
        let project = directories::ProjectDirs::from("", "", "CodexSkinLite")
            .ok_or_else(|| anyhow::anyhow!("macOS application support directory is unavailable"))?;
        Ok(Self::from_root(project.data_dir()))
    }

    pub fn for_test(root: &Path) -> Self {
        Self::from_root(root)
    }

    fn from_root(root: &Path) -> Self {
        let root = root.to_path_buf();
        Self {
            settings_file: root.join("settings.json"),
            themes_dir: root.join("themes"),
            logs_dir: root.join("logs"),
            root,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn settings_file(&self) -> &Path {
        &self.settings_file
    }

    pub fn settings_temp_file(&self) -> PathBuf {
        self.root.join("settings.json.tmp")
    }

    pub fn themes_dir(&self) -> &Path {
        &self.themes_dir
    }

    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }
}
