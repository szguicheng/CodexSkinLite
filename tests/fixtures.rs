#![allow(dead_code)]

use std::io::{Cursor, Write};

use base64::Engine as _;
use serde_json::json;
use sha2::{Digest, Sha256};

use codex_skin_lite::paths::AppPaths;
use codex_skin_lite::theme::ThemeStore;

pub struct ThemeEnvironment {
    pub _dir: tempfile::TempDir,
    pub paths: AppPaths,
    pub store: ThemeStore,
}

pub fn theme_environment() -> ThemeEnvironment {
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::for_test(dir.path());
    let store = ThemeStore::new(paths.clone());
    ThemeEnvironment {
        _dir: dir,
        paths,
        store,
    }
}

pub fn theme_environment_with_active(_id: &str) -> ThemeEnvironment {
    let env = theme_environment();
    env.store.import_zip_bytes(&valid_theme_zip()).unwrap();
    env
}

pub struct ThemeZipOptions {
    pub theme_id: String,
    pub platform: String,
    pub capabilities: Vec<String>,
    pub image_hash: Option<String>,
    pub image_name: String,
    pub image_media_type: String,
    pub image_bytes: Vec<u8>,
    pub extra_entry: Option<(String, Vec<u8>)>,
}

impl ThemeZipOptions {
    pub fn valid() -> Self {
        Self {
            theme_id: "eva-warm-cream".into(),
            platform: "macos".into(),
            capabilities: vec!["background".into(), "tokens".into(), "safe-css".into()],
            image_hash: None,
            image_name: "background.png".into(),
            image_media_type: "image/png".into(),
            image_bytes: base64::engine::general_purpose::STANDARD
                .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=")
                .unwrap(),
            extra_entry: None,
        }
    }
}

pub fn valid_theme_zip() -> Vec<u8> {
    theme_zip(ThemeZipOptions::valid())
}

pub fn theme_zip(options: ThemeZipOptions) -> Vec<u8> {
    let css = br#"
        [data-ds-part="main"] { background-color: #111111; }
        [data-ds-part="composer"] { border-radius: 18px; backdrop-filter: blur(12px); opacity: 0.80; }
    "#
    .to_vec();
    let theme = serde_json::to_vec(&json!({
        "schemaVersion": 1,
        "id": options.theme_id,
        "name": options.theme_id,
        "image": options.image_name,
        "appearance": "light",
        "art": { "focusX": 0.44, "focusY": 0.38, "safeArea": "none", "taskMode": "ambient" },
        "colors": {
            "background": "#fffaf0", "panel": "#fff8e8", "panelAlt": "#fff5df",
            "accent": "#e98f68", "accentAlt": "#f0a681", "secondary": "#c79579",
            "highlight": "#ffd6bd", "text": "#2b211d", "muted": "#8b7468", "line": "#ead8ca"
        }
    }))
    .unwrap();
    let image_hash = options
        .image_hash
        .unwrap_or_else(|| sha256(&options.image_bytes));
    let manifest = serde_json::to_vec(&json!({
        "packageVersion": 1,
        "themeId": options.theme_id,
        "version": "1.0.0",
        "skinApiVersion": 1,
        "minClientVersion": "0.0.0",
        "platforms": [options.platform],
        "capabilities": options.capabilities,
        "publisher": { "id": "fixture", "displayName": "Fixture" },
        "license": "MIT",
        "provenance": { "aiGenerated": false, "summary": "Test fixture" },
        "files": [
            { "path": "theme.json", "mediaType": "application/json", "bytes": theme.len(), "sha256": sha256(&theme) },
            { "path": options.image_name, "mediaType": options.image_media_type, "bytes": options.image_bytes.len(), "sha256": image_hash },
            { "path": "theme.css", "mediaType": "text/css", "bytes": css.len(), "sha256": sha256(&css) }
        ],
        "createdAt": "2026-08-31T00:00:00Z"
    }))
    .unwrap();

    let mut archive = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut archive);
        let file_options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, bytes) in [
            ("manifest.json", manifest.as_slice()),
            ("theme.json", theme.as_slice()),
            ("theme.css", css.as_slice()),
            (options.image_name.as_str(), options.image_bytes.as_slice()),
        ] {
            writer.start_file(name, file_options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        if let Some((name, bytes)) = options.extra_entry {
            writer.start_file(name, file_options).unwrap();
            writer.write_all(&bytes).unwrap();
        }
        writer.finish().unwrap();
    }
    archive.into_inner()
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
