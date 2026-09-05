use std::cell::RefCell;
use std::sync::Arc;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadOnly, sel};
use objc2_app_kit::{
    NSBackingStoreType, NSBox, NSBoxType, NSButton, NSColor, NSControlStateValueOff,
    NSControlStateValueOn, NSFont, NSPopUpButton, NSTextField, NSTitlePosition, NSView, NSWindow,
    NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use dispatch2::MainThreadBound;

use crate::model::{AppSnapshot, ConnectionState};

use super::AppKitState;

pub(super) fn show(
    mtm: MainThreadMarker,
    target: &AnyObject,
    state: &Arc<AppKitState>,
    slot: &RefCell<Option<Retained<NSWindow>>>,
) {
    if let Some(window) = slot.borrow_mut().take() {
        window.close();
    }
    let snapshot = state.snapshot();
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(640.0, 620.0));
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            rect,
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    window.setTitle(&NSString::from_str("CodexSkinLite 设置"));
    unsafe { window.setReleasedWhenClosed(false) };
    window.setMinSize(NSSize::new(600.0, 560.0));
    window.center();
    let content = NSView::initWithFrame(NSView::alloc(mtm), rect);
    window.setContentView(Some(&content));

    add_card(&content, 16.0, 476.0, 608.0, 124.0, mtm);
    add_card(&content, 16.0, 322.0, 608.0, 140.0, mtm);
    add_card(&content, 16.0, 214.0, 608.0, 92.0, mtm);
    add_card(&content, 16.0, 16.0, 608.0, 182.0, mtm);

    let title = add_label(&content, "CodexSkinLite", 36.0, 548.0, 360.0, 30.0, mtm);
    title.setFont(Some(&NSFont::boldSystemFontOfSize(20.0)));
    add_label(
        &content,
        "给你的 Codex 换一件轻盈、可爱的外套",
        36.0,
        518.0,
        390.0,
        22.0,
        mtm,
    );
    let cat = add_label(&content, "ฅ^•ﻌ•^ฅ", 510.0, 520.0, 100.0, 36.0, mtm);
    cat.setFont(Some(&NSFont::systemFontOfSize(24.0)));
    add_label(
        &content,
        "主题 · 宽度 · 连接",
        36.0,
        492.0,
        300.0,
        20.0,
        mtm,
    );

    let appearance = add_label(&content, "外观", 36.0, 430.0, 500.0, 24.0, mtm);
    appearance.setFont(Some(&NSFont::boldSystemFontOfSize(13.0)));
    let theme_enabled = unsafe {
        NSButton::checkboxWithTitle_target_action(
            &NSString::from_str("启用主题"),
            Some(target),
            Some(sel!(toggleTheme:)),
            mtm,
        )
    };
    theme_enabled.setFrameOrigin(NSPoint::new(36.0, 388.0));
    if snapshot
        .as_ref()
        .is_some_and(|value| value.settings.theme_enabled)
    {
        theme_enabled.setState(NSControlStateValueOn);
    }
    content.addSubview(&theme_enabled);

    let popup = NSPopUpButton::initWithFrame_pullsDown(
        NSPopUpButton::alloc(mtm),
        NSRect::new(NSPoint::new(160.0, 381.0), NSSize::new(180.0, 30.0)),
        false,
    );
    popup.addItemWithTitle(&NSString::from_str("无"));
    for theme in snapshot
        .as_ref()
        .map(|value| value.themes.as_slice())
        .unwrap_or(&[])
    {
        popup.addItemWithTitle(&NSString::from_str(&theme.id));
    }
    if let Some(id) = snapshot
        .as_ref()
        .and_then(|value| value.settings.active_theme_id.as_deref())
    {
        popup.selectItemWithTitle(&NSString::from_str(id));
    } else {
        popup.selectItemWithTitle(&NSString::from_str("无"));
    }
    unsafe {
        popup.setTarget(Some(target));
        popup.setAction(Some(sel!(selectTheme:)));
    }
    content.addSubview(&popup);
    add_button(
        &content,
        "导入 ZIP…",
        360.0,
        381.0,
        100.0,
        target,
        sel!(importTheme:),
        mtm,
    );
    let gallery = add_button(
        &content,
        "远程主题画廊",
        468.0,
        381.0,
        135.0,
        target,
        sel!(openThemeGallery:),
        mtm,
    );
    gallery.setToolTip(Some(&NSString::from_str("https://dreamskin.cc/gallery")));
    let customize_theme = add_button(
        &content,
        "自定义主题…",
        360.0,
        338.0,
        150.0,
        target,
        sel!(customizeTheme:),
        mtm,
    );
    customize_theme.setEnabled(true);
    if snapshot.as_ref().is_some_and(|value| {
        matches!(
            value.connection,
            crate::model::ConnectionState::RestartRequired
        )
    }) {
        add_button(
            &content,
            "确认重启",
            480.0,
            62.0,
            115.0,
            target,
            sel!(confirmRestart:),
            mtm,
        );
    }

    let layout = add_label(&content, "布局", 36.0, 272.0, 500.0, 24.0, mtm);
    layout.setFont(Some(&NSFont::boldSystemFontOfSize(13.0)));
    let centered = unsafe {
        NSButton::checkboxWithTitle_target_action(
            &NSString::from_str("对话居中宽度"),
            Some(target),
            Some(sel!(toggleCentered:)),
            mtm,
        )
    };
    centered.setFrameOrigin(NSPoint::new(36.0, 237.0));
    if snapshot
        .as_ref()
        .is_some_and(|value| value.settings.conversation_centered)
    {
        centered.setState(NSControlStateValueOn);
    }
    content.addSubview(&centered);
    let width = NSTextField::initWithFrame(
        NSTextField::alloc(mtm),
        NSRect::new(NSPoint::new(210.0, 232.0), NSSize::new(90.0, 28.0)),
    );
    width.setStringValue(&NSString::from_str(
        &snapshot
            .as_ref()
            .map(|value| value.settings.conversation_max_width)
            .unwrap_or(900)
            .to_string(),
    ));
    unsafe {
        width.setTarget(Some(target));
        width.setAction(Some(sel!(setWidth:)));
    }
    content.addSubview(&width);
    add_label(
        &content,
        "px（输入后按回车）",
        310.0,
        237.0,
        180.0,
        22.0,
        mtm,
    );

    let codex = add_label(&content, "Codex", 36.0, 168.0, 500.0, 24.0, mtm);
    codex.setFont(Some(&NSFont::boldSystemFontOfSize(13.0)));
    let status = snapshot
        .as_ref()
        .map(|value| connection_status(&value.connection))
        .unwrap_or_else(|| "状态：未连接".into());
    let status_label = add_label(&content, &status, 36.0, 140.0, 500.0, 22.0, mtm);
    status_label.setToolTip(Some(&NSString::from_str(&status)));
    let app_path = snapshot
        .as_ref()
        .map(|value| value.settings.codex_app_path.display().to_string())
        .unwrap_or_else(|| "/Applications/Codex.app".into());
    add_label(&content, &app_path, 36.0, 110.0, 560.0, 22.0, mtm);
    add_button(
        &content,
        "选择 Codex.app…",
        36.0,
        62.0,
        145.0,
        target,
        sel!(selectCodex:),
        mtm,
    );
    add_button(
        &content,
        "打开 Codex",
        200.0,
        62.0,
        115.0,
        target,
        sel!(openCodex:),
        mtm,
    );
    add_button(
        &content,
        "重新连接",
        330.0,
        62.0,
        100.0,
        target,
        sel!(reconnect:),
        mtm,
    );
    let disconnect = add_button(
        &content,
        "断开连接",
        450.0,
        62.0,
        100.0,
        target,
        sel!(disconnect:),
        mtm,
    );
    disconnect.setToolTip(Some(&NSString::from_str(
        "撤销主题和布局并停止注入；保留 Codex 与正在进行的任务。重新连接后恢复保存的设置。",
    )));
    if let Some(error) = state.latest_error() {
        add_label(&content, &error, 36.0, 28.0, 560.0, 28.0, mtm);
    }

    let ui = Arc::new(MainThreadBound::new(
        SettingsUi {
            status_label,
            theme_enabled,
            theme_popup: popup,
            customize_theme,
            centered,
            width,
        },
        mtm,
    ));
    state.set_refresher(Arc::new(move |snapshot| {
        let ui = ui.clone();
        ui.get_on_main(move |handles| handles.refresh(&snapshot));
    }));

    window.makeKeyAndOrderFront(None);
    window.orderFrontRegardless();
    *slot.borrow_mut() = Some(window);
}

