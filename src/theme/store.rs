use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::paths::AppPaths;

use super::{
    ThemeCustomization, ThemeError, compile_customization_css, compile_safe_css, validate_package,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThemeSummary {
    pub id: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemePayload {
    pub id: String,
    pub signature: String,
    pub theme: serde_json::Value,
    pub compiled_css: String,
    pub customization: ThemeCustomization,
    pub image_mime: String,
    pub image_base64: String,
}

#[derive(Debug, Clone)]
pub struct ThemeStore {
    paths: AppPaths,
}

impl ThemeStore {
    pub fn new(paths: AppPaths) -> Self {
        Self { paths }
    }

    pub fn import_zip_bytes(&self, bytes: &[u8]) -> Result<ThemeSummary, ThemeError> {
        let package = validate_package(bytes)?;
        let compiled = compile_safe_css(&package.css)?;
        fs::create_dir_all(self.paths.themes_dir()).map_err(anyhow_error)?;
        let staging = tempfile::Builder::new()
            .prefix(".importing-")
            .tempdir_in(self.paths.themes_dir())
            .map_err(anyhow_error)?;
        let staging_path = staging.path();
        write_sync(&staging_path.join("manifest.json"), &package.manifest_bytes)?;
        write_sync(&staging_path.join("theme.json"), &package.theme_bytes)?;
        write_sync(&staging_path.join("theme.css"), package.css.as_bytes())?;
        write_sync(&staging_path.join("compiled.css"), compiled.as_bytes())?;
        write_sync(
            &staging_path.join(&package.image_name),
            &package.image_bytes,
        )?;
        if let Some(license) = &package.license_text {
            write_sync(&staging_path.join("LICENSE.txt"), license.as_bytes())?;
        }

        let target = self.paths.themes_dir().join(&package.manifest.theme_id);
        reject_symlink(&target)?;
        let backup = self.paths.themes_dir().join(format!(
            ".backup-{}-{}",
            package.manifest.theme_id,
            unique_suffix()
        ));
        let had_target = target.exists();
        if had_target {
            fs::rename(&target, &backup).map_err(anyhow_error)?;
        }
        let published = staging.keep();
        if let Err(error) = fs::rename(&published, &target) {
            if had_target {
                let _ = fs::rename(&backup, &target);
            }
            return Err(anyhow_error(error));
        }
        if had_target {
            fs::remove_dir_all(&backup).map_err(anyhow_error)?;
        }
        Ok(ThemeSummary {
            id: package.manifest.theme_id,
            name: package
                .theme
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Imported Theme")
                .to_string(),
            version: package.manifest.version,
        })
    }

    pub fn list(&self) -> Result<Vec<ThemeSummary>, ThemeError> {
        if !self.paths.themes_dir().exists() {
            return Ok(Vec::new());
        }
        let mut themes = Vec::new();
        for entry in fs::read_dir(self.paths.themes_dir()).map_err(anyhow_error)? {
            let entry = entry.map_err(anyhow_error)?;
            let metadata = fs::symlink_metadata(entry.path()).map_err(anyhow_error)?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                continue;
            }
            let manifest_bytes = match fs::read(entry.path().join("manifest.json")) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };
            let theme_bytes = match fs::read(entry.path().join("theme.json")) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };
            let manifest =
                match serde_json::from_slice::<super::DreamSkinPackageManifest>(&manifest_bytes) {
                    Ok(value) => value,
                    Err(_) => continue,
                };
            let theme = serde_json::from_slice::<serde_json::Value>(&theme_bytes).ok();
            themes.push(ThemeSummary {
                id: manifest.theme_id,
                name: theme
                    .as_ref()
                    .and_then(|value| value.get("name"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Imported Theme")
                    .to_string(),
                version: manifest.version,
            });
        }
        themes.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
        Ok(themes)
    }

    pub fn load_payload(&self, id: &str) -> Result<ThemePayload, ThemeError> {
        let customization = self.load_customization(id)?;
        self.load_payload_inner(id, &customization)
    }

    pub fn load_payload_with_customization(
        &self,
        id: &str,
        customization: &ThemeCustomization,
    ) -> Result<ThemePayload, ThemeError> {
        let customization = customization.clone().normalized()?;
        self.load_payload_inner(id, &customization)
    }

    pub fn load_customization(&self, id: &str) -> Result<ThemeCustomization, ThemeError> {
        validate_stored_id(id)?;
        let directory = self.paths.themes_dir().join(id);
        ensure_real_directory(&directory)?;
        let path = directory.join("customization.json");
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ThemeCustomization::default());
            }
            Err(error) => {
                tracing::warn!(theme_id = id, %error, "unable to read theme customization");
                return Ok(ThemeCustomization::default());
            }
        };
        if bytes.len() > 32 * 1024 {
            tracing::warn!(theme_id = id, "theme customization exceeds 32 KiB");
            return Ok(ThemeCustomization::default());
        }
        match serde_json::from_slice::<ThemeCustomization>(&bytes)
            .map_err(|error| ThemeError::InvalidCustomization(error.to_string()))
            .and_then(ThemeCustomization::normalized)
        {
            Ok(customization) => Ok(customization),
            Err(error) => {
                tracing::warn!(theme_id = id, %error, "ignoring invalid theme customization");
                Ok(ThemeCustomization::default())
            }
        }
    }

    pub fn save_customization(
        &self,
        id: &str,
        customization: &ThemeCustomization,
    ) -> Result<(), ThemeError> {
        validate_stored_id(id)?;
        let directory = self.paths.themes_dir().join(id);
        ensure_real_directory(&directory)?;
        let customization = customization.clone().normalized()?;
        let path = directory.join("customization.json");
        if customization.is_default() {
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(anyhow_error(error)),
            }
            return Ok(());
        }
        let bytes = serde_json::to_vec_pretty(&customization)
            .map_err(|error| ThemeError::InvalidCustomization(error.to_string()))?;
        write_sync(&directory.join("customization.json.tmp"), &bytes)?;
        fs::rename(directory.join("customization.json.tmp"), path).map_err(anyhow_error)
    }

    fn load_payload_inner(
        &self,
        id: &str,
        customization: &ThemeCustomization,
    ) -> Result<ThemePayload, ThemeError> {
        validate_stored_id(id)?;
        let directory = self.paths.themes_dir().join(id);
        ensure_real_directory(&directory)?;
        let manifest_bytes = read_required(&directory.join("manifest.json"))?;
        let manifest = serde_json::from_slice::<super::DreamSkinPackageManifest>(&manifest_bytes)
            .map_err(|error| {
            ThemeError::InvalidStoredTheme(format!("manifest.json: {error}"))
        })?;
        if manifest.theme_id != id {
            return Err(ThemeError::InvalidStoredTheme(
                "manifest themeId does not match directory".into(),
            ));
        }
        let theme_bytes = read_required(&directory.join("theme.json"))?;
        let compiled_bytes = read_required(&directory.join("compiled.css"))?;
        let theme = serde_json::from_slice::<serde_json::Value>(&theme_bytes)
            .map_err(|error| ThemeError::InvalidStoredTheme(format!("theme.json: {error}")))?;
        if theme.get("id").and_then(serde_json::Value::as_str) != Some(id) {
            return Err(ThemeError::InvalidStoredTheme(
                "theme.json id does not match directory".into(),
            ));
        }
        let mut compiled_css = String::from_utf8(compiled_bytes.clone())
            .map_err(|error| ThemeError::InvalidStoredTheme(format!("compiled.css: {error}")))?;
        let customization_css = compile_customization_css(customization)?;
        if !customization_css.is_empty() {
            compiled_css.push('\n');
            compiled_css.push_str(&customization_css);
        }
        let (image_path, image_mime) = find_image(&directory)?;
        let image = read_required(&image_path)?;
        let mut hasher = Sha256::new();
        hasher.update(id.as_bytes());
        hasher.update(&theme_bytes);
        hasher.update(&compiled_bytes);
        hasher.update(&image);
        Ok(ThemePayload {
            id: id.to_string(),
            signature: format!("{:x}", hasher.finalize()),
            theme,
            compiled_css,
            customization: customization.clone(),
            image_mime: image_mime.into(),
            image_base64: base64::engine::general_purpose::STANDARD.encode(image),
        })
    }

    pub fn delete(&self, id: &str, active_id: Option<&str>) -> Result<(), ThemeError> {
        validate_stored_id(id)?;
        if active_id == Some(id) {
            return Err(ThemeError::ActiveTheme);
        }
        let directory = self.paths.themes_dir().join(id);
        if !directory.exists() {
            return Ok(());
        }
        ensure_real_directory(&directory)?;
        fs::remove_dir_all(directory).map_err(anyhow_error)
    }
}

