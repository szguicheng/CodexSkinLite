use std::collections::{BTreeMap, HashSet};
use std::io::{Cursor, Read};
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

const PACKAGE_LIMIT: usize = 32 * 1024 * 1024;
const UNPACKED_LIMIT: usize = 64 * 1024 * 1024;
const ENTRY_LIMIT: usize = 32;
const JSON_LIMIT: usize = 256 * 1024;
const CSS_LIMIT: usize = 256 * 1024;
const IMAGE_LIMIT: usize = 32 * 1024 * 1024;
const LICENSE_LIMIT: usize = 256 * 1024;
const SIGNATURE_LIMIT: usize = 16 * 1024;
const ALLOWED_FILES: &[&str] = &[
    "manifest.json",
    "theme.json",
    "theme.css",
    "background.webp",
    "background.jpg",
    "background.png",
    "LICENSE.txt",
    "manifest.sig",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DreamSkinPackageManifest {
    pub package_version: u8,
    pub theme_id: String,
    pub version: String,
    pub skin_api_version: u8,
    pub min_client_version: String,
    pub platforms: Vec<String>,
    pub capabilities: Vec<String>,
    pub publisher: DreamSkinPackagePublisher,
    pub license: String,
    pub provenance: DreamSkinPackageProvenance,
    pub files: Vec<DreamSkinPackageFile>,
    pub created_at: String,
    #[serde(default)]
    pub key_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DreamSkinPackagePublisher {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DreamSkinPackageProvenance {
    pub ai_generated: bool,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DreamSkinPackageFile {
    pub path: String,
    pub media_type: String,
    pub bytes: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedThemePackage {
    pub manifest: DreamSkinPackageManifest,
    pub manifest_bytes: Vec<u8>,
    pub theme: Value,
    pub theme_bytes: Vec<u8>,
    pub css: String,
    pub image_name: String,
    pub image_bytes: Vec<u8>,
    pub license_text: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("unsafe ZIP path: {0}")]
    UnsafePath(String),
    #[error("unsupported package file: {0}")]
    UnsupportedFile(String),
    #[error("invalid manifest: {0}")]
    InvalidManifest(String),
    #[error("invalid image: {0}")]
    InvalidImage(String),
    #[error("invalid CSS: {0}")]
    InvalidCss(String),
    #[error("archive limit exceeded: {0}")]
    Limit(String),
    #[error("active theme cannot be deleted")]
    ActiveTheme,
    #[error("stored theme is incomplete: {0}")]
    InvalidStoredTheme(String),
    #[error("invalid customization: {0}")]
    InvalidCustomization(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub fn validate_package(bytes: &[u8]) -> Result<ValidatedThemePackage, ThemeError> {
    if bytes.is_empty() || bytes.len() > PACKAGE_LIMIT {
        return Err(ThemeError::Limit("package exceeds 32 MiB".into()));
    }
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| ThemeError::Other(anyhow::anyhow!(error)))?;
    if archive.is_empty() || archive.len() > ENTRY_LIMIT {
        return Err(ThemeError::Limit(
            "entry count must be between 1 and 32".into(),
        ));
    }

    let mut unpacked = 0usize;
    let mut raw_entries = Vec::new();
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|error| ThemeError::Other(anyhow::anyhow!(error)))?;
        let name = file.name().to_string();
        validate_entry_name(&name)?;
        if file.encrypted() || file.is_symlink() {
            return Err(ThemeError::UnsafePath(name));
        }
        if file.is_dir() {
            continue;
        }
        let base_name = name.rsplit('/').next().unwrap_or_default();
        if base_name == ".DS_Store" || name.starts_with("__MACOSX/") {
            continue;
        }
        if !ALLOWED_FILES.contains(&base_name) {
            return Err(ThemeError::UnsupportedFile(name));
        }
        let limit = file_limit(base_name);
        let mut content = Vec::new();
        file.by_ref()
            .take((limit + 1) as u64)
            .read_to_end(&mut content)
            .map_err(|error| ThemeError::Other(error.into()))?;
        if content.is_empty() || content.len() > limit {
            return Err(ThemeError::Limit(format!(
                "{base_name} exceeds its size limit"
            )));
        }
        unpacked = unpacked.saturating_add(content.len());
        if unpacked > UNPACKED_LIMIT {
            return Err(ThemeError::Limit("unpacked package exceeds 64 MiB".into()));
        }
        raw_entries.push((name, content));
    }
    let mut files = normalize_entries(raw_entries)?;

    let manifest_bytes = take_required(&mut files, "manifest.json")?;
    let theme_bytes = take_required(&mut files, "theme.json")?;
    let css_bytes = take_required(&mut files, "theme.css")?;
    let image_names = ["background.webp", "background.jpg", "background.png"]
        .into_iter()
        .filter(|name| files.contains_key(*name))
        .collect::<Vec<_>>();
    if image_names.len() != 1 {
        return Err(ThemeError::InvalidImage(
            "package must contain exactly one background image".into(),
        ));
    }
    let image_name = image_names[0].to_string();
    let image_bytes = files.remove(&image_name).expect("image name was checked");
    validate_image(&image_name, &image_bytes)?;
    let license_bytes = files.remove("LICENSE.txt");
    let _signature = files.remove("manifest.sig");
    if !files.is_empty() {
        return Err(ThemeError::UnsupportedFile(
            files.keys().next().cloned().unwrap_or_default(),
        ));
    }

    let manifest: DreamSkinPackageManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| ThemeError::InvalidManifest(error.to_string()))?;
    let theme: Value = serde_json::from_slice(&theme_bytes)
        .map_err(|error| ThemeError::InvalidManifest(format!("theme.json: {error}")))?;
    let css = String::from_utf8(css_bytes.clone())
        .map_err(|error| ThemeError::InvalidCss(error.to_string()))?;
    super::safe_css::validate_safe_css(&css)?;
    validate_manifest(&manifest)?;
    validate_theme(&theme, &manifest, &image_name)?;
    validate_declared_files(
        &manifest,
        &theme_bytes,
        &css_bytes,
        &image_name,
        &image_bytes,
        license_bytes.as_deref(),
    )?;
    let license_text = license_bytes
        .map(String::from_utf8)
        .transpose()
        .map_err(|error| ThemeError::InvalidManifest(format!("LICENSE.txt: {error}")))?;

    Ok(ValidatedThemePackage {
        manifest,
        manifest_bytes,
        theme,
        theme_bytes,
        css,
        image_name,
        image_bytes,
        license_text,
    })
}

fn validate_entry_name(name: &str) -> Result<(), ThemeError> {
    if name.is_empty() || name.contains('\0') || name.contains('\\') || name.starts_with('/') {
        return Err(ThemeError::UnsafePath(name.into()));
    }
    if Path::new(name).components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(ThemeError::UnsafePath(name.into()));
    }
    Ok(())
}

fn normalize_entries(
    entries: Vec<(String, Vec<u8>)>,
) -> Result<BTreeMap<String, Vec<u8>>, ThemeError> {
    let common_prefix = entries
        .iter()
        .filter_map(|(name, _)| name.split_once('/').map(|(prefix, _)| prefix))
        .next()
        .filter(|prefix| {
            entries.iter().all(|(name, _)| {
                name.split_once('/')
                    .is_some_and(|(candidate, rest)| candidate == *prefix && !rest.contains('/'))
            })
        })
        .map(str::to_string);
    let mut normalized = BTreeMap::new();
    for (name, bytes) in entries {
        let name = if let Some(prefix) = &common_prefix {
            name.strip_prefix(&format!("{prefix}/"))
                .unwrap_or(&name)
                .to_string()
        } else {
            if name.contains('/') {
                return Err(ThemeError::UnsafePath(name));
            }
            name
        };
        if normalized.insert(name.clone(), bytes).is_some() {
            return Err(ThemeError::InvalidManifest(format!(
                "duplicate package file: {name}"
            )));
        }
    }
    Ok(normalized)
}

fn take_required(files: &mut BTreeMap<String, Vec<u8>>, name: &str) -> Result<Vec<u8>, ThemeError> {
    files
        .remove(name)
        .ok_or_else(|| ThemeError::InvalidManifest(format!("missing {name}")))
}

fn file_limit(name: &str) -> usize {
    match name {
        "manifest.json" | "theme.json" => JSON_LIMIT,
        "theme.css" => CSS_LIMIT,
        "LICENSE.txt" => LICENSE_LIMIT,
        "manifest.sig" => SIGNATURE_LIMIT,
        _ => IMAGE_LIMIT,
    }
}

fn validate_manifest(manifest: &DreamSkinPackageManifest) -> Result<(), ThemeError> {
    if manifest.package_version != 1 || manifest.skin_api_version != 1 {
        return Err(ThemeError::InvalidManifest(
            "unsupported package or Skin API version".into(),
        ));
    }
    if !valid_theme_id(&manifest.theme_id) {
        return Err(ThemeError::InvalidManifest("invalid themeId".into()));
    }
    semver::Version::parse(&manifest.version)
        .map_err(|error| ThemeError::InvalidManifest(format!("version: {error}")))?;
    semver::Version::parse(&manifest.min_client_version)
        .map_err(|error| ThemeError::InvalidManifest(format!("minClientVersion: {error}")))?;
    if !manifest.platforms.iter().any(|value| value == "macos") {
        return Err(ThemeError::InvalidManifest(
            "package does not support macos".into(),
        ));
    }
    for required in ["background", "tokens", "safe-css"] {
        if !manifest.capabilities.iter().any(|value| value == required) {
            return Err(ThemeError::InvalidManifest(format!(
                "missing required capability: {required}"
            )));
        }
    }
    if manifest.publisher.id.trim().is_empty()
        || manifest.publisher.display_name.trim().is_empty()
        || manifest.license.trim().is_empty()
        || manifest.provenance.summary.trim().is_empty()
    {
        return Err(ThemeError::InvalidManifest(
            "publisher, license, and provenance are required".into(),
        ));
    }
    let mut paths = HashSet::new();
    if manifest.files.is_empty()
        || manifest.files.iter().any(|file| {
            !paths.insert(file.path.as_str())
                || !ALLOWED_FILES.contains(&file.path.as_str())
                || file.sha256.len() != 64
                || !file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
    {
        return Err(ThemeError::InvalidManifest("invalid files list".into()));
    }
    Ok(())
}

fn validate_theme(
    theme: &Value,
    manifest: &DreamSkinPackageManifest,
    image_name: &str,
) -> Result<(), ThemeError> {
    if theme.get("schemaVersion").and_then(Value::as_u64) != Some(1)
        || theme.get("id").and_then(Value::as_str) != Some(manifest.theme_id.as_str())
        || theme.get("image").and_then(Value::as_str) != Some(image_name)
        || !theme.get("colors").is_some_and(Value::is_object)
    {
        return Err(ThemeError::InvalidManifest(
            "theme.json does not match manifest".into(),
        ));
    }
    Ok(())
}

fn validate_declared_files(
    manifest: &DreamSkinPackageManifest,
    theme: &[u8],
    css: &[u8],
    image_name: &str,
    image: &[u8],
    license: Option<&[u8]>,
) -> Result<(), ThemeError> {
    let actual = [
        ("theme.json", Some(theme)),
        ("theme.css", Some(css)),
        (image_name, Some(image)),
        ("LICENSE.txt", license),
    ];
    for file in &manifest.files {
        let expected_media_type = match file.path.as_str() {
            "theme.json" => "application/json",
            "theme.css" => "text/css",
            "background.png" => "image/png",
            "background.jpg" => "image/jpeg",
            "background.webp" => "image/webp",
            "LICENSE.txt" => "text/plain",
            "manifest.sig" => "application/octet-stream",
            _ => "",
        };
        if file.media_type != expected_media_type {
            return Err(ThemeError::InvalidManifest(format!(
                "media type mismatch: {}",
                file.path
            )));
        }
        let bytes = actual
            .iter()
            .find_map(|(name, bytes)| (*name == file.path).then_some(*bytes).flatten())
            .ok_or_else(|| {
                ThemeError::InvalidManifest(format!("declared file is missing: {}", file.path))
            })?;
        if file.bytes != bytes.len() || !file.sha256.eq_ignore_ascii_case(&sha256(bytes)) {
            return Err(ThemeError::InvalidManifest(format!(
                "file digest mismatch: {}",
                file.path
            )));
        }
    }
    for (name, _bytes) in actual.into_iter().filter(|(_, bytes)| bytes.is_some()) {
        if !manifest.files.iter().any(|file| file.path == name) {
            return Err(ThemeError::InvalidManifest(format!(
                "file is not declared: {name}"
            )));
        }
    }
    Ok(())
}

fn validate_image(name: &str, bytes: &[u8]) -> Result<(), ThemeError> {
    let valid = match name {
        "background.png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "background.jpg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "background.webp" => {
            bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP"
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(ThemeError::InvalidImage(format!(
            "image content does not match {name}"
        )))
    }
}

fn valid_theme_id(value: &str) -> bool {
    let len = value.len();
    (1..=128).contains(&len)
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'-') && index > 0
        })
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
