use std::collections::HashSet;

use anyhow::{Context, bail};

use super::ThemeError;

const CSS_LIMIT: usize = 256 * 1024;

pub fn compile_safe_css(css: &str) -> Result<String, ThemeError> {
    compile_safe_css_inner(css).map_err(css_error)
}

fn css_error(error: anyhow::Error) -> ThemeError {
    ThemeError::InvalidCss(error.to_string())
}

pub fn validate_safe_css(css: &str) -> Result<(), ThemeError> {
    parse_safe_css(css).map(|_| ()).map_err(css_error)
}

/// Compile validated community CSS into the trusted cascade used by the renderer.
/// The package format deliberately forbids `!important`; the client adds it only
/// after validation so community rules cannot smuggle arbitrary declarations in.
fn compile_safe_css_inner(css: &str) -> anyhow::Result<String> {
    let rules = parse_safe_css(css)?;
    let mut compiled = String::from("@layer dreamskin-community {\n");
    for rule in rules {
        compiled.push_str("  ");
        compiled.push_str(rule.selector);
        compiled.push_str(" {\n");
        for (property, value) in &rule.declarations {
            compiled.push_str("    ");
            compiled.push_str(property);
            compiled.push_str(": ");
            compiled.push_str(value);
            compiled.push_str(" !important;\n");
            if *property == "background-color" && matches!(rule.part, "sidebar" | "main" | "home") {
                compiled.push_str("    background-image: none !important;\n");
            }
        }
        compiled.push_str("  }\n");

        if rule.part == "root" {
            let bridged = rule
                .declarations
                .iter()
                .filter(|(property, _)| {
                    matches!(
                        *property,
                        "background-color"
                            | "color"
                            | "font-family"
                            | "font-size"
                            | "font-weight"
                            | "letter-spacing"
                            | "line-height"
                    )
                })
                .collect::<Vec<_>>();
            if !bridged.is_empty() {
                compiled.push_str("  ");
                compiled.push_str(rule.selector);
                compiled.push_str(" body {\n");
                for (property, value) in bridged {
                    compiled.push_str(&format!("    {property}: {value} !important;\n"));
                }
                compiled.push_str("  }\n");
            }
        }
        if rule.part == "composer-toolbar"
            && let Some((_, value)) = rule
                .declarations
                .iter()
                .find(|(property, _)| *property == "color")
        {
            compiled.push_str("  ");
            compiled.push_str(rule.selector);
            compiled.push_str(" :where(button:not([class~=\"bg-token-foreground\"]), button:not([class~=\"bg-token-foreground\"]) *) {\n");
            compiled.push_str(&format!("    color: {value} !important;\n  }}\n"));
        }
    }
    compiled.push_str("}\n");
    Ok(compiled)
}

#[derive(Debug)]
pub(crate) struct SafeCssRule<'a> {
    pub(crate) selector: &'a str,
    pub(crate) part: &'a str,
    pub(crate) declarations: Vec<(&'a str, &'a str)>,
}

