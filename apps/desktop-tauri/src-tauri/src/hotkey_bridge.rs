use std::sync::{Arc, Mutex};

use omni_palette::{
    domain::{
        action::ContextRoot,
        hotkey::{Key, KeyboardShortcut},
    },
    runtime_state::OmniRuntimeState,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{
    Code as GlobalShortcutCode, GlobalShortcutExt, Modifiers as GlobalShortcutModifiers,
    Shortcut as GlobalShortcut, ShortcutEvent, ShortcutState,
};

#[cfg(target_os = "windows")]
use omni_palette::platform::{
    platform_interface::{get_all_context, RawWindowHandleExt},
    windows::sender::hotkey_sender::send_shortcut,
};

pub const HOTKEY_EVENT_NAME: &str = "omni://palette-activation-requested";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeyStatusDto {
    pub running: bool,
    pub paused: bool,
    pub activation_hint: String,
    pub activation_count: u64,
    pub ignored_passthrough_count: u64,
    pub last_event: Option<HotkeyEventPayloadDto>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeyEventPayloadDto {
    pub kind: HotkeyEventKindDto,
    pub shortcut: String,
    pub process_name: Option<String>,
    pub activation_count: u64,
    pub ignored_passthrough_count: u64,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyEventKindDto {
    ActivationRequested,
    IgnoredPassthrough,
    ListenerError,
}

#[derive(Clone)]
struct HotkeyStatusStore {
    inner: Arc<Mutex<HotkeyStatusState>>,
}

#[derive(Debug)]
struct HotkeyStatusState {
    running: bool,
    paused: bool,
    activation_hint: String,
    activation_count: u64,
    ignored_passthrough_count: u64,
    last_event: Option<HotkeyEventPayloadDto>,
    last_error: Option<String>,
}

impl HotkeyStatusStore {
    fn new(activation_hint: String) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HotkeyStatusState {
                running: false,
                paused: false,
                activation_hint,
                activation_count: 0,
                ignored_passthrough_count: 0,
                last_event: None,
                last_error: None,
            })),
        }
    }

    fn snapshot(&self) -> HotkeyStatusDto {
        let state = self.inner.lock().expect("hotkey status should lock");
        HotkeyStatusDto {
            running: state.running,
            paused: state.paused,
            activation_hint: state.activation_hint.clone(),
            activation_count: state.activation_count,
            ignored_passthrough_count: state.ignored_passthrough_count,
            last_event: state.last_event.clone(),
            last_error: state.last_error.clone(),
        }
    }

    fn record_running(&self) {
        let mut state = self.inner.lock().expect("hotkey status should lock");
        state.running = true;
        state.paused = false;
        state.last_error = None;
    }

    fn record_paused(&self) {
        let mut state = self.inner.lock().expect("hotkey status should lock");
        state.running = false;
        state.paused = true;
        state.last_error = None;
    }

    fn record_resume_error(&self, message: String) -> HotkeyEventPayloadDto {
        let mut state = self.inner.lock().expect("hotkey status should lock");
        state.running = false;
        state.paused = true;
        state.last_error = Some(message.clone());
        let payload = HotkeyEventPayloadDto {
            kind: HotkeyEventKindDto::ListenerError,
            shortcut: state.activation_hint.clone(),
            process_name: None,
            activation_count: state.activation_count,
            ignored_passthrough_count: state.ignored_passthrough_count,
            message: Some(message),
        };
        state.last_event = Some(payload.clone());
        payload
    }

    fn is_paused(&self) -> bool {
        self.inner.lock().expect("hotkey status should lock").paused
    }

    fn update_activation_hint(&self, activation_hint: String) {
        let mut state = self.inner.lock().expect("hotkey status should lock");
        state.activation_hint = activation_hint;
        state.last_error = None;
    }

    fn record_activation(
        &self,
        shortcut: KeyboardShortcut,
        process_name: Option<String>,
    ) -> HotkeyEventPayloadDto {
        let mut state = self.inner.lock().expect("hotkey status should lock");
        state.activation_count += 1;
        state.last_error = None;
        let payload = HotkeyEventPayloadDto {
            kind: HotkeyEventKindDto::ActivationRequested,
            shortcut: shortcut.to_string(),
            process_name,
            activation_count: state.activation_count,
            ignored_passthrough_count: state.ignored_passthrough_count,
            message: None,
        };
        state.last_event = Some(payload.clone());
        payload
    }

    fn record_ignored_passthrough(
        &self,
        shortcut: KeyboardShortcut,
        process_name: Option<String>,
    ) -> HotkeyEventPayloadDto {
        let mut state = self.inner.lock().expect("hotkey status should lock");
        state.ignored_passthrough_count += 1;
        state.last_error = None;
        let payload = HotkeyEventPayloadDto {
            kind: HotkeyEventKindDto::IgnoredPassthrough,
            shortcut: shortcut.to_string(),
            process_name,
            activation_count: state.activation_count,
            ignored_passthrough_count: state.ignored_passthrough_count,
            message: None,
        };
        state.last_event = Some(payload.clone());
        payload
    }

    fn record_error(&self, message: String) -> HotkeyEventPayloadDto {
        let mut state = self.inner.lock().expect("hotkey status should lock");
        state.running = false;
        state.paused = false;
        state.last_error = Some(message.clone());
        let payload = HotkeyEventPayloadDto {
            kind: HotkeyEventKindDto::ListenerError,
            shortcut: state.activation_hint.clone(),
            process_name: None,
            activation_count: state.activation_count,
            ignored_passthrough_count: state.ignored_passthrough_count,
            message: Some(message),
        };
        state.last_event = Some(payload.clone());
        payload
    }
}

