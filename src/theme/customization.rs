use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::package::ThemeError;
use super::safe_css::compile_safe_css;

pub const CUSTOMIZATION_SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SurfacePart {
    Main,
    Sidebar,
    Thread,
    Message,
    Composer,
    Header,
}

impl SurfacePart {
    pub const ALL: [Self; 6] = [
        Self::Main,
        Self::Sidebar,
        Self::Thread,
        Self::Message,
        Self::Composer,
        Self::Header,
    ];

    pub const fn css_name(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Sidebar => "sidebar",
            Self::Thread => "thread",
            Self::Message => "message",
            Self::Composer => "composer",
            Self::Header => "header",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShadowPreset {
    None,
    Soft,
    Strong,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct ThemeCustomization {
    pub schema_version: u8,
    pub background: BackgroundCustomization,
    pub colors: PaletteCustomization,
    pub surfaces: BTreeMap<SurfacePart, SurfaceCustomization>,
    pub composer: ComposerCustomization,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct BackgroundCustomization {
    pub position_x: u8,
    pub position_y: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct PaletteCustomization {
    pub background: Option<String>,
    pub panel: Option<String>,
    pub accent: Option<String>,
    pub text: Option<String>,
    pub line: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct SurfaceCustomization {
    pub opacity: Option<u8>,
    pub blur_px: Option<u8>,
    pub radius_px: Option<u8>,
    pub shadow: Option<ShadowPreset>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct ComposerCustomization {
    pub bottom_inset_px: u16,
    pub horizontal_inset_px: u16,
}

impl Default for BackgroundCustomization {
    fn default() -> Self {
        Self {
            position_x: 50,
            position_y: 50,
        }
    }
}

impl Default for ThemeCustomization {
    fn default() -> Self {
        Self {
            schema_version: CUSTOMIZATION_SCHEMA_VERSION,
            background: BackgroundCustomization::default(),
            colors: PaletteCustomization::default(),
            surfaces: BTreeMap::new(),
            composer: ComposerCustomization::default(),
        }
    }
}

impl ThemeCustomization {
    pub fn normalized(mut self) -> Result<Self, ThemeError> {
        if self.schema_version != CUSTOMIZATION_SCHEMA_VERSION {
            return Err(ThemeError::InvalidCustomization(format!(
                "unsupported schema version: {}",
                self.schema_version
            )));
        }
        self.background.position_x = self.background.position_x.min(100);
        self.background.position_y = self.background.position_y.min(100);
        self.colors.background = normalize_color("background", self.colors.background)?;
        self.colors.panel = normalize_color("panel", self.colors.panel)?;
        self.colors.accent = normalize_color("accent", self.colors.accent)?;
        self.colors.text = normalize_color("text", self.colors.text)?;
        self.colors.line = normalize_color("line", self.colors.line)?;
        for surface in self.surfaces.values_mut() {
            surface.opacity = surface.opacity.map(|value| value.clamp(65, 100));
            surface.blur_px = surface.blur_px.map(|value| value.min(30));
            surface.radius_px = surface.radius_px.map(|value| value.min(28));
        }
        self.composer.bottom_inset_px = self.composer.bottom_inset_px.min(80);
        self.composer.horizontal_inset_px = self.composer.horizontal_inset_px.min(120);
        Ok(self)
    }

    pub fn is_default(&self) -> bool {
        self.background == BackgroundCustomization::default()
            && self.colors == PaletteCustomization::default()
            && self.surfaces.values().all(SurfaceCustomization::is_default)
            && self.composer == ComposerCustomization::default()
    }
}

impl SurfaceCustomization {
    fn is_default(&self) -> bool {
        self.opacity.is_none()
            && self.blur_px.is_none()
            && self.radius_px.is_none()
            && self.shadow.is_none()
    }
}

fn normalize_color(name: &str, value: Option<String>) -> Result<Option<String>, ThemeError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let Some(hex) = value.strip_prefix('#') else {
        return Err(ThemeError::InvalidCustomization(format!(
            "{name} must be a hexadecimal CSS color"
        )));
    };
    if !matches!(hex.len(), 3 | 4 | 6 | 8) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ThemeError::InvalidCustomization(format!(
            "{name} must use #RGB, #RGBA, #RRGGBB, or #RRGGBBAA"
        )));
    }
    Ok(Some(format!("#{hex}").to_ascii_lowercase()))
}

pub fn compile_customization_css(customization: &ThemeCustomization) -> Result<String, ThemeError> {
    let customization = customization.clone().normalized()?;
    let mut css = String::new();
    for part in SurfacePart::ALL {
        let Some(surface) = customization.surfaces.get(&part) else {
            continue;
        };
        if surface.is_default() {
            continue;
        }
        css.push_str(&format!("[data-ds-part=\"{}\"] {{", part.css_name()));
        if let Some(opacity) = surface.opacity {
            css.push_str(&format!(" opacity: {:.2};", f64::from(opacity) / 100.0));
        }
        if let Some(blur_px) = surface.blur_px {
            css.push_str(&format!(" backdrop-filter: blur({blur_px}px);"));
        }
        if let Some(radius_px) = surface.radius_px {
            css.push_str(&format!(" border-radius: {radius_px}px;"));
        }
        if let Some(shadow) = surface.shadow {
            css.push_str(" box-shadow: ");
            css.push_str(match shadow {
                ShadowPreset::None => "none",
                ShadowPreset::Soft => "0 8px 24px rgba(0,0,0,0.16)",
                ShadowPreset::Strong => "0 12px 32px rgba(0,0,0,0.24)",
            });
            css.push(';');
        }
        css.push_str(" }\n");
    }
    if css.is_empty() {
        return Ok(String::new());
    }
    compile_safe_css(&css)
}