pub(crate) fn parse_safe_css(css: &str) -> anyhow::Result<Vec<SafeCssRule<'_>>> {
    if css.is_empty() || css.len() > CSS_LIMIT {
        bail!("theme.css 为空或超过 256 KiB 限制");
    }
    if css.chars().any(forbidden_css_character)
        || css.contains("/*")
        || css.contains("*/")
        || css.contains('\\')
    {
        bail!("theme.css 包含不允许的控制字符、注释或转义");
    }
    let allowed_parts = [
        "root",
        "sidebar",
        "main",
        "header",
        "home",
        "home-hero",
        "project-list",
        "thread",
        "message",
        "composer",
        "composer-toolbar",
        "composer-toolbar-empty",
        "dialog",
    ];
    let mut parsed = Vec::new();
    let mut declarations = 0usize;
    let mut rest = css;
    loop {
        rest = rest.trim_start_matches(|character: char| {
            character.is_ascii_whitespace() || character == '\u{000c}'
        });
        if rest.is_empty() {
            break;
        }
        let open = rest.find('{').context("theme.css 规则缺少开始括号")?;
        let selector = rest[..open].trim();
        if selector.is_empty()
            || selector.contains([',', '@', ';', '}'])
            || rest[open + 1..].find('{').is_some_and(|nested| {
                rest[open + 1..]
                    .find('}')
                    .is_none_or(|close| nested < close)
            })
        {
            bail!("theme.css 选择器或规则语法无效");
        }
        let close = open
            + 1
            + rest[open + 1..]
                .find('}')
                .context("theme.css 规则缺少结束括号")?;
        let Some(after_prefix) = selector.strip_prefix("[data-ds-part=\"") else {
            bail!("theme.css 只能选择 Skin API 注册的 data-ds-part");
        };
        let (part, suffix) = after_prefix
            .split_once("\"]")
            .context("theme.css data-ds-part 语法无效")?;
        if !allowed_parts.contains(&part)
            || (!suffix.is_empty() && suffix != ":hover" && suffix != ":focus-visible")
        {
            bail!("theme.css 使用了未注册的 Skin API part 或状态");
        }

        let mut rule_declarations = Vec::new();
        let mut seen = HashSet::new();
        for declaration in rest[open + 1..close]
            .split(';')
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let (property, value) = declaration
                .split_once(':')
                .context("theme.css 声明语法无效")?;
            let property = property.trim();
            let value = value.trim();
            if !property
                .bytes()
                .enumerate()
                .all(|(index, byte)| byte.is_ascii_lowercase() || (index > 0 && byte == b'-'))
                || !seen.insert(property)
            {
                bail!("theme.css CSS 属性无效或重复：{property}");
            }
            if value.is_empty()
                || value.chars().count() > 512
                || value.contains(['{', '}', '<', '>', '!', ';'])
                || !validate_property_value(property, value)
            {
                bail!("theme.css 属性值不受 Safe CSS 支持：{property}");
            }
            declarations += 1;
            if declarations > 512 {
                bail!("theme.css 声明数量超过限制");
            }
            rule_declarations.push((property, value));
        }
        if rule_declarations.is_empty() {
            bail!("theme.css 每条规则必须至少包含一个声明");
        }
        parsed.push(SafeCssRule {
            selector,
            part,
            declarations: rule_declarations,
        });
        if parsed.len() > 128 {
            bail!("theme.css 规则数量超过限制");
        }
        rest = &rest[close + 1..];
    }
    if parsed.is_empty() {
        bail!("theme.css 必须至少包含一条规则");
    }
    Ok(parsed)
}

fn forbidden_css_character(character: char) -> bool {
    matches!(
        character as u32,
        0x0000..=0x0008 | 0x000b | 0x000e..=0x001f | 0x007f..=0x009f
            | 0x2028 | 0x2029 | 0x200e | 0x200f | 0x202a..=0x202e | 0x2066..=0x2069 | 0xfeff
    )
}

fn validate_property_value(property: &str, value: &str) -> bool {
    const COLOR_PROPERTIES: &[&str] = &[
        "color",
        "background-color",
        "border-color",
        "border-top-color",
        "border-right-color",
        "border-bottom-color",
        "border-left-color",
    ];
    const WIDTH_PROPERTIES: &[&str] = &[
        "border-width",
        "border-top-width",
        "border-right-width",
        "border-bottom-width",
        "border-left-width",
    ];
    const STYLE_PROPERTIES: &[&str] = &[
        "border-style",
        "border-top-style",
        "border-right-style",
        "border-bottom-style",
        "border-left-style",
    ];
    const RADIUS_PROPERTIES: &[&str] = &[
        "border-radius",
        "border-top-left-radius",
        "border-top-right-radius",
        "border-bottom-right-radius",
        "border-bottom-left-radius",
    ];
    const SPACING_PROPERTIES: &[&str] = &["gap", "row-gap", "column-gap"];
    if COLOR_PROPERTIES.contains(&property) {
        return valid_css_color(value, property);
    }
    if WIDTH_PROPERTIES.contains(&property) {
        return repeated_values(value, 1, 4, |item| zero_or_px(item, 0.0, 4.0));
    }
    if STYLE_PROPERTIES.contains(&property) {
        return repeated_values(value, 1, 4, |item| {
            matches!(
                item.to_ascii_lowercase().as_str(),
                "none" | "solid" | "dashed" | "dotted"
            )
        });
    }
    if RADIUS_PROPERTIES.contains(&property) {
        return registered_var(value, &["--ds-theme-surface-radius"])
            || repeated_values(value, 1, 4, |item| zero_or_px(item, 0.0, 28.0));
    }
    if SPACING_PROPERTIES.contains(&property) {
        return zero_or_px(value, 0.0, 24.0);
    }
    match property {
        "box-shadow" => valid_box_shadow(value),
        "opacity" => {
            registered_var(value, &["--ds-theme-surface-opacity"]) || numeric(value, "", 0.65, 1.0)
        }
        "backdrop-filter" => valid_backdrop_filter(value),
        "font-family" => valid_font_family(value),
        "font-size" => numeric(value, "px", 12.0, 20.0),
        "font-weight" => matches!(
            value.to_ascii_lowercase().as_str(),
            "400" | "500" | "600" | "700" | "normal" | "bold"
        ),
        "line-height" => numeric(value, "", 1.1, 1.8),
        "letter-spacing" => value == "0" || numeric(value, "px", 0.0, 2.0),
        "transition-duration" => valid_transition_duration(value),
        "transition-property" => valid_transition_property(value),
        _ => false,
    }
}