trait HotkeyEventSink: Send + Sync {
    fn emit_hotkey_event(&self, payload: HotkeyEventPayloadDto) -> Result<(), String>;
}

pub trait PaletteActivationHandler: Send + Sync {
    fn handle_palette_activation(&self, context: ContextRoot);
    fn handle_guide_activation(&self) -> bool {
        false
    }
}

struct TauriHotkeyEventSink {
    app: AppHandle,
}

impl TauriHotkeyEventSink {
    fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl HotkeyEventSink for TauriHotkeyEventSink {
    fn emit_hotkey_event(&self, payload: HotkeyEventPayloadDto) -> Result<(), String> {
        self.app
            .emit(HOTKEY_EVENT_NAME, payload)
            .map_err(|err| format!("Failed to emit hotkey event: {err}"))
    }
}

trait GlobalShortcutRegistrar: Send + Sync {
    fn register_activation_shortcut(&self, shortcut: KeyboardShortcut) -> Result<(), String>;
    fn unregister_activation_shortcut(&self, shortcut: KeyboardShortcut) -> Result<(), String>;
}

struct TauriGlobalShortcutRegistrar {
    app: AppHandle,
}

impl TauriGlobalShortcutRegistrar {
    fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl GlobalShortcutRegistrar for TauriGlobalShortcutRegistrar {
    fn register_activation_shortcut(&self, shortcut: KeyboardShortcut) -> Result<(), String> {
        self.app
            .global_shortcut()
            .register(global_shortcut_from_keyboard_shortcut(shortcut))
            .map_err(|err| format!("Failed to register activation shortcut {shortcut}: {err}"))
    }

    fn unregister_activation_shortcut(&self, shortcut: KeyboardShortcut) -> Result<(), String> {
        self.app
            .global_shortcut()
            .unregister(global_shortcut_from_keyboard_shortcut(shortcut))
            .map_err(|err| format!("Failed to unregister activation shortcut {shortcut}: {err}"))
    }
}

trait HotkeyForwarder: Send + Sync {
    fn forward_shortcut(&self, shortcut: KeyboardShortcut) -> Result<(), String>;
}

struct PlatformHotkeyForwarder;

impl HotkeyForwarder for PlatformHotkeyForwarder {
    fn forward_shortcut(&self, shortcut: KeyboardShortcut) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            send_shortcut(&shortcut);
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = shortcut;
            Err("Hotkey passthrough is only supported on Windows".to_string())
        }
    }
}

struct ActiveWindowContext {
    context: ContextRoot,
    process_name: Option<String>,
}

trait ActiveProcessProvider: Send + Sync {
    fn active_window_context(&self) -> ActiveWindowContext;
}

#[cfg(target_os = "windows")]
struct WindowsActiveProcessProvider;

#[cfg(target_os = "windows")]
impl ActiveProcessProvider for WindowsActiveProcessProvider {
    fn active_window_context(&self) -> ActiveWindowContext {
        let context = get_all_context();
        let process_name = context
            .get_active()
            .and_then(|handle| handle.get_app_process_name());
        ActiveWindowContext {
            context,
            process_name,
        }
    }
}

#[cfg(not(target_os = "windows"))]
struct EmptyActiveProcessProvider;

#[cfg(not(target_os = "windows"))]
impl ActiveProcessProvider for EmptyActiveProcessProvider {
    fn active_window_context(&self) -> ActiveWindowContext {
        ActiveWindowContext {
            context: ContextRoot {
                fg_context: Vec::new(),
                bg_context: Vec::new(),
                active_interaction: Default::default(),
            },
            process_name: None,
        }
    }
}

pub struct HotkeyBridge {
    status: HotkeyStatusStore,
    activation_shortcut: Mutex<KeyboardShortcut>,
    registrar: Arc<dyn GlobalShortcutRegistrar>,
    forwarder: Arc<dyn HotkeyForwarder>,
    runtime_state: OmniRuntimeState,
    event_sink: Arc<dyn HotkeyEventSink>,
    activation_handler: Arc<dyn PaletteActivationHandler>,
    active_process_provider: Arc<dyn ActiveProcessProvider>,
}

