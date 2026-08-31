#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    OpenCodex,
    Reconnect,
    OpenSettings,
    Quit,
}

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadOnly, sel};
use objc2_app_kit::{NSMenu, NSMenuItem};
use objc2_foundation::NSString;

pub(crate) fn build_menu(mtm: MainThreadMarker, target: &AnyObject) -> Retained<NSMenu> {
    let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str("CodexSkinLite"));
    for (title, action) in [
        ("打开 Codex", sel!(openCodex:)),
        ("重新连接", sel!(reconnect:)),
        ("设置…", sel!(openSettings:)),
    ] {
        let item = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(title),
                Some(action),
                &NSString::new(),
            )
        };
        unsafe { item.setTarget(Some(target)) };
        menu.addItem(&item);
    }
    menu.addItem(&NSMenuItem::separatorItem(mtm));
    let quit = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            NSMenuItem::alloc(mtm),
            &NSString::from_str("退出"),
            Some(sel!(quit:)),
            &NSString::from_str("q"),
        )
    };
    unsafe { quit.setTarget(Some(target)) };
    menu.addItem(&quit);
    menu
}

impl MenuAction {
    pub const ALL: [Self; 4] = [
        Self::OpenCodex,
        Self::Reconnect,
        Self::OpenSettings,
        Self::Quit,
    ];
}
