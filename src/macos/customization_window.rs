use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::sel;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSButton, NSPopUpButton, NSScrollView, NSTextField, NSView, NSWindow,
    NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use crate::theme::{
    BackgroundFillMode, BackgroundImageCustomization, ShadowPreset, SurfacePart, ThemeCustomization,
};

use super::AppKitState;

const TAG_BACKGROUND_X: isize = 1001;
const TAG_BACKGROUND_Y: isize = 1002;
const TAG_BACKGROUND_OPACITY: isize = 1003;
const TAG_BACKGROUND_FILL_MODE: isize = 1004;
const TAG_BACKGROUND_IMAGE: isize = 1005;
const TAG_COLOR_BACKGROUND: isize = 1010;
const TAG_COLOR_PANEL: isize = 1011;
const TAG_COLOR_ACCENT: isize = 1012;
const TAG_COLOR_TEXT: isize = 1013;
const TAG_COLOR_LINE: isize = 1014;
const TAG_SURFACE_PART: isize = 1019;
const TAG_SURFACE_OPACITY: isize = 1020;
const TAG_SURFACE_BLUR: isize = 1021;
const TAG_SURFACE_RADIUS: isize = 1022;
const TAG_SURFACE_SHADOW: isize = 1023;
const TAG_COMPOSER_BOTTOM: isize = 1030;
const TAG_COMPOSER_HORIZONTAL: isize = 1031;
const TAG_STATUS: isize = 1040;