impl HotkeyBridge {
    pub fn start(
        runtime_state: OmniRuntimeState,
        app: AppHandle,
        activation_handler: Arc<dyn PaletteActivationHandler>,
    ) -> Self {
        #[cfg(target_os = "windows")]
        let active_process_provider: Arc<dyn ActiveProcessProvider> =
            Arc::new(WindowsActiveProcessProvider);
        #[cfg(not(target_os = "windows"))]
        let active_process_provider: Arc<dyn ActiveProcessProvider> =
            Arc::new(EmptyActiveProcessProvider);

        Self::with_components(
            runtime_state,
            Arc::new(TauriGlobalShortcutRegistrar::new(app.clone())),
            Arc::new(PlatformHotkeyForwarder),
            Arc::new(TauriHotkeyEventSink::new(app)),
            activation_handler,
            active_process_provider,
        )
    }

    fn with_components(
        runtime_state: OmniRuntimeState,
        registrar: Arc<dyn GlobalShortcutRegistrar>,
        forwarder: Arc<dyn HotkeyForwarder>,
        event_sink: Arc<dyn HotkeyEventSink>,
        activation_handler: Arc<dyn PaletteActivationHandler>,
        active_process_provider: Arc<dyn ActiveProcessProvider>,
    ) -> Self {
        let activation_shortcut = runtime_state.config().activation;
        let activation_hint = activation_shortcut.to_string();
        let status = HotkeyStatusStore::new(activation_hint.clone());

        if let Err(err) = registrar.register_activation_shortcut(activation_shortcut) {
            status.record_error(err);
        } else {
            status.record_running();
        }

        Self {
            status,
            activation_shortcut: Mutex::new(activation_shortcut),
            registrar,
            forwarder,
            runtime_state,
            event_sink,
            activation_handler,
            active_process_provider,
        }
    }

    pub fn status(&self) -> HotkeyStatusDto {
        self.status.snapshot()
    }

    pub fn pause_activation(&self) -> Result<(), String> {
        if self.status.is_paused() {
            return Ok(());
        }

        let shortcut = *self
            .activation_shortcut
            .lock()
            .expect("activation shortcut should lock");

        if let Err(err) = self.registrar.unregister_activation_shortcut(shortcut) {
            let message = format!("Could not unregister paused activation shortcut: {err}");
            self.status.record_error(message.clone());
            return Err(message);
        }

        self.status.record_paused();
        Ok(())
    }

    pub fn resume_activation(&self) -> Result<(), String> {
        if !self.status.is_paused() && self.status.snapshot().running {
            return Ok(());
        }

        let shortcut = *self
            .activation_shortcut
            .lock()
            .expect("activation shortcut should lock");

        if let Err(err) = self.registrar.register_activation_shortcut(shortcut) {
            let message = format!("Could not resume activation shortcut {shortcut}: {err}");
            self.status.record_resume_error(message.clone());
            return Err(message);
        }

        self.status.record_running();
        Ok(())
    }

    pub fn update_activation_shortcut(&self, shortcut: KeyboardShortcut) -> Result<(), String> {
        let mut active_shortcut = self
            .activation_shortcut
            .lock()
            .expect("activation shortcut should lock");
        let previous_shortcut = *active_shortcut;
        if previous_shortcut == shortcut {
            return Ok(());
        }

        if self.status.is_paused() {
            *active_shortcut = shortcut;
            self.status.update_activation_hint(shortcut.to_string());
            return Ok(());
        }

        if let Err(err) = self
            .registrar
            .unregister_activation_shortcut(previous_shortcut)
        {
            let message = format!("Could not unregister previous activation shortcut: {err}");
            self.status.record_error(message.clone());
            return Err(message);
        }

        if let Err(err) = self.registrar.register_activation_shortcut(shortcut) {
            let rollback_result = self
                .registrar
                .register_activation_shortcut(previous_shortcut);
            let message = match rollback_result {
                Ok(()) => {
                    format!("Could not register activation shortcut {shortcut}; restored previous shortcut {previous_shortcut}: {err}")
                }
                Err(rollback_err) => {
                    format!("Could not register activation shortcut {shortcut}: {err}; additionally failed to restore previous shortcut {previous_shortcut}: {rollback_err}")
                }
            };
            self.status.record_error(message.clone());
            return Err(message);
        }

        *active_shortcut = shortcut;
        self.status.update_activation_hint(shortcut.to_string());
        self.status.record_running();
        Ok(())
    }

    pub fn handle_global_shortcut_event(&self, event: ShortcutEvent) {
        if event.state() != ShortcutState::Pressed {
            return;
        }
        if self.status.is_paused() {
            return;
        }
        let shortcut = *self
            .activation_shortcut
            .lock()
            .expect("activation shortcut should lock");
        let active_context = self.active_process_provider.active_window_context();
        handle_hotkey_event(
            shortcut,
            &self.runtime_state,
            active_context,
            self.forwarder.as_ref(),
            self.event_sink.as_ref(),
            self.activation_handler.as_ref(),
            &self.status,
        );
    }
}

