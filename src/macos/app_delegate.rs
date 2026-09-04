use std::cell::RefCell;
use std::sync::Arc;

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSButton, NSColorWell, NSControlStateValueOn,
    NSOpenPanel, NSPopUpButton, NSStatusBar, NSStatusItem, NSTextField, NSVariableStatusItemLength,
    NSWindow, NSWorkspace,
};
use objc2_foundation::{NSObjectProtocol, NSString, NSURL};

use crate::controller::{AppCommand, ControllerHandle};

use super::{AppKitState, customization_window, menu, settings_window};

struct ActionTargetIvars {
    controller: ControllerHandle,
    state: Arc<AppKitState>,
    settings_window: RefCell<Option<Retained<NSWindow>>>,
    customization_window: RefCell<Option<Retained<NSWindow>>>,
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

        #[unsafe(method(openThemeGallery:))]
        fn open_theme_gallery(&self, _sender: Option<&AnyObject>) {
            let Some(url) = NSURL::URLWithString(&NSString::from_str(
                "https://dreamskin.cc/gallery",
            )) else {
                return;
            };
            let _ = NSWorkspace::sharedWorkspace().openURL(&url);
        }

        #[unsafe(method(customizeTheme:))]
        fn customize_theme(&self, _sender: Option<&AnyObject>) {
            let mtm = MainThreadMarker::from(self);
            customization_window::show(
                mtm,
                self,
                &self.ivars().state,
                &self.ivars().customization_window,
            );
        }

        #[unsafe(method(selectCustomizationComponent:))]
        fn select_customization_component(&self, sender: &NSPopUpButton) {
            let windows = self.ivars().customization_window.borrow();
            let Some(window) = windows.as_ref() else {
                return;
            };
            match customization_window::select_surface(
                window,
                &self.ivars().state,
                sender.indexOfSelectedItem(),
            ) {
                Ok(()) => {}
                Err(error) => customization_window::set_status(window, &error),
            }
        }

        #[unsafe(method(selectCustomizationFill:))]
        fn select_customization_fill(&self, sender: &NSPopUpButton) {
            let windows = self.ivars().customization_window.borrow();
            let Some(window) = windows.as_ref() else {
                return;
            };
            match customization_window::select_fill(
                window,
                &self.ivars().state,
                sender.indexOfSelectedItem(),
            ) {
                Ok(()) => {}
                Err(error) => customization_window::set_status(window, &error),
            }
        }

        #[unsafe(method(selectCustomizationColor:))]
        fn select_customization_color(&self, sender: &NSColorWell) {
            let windows = self.ivars().customization_window.borrow();
            let Some(window) = windows.as_ref() else {
                return;
            };
            customization_window::sync_hex_from_color_well(window, sender);
        }

        #[unsafe(method(editCustomizationColor:))]
        fn edit_customization_color(&self, sender: &NSTextField) {
            let windows = self.ivars().customization_window.borrow();
            let Some(window) = windows.as_ref() else {
                return;
            };
            customization_window::sync_color_well_from_field(window, sender);
        }

        #[unsafe(method(selectCustomizationImage:))]
        fn select_customization_image(&self, _sender: Option<&AnyObject>) {
            let windows = self.ivars().customization_window.borrow();
            let Some(window) = windows.as_ref() else {
                return;
            };
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
                if let Err(error) = customization_window::set_image_path(
                    window,
                    &self.ivars().state,
                    path,
                ) {
                    customization_window::set_status(window, &error);
                }
            }
        }

        #[unsafe(method(previewCustomization:))]
        fn preview_customization(&self, _sender: Option<&AnyObject>) {
            let windows = self.ivars().customization_window.borrow();
            let Some(window) = windows.as_ref() else {
                return;
            };
            match customization_window::collect_draft(window, &self.ivars().state) {
                Ok(draft) => {
                    self.ivars().state.set_customization_draft(draft.clone());
                    if self
                        .ivars()
                        .controller
                        .send(AppCommand::PreviewThemeCustomization(draft))
                        .is_ok()
                    {
                        customization_window::set_status(window, "预览已发送（未保存）");
                    } else {
                        customization_window::set_status(window, "预览请求发送失败");
                    }
                }
                Err(error) => customization_window::set_status(window, &error),
            }
        }

        #[unsafe(method(saveCustomization:))]
        fn save_customization(&self, _sender: Option<&AnyObject>) {
            let windows = self.ivars().customization_window.borrow();
            let Some(window) = windows.as_ref() else {
                return;
            };
            match customization_window::collect_draft(window, &self.ivars().state) {
                Ok(draft) => {
                    self.ivars().state.set_customization_draft(draft.clone());
                    if self
                        .ivars()
                        .controller
                        .send(AppCommand::SaveThemeCustomization(draft))
                        .is_ok()
                    {
                        customization_window::set_status(window, "保存已提交");
                    } else {
                        customization_window::set_status(window, "保存请求发送失败");
                    }
                }
                Err(error) => customization_window::set_status(window, &error),
            }
        }

        #[unsafe(method(resetCustomization:))]
        fn reset_customization(&self, _sender: Option<&AnyObject>) {
            let windows = self.ivars().customization_window.borrow();
            let Some(window) = windows.as_ref() else {
                return;
            };
            customization_window::reset_draft(window, &self.ivars().state);
            customization_window::set_status(window, "已恢复默认，请预览后保存");
        }

        #[unsafe(method(closeCustomization:))]
        fn close_customization(&self, _sender: Option<&AnyObject>) {
            if let Some(window) = self.ivars().customization_window.borrow_mut().take() {
                window.close();
            }
            self.ivars().state.clear_customization_draft();
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
                if let Some(window) = self.ivars().customization_window.borrow_mut().take() {
                    window.close();
                }
                self.ivars().state.clear_customization_draft();
                let selected = title.to_string();
                let title = if selected == "无" {
                    String::new()
                } else {
                    selected
                };
                let _ = self
                    .ivars()
                    .controller
                    .send(AppCommand::ActivateTheme(title));
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
            customization_window: RefCell::new(None),
        });
        unsafe { msg_send![super(this), init] }
    }
}

fn startup_activation_policy() -> NSApplicationActivationPolicy {
    NSApplicationActivationPolicy::Accessory
}

pub(super) fn run(controller: ControllerHandle, state: Arc<AppKitState>) -> ! {
    let mtm = MainThreadMarker::new().expect("AppKit must run on the main thread");
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(startup_activation_policy());
    let target = ActionTarget::new(controller, state);
    let status_item =
        NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
    if let Some(button) = status_item.button(mtm) {
        button.setTitle(&NSString::from_str("◐"));
        button.setToolTip(Some(&NSString::from_str("CodexSkinLite")));
    }
    let menu = menu::build_menu(mtm, &target);
    status_item.setMenu(Some(&menu));
    let _keep_alive: (Retained<ActionTarget>, Retained<NSStatusItem>) = (target, status_item);
    app.finishLaunching();
    app.activate();
    settings_window::show(
        mtm,
        &_keep_alive.0,
        &_keep_alive.0.ivars().state,
        &_keep_alive.0.ivars().settings_window,
    );
    app.run();
    std::process::exit(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_settings_start_keeps_accessory_activation_policy() {
        assert_eq!(
            startup_activation_policy(),
            NSApplicationActivationPolicy::Accessory
        );
    }
}