pub(super) fn show(
    mtm: MainThreadMarker,
    target: &AnyObject,
    state: &Arc<AppKitState>,
    slot: &RefCell<Option<Retained<NSWindow>>>,
) {
    if let Some(window) = slot.borrow_mut().take() {
        window.close();
    }
    let Some(snapshot) = state.snapshot() else {
        return;
    };
    let theme_id = snapshot.settings.active_theme_id.as_deref();
    let draft = if theme_id.is_some() {
        snapshot.active_theme_customization.clone()
    } else {
        ThemeCustomization::default()
    };
    state.set_customization_draft(draft.clone());
    state.set_customization_surface(SurfacePart::Main);

    let window_rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(660.0, 720.0));
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            window_rect,
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Resizable,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    window.setTitle(&NSString::from_str(&format!(
        "自定义主题：{}",
        theme_id.unwrap_or("无")
    )));
    unsafe { window.setReleasedWhenClosed(false) };
    window.setMinSize(NSSize::new(560.0, 520.0));
    window.center();

    let scroll = NSScrollView::initWithFrame(
        NSScrollView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(660.0, 720.0)),
    );
    scroll.setHasVerticalScroller(true);
    scroll.setDrawsBackground(false);
    let document = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(640.0, 980.0)),
    );
    scroll.setDocumentView(Some(&document));
    window.setContentView(Some(&scroll));

    add_label(&document, "主题自定义", 24.0, 930.0, 560.0, 28.0, mtm);
    add_label(
        &document,
        "预览只应用到当前 Codex，保存后才会写入主题配置。",
        24.0,
        902.0,
        580.0,
        22.0,
        mtm,
    );

    add_label(&document, "背景图片", 24.0, 858.0, 300.0, 24.0, mtm);
    add_button(
        &document,
        "选择图片…",
        24.0,
        818.0,
        110.0,
        target,
        sel!(selectCustomizationImage:),
        mtm,
    );
    add_label(
        &document,
        &format_image_status(draft.background.image.as_ref()),
        148.0,
        822.0,
        420.0,
        22.0,
        mtm,
    )
    .setTag(TAG_BACKGROUND_IMAGE);
    add_labeled_field(
        &document,
        "偏移 X px",
        TAG_BACKGROUND_X,
        &draft.background.offset_x_px.to_string(),
        24.0,
        778.0,
        mtm,
    );
    add_labeled_field(
        &document,
        "偏移 Y px",
        TAG_BACKGROUND_Y,
        &draft.background.offset_y_px.to_string(),
        220.0,
        778.0,
        mtm,
    );
    add_label(&document, "填充", 416.0, 782.0, 50.0, 22.0, mtm);
    let fill_popup = add_popup(
        &document,
        TAG_BACKGROUND_FILL_MODE,
        &["覆盖填充", "完整显示", "拉伸填充"],
        466.0,
        778.0,
        130.0,
        target,
        sel!(selectCustomizationFill:),
        mtm,
    );
    fill_popup.selectItemAtIndex(fill_mode_index(draft.background.fill_mode));
    add_labeled_field(
        &document,
        "透明度 0-100",
        TAG_BACKGROUND_OPACITY,
        &draft.background.opacity.to_string(),
        24.0,
        730.0,
        mtm,
    );

    add_label(
        &document,
        "颜色覆盖（留空表示跟随主题）",
        24.0,
        680.0,
        360.0,
        24.0,
        mtm,
    );
    add_labeled_field(
        &document,
        "背景",
        TAG_COLOR_BACKGROUND,
        draft.colors.background.as_deref().unwrap_or_default(),
        24.0,
        640.0,
        mtm,
    );
    add_labeled_field(
        &document,
        "面板",
        TAG_COLOR_PANEL,
        draft.colors.panel.as_deref().unwrap_or_default(),
        220.0,
        640.0,
        mtm,
    );
    add_labeled_field(
        &document,
        "强调",
        TAG_COLOR_ACCENT,
        draft.colors.accent.as_deref().unwrap_or_default(),
        416.0,
        640.0,
        mtm,
    );
    add_labeled_field(
        &document,
        "文字",
        TAG_COLOR_TEXT,
        draft.colors.text.as_deref().unwrap_or_default(),
        24.0,
        592.0,
        mtm,
    );
    add_labeled_field(
        &document,
        "分割线",
        TAG_COLOR_LINE,
        draft.colors.line.as_deref().unwrap_or_default(),
        220.0,
        592.0,
        mtm,
    );

    add_label(&document, "组件样式", 24.0, 530.0, 300.0, 24.0, mtm);
    let component_popup = add_popup(
        &document,
        TAG_SURFACE_PART,
        &SurfacePart::ALL
            .into_iter()
            .map(SurfacePart::display_name)
            .collect::<Vec<_>>(),
        24.0,
        490.0,
        230.0,
        target,
        sel!(selectCustomizationComponent:),
        mtm,
    );
    component_popup.selectItemAtIndex(0);
    add_labeled_field(
        &document,
        "透明度 65-100",
        TAG_SURFACE_OPACITY,
        "",
        24.0,
        442.0,
        mtm,
    );
    add_labeled_field(
        &document,
        "模糊 px",
        TAG_SURFACE_BLUR,
        "",
        220.0,
        442.0,
        mtm,
    );
    add_labeled_field(
        &document,
        "圆角 px",
        TAG_SURFACE_RADIUS,
        "",
        416.0,
        442.0,
        mtm,
    );
    add_label(&document, "阴影", 24.0, 402.0, 70.0, 22.0, mtm);
    let shadow_popup = add_popup(
        &document,
        TAG_SURFACE_SHADOW,
        &["跟随主题", "无阴影", "柔和", "明显"],
        96.0,
        398.0,
        180.0,
        target,
        sel!(selectCustomizationComponent:),
        mtm,
    );
    unsafe {
        shadow_popup.setTarget(None);
        shadow_popup.setAction(None);
    }
    shadow_popup.selectItemAtIndex(0);

    add_label(&document, "输入框位置", 24.0, 342.0, 300.0, 24.0, mtm);
    add_labeled_field(
        &document,
        "距底部 px",
        TAG_COMPOSER_BOTTOM,
        &draft.composer.bottom_inset_px.to_string(),
        24.0,
        302.0,
        mtm,
    );
    add_labeled_field(
        &document,
        "左右内缩 px",
        TAG_COMPOSER_HORIZONTAL,
        &draft.composer.horizontal_inset_px.to_string(),
        220.0,
        302.0,
        mtm,
    );

    add_label(
        &document,
        if theme_id.is_some() {
            "未预览"
        } else {
            "当前未选择主题：此处显示空白草稿"
        },
        24.0,
        240.0,
        560.0,
        24.0,
        mtm,
    )
    .setTag(TAG_STATUS);
    add_button(
        &document,
        "预览",
        24.0,
        180.0,
        100.0,
        target,
        sel!(previewCustomization:),
        mtm,
    );
    let save = add_button(
        &document,
        "保存",
        140.0,
        180.0,
        100.0,
        target,
        sel!(saveCustomization:),
        mtm,
    );
    save.setKeyEquivalent(&NSString::from_str("s"));
    add_button(
        &document,
        "恢复默认",
        256.0,
        180.0,
        120.0,
        target,
        sel!(resetCustomization:),
        mtm,
    );
    add_button(
        &document,
        "关闭",
        392.0,
        180.0,
        100.0,
        target,
        sel!(closeCustomization:),
        mtm,
    );

    populate_controls(&document, &draft, SurfacePart::Main);
    let _ = document.scrollRectToVisible(NSRect::new(
        NSPoint::new(0.0, 720.0),
        NSSize::new(640.0, 190.0),
    ));
    window.makeKeyAndOrderFront(None);
    *slot.borrow_mut() = Some(window);
}