fn handle_hotkey_event(
    shortcut: KeyboardShortcut,
    runtime_state: &OmniRuntimeState,
    active_context: ActiveWindowContext,
    forwarder: &dyn HotkeyForwarder,
    event_sink: &dyn HotkeyEventSink,
    activation_handler: &dyn PaletteActivationHandler,
    status: &HotkeyStatusStore,
) {
    if activation_handler.handle_guide_activation() {
        return;
    }

    let payload = if active_context
        .process_name
        .as_deref()
        .is_some_and(|process_name| runtime_state.is_ignored_process_name(process_name))
    {
        if let Err(err) = forwarder.forward_shortcut(shortcut) {
            let payload = status.record_error(format!("Failed to forward ignored hotkey: {err}"));
            let _ = event_sink.emit_hotkey_event(payload);
            return;
        }
        status.record_ignored_passthrough(shortcut, active_context.process_name)
    } else {
        let payload = status.record_activation(shortcut, active_context.process_name);
        activation_handler.handle_palette_activation(active_context.context);
        payload
    };

    if let Err(err) = event_sink.emit_hotkey_event(payload) {
        status.record_error(err);
    }
}

fn global_shortcut_from_keyboard_shortcut(shortcut: KeyboardShortcut) -> GlobalShortcut {
    let mut modifiers = GlobalShortcutModifiers::empty();
    if shortcut.modifier.control {
        modifiers.insert(GlobalShortcutModifiers::CONTROL);
    }
    if shortcut.modifier.shift {
        modifiers.insert(GlobalShortcutModifiers::SHIFT);
    }
    if shortcut.modifier.alt {
        modifiers.insert(GlobalShortcutModifiers::ALT);
    }
    if shortcut.modifier.win {
        modifiers.insert(GlobalShortcutModifiers::SUPER);
    }

    GlobalShortcut::new(Some(modifiers), global_shortcut_code(shortcut.key))
}

