use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::package::ThemeError;
use super::safe_css::{compile_safe_css, parse_safe_css};

pub const CUSTOMIZATION_SCHEMA_VERSION: u8 = 1;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum SurfacePart {
    #[default]
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

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Main => "主内容区",
            Self::Sidebar => "侧边栏",
            Self::Thread => "对话区域",
            Self::Message => "消息区域",
            Self::Composer => "输入框",
            Self::Header => "顶部栏",
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackgroundFillMode {
    #[default]
    Cover,
    Contain,
    Stretch,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct BackgroundImageCustomization {
    pub file_name: String,
    #[serde(skip)]
    pub source_path: Option<PathBuf>,
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
    pub position_x: Option<u8>,
    pub position_y: Option<u8>,
    pub offset_x_px: i16,
    pub offset_y_px: i16,
    pub fill_mode: BackgroundFillMode,
    pub opacity: u8,
    pub use_native_bottom_gradient: bool,
    pub image: Option<BackgroundImageCustomization>,
}

impl Default for BackgroundCustomization {
    fn default() -> Self {
        Self {
            position_x: None,
            position_y: None,
            offset_x_px: 0,
            offset_y_px: 0,
            fill_mode: BackgroundFillMode::Cover,
            opacity: 100,
            use_native_bottom_gradient: true,
            image: None,
        }
    }
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
        self.background.position_x = self.background.position_x.map(|value| value.min(100));
        self.background.position_y = self.background.position_y.map(|value| value.min(100));
        self.background.offset_x_px = self.background.offset_x_px.clamp(-2000, 2000);
        self.background.offset_y_px = self.background.offset_y_px.clamp(-2000, 2000);
        self.background.opacity = self.background.opacity.min(100);
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

impl ThemeCustomization {
    pub(crate) fn with_saved_overrides(mut self, overrides: &Self) -> Self {
        self.background.use_native_bottom_gradient =
            overrides.background.use_native_bottom_gradient;
        if overrides.background.position_x.is_some() {
            self.background.position_x = overrides.background.position_x;
        }
        if overrides.background.position_y.is_some() {
            self.background.position_y = overrides.background.position_y;
        }
        if overrides.background.offset_x_px != 0 {
            self.background.offset_x_px = overrides.background.offset_x_px;
        }
        if overrides.background.offset_y_px != 0 {
            self.background.offset_y_px = overrides.background.offset_y_px;
        }
        if overrides.background.fill_mode != BackgroundFillMode::Cover {
            self.background.fill_mode = overrides.background.fill_mode;
        }
        if overrides.background.opacity != 100 {
            self.background.opacity = overrides.background.opacity;
        }
        self.background.image = overrides.background.image.clone().or(self.background.image);
        self.colors.background = overrides
            .colors
            .background
            .clone()
            .or(self.colors.background);
        self.colors.panel = overrides.colors.panel.clone().or(self.colors.panel);
        self.colors.accent = overrides.colors.accent.clone().or(self.colors.accent);
        self.colors.text = overrides.colors.text.clone().or(self.colors.text);
        self.colors.line = overrides.colors.line.clone().or(self.colors.line);
        for (part, saved) in &overrides.surfaces {
            let current = self.surfaces.entry(*part).or_default();
            current.opacity = saved.opacity.or(current.opacity);
            current.blur_px = saved.blur_px.or(current.blur_px);
            current.radius_px = saved.radius_px.or(current.radius_px);
            current.shadow = saved.shadow.or(current.shadow);
        }
        if overrides.composer.bottom_inset_px != 0 {
            self.composer.bottom_inset_px = overrides.composer.bottom_inset_px;
        }
        if overrides.composer.horizontal_inset_px != 0 {
            self.composer.horizontal_inset_px = overrides.composer.horizontal_inset_px;
        }
        self
    }
}

pub(crate) fn package_defaults(theme: &Value, css: &str) -> ThemeCustomization {
    let mut customization = ThemeCustomization::default();
    customization.background.position_x = theme
        .get("art")
        .and_then(|art| art.get("focusX"))
        .and_then(focus_percent);
    customization.background.position_y = theme
        .get("art")
        .and_then(|art| art.get("focusY"))
        .and_then(focus_percent);
    customization.colors.background = theme_color(theme, "background");
    customization.colors.panel = theme_color(theme, "panel");
    customization.colors.accent = theme_color(theme, "accent");
    customization.colors.text = theme_color(theme, "text");
    customization.colors.line = theme_color(theme, "line");

    if let Ok(rules) = parse_safe_css(css) {
        for rule in rules {
            let Some(part) = SurfacePart::ALL.into_iter().find(|part| {
                rule.part == part.css_name()
                    && rule.selector == format!("[data-ds-part=\"{}\"]", part.css_name())
            }) else {
                continue;
            };
            let surface = customization.surfaces.entry(part).or_default();
            for (property, value) in rule.declarations {
                match property {
                    "opacity" => surface.opacity = parse_opacity(value),
                    "backdrop-filter" => surface.blur_px = parse_blur(value),
                    "border-radius" => surface.radius_px = parse_px(value, 28),
                    "box-shadow" if value.eq_ignore_ascii_case("none") => {
                        surface.shadow = Some(ShadowPreset::None)
                    }
                    _ => {}
                }
            }
        }
    }
    customization
}

fn theme_color(theme: &Value, name: &str) -> Option<String> {
    let value = theme.get("colors")?.get(name)?.as_str()?.to_string();
    normalize_color(name, Some(value)).ok().flatten()
}

fn focus_percent(value: &Value) -> Option<u8> {
    let value = value.as_f64()?;
    value
        .is_finite()
        .then(|| (value.clamp(0.0, 1.0) * 100.0).round() as u8)
}

fn parse_opacity(value: &str) -> Option<u8> {
    let value = value.parse::<f64>().ok()?;
    (value.is_finite() && (0.65..=1.0).contains(&value)).then(|| (value * 100.0).round() as u8)
}

fn parse_blur(value: &str) -> Option<u8> {
    value.split_whitespace().find_map(|token| {
        token
            .strip_prefix("blur(")
            .and_then(|value| value.strip_suffix(')'))
            .and_then(|value| parse_px(value, 30))
    })
}

fn parse_px(value: &str, maximum: u8) -> Option<u8> {
    let value = value.trim();
    let number = if value == "0" {
        0.0
    } else {
        value.strip_suffix("px")?.parse::<f64>().ok()?
    };
    (number.is_finite() && (0.0..=f64::from(maximum)).contains(&number))
        .then(|| number.round() as u8)
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