pub(super) fn collect_draft(
    window: &NSWindow,
    state: &AppKitState,
) -> Result<ThemeCustomization, String> {
    let mut draft = state.customization_draft().unwrap_or_default();
    let content = window
        .contentView()
        .ok_or_else(|| "自定义窗口内容不可用".to_string())?;
    draft.background.offset_x_px =
        read_bounded_i16(&content, TAG_BACKGROUND_X, "图片水平偏移", -2000, 2000)?;
    draft.background.offset_y_px =
        read_bounded_i16(&content, TAG_BACKGROUND_Y, "图片垂直偏移", -2000, 2000)?;
    draft.background.opacity =
        read_bounded_u16(&content, TAG_BACKGROUND_OPACITY, "图片透明度", 100)? as u8;
    let fill_mode = popup(&content, TAG_BACKGROUND_FILL_MODE)
        .ok_or_else(|| "图片填充控件不可用".to_string())?
        .indexOfSelectedItem();
    draft.background.fill_mode = fill_mode_from_index(fill_mode)?;
    draft.colors.background = read_optional_color(&content, TAG_COLOR_BACKGROUND, "背景色")?;
    draft.colors.panel = read_optional_color(&content, TAG_COLOR_PANEL, "面板色")?;
    draft.colors.accent = read_optional_color(&content, TAG_COLOR_ACCENT, "强调色")?;
    draft.colors.text = read_optional_color(&content, TAG_COLOR_TEXT, "文字色")?;
    draft.colors.line = read_optional_color(&content, TAG_COLOR_LINE, "分割线颜色")?;

    let surface_part = state.customization_surface();
    let surface = draft.surfaces.entry(surface_part).or_default();
    surface.opacity = read_optional_bounded_u16(&content, TAG_SURFACE_OPACITY, "透明度", 100)?
        .map(|value| value as u8);
    surface.blur_px =
        read_optional_bounded_u16(&content, TAG_SURFACE_BLUR, "模糊", 30)?.map(|value| value as u8);
    surface.radius_px = read_optional_bounded_u16(&content, TAG_SURFACE_RADIUS, "圆角", 28)?
        .map(|value| value as u8);
    let shadow = popup(&content, TAG_SURFACE_SHADOW)
        .ok_or_else(|| "阴影控件不可用".to_string())?
        .indexOfSelectedItem();
    surface.shadow = match shadow {
        0 => None,
        1 => Some(ShadowPreset::None),
        2 => Some(ShadowPreset::Soft),
        3 => Some(ShadowPreset::Strong),
        _ => return Err("阴影选项无效".into()),
    };

    draft.composer.bottom_inset_px =
        read_bounded_u16(&content, TAG_COMPOSER_BOTTOM, "输入框底部距离", 80)?;
    draft.composer.horizontal_inset_px =
        read_bounded_u16(&content, TAG_COMPOSER_HORIZONTAL, "输入框左右距离", 120)?;
    draft.normalized().map_err(|error| error.to_string())
}

