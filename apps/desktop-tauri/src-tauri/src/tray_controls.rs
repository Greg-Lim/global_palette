use std::{io, sync::Arc};

use tauri::menu::{Menu, MenuItem};

pub const TRAY_ID: &str = "main";
pub const TRAY_SETTINGS_ID: &str = "tray-settings";
pub const TRAY_TOGGLE_PAUSE_ID: &str = "tray-toggle-pause";
pub const PAUSE_LABEL: &str = "Pause Omni Palette";
pub const PLAY_LABEL: &str = "Play Omni Palette";

pub trait TrayMenuController: Send + Sync {
    fn show_settings(&self);
    fn toggle_pause(&self) -> bool;
}

pub fn pause_toggle_label(paused: bool) -> &'static str {
    if paused {
        PLAY_LABEL
    } else {
        PAUSE_LABEL
    }
}

pub fn handle_tray_menu_action(menu_id: &str, controller: &dyn TrayMenuController) -> bool {
    match menu_id {
        TRAY_SETTINGS_ID => {
            controller.show_settings();
            true
        }
        TRAY_TOGGLE_PAUSE_ID => {
            controller.toggle_pause();
            true
        }
        _ => false,
    }
}

pub fn install_tray_menu(
    app: &tauri::AppHandle,
    controller: Arc<dyn TrayMenuController>,
) -> tauri::Result<()> {
    let settings_item = MenuItem::with_id(app, TRAY_SETTINGS_ID, "Settings", true, None::<&str>)?;
    let toggle_item =
        MenuItem::with_id(app, TRAY_TOGGLE_PAUSE_ID, PAUSE_LABEL, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings_item, &toggle_item])?;
    let tray = app.tray_by_id(TRAY_ID).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Tauri tray icon `{TRAY_ID}` was not created"),
        )
    })?;

    tray.set_menu(Some(menu))?;
    tray.set_show_menu_on_left_click(true)?;

    let toggle_item_for_handler = toggle_item.clone();
    app.on_menu_event(move |_app, event| {
        if event.id() == TRAY_TOGGLE_PAUSE_ID {
            let paused = controller.toggle_pause();
            let _ = toggle_item_for_handler.set_text(pause_toggle_label(paused));
        } else {
            handle_tray_menu_action(event.id().as_ref(), controller.as_ref());
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct RecordingTrayMenuController {
        calls: Mutex<Vec<&'static str>>,
        paused: Mutex<bool>,
    }

    impl TrayMenuController for RecordingTrayMenuController {
        fn show_settings(&self) {
            self.calls
                .lock()
                .expect("calls should lock")
                .push("settings");
        }

        fn toggle_pause(&self) -> bool {
            let mut paused = self.paused.lock().expect("paused should lock");
            *paused = !*paused;
            self.calls.lock().expect("calls should lock").push("toggle");
            *paused
        }
    }

    impl RecordingTrayMenuController {
        fn calls(&self) -> Vec<&'static str> {
            self.calls.lock().expect("calls should lock").clone()
        }
    }

    #[test]
    fn tray_menu_handler_calls_settings_for_settings_item() {
        let controller = RecordingTrayMenuController::default();

        let handled = handle_tray_menu_action(TRAY_SETTINGS_ID, &controller);

        assert!(handled);
        assert_eq!(controller.calls(), vec!["settings"]);
    }

    #[test]
    fn tray_menu_handler_toggles_pause_for_pause_play_item() {
        let controller = RecordingTrayMenuController::default();

        let handled = handle_tray_menu_action(TRAY_TOGGLE_PAUSE_ID, &controller);

        assert!(handled);
        assert_eq!(controller.calls(), vec!["toggle"]);
    }

    #[test]
    fn tray_pause_toggle_label_switches_between_pause_and_play() {
        assert_eq!(pause_toggle_label(false), PAUSE_LABEL);
        assert_eq!(pause_toggle_label(true), PLAY_LABEL);
    }
}