fn global_shortcut_code(key: Key) -> GlobalShortcutCode {
    match key {
        Key::KeyA => GlobalShortcutCode::KeyA,
        Key::KeyB => GlobalShortcutCode::KeyB,
        Key::KeyC => GlobalShortcutCode::KeyC,
        Key::KeyD => GlobalShortcutCode::KeyD,
        Key::KeyE => GlobalShortcutCode::KeyE,
        Key::KeyF => GlobalShortcutCode::KeyF,
        Key::KeyG => GlobalShortcutCode::KeyG,
        Key::KeyH => GlobalShortcutCode::KeyH,
        Key::KeyI => GlobalShortcutCode::KeyI,
        Key::KeyJ => GlobalShortcutCode::KeyJ,
        Key::KeyK => GlobalShortcutCode::KeyK,
        Key::KeyL => GlobalShortcutCode::KeyL,
        Key::KeyM => GlobalShortcutCode::KeyM,
        Key::KeyN => GlobalShortcutCode::KeyN,
        Key::KeyO => GlobalShortcutCode::KeyO,
        Key::KeyP => GlobalShortcutCode::KeyP,
        Key::KeyQ => GlobalShortcutCode::KeyQ,
        Key::KeyR => GlobalShortcutCode::KeyR,
        Key::KeyS => GlobalShortcutCode::KeyS,
        Key::KeyT => GlobalShortcutCode::KeyT,
        Key::KeyU => GlobalShortcutCode::KeyU,
        Key::KeyV => GlobalShortcutCode::KeyV,
        Key::KeyW => GlobalShortcutCode::KeyW,
        Key::KeyX => GlobalShortcutCode::KeyX,
        Key::KeyY => GlobalShortcutCode::KeyY,
        Key::KeyZ => GlobalShortcutCode::KeyZ,
        Key::Key0 => GlobalShortcutCode::Digit0,
        Key::Key1 => GlobalShortcutCode::Digit1,
        Key::Key2 => GlobalShortcutCode::Digit2,
        Key::Key3 => GlobalShortcutCode::Digit3,
        Key::Key4 => GlobalShortcutCode::Digit4,
        Key::Key5 => GlobalShortcutCode::Digit5,
        Key::Key6 => GlobalShortcutCode::Digit6,
        Key::Key7 => GlobalShortcutCode::Digit7,
        Key::Key8 => GlobalShortcutCode::Digit8,
        Key::Key9 => GlobalShortcutCode::Digit9,
        Key::F1 => GlobalShortcutCode::F1,
        Key::F2 => GlobalShortcutCode::F2,
        Key::F3 => GlobalShortcutCode::F3,
        Key::F4 => GlobalShortcutCode::F4,
        Key::F5 => GlobalShortcutCode::F5,
        Key::F6 => GlobalShortcutCode::F6,
        Key::F7 => GlobalShortcutCode::F7,
        Key::F8 => GlobalShortcutCode::F8,
        Key::F9 => GlobalShortcutCode::F9,
        Key::F10 => GlobalShortcutCode::F10,
        Key::F11 => GlobalShortcutCode::F11,
        Key::F12 => GlobalShortcutCode::F12,
        Key::Semicolon => GlobalShortcutCode::Semicolon,
        Key::Equal => GlobalShortcutCode::Equal,
        Key::Comma => GlobalShortcutCode::Comma,
        Key::Minus => GlobalShortcutCode::Minus,
        Key::Period => GlobalShortcutCode::Period,
        Key::Slash => GlobalShortcutCode::Slash,
        Key::Grave => GlobalShortcutCode::Backquote,
        Key::LeftBracket => GlobalShortcutCode::BracketLeft,
        Key::Backslash => GlobalShortcutCode::Backslash,
        Key::RightBracket => GlobalShortcutCode::BracketRight,
        Key::Apostrophe => GlobalShortcutCode::Quote,
        Key::Enter => GlobalShortcutCode::Enter,
        Key::Space => GlobalShortcutCode::Space,
        Key::Tab => GlobalShortcutCode::Tab,
        Key::Escape => GlobalShortcutCode::Escape,
        Key::Delete => GlobalShortcutCode::Delete,
        Key::BackSpace => GlobalShortcutCode::Backspace,
        Key::Home => GlobalShortcutCode::Home,
        Key::End => GlobalShortcutCode::End,
        Key::PageUp => GlobalShortcutCode::PageUp,
        Key::PageDown => GlobalShortcutCode::PageDown,
        Key::Insert => GlobalShortcutCode::Insert,
        Key::PrintScreen => GlobalShortcutCode::PrintScreen,
        Key::ScrollLock => GlobalShortcutCode::ScrollLock,
        Key::Pause => GlobalShortcutCode::Pause,
        Key::LeftArrow => GlobalShortcutCode::ArrowLeft,
        Key::RightArrow => GlobalShortcutCode::ArrowRight,
        Key::UpArrow => GlobalShortcutCode::ArrowUp,
        Key::DownArrow => GlobalShortcutCode::ArrowDown,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        sync::{Arc, Mutex},
        time::{SystemTime, UNIX_EPOCH},
    };

    use omni_palette::{
        config::runtime::{RuntimeConfig, RuntimePaths},
        domain::{
            action::{ContextRoot, Os},
            hotkey::{HotkeyModifiers, Key, KeyboardShortcut},
        },
        runtime_state::{OmniRuntimeState, RuntimeStateLoadOptions},
    };

    use super::*;

    #[test]
    fn hotkey_status_starts_not_running() {
        let status = HotkeyStatusStore::new("Ctrl+Shift+P".to_string());

        assert_eq!(
            status.snapshot(),
            HotkeyStatusDto {
                running: false,
                paused: false,
                activation_hint: "Ctrl+Shift+P".to_string(),
                activation_count: 0,
                ignored_passthrough_count: 0,
                last_event: None,
                last_error: None,
            }
        );
    }

    #[test]
    fn starting_bridge_registers_activation_shortcut_and_records_running_status() {
        let runtime = runtime_with_ignored_processes(&[]);
        let registrar = Arc::new(RecordingGlobalShortcutRegistrar::default());
        let bridge = bridge_with_components(runtime, registrar.clone());

        assert!(bridge.status().running);
        assert_eq!(registrar.calls(), vec!["register:Ctrl+Shift+P"]);
    }

    #[test]
    fn listener_startup_failure_records_controlled_error() {
        let bridge = HotkeyBridge::with_components(
            runtime_with_ignored_processes(&[]),
            Arc::new(RecordingGlobalShortcutRegistrar::failing_register(
                "register failed",
            )),
            Arc::new(RecordingForwarder::default()),
            Arc::new(RecordingEventSink::default()),
            Arc::new(RecordingActivationHandler::default()),
            Arc::new(RecordingActiveProcessProvider::default()),
        );

        assert_eq!(bridge.status().running, false);
        assert_eq!(
            bridge.status().last_error,
            Some("register failed".to_string())
        );
    }

    #[test]
    fn non_ignored_activation_records_event_without_forwarding() {
        let runtime = runtime_with_ignored_processes(&["code.exe"]);
        let status = HotkeyStatusStore::new("Ctrl+Shift+P".to_string());
        status.record_running();
        let sink = RecordingEventSink::default();
        let forwarder = RecordingForwarder::default();
        let activation_handler = RecordingActivationHandler::default();
        let shortcut = activation_shortcut();

        handle_hotkey_event(
            shortcut,
            &runtime,
            ActiveWindowContext {
                context: empty_context(),
                process_name: Some("notepad.exe".to_string()),
            },
            &forwarder,
            &sink,
            &activation_handler,
            &status,
        );

        let snapshot = status.snapshot();
        assert_eq!(snapshot.activation_count, 1);
        assert_eq!(snapshot.ignored_passthrough_count, 0);
        assert_eq!(forwarder.forwarded_shortcuts(), Vec::<String>::new());
        assert_eq!(activation_handler.activation_count(), 1);
        assert_eq!(
            sink.events(),
            vec![HotkeyEventPayloadDto {
                kind: HotkeyEventKindDto::ActivationRequested,
                shortcut: "Ctrl+Shift+P".to_string(),
                process_name: Some("notepad.exe".to_string()),
                activation_count: 1,
                ignored_passthrough_count: 0,
                message: None,
            }]
        );
    }

    #[test]
    fn ignored_foreground_process_records_passthrough_and_forwards_shortcut() {
        let runtime = runtime_with_ignored_processes(&["code.exe"]);
        let status = HotkeyStatusStore::new("Ctrl+Shift+P".to_string());
        status.record_running();
        let sink = RecordingEventSink::default();
        let forwarder = RecordingForwarder::default();
        let activation_handler = RecordingActivationHandler::default();
        let shortcut = activation_shortcut();

        handle_hotkey_event(
            shortcut,
            &runtime,
            ActiveWindowContext {
                context: empty_context(),
                process_name: Some("Code.exe".to_string()),
            },
            &forwarder,
            &sink,
            &activation_handler,
            &status,
        );

        let snapshot = status.snapshot();
        assert_eq!(snapshot.activation_count, 0);
        assert_eq!(snapshot.ignored_passthrough_count, 1);
        assert_eq!(
            forwarder.forwarded_shortcuts(),
            vec!["Ctrl+Shift+P".to_string()]
        );
        assert_eq!(activation_handler.activation_count(), 0);
        assert_eq!(
            sink.events(),
            vec![HotkeyEventPayloadDto {
                kind: HotkeyEventKindDto::IgnoredPassthrough,
                shortcut: "Ctrl+Shift+P".to_string(),
                process_name: Some("Code.exe".to_string()),
                activation_count: 0,
                ignored_passthrough_count: 1,
                message: None,
            }]
        );
    }

    #[test]
    fn guide_activation_executes_active_guide_without_palette_activation() {
        let runtime = runtime_with_ignored_processes(&[]);
        let status = HotkeyStatusStore::new("Ctrl+Shift+P".to_string());
        status.record_running();
        let sink = RecordingEventSink::default();
        let forwarder = RecordingForwarder::default();
        let activation_handler = RecordingActivationHandler::with_guide_activation();

        handle_hotkey_event(
            activation_shortcut(),
            &runtime,
            ActiveWindowContext {
                context: empty_context(),
                process_name: Some("notepad.exe".to_string()),
            },
            &forwarder,
            &sink,
            &activation_handler,
            &status,
        );

        assert_eq!(activation_handler.guide_activation_count(), 1);
        assert_eq!(activation_handler.activation_count(), 0);
        assert_eq!(status.snapshot().activation_count, 0);
        assert_eq!(sink.events(), Vec::new());
    }

    #[test]
    fn updating_activation_shortcut_unregisters_old_and_registers_new_shortcut() {
        let runtime = runtime_with_ignored_processes(&[]);
        let registrar = Arc::new(RecordingGlobalShortcutRegistrar::default());
        let bridge = bridge_with_components(runtime, registrar.clone());

        bridge
            .update_activation_shortcut(ctrl_alt_space_shortcut())
            .expect("activation shortcut should update");

        assert_eq!(bridge.status().activation_hint, "Ctrl+Alt+Space");
        assert!(bridge.status().running);
        assert_eq!(
            registrar.calls(),
            vec![
                "register:Ctrl+Shift+P",
                "unregister:Ctrl+Shift+P",
                "register:Ctrl+Alt+Space",
            ]
        );
    }

    #[test]
    fn pausing_activation_unregisters_activation_shortcut_and_reports_paused() {
        let runtime = runtime_with_ignored_processes(&[]);
        let registrar = Arc::new(RecordingGlobalShortcutRegistrar::default());
        let bridge = bridge_with_components(runtime, registrar.clone());

        bridge
            .pause_activation()
            .expect("activation shortcut should pause");

        let status = bridge.status();
        assert!(!status.running);
        assert!(status.paused);
        assert_eq!(status.activation_hint, "Ctrl+Shift+P");
        assert_eq!(
            registrar.calls(),
            vec!["register:Ctrl+Shift+P", "unregister:Ctrl+Shift+P"]
        );
    }

    #[test]
    fn resuming_activation_reregisters_stored_shortcut_and_reports_running() {
        let runtime = runtime_with_ignored_processes(&[]);
        let registrar = Arc::new(RecordingGlobalShortcutRegistrar::default());
        let bridge = bridge_with_components(runtime, registrar.clone());

        bridge
            .pause_activation()
            .expect("activation shortcut should pause");
        bridge
            .resume_activation()
            .expect("activation shortcut should resume");

        let status = bridge.status();
        assert!(status.running);
        assert!(!status.paused);
        assert_eq!(
            registrar.calls(),
            vec![
                "register:Ctrl+Shift+P",
                "unregister:Ctrl+Shift+P",
                "register:Ctrl+Shift+P",
            ]
        );
    }

    #[test]
    fn updating_activation_shortcut_while_paused_defers_registration_until_resume() {
        let runtime = runtime_with_ignored_processes(&[]);
        let registrar = Arc::new(RecordingGlobalShortcutRegistrar::default());
        let bridge = bridge_with_components(runtime, registrar.clone());

        bridge
            .pause_activation()
            .expect("activation shortcut should pause");
        bridge
            .update_activation_shortcut(ctrl_alt_space_shortcut())
            .expect("paused activation shortcut should update without registering");

        assert_eq!(bridge.status().activation_hint, "Ctrl+Alt+Space");
        assert!(!bridge.status().running);
        assert!(bridge.status().paused);
        assert_eq!(
            registrar.calls(),
            vec!["register:Ctrl+Shift+P", "unregister:Ctrl+Shift+P"]
        );

        bridge
            .resume_activation()
            .expect("stored activation shortcut should register on resume");

        assert!(bridge.status().running);
        assert!(!bridge.status().paused);
        assert_eq!(
            registrar.calls(),
            vec![
                "register:Ctrl+Shift+P",
                "unregister:Ctrl+Shift+P",
                "register:Ctrl+Alt+Space",
            ]
        );
    }

    #[test]
    fn paused_global_shortcut_events_do_not_activate_palette_or_guide() {
        let runtime = runtime_with_ignored_processes(&[]);
        let registrar = Arc::new(RecordingGlobalShortcutRegistrar::default());
        let sink = Arc::new(RecordingEventSink::default());
        let activation_handler = Arc::new(RecordingActivationHandler::default());
        let bridge = HotkeyBridge::with_components(
            runtime,
            registrar,
            Arc::new(RecordingForwarder::default()),
            sink.clone(),
            activation_handler.clone(),
            Arc::new(RecordingActiveProcessProvider::default()),
        );

        bridge
            .pause_activation()
            .expect("activation shortcut should pause");
        bridge.handle_global_shortcut_event(ShortcutEvent {
            id: 1,
            state: ShortcutState::Pressed,
        });

        assert_eq!(activation_handler.activation_count(), 0);
        assert_eq!(activation_handler.guide_activation_count(), 0);
        assert_eq!(bridge.status().activation_count, 0);
        assert_eq!(sink.events(), Vec::new());
    }

    #[test]
    fn failed_activation_update_restores_previous_registration_and_reports_failure() {
        let runtime = runtime_with_ignored_processes(&[]);
        let registrar = Arc::new(RecordingGlobalShortcutRegistrar::failing_for_shortcut(
            ctrl_alt_space_shortcut(),
            "shortcut unavailable",
        ));
        let bridge = bridge_with_components(runtime, registrar.clone());

        let result = bridge.update_activation_shortcut(ctrl_alt_space_shortcut());

        assert!(result.is_err());
        assert_eq!(bridge.status().activation_hint, "Ctrl+Shift+P");
        assert_eq!(bridge.status().running, false);
        assert_eq!(
            registrar.calls(),
            vec![
                "register:Ctrl+Shift+P",
                "unregister:Ctrl+Shift+P",
                "register:Ctrl+Alt+Space",
                "register:Ctrl+Shift+P",
            ]
        );
        assert!(bridge
            .status()
            .last_error
            .as_deref()
            .is_some_and(|message| message.contains("restored previous shortcut")));
    }

    #[test]
    fn maps_runtime_shortcuts_to_tauri_global_shortcuts() {
        let shortcut = global_shortcut_from_keyboard_shortcut(ctrl_alt_space_shortcut());

        assert!(shortcut.matches(
            GlobalShortcutModifiers::CONTROL | GlobalShortcutModifiers::ALT,
            GlobalShortcutCode::Space
        ));
    }

    fn bridge_with_components(
        runtime: OmniRuntimeState,
        registrar: Arc<RecordingGlobalShortcutRegistrar>,
    ) -> HotkeyBridge {
        HotkeyBridge::with_components(
            runtime,
            registrar,
            Arc::new(RecordingForwarder::default()),
            Arc::new(RecordingEventSink::default()),
            Arc::new(RecordingActivationHandler::default()),
            Arc::new(RecordingActiveProcessProvider::default()),
        )
    }

    #[derive(Default)]
    struct RecordingEventSink {
        events: Mutex<Vec<HotkeyEventPayloadDto>>,
    }

    impl RecordingEventSink {
        fn events(&self) -> Vec<HotkeyEventPayloadDto> {
            self.events.lock().expect("events should lock").clone()
        }
    }

    impl HotkeyEventSink for RecordingEventSink {
        fn emit_hotkey_event(&self, payload: HotkeyEventPayloadDto) -> Result<(), String> {
            self.events
                .lock()
                .expect("events should lock")
                .push(payload);
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingActivationHandler {
        count: Mutex<u64>,
        guide_activation_count: Mutex<u64>,
        guide_activation_result: bool,
    }

    impl RecordingActivationHandler {
        fn with_guide_activation() -> Self {
            Self {
                guide_activation_result: true,
                ..Default::default()
            }
        }

        fn activation_count(&self) -> u64 {
            *self.count.lock().expect("count should lock")
        }

        fn guide_activation_count(&self) -> u64 {
            *self
                .guide_activation_count
                .lock()
                .expect("guide count should lock")
        }
    }

    impl PaletteActivationHandler for RecordingActivationHandler {
        fn handle_palette_activation(&self, _context: ContextRoot) {
            *self.count.lock().expect("count should lock") += 1;
        }

        fn handle_guide_activation(&self) -> bool {
            *self
                .guide_activation_count
                .lock()
                .expect("guide count should lock") += 1;
            self.guide_activation_result
        }
    }

    #[derive(Default)]
    struct RecordingForwarder {
        shortcuts: Mutex<Vec<String>>,
    }

    impl RecordingForwarder {
        fn forwarded_shortcuts(&self) -> Vec<String> {
            self.shortcuts
                .lock()
                .expect("shortcuts should lock")
                .clone()
        }
    }

    impl HotkeyForwarder for RecordingForwarder {
        fn forward_shortcut(&self, shortcut: KeyboardShortcut) -> Result<(), String> {
            self.shortcuts
                .lock()
                .expect("shortcuts should lock")
                .push(shortcut.to_string());
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingGlobalShortcutRegistrar {
        calls: Mutex<Vec<String>>,
        fail_all_registers: Option<String>,
        fail_shortcut: Option<(KeyboardShortcut, String)>,
    }

    impl RecordingGlobalShortcutRegistrar {
        fn failing_register(message: &str) -> Self {
            Self {
                fail_all_registers: Some(message.to_string()),
                ..Default::default()
            }
        }

        fn failing_for_shortcut(shortcut: KeyboardShortcut, message: &str) -> Self {
            Self {
                fail_shortcut: Some((shortcut, message.to_string())),
                ..Default::default()
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().expect("calls should lock").clone()
        }
    }

    impl GlobalShortcutRegistrar for RecordingGlobalShortcutRegistrar {
        fn register_activation_shortcut(&self, shortcut: KeyboardShortcut) -> Result<(), String> {
            self.calls
                .lock()
                .expect("calls should lock")
                .push(format!("register:{shortcut}"));
            if let Some(message) = &self.fail_all_registers {
                return Err(message.clone());
            }
            if let Some((failing_shortcut, message)) = &self.fail_shortcut {
                if shortcut == *failing_shortcut {
                    return Err(message.clone());
                }
            }
            Ok(())
        }

        fn unregister_activation_shortcut(&self, shortcut: KeyboardShortcut) -> Result<(), String> {
            self.calls
                .lock()
                .expect("calls should lock")
                .push(format!("unregister:{shortcut}"));
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingActiveProcessProvider {
        process_name: Option<String>,
    }

    impl ActiveProcessProvider for RecordingActiveProcessProvider {
        fn active_window_context(&self) -> ActiveWindowContext {
            ActiveWindowContext {
                context: empty_context(),
                process_name: self.process_name.clone(),
            }
        }
    }

    fn empty_context() -> ContextRoot {
        ContextRoot {
            fg_context: Vec::new(),
            bg_context: Vec::new(),
            active_interaction: Default::default(),
        }
    }

    fn activation_shortcut() -> KeyboardShortcut {
        RuntimeConfig::default_activation_shortcut()
    }

    fn ctrl_alt_space_shortcut() -> KeyboardShortcut {
        KeyboardShortcut {
            modifier: HotkeyModifiers {
                control: true,
                alt: true,
                ..Default::default()
            },
            key: Key::Space,
        }
    }

    fn runtime_with_ignored_processes(process_names: &[&str]) -> OmniRuntimeState {
        let root = runtime_test_root("hotkey-bridge-ignored");
        let names = process_names
            .iter()
            .map(|name| format!("\"{name}\""))
            .collect::<Vec<_>>()
            .join(", ");
        std::fs::write(root.join("ignore.toml"), format!("windows = [{names}]"))
            .expect("ignore config should be written");

        OmniRuntimeState::load(RuntimeStateLoadOptions {
            bundled_extensions_root: root.clone(),
            user_extensions_root: None,
            dev_config_path: root.join("config.toml"),
            runtime_paths: RuntimePaths {
                config_path: None,
                local_cache_root: None,
            },
            current_os: Os::Windows,
        })
    }

    fn runtime_test_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let root = PathBuf::from("target")
            .join("tauri-hotkey-bridge-tests")
            .join(format!("{name}-{nanos}"));
        if root.exists() {
            std::fs::remove_dir_all(&root).expect("runtime test root should reset");
        }
        std::fs::create_dir_all(root.join("static")).expect("static dir should be created");
        root
    }
}
