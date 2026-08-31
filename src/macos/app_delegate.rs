use std::cell::RefCell;
use std::sync::Arc;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSButton, NSControlStateValueOn, NSOpenPanel,
    NSPopUpButton, NSStatusBar, NSStatusItem, NSTextField, NSVariableStatusItemLength,
};
use objc2_foundation::{NSObjectProtocol, NSString};

use crate::controller::{AppCommand, ControllerHandle};

use super::{AppKitState, menu, settings_window};

struct ActionTargetIvars {
    controller: ControllerHandle,
    state: Arc<AppKitState>,
    settings_window: RefCell<Option<Retained<objc2_app_kit::NSWindow>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ActionTargetIvars]
    struct ActionTarget;

    impl ActionTarget {
        #[unsafe(method(openCodex:))]
        fn open_codex(&self, _sender: Option<&AnyObject>) {
            let _ = self.ivars().controller.send(AppCommand::OpenCodex);
        }

        #[unsafe(method(reconnect:))]
        fn reconnect(&self, _sender: Option<&AnyObject>) {
            let _ = self.ivars().controller.send(AppCommand::Reconnect);
        }

        #[unsafe(method(confirmRestart:))]
        fn confirm_restart(&self, _sender: Option<&AnyObject>) {
            let _ = self.ivars().controller.send(AppCommand::ConfirmRestart);
        }

        #[unsafe(method(openSettings:))]
        fn open_settings(&self, _sender: Option<&AnyObject>) {
            let mtm = MainThreadMarker::from(self);
            settings_window::show(mtm, self, &self.ivars().state, &self.ivars().settings_window);
        }

        #[unsafe(method(quit:))]
        fn quit(&self, _sender: Option<&AnyObject>) {
            let _ = self.ivars().controller.send(AppCommand::Shutdown);
            NSApplication::sharedApplication(MainThreadMarker::from(self)).terminate(None);
        }

        #[unsafe(method(toggleTheme:))]
        fn toggle_theme(&self, sender: &NSButton) {
            let enabled = sender.state() == NSControlStateValueOn;
            let _ = self
                .ivars()
                .controller
                .send(AppCommand::SetThemeEnabled(enabled));
        }

        #[unsafe(method(selectTheme:))]
        fn select_theme(&self, sender: &NSPopUpButton) {
            if let Some(title) = sender.titleOfSelectedItem() {
                let _ = self
                    .ivars()
                    .controller
                    .send(AppCommand::ActivateTheme(title.to_string()));
            }
        }

        #[unsafe(method(toggleCentered:))]
        fn toggle_centered(&self, sender: &NSButton) {
            let enabled = sender.state() == NSControlStateValueOn;
            let _ = self
                .ivars()
                .controller
                .send(AppCommand::SetConversationCentered(enabled));
        }

        #[unsafe(method(setWidth:))]
        fn set_width(&self, sender: &NSTextField) {
            let width = sender.integerValue().clamp(320, 4000) as u16;
            let _ = self
                .ivars()
                .controller
                .send(AppCommand::SetConversationWidth(width));
        }

        #[unsafe(method(importTheme:))]
        fn import_theme(&self, _sender: Option<&AnyObject>) {
            let mtm = MainThreadMarker::from(self);
            let panel = NSOpenPanel::openPanel(mtm);
            panel.setCanChooseFiles(true);
            panel.setCanChooseDirectories(false);
            panel.setAllowsMultipleSelection(false);
            if panel.runModal() == objc2_app_kit::NSModalResponseOK
                && let Some(url) = panel.URLs().firstObject()
                && let Some(path) = url.path()
            {
                let path = std::path::PathBuf::from(path.to_string());
                if path.extension().and_then(|value| value.to_str()) == Some("zip") {
                    let _ = self.ivars().controller.send(AppCommand::ImportTheme(path));
                }
            }
        }

        #[unsafe(method(selectCodex:))]
        fn select_codex(&self, _sender: Option<&AnyObject>) {
            let mtm = MainThreadMarker::from(self);
            let panel = NSOpenPanel::openPanel(mtm);
            panel.setCanChooseFiles(false);
            panel.setCanChooseDirectories(true);
            panel.setAllowsMultipleSelection(false);
            if panel.runModal() == objc2_app_kit::NSModalResponseOK
                && let Some(url) = panel.URLs().firstObject()
                && let Some(path) = url.path()
            {
                let _ = self
                    .ivars()
                    .controller
                    .send(AppCommand::SetCodexPath(path.to_string().into()));
            }
        }
    }

    unsafe impl NSObjectProtocol for ActionTarget {}
);

impl ActionTarget {
    fn new(controller: ControllerHandle, state: Arc<AppKitState>) -> Retained<Self> {
        let mtm = MainThreadMarker::new().expect("action target must be created on main thread");
        let this = Self::alloc(mtm).set_ivars(ActionTargetIvars {
            controller,
            state,
            settings_window: RefCell::new(None),
        });
        unsafe { msg_send![super(this), init] }
    }
}

pub(super) fn run(controller: ControllerHandle, state: Arc<AppKitState>) -> ! {
    let mtm = MainThreadMarker::new().expect("AppKit must run on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    let open_settings = std::env::var_os("CODEX_SKIN_LITE_OPEN_SETTINGS").is_some();
    app.setActivationPolicy(if open_settings {
        NSApplicationActivationPolicy::Regular
    } else {
        NSApplicationActivationPolicy::Accessory
    });
    let target = ActionTarget::new(controller, state);
    let status_item =
        NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
    if let Some(button) = status_item.button(mtm) {
        button.setTitle(&NSString::from_str("◐"));
        button.setToolTip(Some(&NSString::from_str("CodexSkinLite")));
    }
    let menu = menu::build_menu(mtm, &target);
    status_item.setMenu(Some(&menu));
    if open_settings {
        settings_window::show(
            mtm,
            &target,
            &target.ivars().state,
            &target.ivars().settings_window,
        );
    }
    let _keep_alive: (Retained<ActionTarget>, Retained<NSStatusItem>) = (target, status_item);
    app.finishLaunching();
    app.run();
    std::process::exit(0)
}