fn split_top_level(value: &str, separator: char) -> Option<Vec<&str>> {
    let mut values = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;
    for (index, character) in value.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
            }
            _ if character == separator && depth == 0 => {
                values.push(value[start..index].trim());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    values.push(value[start..].trim());
    Some(values)
}

fn split_whitespace(value: &str) -> Option<Vec<&str>> {
    let mut values = Vec::new();
    let mut start = None;
    let mut depth = 0i32;
    for (index, character) in value.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
            }
            _ => {}
        }
        if character.is_ascii_whitespace() && depth == 0 {
            if let Some(begin) = start.take() {
                values.push(&value[begin..index]);
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }
    if depth != 0 {
        return None;
    }
    if let Some(begin) = start {
        values.push(&value[begin..]);
    }
    Some(values)
}

fn numeric(value: &str, unit: &str, minimum: f64, maximum: f64) -> bool {
    let number = if unit.is_empty() {
        value
    } else if value.len() >= unit.len()
        && value[value.len() - unit.len()..].eq_ignore_ascii_case(unit)
    {
        &value[..value.len() - unit.len()]
    } else {
        return false;
    };
    if !valid_number_syntax(number) {
        return false;
    }
    number
        .parse::<f64>()
        .is_ok_and(|number| number.is_finite() && number >= minimum && number <= maximum)
}

fn valid_number_syntax(value: &str) -> bool {
    let value = value.strip_prefix('-').unwrap_or(value);
    if value.is_empty() {
        return false;
    }
    let (integer, fraction) = value
        .split_once('.')
        .map_or((value, None), |(left, right)| (left, Some(right)));
    if fraction.is_some_and(|fraction| {
        fraction.is_empty() || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    }) {
        return false;
    }
    if integer.is_empty() {
        return fraction.is_some();
    }
    integer.bytes().all(|byte| byte.is_ascii_digit())
        && (integer == "0" || !integer.starts_with('0'))
}

fn zero_or_px(value: &str, minimum: f64, maximum: f64) -> bool {
    value == "0" || numeric(value, "px", minimum, maximum)
}

fn registered_var(value: &str, allowed: &[&str]) -> bool {
    value
        .strip_prefix("var(")
        .and_then(|value| value.strip_suffix(')'))
        .map(str::trim)
        .is_some_and(|name| allowed.contains(&name))
}

fn valid_css_color(value: &str, property: &str) -> bool {
    const COLOR_VARIABLES: &[&str] = &[
        "--ds-theme-color-background",
        "--ds-theme-color-panel",
        "--ds-theme-color-panel-alt",
        "--ds-theme-color-accent",
        "--ds-theme-color-accent-alt",
        "--ds-theme-color-secondary",
        "--ds-theme-color-highlight",
        "--ds-theme-color-text",
        "--ds-theme-color-muted",
        "--ds-theme-color-line",
    ];
    if registered_var(value, COLOR_VARIABLES) {
        return true;
    }
    if let Some(hex) = value.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8)
            && hex.bytes().all(|byte| byte.is_ascii_hexdigit());
    }
    if value.eq_ignore_ascii_case("currentcolor") {
        return true;
    }
    if value.eq_ignore_ascii_case("transparent") {
        return property != "color";
    }
    let (kind, inside) = if value.len() > 5
        && value
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("rgb("))
        && value.ends_with(')')
    {
        ("rgb", &value[4..value.len() - 1])
    } else if value.len() > 6
        && value
            .get(..5)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("rgba("))
        && value.ends_with(')')
    {
        ("rgba", &value[5..value.len() - 1])
    } else {
        return false;
    };
    let Some(parts) = split_top_level(inside, ',') else {
        return false;
    };
    let expected = if kind == "rgb" { 3 } else { 4 };
    parts.len() == expected
        && parts[..3].iter().all(|part| color_channel(part))
        && (expected == 3 || alpha_channel(parts[3]))
}

fn color_channel(value: &str) -> bool {
    value.strip_suffix('%').map_or_else(
        || {
            value
                .parse::<u16>()
                .is_ok_and(|number| number <= 255 && (value == "0" || !value.starts_with('0')))
        },
        |number| numeric(number, "", 0.0, 100.0),
    )
}