pub(super) fn select_surface(
    window: &NSWindow,
    state: &AppKitState,
    index: isize,
) -> Result<(), String> {
    let draft = collect_draft(window, state)?;
    state.set_customization_draft(draft.clone());
    let surface = *SurfacePart::ALL
        .get(usize::try_from(index).map_err(|_| "组件选项无效".to_string())?)
        .ok_or_else(|| "组件选项无效".to_string())?;
    state.set_customization_surface(surface);
    let content = window
        .contentView()
        .ok_or_else(|| "自定义窗口内容不可用".to_string())?;
    populate_surface_controls(&content, &draft, surface);
    Ok(())
}

pub(super) fn select_fill(
    window: &NSWindow,
    state: &AppKitState,
    index: isize,
) -> Result<(), String> {
    let mut draft = collect_draft(window, state)?;
    draft.background.fill_mode = fill_mode_from_index(index)?;
    state.set_customization_draft(draft);
    Ok(())
}

pub(super) fn set_image_path(
    window: &NSWindow,
    state: &AppKitState,
    path: PathBuf,
) -> Result<(), String> {
    if !crate::theme::custom_image_name(&path).is_some() {
        return Err("图片必须是 PNG、JPG 或 WebP".into());
    }
    let mut draft = state.customization_draft().unwrap_or_default();
    draft.background.image = Some(BackgroundImageCustomization {
        file_name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "已选择图片".into()),
        source_path: Some(path),
    });
    state.set_customization_draft(draft.clone());
    if let Some(content) = window.contentView()
        && let Some(field) = text_field(&content, TAG_BACKGROUND_IMAGE)
    {
        field.setStringValue(&NSString::from_str(&format_image_status(
            draft.background.image.as_ref(),
        )));
    }
    Ok(())
}

pub(super) fn reset_draft(window: &NSWindow, state: &AppKitState) {
    let draft = ThemeCustomization::default();
    state.set_customization_draft(draft.clone());
    state.set_customization_surface(SurfacePart::Main);
    if let Some(content) = window.contentView() {
        populate_controls(&content, &draft, SurfacePart::Main);
    }
}

pub(super) fn set_status(window: &NSWindow, message: &str) {
    if let Some(content) = window.contentView()
        && let Some(field) = text_field(&content, TAG_STATUS)
    {
        field.setStringValue(&NSString::from_str(message));
    }
}