struct SettingsUi {
    status_label: Retained<NSTextField>,
    theme_enabled: Retained<NSButton>,
    theme_popup: Retained<NSPopUpButton>,
    customize_theme: Retained<NSButton>,
    centered: Retained<NSButton>,
    width: Retained<NSTextField>,
}

impl SettingsUi {
    fn refresh(&self, snapshot: &AppSnapshot) {
        let status = NSString::from_str(&connection_status(&snapshot.connection));
        self.status_label.setStringValue(&status);
        self.status_label.setToolTip(Some(&status));
        self.theme_enabled
            .setState(if snapshot.settings.theme_enabled {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
        self.centered
            .setState(if snapshot.settings.conversation_centered {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
        self.width.setStringValue(&NSString::from_str(
            &snapshot.settings.conversation_max_width.to_string(),
        ));
        self.theme_popup.removeAllItems();
        self.customize_theme.setEnabled(true);
        self.theme_popup.addItemWithTitle(&NSString::from_str("无"));
        for theme in &snapshot.themes {
            self.theme_popup
                .addItemWithTitle(&NSString::from_str(&theme.id));
        }
        if let Some(id) = &snapshot.settings.active_theme_id {
            self.theme_popup
                .selectItemWithTitle(&NSString::from_str(id));
        } else {
            self.theme_popup
                .selectItemWithTitle(&NSString::from_str("无"));
        }
    }
}

fn connection_status(connection: &ConnectionState) -> String {
    match connection {
        ConnectionState::Suspended => "已断开：Codex 原生界面（任务继续运行）".into(),
        ConnectionState::Disconnected => "状态：未连接".into(),
        ConnectionState::Connecting => "状态：正在连接…".into(),
        ConnectionState::Connected => "状态：已连接".into(),
        ConnectionState::RestartRequired => "状态：需要确认重启后连接".into(),
        ConnectionState::CompatibilityWarning(error) => format!("状态：{error}"),
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
