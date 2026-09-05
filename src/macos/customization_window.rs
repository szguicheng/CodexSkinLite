use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::sel;
use objc2::{AnyThread, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSBox, NSBoxType, NSButton, NSColor, NSColorSpace, NSColorWell,
    NSColorWellStyle, NSControlStateValueOff, NSControlStateValueOn, NSFont, NSImage,
    NSImageScaling, NSImageView, NSPopUpButton, NSScrollView, NSTextField, NSTitlePosition, NSView,
    NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSData, NSPoint, NSRect, NSSize, NSString};

use crate::theme::{
    BackgroundFillMode, BackgroundImageCustomization, ShadowPreset, SurfacePart, ThemeCustomization,
};

use super::AppKitState;

const TAG_BACKGROUND_X: isize = 1001;
const TAG_BACKGROUND_Y: isize = 1002;
const TAG_BACKGROUND_OPACITY: isize = 1003;
const TAG_BACKGROUND_FILL_MODE: isize = 1004;
const TAG_BACKGROUND_IMAGE: isize = 1005;
const TAG_NATIVE_BOTTOM_GRADIENT: isize = 1006;
const TAG_NATIVE_TOP_GRADIENT: isize = 1007;
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
const TAG_PREVIEW_STATUS: isize = 1041;
const TAG_COLOR_WELL_BACKGROUND: isize = 1050;
const TAG_COLOR_WELL_PANEL: isize = 1051;
const TAG_COLOR_WELL_ACCENT: isize = 1052;
const TAG_COLOR_WELL_TEXT: isize = 1053;
const TAG_COLOR_WELL_LINE: isize = 1054;