fn write_sync(path: &Path, bytes: &[u8]) -> Result<(), ThemeError> {
    let mut file = File::create(path).map_err(anyhow_error)?;
    file.write_all(bytes).map_err(anyhow_error)?;
    file.sync_all().map_err(anyhow_error)
}

fn read_required(path: &Path) -> Result<Vec<u8>, ThemeError> {
    fs::read(path)
        .map_err(|error| ThemeError::InvalidStoredTheme(format!("{}: {error}", path.display())))
}

fn ensure_real_directory(path: &Path) -> Result<(), ThemeError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| ThemeError::InvalidStoredTheme(format!("{}: {error}", path.display())))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ThemeError::InvalidStoredTheme(format!(
            "{} is not a real directory",
            path.display()
        )));
    }
    Ok(())
}

fn reject_symlink(path: &Path) -> Result<(), ThemeError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(ThemeError::InvalidStoredTheme(
            format!("{} is a symbolic link", path.display()),
        )),
        Ok(_) | Err(_) => Ok(()),
    }
}

fn find_image(directory: &Path) -> Result<(PathBuf, &'static str), ThemeError> {
    let candidates = [
        ("background.webp", "image/webp"),
        ("background.jpg", "image/jpeg"),
        ("background.png", "image/png"),
    ]
    .into_iter()
    .filter_map(|(name, mime)| {
        let path = directory.join(name);
        path.is_file().then_some((path, mime))
    })
    .collect::<Vec<_>>();
    if candidates.len() != 1 {
        return Err(ThemeError::InvalidStoredTheme(
            "expected exactly one background image".into(),
        ));
    }
    Ok(candidates.into_iter().next().expect("one candidate"))
}

fn validate_stored_id(id: &str) -> Result<(), ThemeError> {
    if id.is_empty()
        || id.len() > 128
        || !id.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'-') && index > 0
        })
    {
        return Err(ThemeError::InvalidStoredTheme("invalid theme id".into()));
    }
    Ok(())
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn anyhow_error(error: impl Into<anyhow::Error>) -> ThemeError {
    ThemeError::Other(error.into())
}