fn alpha_channel(value: &str) -> bool {
    value.strip_suffix('%').map_or_else(
        || numeric(value, "", 0.0, 1.0),
        |number| numeric(number, "", 0.0, 100.0),
    )
}

fn repeated_values(
    value: &str,
    minimum: usize,
    maximum: usize,
    validator: impl Fn(&str) -> bool,
) -> bool {
    split_whitespace(value).is_some_and(|items| {
        (minimum..=maximum).contains(&items.len()) && items.into_iter().all(validator)
    })
}

fn valid_box_shadow(value: &str) -> bool {
    if value.eq_ignore_ascii_case("none") {
        return true;
    }
    split_top_level(value, ',').is_some_and(|shadows| {
        (1..=2).contains(&shadows.len())
            && shadows.into_iter().all(|shadow| {
                let Some(mut values) = split_whitespace(shadow) else {
                    return false;
                };
                if values
                    .first()
                    .is_some_and(|value| value.eq_ignore_ascii_case("inset"))
                {
                    values.remove(0);
                }
                if !(3..=5).contains(&values.len()) {
                    return false;
                }
                let color = values.pop().unwrap_or_default();
                valid_css_color(color, "box-shadow")
                    && (2..=4).contains(&values.len())
                    && zero_or_px(values[0], -32.0, 32.0)
                    && zero_or_px(values[1], -32.0, 32.0)
                    && values
                        .get(2)
                        .is_none_or(|value| zero_or_px(value, 0.0, 48.0))
                    && values
                        .get(3)
                        .is_none_or(|value| zero_or_px(value, -8.0, 16.0))
            })
    })
}

fn valid_font_family(value: &str) -> bool {
    split_top_level(value, ',').is_some_and(|families| {
        !families.is_empty()
            && families.len() <= 4
            && families.into_iter().all(|family| {
                matches!(
                    family.to_ascii_lowercase().as_str(),
                    "system-ui"
                        | "-apple-system"
                        | "blinkmacsystemfont"
                        | "ui-sans-serif"
                        | "ui-rounded"
                        | "ui-serif"
                        | "ui-monospace"
                        | "sans-serif"
                        | "serif"
                        | "monospace"
                )
            })
    })
}

fn valid_transition_duration(value: &str) -> bool {
    split_top_level(value, ',').is_some_and(|durations| {
        !durations.is_empty()
            && durations.len() <= 4
            && durations.into_iter().all(|duration| {
                duration == "0"
                    || numeric(duration, "ms", 0.0, 400.0)
                    || numeric(duration, "s", 0.0, 0.4)
            })
    })
}

fn transition_targets() -> &'static [&'static str] {
    &[
        "color",
        "background-color",
        "border-color",
        "border-top-color",
        "border-right-color",
        "border-bottom-color",
        "border-left-color",
        "border-width",
        "border-top-width",
        "border-right-width",
        "border-bottom-width",
        "border-left-width",
        "border-radius",
        "border-top-left-radius",
        "border-top-right-radius",
        "border-bottom-right-radius",
        "border-bottom-left-radius",
        "gap",
        "row-gap",
        "column-gap",
        "box-shadow",
        "opacity",
        "backdrop-filter",
        "font-size",
        "font-weight",
        "line-height",
        "letter-spacing",
    ]
}

fn valid_transition_property(value: &str) -> bool {
    split_top_level(value, ',').is_some_and(|properties| {
        !properties.is_empty()
            && properties.len() <= 4
            && properties
                .into_iter()
                .all(|property| transition_targets().contains(&property))
    })
}

fn valid_backdrop_filter(value: &str) -> bool {
    if value.eq_ignore_ascii_case("none") {
        return true;
    }
    let Some(filters) = split_whitespace(value) else {
        return false;
    };
    if filters.is_empty() || filters.len() > 4 {
        return false;
    }
    let mut seen = HashSet::new();
    for (index, filter) in filters.into_iter().enumerate() {
        let Some((name, argument)) = filter.split_once('(') else {
            return false;
        };
        let Some(argument) = argument.strip_suffix(')') else {
            return false;
        };
        let name = name.to_ascii_lowercase();
        let argument = argument.trim();
        if !seen.insert(name.clone()) {
            return false;
        }
        let valid = match name.as_str() {
            "blur" => {
                index == 0
                    && (registered_var(argument, &["--ds-theme-surface-blur"])
                        || zero_or_px(argument, 0.0, 30.0))
            }
            "saturate" => numeric(argument, "", 0.5, 2.0),
            "brightness" | "contrast" => numeric(argument, "", 0.8, 1.5),
            _ => false,
        };
        if !valid {
            return false;
        }
    }
    seen.contains("blur")
}
