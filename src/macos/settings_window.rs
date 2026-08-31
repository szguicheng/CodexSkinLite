use std::cell::RefCell;
use std::sync::Arc;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadOnly, sel};
use objc2_app_kit::{
    NSBackingStoreType, NSButton, NSControlStateValueOff, NSControlStateValueOn, NSPopUpButton,
    NSTextField, NSView, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use dispatch2::MainThreadBound;

use crate::model::AppSnapshot;

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
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(520.0, 430.0));
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
    window.center();
    let content = NSView::initWithFrame(NSView::alloc(mtm), rect);
    window.setContentView(Some(&content));

    add_label(&content, "外观", 24.0, 380.0, 470.0, 24.0, mtm);
    let theme_enabled = unsafe {
        NSButton::checkboxWithTitle_target_action(
            &NSString::from_str("启用主题"),
            Some(target),
            Some(sel!(toggleTheme:)),
            mtm,
        )
    };
    theme_enabled.setFrameOrigin(NSPoint::new(24.0, 345.0));
    if snapshot
        .as_ref()
        .is_some_and(|value| value.settings.theme_enabled)
    {
        theme_enabled.setState(NSControlStateValueOn);
    }
    content.addSubview(&theme_enabled);

    let popup = NSPopUpButton::initWithFrame_pullsDown(
        NSPopUpButton::alloc(mtm),
        NSRect::new(NSPoint::new(150.0, 338.0), NSSize::new(220.0, 30.0)),
        false,
    );
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
    }
    unsafe {
        popup.setTarget(Some(target));
        popup.setAction(Some(sel!(selectTheme:)));
    }
    content.addSubview(&popup);
    add_button(
        &content,
        "导入 ZIP…",
        385.0,
        338.0,
        105.0,
        target,
        sel!(importTheme:),
        mtm,
    );
    if snapshot.as_ref().is_some_and(|value| {
        matches!(
            value.connection,
            crate::model::ConnectionState::RestartRequired
        )
    }) {
        add_button(
            &content,
            "确认重启",
            425.0,
            85.0,
            85.0,
            target,
            sel!(confirmRestart:),
            mtm,
        );
    }

    add_label(&content, "布局", 24.0, 285.0, 470.0, 24.0, mtm);
    let centered = unsafe {
        NSButton::checkboxWithTitle_target_action(
            &NSString::from_str("对话居中宽度"),
            Some(target),
            Some(sel!(toggleCentered:)),
            mtm,
        )
    };
    centered.setFrameOrigin(NSPoint::new(24.0, 250.0));
    if snapshot
        .as_ref()
        .is_some_and(|value| value.settings.conversation_centered)
    {
        centered.setState(NSControlStateValueOn);
    }
    content.addSubview(&centered);
    let width = NSTextField::initWithFrame(
        NSTextField::alloc(mtm),
        NSRect::new(NSPoint::new(200.0, 245.0), NSSize::new(90.0, 28.0)),
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
        300.0,
        250.0,
        180.0,
        22.0,
        mtm,
    );

    add_label(&content, "Codex", 24.0, 190.0, 470.0, 24.0, mtm);
    let status = snapshot
        .as_ref()
        .map(|value| format!("状态：{:?}", value.connection))
        .unwrap_or_else(|| "状态：未连接".into());
    let status_label = add_label(&content, &status, 24.0, 158.0, 460.0, 22.0, mtm);
    let app_path = snapshot
        .as_ref()
        .map(|value| value.settings.codex_app_path.display().to_string())
        .unwrap_or_else(|| "/Applications/Codex.app".into());
    add_label(&content, &app_path, 24.0, 128.0, 460.0, 22.0, mtm);
    add_button(
        &content,
        "选择 Codex.app…",
        24.0,
        85.0,
        145.0,
        target,
        sel!(selectCodex:),
        mtm,
    );
    add_button(
        &content,
        "打开 Codex",
        185.0,
        85.0,
        115.0,
        target,
        sel!(openCodex:),
        mtm,
    );
    add_button(
        &content,
        "重新连接",
        315.0,
        85.0,
        100.0,
        target,
        sel!(reconnect:),
        mtm,
    );
    if let Some(error) = state.latest_error() {
        add_label(&content, &error, 24.0, 30.0, 470.0, 42.0, mtm);
    }

    let ui = Arc::new(MainThreadBound::new(
        SettingsUi {
            status_label,
            theme_enabled,
            theme_popup: popup,
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
    *slot.borrow_mut() = Some(window);
}

struct SettingsUi {
    status_label: Retained<NSTextField>,
    theme_enabled: Retained<NSButton>,
    theme_popup: Retained<NSPopUpButton>,
    centered: Retained<NSButton>,
    width: Retained<NSTextField>,
}

impl SettingsUi {
    fn refresh(&self, snapshot: &AppSnapshot) {
        self.status_label
            .setStringValue(&NSString::from_str(&format!(
                "状态：{:?}",
                snapshot.connection
            )));
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
        for theme in &snapshot.themes {
            self.theme_popup
                .addItemWithTitle(&NSString::from_str(&theme.id));
        }
        if let Some(id) = &snapshot.settings.active_theme_id {
            self.theme_popup
                .selectItemWithTitle(&NSString::from_str(id));
        }
    }
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
) {
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
}