const PREVIEW_SCREEN_WIDTH: f64 = 1920.0;
const PREVIEW_SCREEN_HEIGHT: f64 = 971.0;
const PREVIEW_X: f64 = 520.0;
const PREVIEW_Y: f64 = 285.0;
const PREVIEW_WIDTH: f64 = 432.0;
const PREVIEW_HEIGHT: f64 = PREVIEW_WIDTH * PREVIEW_SCREEN_HEIGHT / PREVIEW_SCREEN_WIDTH;
const PREVIEW_IMAGE_X: f64 = PREVIEW_X;
const PREVIEW_IMAGE_Y: f64 = PREVIEW_Y;
const PREVIEW_IMAGE_WIDTH: f64 = PREVIEW_WIDTH;
const PREVIEW_IMAGE_HEIGHT: f64 = PREVIEW_HEIGHT;

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

    let window_rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1040.0, 780.0));
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
    window.setMinSize(NSSize::new(920.0, 600.0));
    window.center();

    let scroll = NSScrollView::initWithFrame(
        NSScrollView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1040.0, 780.0)),
    );
    scroll.setHasVerticalScroller(true);
    scroll.setDrawsBackground(false);
    let document = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1000.0, 1120.0)),
    );
    scroll.setDocumentView(Some(&document));
    window.setContentView(Some(&scroll));

    add_card(&document, 24.0, 1028.0, 952.0, 72.0, mtm);
    add_card(&document, 24.0, 815.0, 952.0, 195.0, mtm);
    add_card(&document, 24.0, 585.0, 952.0, 210.0, mtm);
    add_card(&document, 24.0, 245.0, 952.0, 320.0, mtm);
    add_card(&document, 24.0, 100.0, 952.0, 125.0, mtm);

    let heading = add_label(&document, "主题自定义", 48.0, 1058.0, 560.0, 28.0, mtm);
    heading.setFont(Some(&NSFont::boldSystemFontOfSize(18.0)));
    add_label(
        &document,
        "先点预览看看效果，保存后才会写入这个主题。",
        48.0,
        1035.0,
        580.0,
        22.0,
        mtm,
    );
    add_label(&document, "ฅ^•ﻌ•^ฅ", 875.0, 1045.0, 90.0, 36.0, mtm)
        .setFont(Some(&NSFont::systemFontOfSize(24.0)));

    let background_heading = add_label(&document, "背景图片", 48.0, 955.0, 300.0, 24.0, mtm);
    background_heading.setFont(Some(&NSFont::boldSystemFontOfSize(13.0)));
    add_button(
        &document,
        "选择图片…",
        48.0,
        911.0,
        136.0,
        target,
        sel!(selectCustomizationImage:),
        mtm,
    );
    add_label(
        &document,
        &format_image_status(draft.background.image.as_ref()),
        200.0,
        915.0,
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
        48.0,
        858.0,
        mtm,
    );
    add_labeled_field(
        &document,
        "偏移 Y px",
        TAG_BACKGROUND_Y,
        &draft.background.offset_y_px.to_string(),
        300.0,
        858.0,
        mtm,
    );
    add_label(&document, "填充", 552.0, 862.0, 56.0, 22.0, mtm);
    let fill_popup = add_popup(
        &document,
        TAG_BACKGROUND_FILL_MODE,
        &["覆盖填充", "完整显示", "拉伸填充"],
        616.0,
        858.0,
        170.0,
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
        48.0,
        816.0,
        mtm,
    );

    for (title, tag, checked, x) in [
        (
            "使用 Codex 默认顶部渐变",
            TAG_NATIVE_TOP_GRADIENT,
            draft.background.use_native_top_gradient,
            300.0,
        ),
        (
            "使用 Codex 默认底部渐变",
            TAG_NATIVE_BOTTOM_GRADIENT,
            draft.background.use_native_bottom_gradient,
            620.0,
        ),
    ] {
        let native_gradient = unsafe {
            NSButton::checkboxWithTitle_target_action(&NSString::from_str(title), None, None, mtm)
        };
        native_gradient.setTag(tag);
        native_gradient.setFrame(NSRect::new(
            NSPoint::new(x, 816.0),
            NSSize::new(300.0, 24.0),
        ));
        native_gradient.setToolTip(Some(&NSString::from_str(
            "取消勾选则显示主题背景；点击预览查看效果，保存后保留此选择。",
        )));
        native_gradient.setState(if checked {
            NSControlStateValueOn
        } else {
            NSControlStateValueOff
        });
        document.addSubview(&native_gradient);
    }

    add_label(&document, "颜色覆盖", 48.0, 755.0, 360.0, 24.0, mtm);
    add_color_field(
        &document,
        "背景",
        TAG_COLOR_BACKGROUND,
        TAG_COLOR_WELL_BACKGROUND,
        draft.colors.background.as_deref().unwrap_or_default(),
        48.0,
        685.0,
        target,
        mtm,
    );
    add_color_field(
        &document,
        "面板",
        TAG_COLOR_PANEL,
        TAG_COLOR_WELL_PANEL,
        draft.colors.panel.as_deref().unwrap_or_default(),
        354.0,
        685.0,
        target,
        mtm,
    );
    add_color_field(
        &document,
        "强调",
        TAG_COLOR_ACCENT,
        TAG_COLOR_WELL_ACCENT,
        draft.colors.accent.as_deref().unwrap_or_default(),
        660.0,
        685.0,
        target,
        mtm,
    );
    add_color_field(
        &document,
        "文字",
        TAG_COLOR_TEXT,
        TAG_COLOR_WELL_TEXT,
        draft.colors.text.as_deref().unwrap_or_default(),
        48.0,
        625.0,
        target,
        mtm,
    );
    add_color_field(
        &document,
        "分割线",
        TAG_COLOR_LINE,
        TAG_COLOR_WELL_LINE,
        draft.colors.line.as_deref().unwrap_or_default(),
        354.0,
        625.0,
        target,
        mtm,
    );

    let component_heading = add_label(&document, "组件样式", 48.0, 505.0, 300.0, 24.0, mtm);
    component_heading.setFont(Some(&NSFont::boldSystemFontOfSize(13.0)));
    let component_popup = add_popup(
        &document,
        TAG_SURFACE_PART,
        &SurfacePart::ALL
            .into_iter()
            .map(SurfacePart::display_name)
            .collect::<Vec<_>>(),
        48.0,
        452.0,
        280.0,
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
        48.0,
        405.0,
        mtm,
    );
    add_labeled_field(
        &document,
        "模糊 px",
        TAG_SURFACE_BLUR,
        "",
        292.0,
        405.0,
        mtm,
    );
    add_labeled_field(
        &document,
        "圆角 px",
        TAG_SURFACE_RADIUS,
        "",
        48.0,
        361.0,
        mtm,
    );
    add_label(&document, "阴影", 292.0, 365.0, 56.0, 22.0, mtm);
    let shadow_popup = add_popup(
        &document,
        TAG_SURFACE_SHADOW,
        &["跟随主题", "无阴影", "柔和", "明显"],
        348.0,
        361.0,
        154.0,
        target,
        sel!(selectCustomizationComponent:),
        mtm,
    );
    unsafe {
        shadow_popup.setTarget(None);
        shadow_popup.setAction(None);
    }
    shadow_popup.selectItemAtIndex(0);
    add_component_preview(&document, SurfacePart::Main, mtm);

    let composer_heading = add_label(&document, "输入框位置", 48.0, 190.0, 300.0, 24.0, mtm);
    composer_heading.setFont(Some(&NSFont::boldSystemFontOfSize(13.0)));
    add_labeled_field(
        &document,
        "距底部 px",
        TAG_COMPOSER_BOTTOM,
        &draft.composer.bottom_inset_px.to_string(),
        48.0,
        140.0,
        mtm,
    );
    add_labeled_field(
        &document,
        "左右内缩 px",
        TAG_COMPOSER_HORIZONTAL,
        &draft.composer.horizontal_inset_px.to_string(),
        292.0,
        140.0,
        mtm,
    );

    add_label(
        &document,
        if theme_id.is_some() {
            "未预览"
        } else {
            "当前未选择主题：此处显示空白草稿"
        },
        48.0,
        48.0,
        560.0,
        24.0,
        mtm,
    )
    .setTag(TAG_STATUS);
    add_button(
        &document,
        "预览",
        536.0,
        42.0,
        96.0,
        target,
        sel!(previewCustomization:),
        mtm,
    );
    let save = add_button(
        &document,
        "保存",
        644.0,
        42.0,
        96.0,
        target,
        sel!(saveCustomization:),
        mtm,
    );
    save.setKeyEquivalent(&NSString::from_str("s"));
    add_button(
        &document,
        "恢复默认",
        752.0,
        42.0,
        120.0,
        target,
        sel!(resetCustomization:),
        mtm,
    );
    add_button(
        &document,
        "关闭",
        884.0,
        42.0,
        80.0,
        target,
        sel!(closeCustomization:),
        mtm,
    );

    populate_controls(&document, &draft, SurfacePart::Main);
    let _ = document.scrollRectToVisible(NSRect::new(
        NSPoint::new(0.0, 760.0),
        NSSize::new(1000.0, 360.0),
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
    draft.background.use_native_top_gradient = content
        .viewWithTag(TAG_NATIVE_TOP_GRADIENT)
        .and_then(|view| view.downcast::<NSButton>().ok())
        .ok_or_else(|| "顶部渐变控件不可用".to_string())?
        .state()
        == NSControlStateValueOn;
    draft.background.use_native_bottom_gradient = content
        .viewWithTag(TAG_NATIVE_BOTTOM_GRADIENT)
        .and_then(|view| view.downcast::<NSButton>().ok())
        .ok_or_else(|| "底部渐变控件不可用".to_string())?
        .state()
        == NSControlStateValueOn;
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

pub(super) fn sync_hex_from_color_well(window: &NSWindow, well: &NSColorWell) {
    let Some(field_tag) = color_field_tag(well.tag()) else {
        return;
    };
    let Some(content) = window.contentView() else {
        return;
    };
    set_text_field(&content, field_tag, &hex_from_color(&well.color()));
}

pub(super) fn sync_color_well_from_field(window: &NSWindow, field: &NSTextField) {
    let Some(well_tag) = color_well_tag(field.tag()) else {
        return;
    };
    let Some(content) = window.contentView() else {
        return;
    };
    let Some(well) = color_well(&content, well_tag) else {
        return;
    };
    let value = field.stringValue().to_string();
    if value.trim().is_empty() {
        well.setColor(&NSColor::clearColor());
    } else if let Some(color) = color_from_hex(&value) {
        well.setColor(&color);
    }
}

fn add_card(
    content: &NSView,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    mtm: MainThreadMarker,
) -> Retained<NSBox> {
    let card = NSBox::initWithFrame(
        NSBox::alloc(mtm),
        NSRect::new(NSPoint::new(x, y), NSSize::new(width, height)),
    );
    card.setBoxType(NSBoxType::Custom);
    card.setTitlePosition(NSTitlePosition::NoTitle);
    card.setFillColor(&NSColor::controlBackgroundColor());
    card.setBorderColor(&NSColor::separatorColor());
    card.setBorderWidth(1.0);
    card.setCornerRadius(12.0);
    content.addSubview(&card);
    card
}

fn add_preview_box(
    content: &NSView,
    _tag: isize,
    frame: (f64, f64, f64, f64),
    fill: &NSColor,
    border: &NSColor,
    border_width: f64,
    mtm: MainThreadMarker,
) -> Retained<NSBox> {
    let (x, y, width, height) = frame;
    let box_view = NSBox::initWithFrame(
        NSBox::alloc(mtm),
        NSRect::new(NSPoint::new(x, y), NSSize::new(width, height)),
    );
    box_view.setBoxType(NSBoxType::Custom);
    box_view.setTitlePosition(NSTitlePosition::NoTitle);
    box_view.setFillColor(fill);
    box_view.setBorderColor(border);
    box_view.setBorderWidth(border_width);
    box_view.setCornerRadius(if border_width > 0.0 { 7.0 } else { 0.0 });
    content.addSubview(&box_view);
    box_view
}

fn add_component_preview(content: &NSView, selected: SurfacePart, mtm: MainThreadMarker) {
    add_card(
        content,
        PREVIEW_X,
        PREVIEW_Y,
        PREVIEW_WIDTH,
        PREVIEW_HEIGHT,
        mtm,
    );
    if let Some(image) = codex_preview_image() {
        let image_view = NSImageView::imageViewWithImage(&image, mtm);
        image_view.setFrame(NSRect::new(
            NSPoint::new(PREVIEW_IMAGE_X, PREVIEW_IMAGE_Y),
            NSSize::new(PREVIEW_IMAGE_WIDTH, PREVIEW_IMAGE_HEIGHT),
        ));
        image_view.setImageScaling(NSImageScaling::ScaleProportionallyUpOrDown);
        image_view.setToolTip(Some(&NSString::from_str(
            "当前 Codex 界面截图，正文已模糊处理",
        )));
        content.addSubview(&image_view);
    }
    let clear = NSColor::clearColor();
    add_preview_box(
        content,
        0,
        (PREVIEW_X, PREVIEW_Y, PREVIEW_WIDTH, PREVIEW_HEIGHT),
        &clear,
        &NSColor::separatorColor(),
        1.0,
        mtm,
    );
    for part in SurfacePart::ALL {
        let (x, y, width, height) = scaled_preview_frame(part);
        let outline = add_preview_box(
            content,
            0,
            (x, y, width, height),
            &clear,
            &NSColor::systemRedColor(),
            2.0,
            mtm,
        );
        outline.setToolTip(Some(&NSString::from_str(preview_marker_identifier(part))));
        outline.setHidden(part != selected);
    }
    let status = add_label(
        content,
        &format!("当前正在修改：{}（截图中的红框）", selected.display_name()),
        PREVIEW_X,
        PREVIEW_Y - 29.0,
        PREVIEW_WIDTH,
        20.0,
        mtm,
    );
    status.setTag(TAG_PREVIEW_STATUS);
}

fn codex_preview_image() -> Option<Retained<NSImage>> {
    let bytes = include_bytes!("../../resources/CodexSkinLite-codex-preview.png");
    let data = unsafe { NSData::dataWithBytes_length(bytes.as_ptr().cast(), bytes.len()) };
    NSImage::initWithData(NSImage::alloc(), &data)
}

fn preview_frame(part: SurfacePart) -> (f64, f64, f64, f64) {
    match part {
        SurfacePart::Main => (257.5, 0.0, 1662.5, 971.0),
        SurfacePart::Sidebar => (0.0, 0.0, 257.5, 971.0),
        SurfacePart::Thread => (258.0, 0.0, 1662.0, 924.5),
        SurfacePart::Message => (600.25, 372.5, 1008.0, 70.25),
        SurfacePart::Composer => (585.25, 16.0, 1008.0, 98.0),
        SurfacePart::Header => (0.0, 925.0, 1920.0, 46.0),
    }
}

fn scaled_preview_frame(part: SurfacePart) -> (f64, f64, f64, f64) {
    let (x, y, width, height) = preview_frame(part);
    let scale_x = PREVIEW_IMAGE_WIDTH / PREVIEW_SCREEN_WIDTH;
    let scale_y = PREVIEW_IMAGE_HEIGHT / PREVIEW_SCREEN_HEIGHT;
    (
        PREVIEW_IMAGE_X + x * scale_x,
        PREVIEW_IMAGE_Y + y * scale_y,
        width * scale_x,
        height * scale_y,
    )
}

fn preview_marker_matches(title: &str, selected: SurfacePart) -> bool {
    title == selected.css_name()
}

fn preview_marker_identifier(part: SurfacePart) -> &'static str {
    part.css_name()
}

fn update_preview_selection(content: &NSView, selected: SurfacePart) {
    update_preview_selection_in_view(content, selected);
    if let Some(status) = text_field(content, TAG_PREVIEW_STATUS) {
        status.setStringValue(&NSString::from_str(&format!(
            "当前正在修改：{}（截图中的红框）",
            selected.display_name()
        )));
    }
}

fn update_preview_selection_in_view(content: &NSView, selected: SurfacePart) {
    for view in content.subviews().iter() {
        if let Some(outline) = view.downcast_ref::<NSBox>() {
            let title = outline
                .toolTip()
                .map(|value| value.to_string())
                .unwrap_or_default();
            if SurfacePart::ALL
                .into_iter()
                .any(|part| part.css_name() == title)
            {
                outline.setHidden(!preview_marker_matches(&title, selected));
            }
        } else {
            update_preview_selection_in_view(&view, selected);
        }
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
    set_color_controls(
        content,
        TAG_COLOR_BACKGROUND,
        TAG_COLOR_WELL_BACKGROUND,
        draft.colors.background.as_deref().unwrap_or_default(),
    );
    set_color_controls(
        content,
        TAG_COLOR_PANEL,
        TAG_COLOR_WELL_PANEL,
        draft.colors.panel.as_deref().unwrap_or_default(),
    );
    set_color_controls(
        content,
        TAG_COLOR_ACCENT,
        TAG_COLOR_WELL_ACCENT,
        draft.colors.accent.as_deref().unwrap_or_default(),
    );
    set_color_controls(
        content,
        TAG_COLOR_TEXT,
        TAG_COLOR_WELL_TEXT,
        draft.colors.text.as_deref().unwrap_or_default(),
    );
    set_color_controls(
        content,
        TAG_COLOR_LINE,
        TAG_COLOR_WELL_LINE,
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
    update_preview_selection(content, surface);
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
    update_preview_selection(content, surface);
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

fn hex_rgba(value: &str) -> Option<(u8, u8, u8, u8)> {
    let digits = value.trim().strip_prefix('#')?;
    let expand = |digit: u8| (digit << 4) | digit;
    let byte = |pair: &str| u8::from_str_radix(pair, 16).ok();
    match digits.len() {
        3 => Some((
            expand(hex_nibble(digits.as_bytes()[0])?),
            expand(hex_nibble(digits.as_bytes()[1])?),
            expand(hex_nibble(digits.as_bytes()[2])?),
            0xff,
        )),
        4 => Some((
            expand(hex_nibble(digits.as_bytes()[0])?),
            expand(hex_nibble(digits.as_bytes()[1])?),
            expand(hex_nibble(digits.as_bytes()[2])?),
            expand(hex_nibble(digits.as_bytes()[3])?),
        )),
        6 => Some((
            byte(&digits[0..2])?,
            byte(&digits[2..4])?,
            byte(&digits[4..6])?,
            0xff,
        )),
        8 => Some((
            byte(&digits[0..2])?,
            byte(&digits[2..4])?,
            byte(&digits[4..6])?,
            byte(&digits[6..8])?,
        )),
        _ => None,
    }
}

fn hex_nibble(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn color_from_hex(value: &str) -> Option<Retained<NSColor>> {
    let (red, green, blue, alpha) = hex_rgba(value)?;
    Some(NSColor::colorWithSRGBRed_green_blue_alpha(
        f64::from(red) / 255.0,
        f64::from(green) / 255.0,
        f64::from(blue) / 255.0,
        f64::from(alpha) / 255.0,
    ))
}

fn hex_from_color(color: &NSColor) -> String {
    let space = NSColorSpace::sRGBColorSpace();
    let Some(color) = color.colorUsingColorSpace(&space) else {
        return "#000000".into();
    };
    let component = |value: f64| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    let red = component(color.redComponent());
    let green = component(color.greenComponent());
    let blue = component(color.blueComponent());
    let alpha = component(color.alphaComponent());
    if alpha == 0xff {
        format!("#{red:02x}{green:02x}{blue:02x}")
    } else {
        format!("#{red:02x}{green:02x}{blue:02x}{alpha:02x}")
    }
}

fn color_well(content: &NSView, tag: isize) -> Option<Retained<NSColorWell>> {
    content
        .viewWithTag(tag)
        .and_then(|view| view.downcast::<NSColorWell>().ok())
}

fn color_well_tag(field_tag: isize) -> Option<isize> {
    match field_tag {
        TAG_COLOR_BACKGROUND => Some(TAG_COLOR_WELL_BACKGROUND),
        TAG_COLOR_PANEL => Some(TAG_COLOR_WELL_PANEL),
        TAG_COLOR_ACCENT => Some(TAG_COLOR_WELL_ACCENT),
        TAG_COLOR_TEXT => Some(TAG_COLOR_WELL_TEXT),
        TAG_COLOR_LINE => Some(TAG_COLOR_WELL_LINE),
        _ => None,
    }
}

fn color_field_tag(well_tag: isize) -> Option<isize> {
    match well_tag {
        TAG_COLOR_WELL_BACKGROUND => Some(TAG_COLOR_BACKGROUND),
        TAG_COLOR_WELL_PANEL => Some(TAG_COLOR_PANEL),
        TAG_COLOR_WELL_ACCENT => Some(TAG_COLOR_ACCENT),
        TAG_COLOR_WELL_TEXT => Some(TAG_COLOR_TEXT),
        TAG_COLOR_WELL_LINE => Some(TAG_COLOR_LINE),
        _ => None,
    }
}

fn set_color_controls(content: &NSView, field_tag: isize, well_tag: isize, value: &str) {
    set_text_field(content, field_tag, value);
    if let Some(well) = color_well(content, well_tag) {
        if let Some(color) = color_from_hex(value) {
            well.setColor(&color);
        } else {
            well.setColor(&NSColor::clearColor());
        }
    }
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

#[allow(clippy::too_many_arguments)]
fn add_color_field(
    content: &NSView,
    label: &str,
    field_tag: isize,
    well_tag: isize,
    value: &str,
    x: f64,
    y: f64,
    target: &AnyObject,
    mtm: MainThreadMarker,
) -> Retained<NSTextField> {
    add_label(content, label, x, y + 4.0, 52.0, 22.0, mtm);
    let well = NSColorWell::colorWellWithStyle(NSColorWellStyle::Minimal, mtm);
    well.setFrame(NSRect::new(
        NSPoint::new(x + 54.0, y),
        NSSize::new(30.0, 28.0),
    ));
    well.setTag(well_tag);
    well.setSupportsAlpha(true);
    well.setToolTip(Some(&NSString::from_str("点击打开 macOS 色板")));
    if let Some(color) = color_from_hex(value) {
        well.setColor(&color);
    } else {
        well.setColor(&NSColor::clearColor());
    }
    unsafe {
        well.setTarget(Some(target));
        well.setAction(Some(sel!(selectCustomizationColor:)));
    }
    content.addSubview(&well);

    let field = NSTextField::initWithFrame(
        NSTextField::alloc(mtm),
        NSRect::new(NSPoint::new(x + 90.0, y), NSSize::new(112.0, 28.0)),
    );
    field.setTag(field_tag);
    field.setStringValue(&NSString::from_str(value));
    field.setToolTip(Some(&NSString::from_str(
        "支持 #RGB、#RGBA、#RRGGBB、#RRGGBBAA",
    )));
    unsafe {
        field.setTarget(Some(target));
        field.setAction(Some(sel!(editCustomizationColor:)));
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_hex_components_accept_short_and_alpha_forms() {
        assert_eq!(hex_rgba("#abc"), Some((0xaa, 0xbb, 0xcc, 0xff)));
        assert_eq!(hex_rgba("#11223344"), Some((0x11, 0x22, 0x33, 0x44)));
        assert_eq!(hex_rgba("blue"), None);
    }

    #[test]
    fn preview_frames_identify_the_selected_codex_surface() {
        assert_eq!(
            preview_frame(SurfacePart::Sidebar),
            (0.0, 0.0, 257.5, 971.0)
        );
        assert_eq!(
            preview_frame(SurfacePart::Main),
            (257.5, 0.0, 1662.5, 971.0)
        );
        assert_eq!(
            preview_frame(SurfacePart::Thread),
            (258.0, 0.0, 1662.0, 924.5)
        );
        assert_eq!(
            preview_frame(SurfacePart::Message),
            (600.25, 372.5, 1008.0, 70.25)
        );
        assert_eq!(
            preview_frame(SurfacePart::Composer),
            (585.25, 16.0, 1008.0, 98.0)
        );
        assert_eq!(
            preview_frame(SurfacePart::Header),
            (0.0, 925.0, 1920.0, 46.0)
        );
    }

    #[test]
    fn preview_marker_names_identify_the_selected_surface() {
        assert!(preview_marker_matches("composer", SurfacePart::Composer));
        assert!(!preview_marker_matches("main", SurfacePart::Composer));
    }

    #[test]
    fn preview_marker_identifiers_are_stable() {
        assert_eq!(preview_marker_identifier(SurfacePart::Composer), "composer");
        assert_eq!(preview_marker_identifier(SurfacePart::Header), "header");
    }
}
