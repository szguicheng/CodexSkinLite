mod app_delegate;
mod customization_window;
mod menu;
mod settings_window;

use std::sync::{Arc, Mutex};

use crate::controller::{ControllerHandle, UiSink};
use crate::model::AppSnapshot;
use crate::theme::{SurfacePart, ThemeCustomization};

pub use menu::MenuAction;

#[derive(Default)]
pub struct AppKitState {
    snapshot: Mutex<Option<AppSnapshot>>,
    errors: Mutex<Vec<String>>,
    refresher: Mutex<Option<UiRefresher>>,
    customization_draft: Mutex<Option<ThemeCustomization>>,
    customization_surface: Mutex<SurfacePart>,
}

type UiRefresher = Arc<dyn Fn(AppSnapshot) + Send + Sync>;

impl AppKitState {
    pub fn snapshot(&self) -> Option<AppSnapshot> {
        self.snapshot.lock().ok()?.clone()
    }

    pub fn latest_error(&self) -> Option<String> {
        self.errors.lock().ok()?.last().cloned()
    }

    pub fn set_refresher(&self, refresher: UiRefresher) {
        if let Ok(mut current) = self.refresher.lock() {
            *current = Some(refresher);
        }
    }

    pub fn customization_draft(&self) -> Option<ThemeCustomization> {
        self.customization_draft.lock().ok()?.clone()
    }

    pub fn set_customization_draft(&self, draft: ThemeCustomization) {
        if let Ok(mut current) = self.customization_draft.lock() {
            *current = Some(draft);
        }
    }

    pub fn clear_customization_draft(&self) {
        if let Ok(mut current) = self.customization_draft.lock() {
            *current = None;
        }
    }

    pub fn customization_surface(&self) -> SurfacePart {
        self.customization_surface
            .lock()
            .map(|current| *current)
            .unwrap_or_default()
    }

    pub fn set_customization_surface(&self, surface: SurfacePart) {
        if let Ok(mut current) = self.customization_surface.lock() {
            *current = surface;
        }
    }
}

pub struct AppKitSink {
    state: Arc<AppKitState>,
}

impl AppKitSink {
    pub fn new(state: Arc<AppKitState>) -> Self {
        Self { state }
    }
}

impl UiSink for AppKitSink {
    fn publish(&self, snapshot: AppSnapshot) {
        if let Ok(mut current) = self.state.snapshot.lock() {
            *current = Some(snapshot.clone());
        }
        if let Ok(current) = self.state.refresher.lock()
            && let Some(refresher) = current.clone()
        {
            refresher(snapshot);
        }
    }

    fn report_error(&self, title: &str, message: &str) {
        if let Ok(mut errors) = self.state.errors.lock() {
            errors.push(format!("{title}: {message}"));
            if errors.len() > 20 {
                errors.remove(0);
            }
        }
    }
}

pub fn run(controller: ControllerHandle, state: Arc<AppKitState>) -> ! {
    app_delegate::run(controller, state)
}