fn populate_controls(content: &NSView, draft: &ThemeCustomization, surface: SurfacePart) {
    set_text_field(
        content,
        TAG_BACKGROUND_X,
        &draft.background.offset_x_px.to_string(),
    );
    set_text_field(
        content,
        TAG_BACKGROUND_Y,
        &draft.background.offset_y_px.to_string(),
    );
    if let Some(field) = text_field(content, TAG_BACKGROUND_IMAGE) {
        field.setStringValue(&NSString::from_str(&format_image_status(
            draft.background.image.as_ref(),
        )));
    }
    if let Some(popup) = popup(content, TAG_BACKGROUND_FILL_MODE) {
        popup.selectItemAtIndex(fill_mode_index(draft.background.fill_mode));
    }
    set_text_field(
        content,
        TAG_BACKGROUND_OPACITY,
        &draft.background.opacity.to_string(),
    );
    set_text_field(
        content,
        TAG_COLOR_BACKGROUND,
        draft.colors.background.as_deref().unwrap_or_default(),
    );
    set_text_field(
        content,
        TAG_COLOR_PANEL,
        draft.colors.panel.as_deref().unwrap_or_default(),
    );
    set_text_field(
        content,
        TAG_COLOR_ACCENT,
        draft.colors.accent.as_deref().unwrap_or_default(),
    );
    set_text_field(
        content,
        TAG_COLOR_TEXT,
        draft.colors.text.as_deref().unwrap_or_default(),
    );
    set_text_field(
        content,
        TAG_COLOR_LINE,
        draft.colors.line.as_deref().unwrap_or_default(),
    );
    if let Some(popup) = popup(content, TAG_SURFACE_PART) {
        let index = SurfacePart::ALL
            .into_iter()
            .position(|candidate| candidate == surface)
            .unwrap_or(0);
        popup.selectItemAtIndex(index as isize);
    }
    populate_surface_controls(content, draft, surface);
    set_text_field(
        content,
        TAG_COMPOSER_BOTTOM,
        &draft.composer.bottom_inset_px.to_string(),
    );
    set_text_field(
        content,
        TAG_COMPOSER_HORIZONTAL,
        &draft.composer.horizontal_inset_px.to_string(),
    );
    set_status_on_view(content, TAG_STATUS, "未预览");
}

fn populate_surface_controls(content: &NSView, draft: &ThemeCustomization, surface: SurfacePart) {
    let values = draft.surfaces.get(&surface).cloned().unwrap_or_default();
    set_text_field(
        content,
        TAG_SURFACE_OPACITY,
        &values
            .opacity
            .map(|value| value.to_string())
            .unwrap_or_default(),
    );
    set_text_field(
        content,
        TAG_SURFACE_BLUR,
        &values
            .blur_px
            .map(|value| value.to_string())
            .unwrap_or_default(),
    );
    set_text_field(
        content,
        TAG_SURFACE_RADIUS,
        &values
            .radius_px
            .map(|value| value.to_string())
            .unwrap_or_default(),
    );
    let shadow_index = match values.shadow {
        None => 0,
        Some(ShadowPreset::None) => 1,
        Some(ShadowPreset::Soft) => 2,
        Some(ShadowPreset::Strong) => 3,
    };
    if let Some(popup) = popup(content, TAG_SURFACE_SHADOW) {
        popup.selectItemAtIndex(shadow_index);
    }
}

fn format_image_status(image: Option<&BackgroundImageCustomization>) -> String {
    image
        .map(|image| format!("已选择：{}", image.file_name))
        .unwrap_or_else(|| "跟随主题原图".into())
}

fn fill_mode_index(mode: BackgroundFillMode) -> isize {
    match mode {
        BackgroundFillMode::Cover => 0,
        BackgroundFillMode::Contain => 1,
        BackgroundFillMode::Stretch => 2,
    }
}

fn fill_mode_from_index(index: isize) -> Result<BackgroundFillMode, String> {
    match index {
        0 => Ok(BackgroundFillMode::Cover),
        1 => Ok(BackgroundFillMode::Contain),
        2 => Ok(BackgroundFillMode::Stretch),
        _ => Err("图片填充选项无效".into()),
    }
}

fn read_optional_color(content: &NSView, tag: isize, name: &str) -> Result<Option<String>, String> {
    let value = text_field(content, tag)
        .ok_or_else(|| format!("{name}控件不可用"))?
        .stringValue()
        .to_string();
    let value = value.trim().to_string();
    Ok((!value.is_empty()).then_some(value))
}

fn read_optional_bounded_u16(
    content: &NSView,
    tag: isize,
    name: &str,
    maximum: u16,
) -> Result<Option<u16>, String> {
    let value = text_field(content, tag)
        .ok_or_else(|| format!("{name}控件不可用"))?
        .stringValue()
        .to_string();
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    let parsed = value
        .parse::<u16>()
        .map_err(|_| format!("{name}必须是数字"))?;
    Ok(Some(parsed.min(maximum)))
}

fn read_bounded_u16(content: &NSView, tag: isize, name: &str, maximum: u16) -> Result<u16, String> {
    let value = text_field(content, tag)
        .ok_or_else(|| format!("{name}控件不可用"))?
        .stringValue()
        .to_string();
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{name}不能为空"));
    }
    let parsed = value
        .parse::<u16>()
        .map_err(|_| format!("{name}必须是数字"))?;
    Ok(parsed.min(maximum))
}

fn read_bounded_i16(
    content: &NSView,
    tag: isize,
    name: &str,
    minimum: i32,
    maximum: i32,
) -> Result<i16, String> {
    let value = text_field(content, tag)
        .ok_or_else(|| format!("{name}控件不可用"))?
        .stringValue()
        .to_string();
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{name}不能为空"));
    }
    let parsed = value
        .parse::<i32>()
        .map_err(|_| format!("{name}必须是数字"))?;
    Ok(parsed.clamp(minimum, maximum) as i16)
}

fn text_field(content: &NSView, tag: isize) -> Option<Retained<NSTextField>> {
    content
        .viewWithTag(tag)
        .and_then(|view| view.downcast::<NSTextField>().ok())
}

fn popup(content: &NSView, tag: isize) -> Option<Retained<NSPopUpButton>> {
    content
        .viewWithTag(tag)
        .and_then(|view| view.downcast::<NSPopUpButton>().ok())
}

fn set_text_field(content: &NSView, tag: isize, value: &str) {
    if let Some(field) = text_field(content, tag) {
        field.setStringValue(&NSString::from_str(value));
    }
}

fn set_status_on_view(content: &NSView, tag: isize, value: &str) {
    if let Some(field) = text_field(content, tag) {
        field.setStringValue(&NSString::from_str(value));
    }
}

fn add_labeled_field(
    content: &NSView,
    label: &str,
    tag: isize,
    value: &str,
    x: f64,
    y: f64,
    mtm: MainThreadMarker,
) -> Retained<NSTextField> {
    add_label(content, label, x, y + 4.0, 90.0, 22.0, mtm);
    let field = NSTextField::initWithFrame(
        NSTextField::alloc(mtm),
        NSRect::new(NSPoint::new(x + 88.0, y), NSSize::new(105.0, 28.0)),
    );
    field.setTag(tag);
    field.setStringValue(&NSString::from_str(value));
    content.addSubview(&field);
    field
}

fn add_label(
    content: &NSView,
    text: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    mtm: MainThreadMarker,
) -> Retained<NSTextField> {
    let label = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    label.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(width, height)));
    content.addSubview(&label);
    label
}

#[allow(clippy::too_many_arguments)]
fn add_popup(
    content: &NSView,
    tag: isize,
    titles: &[&str],
    x: f64,
    y: f64,
    width: f64,
    target: &AnyObject,
    action: objc2::runtime::Sel,
    mtm: MainThreadMarker,
) -> Retained<NSPopUpButton> {
    let popup = NSPopUpButton::initWithFrame_pullsDown(
        NSPopUpButton::alloc(mtm),
        NSRect::new(NSPoint::new(x, y), NSSize::new(width, 30.0)),
        false,
    );
    popup.setTag(tag);
    for title in titles {
        popup.addItemWithTitle(&NSString::from_str(title));
    }
    unsafe {
        popup.setTarget(Some(target));
        popup.setAction(Some(action));
    }
    content.addSubview(&popup);
    popup
}

#[allow(clippy::too_many_arguments)]
fn add_button(
    content: &NSView,
    title: &str,
    x: f64,
    y: f64,
    width: f64,
    target: &AnyObject,
    action: objc2::runtime::Sel,
    mtm: MainThreadMarker,
) -> Retained<NSButton> {
    let button = unsafe {
        NSButton::buttonWithTitle_target_action(
            &NSString::from_str(title),
            Some(target),
            Some(action),
            mtm,
        )
    };
    button.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(width, 30.0)));
    content.addSubview(&button);
    button
}
