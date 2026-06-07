mod http;
mod keychain;
mod launch_at_login;
mod provider;

use provider::claude::ClaudeProvider;
use provider::{UsageProvider, UsageState};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder, TrayIconEvent,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;

#[derive(Clone, Copy, PartialEq, Debug)]
enum IconKind {
    Normal,
    Alert,
    Unavailable,
}

fn icon_for_state(state: &UsageState) -> IconKind {
    match state {
        UsageState::Ok(windows) => {
            if windows.iter().any(|w| w.percent_used.unwrap_or(0.0) >= 80.0) {
                IconKind::Alert
            } else {
                IconKind::Normal
            }
        }
        _ => IconKind::Unavailable,
    }
}

struct App {
    tray: tray_icon::TrayIcon,
    id_quit: tray_icon::menu::MenuId,
    id_refresh: tray_icon::menu::MenuId,
    claude: ClaudeProvider,
}

impl App {
    fn build_menu(name: &str, state: &UsageState) -> (Menu, tray_icon::menu::MenuId, tray_icon::menu::MenuId) {
        let menu = Menu::new();
        match state {
            UsageState::NotConfigured => {
                menu.append(&MenuItem::new(format!("{}: not configured", name), false, None)).unwrap();
            }
            UsageState::Stale(msg) => {
                menu.append(&MenuItem::new(format!("{} ⚠  {}", name, msg), false, None))
                    .unwrap();
            }
            UsageState::Error(msg) => {
                menu.append(&MenuItem::new(format!("{} ✕  {}", name, msg), false, None))
                    .unwrap();
            }
            UsageState::Ok(windows) => {
                menu.append(&MenuItem::new(name, false, None)).unwrap();
                for w in windows {
                    let pct = w
                        .percent_used
                        .map(|p| format!("{:.1}%", p))
                        .unwrap_or_else(|| "∞".to_string());
                    let reset = w.resets_at.as_deref().unwrap_or("?");
                    menu.append(&MenuItem::new(
                        format!("  {} — {}  resets {}", w.name, pct, reset),
                        false,
                        None,
                    ))
                    .unwrap();
                }
            }
        }
        let item_refresh = MenuItem::new("Refresh", true, None);
        let item_quit = MenuItem::new("Quit", true, None);
        menu.append(&item_refresh).unwrap();
        menu.append(&item_quit).unwrap();
        let id_refresh = item_refresh.id().clone();
        let id_quit = item_quit.id().clone();
        (menu, id_refresh, id_quit)
    }

    fn refresh(&mut self) {
        let state = self.claude.fetch();
        let (menu, id_refresh, id_quit) = Self::build_menu(self.claude.name(), &state);
        self.id_refresh = id_refresh;
        self.id_quit = id_quit;
        self.tray.set_menu(Some(Box::new(menu)));
        let icon_bytes = match icon_for_state(&state) {
            IconKind::Normal => ICON_NORMAL,
            IconKind::Alert => ICON_ALERT,
            IconKind::Unavailable => ICON_UNAVAILABLE,
        };
        self.tray.set_icon(Some(parse_icon(icon_bytes))).ok();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        self.refresh();
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);

        if let Ok(ev) = MenuEvent::receiver().try_recv() {
            if ev.id == self.id_quit {
                event_loop.exit();
            } else if ev.id == self.id_refresh {
                self.refresh();
            }
        }

        // Drain tray events; no action needed — menu shows automatically on click.
        let _ = TrayIconEvent::receiver().try_recv();
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    set_accessory_policy();

    if let Err(e) = launch_at_login::enable() {
        eprintln!("[launch_at_login] {e}");
    }

    let event_loop = EventLoop::new().expect("failed to create event loop");
    let icon = parse_icon(ICON_UNAVAILABLE);
    let claude = ClaudeProvider::new();
    let (initial_menu, id_refresh, id_quit) = App::build_menu(claude.name(), &UsageState::NotConfigured);

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(initial_menu))
        .with_tooltip("AIUsageBar")
        .with_icon(icon)
        .build()
        .expect("failed to create tray icon");

    let mut app = App { tray, id_quit, id_refresh, claude };
    event_loop.run_app(&mut app).expect("event loop error");
}

#[cfg(target_os = "macos")]
fn set_accessory_policy() {
    use objc2::runtime::AnyClass;
    unsafe {
        let cls = AnyClass::get("NSApplication").unwrap();
        let app: *mut objc2::runtime::AnyObject = objc2::msg_send![cls, sharedApplication];
        let _: bool = objc2::msg_send![app, setActivationPolicy: 1_i64];
    }
}

static ICON_NORMAL: &[u8] = include_bytes!("../icons/brain_normal.png");
static ICON_ALERT: &[u8] = include_bytes!("../icons/brain_alert.png");
static ICON_UNAVAILABLE: &[u8] = include_bytes!("../icons/brain_unavailable.png");

fn parse_icon(bytes: &[u8]) -> tray_icon::Icon {
    let img = image::load_from_memory(bytes)
        .expect("failed to decode icon")
        .into_rgba8();
    let (w, h) = img.dimensions();
    tray_icon::Icon::from_rgba(img.into_raw(), w, h).expect("failed to create icon")
}

#[cfg(test)]
mod tests {
    use super::*;
    use provider::{LimitWindow, UsageState};

    fn window(pct: Option<f32>) -> LimitWindow {
        LimitWindow {
            name: "test".into(),
            percent_used: pct,
            limit: None,
            remaining: None,
            resets_at: None,
            unlimited: false,
        }
    }

    #[test]
    fn icon_normal_when_all_under_threshold() {
        let state = UsageState::Ok(vec![window(Some(50.0)), window(Some(79.9))]);
        assert_eq!(icon_for_state(&state), IconKind::Normal);
    }

    #[test]
    fn icon_alert_when_any_at_threshold() {
        let state = UsageState::Ok(vec![window(Some(50.0)), window(Some(80.0))]);
        assert_eq!(icon_for_state(&state), IconKind::Alert);
    }

    #[test]
    fn icon_alert_when_any_over_threshold() {
        let state = UsageState::Ok(vec![window(Some(95.0))]);
        assert_eq!(icon_for_state(&state), IconKind::Alert);
    }

    #[test]
    fn icon_unavailable_on_error() {
        assert_eq!(icon_for_state(&UsageState::Error("e".into())), IconKind::Unavailable);
    }

    #[test]
    fn icon_unavailable_on_stale() {
        assert_eq!(icon_for_state(&UsageState::Stale("s".into())), IconKind::Unavailable);
    }

    #[test]
    fn icon_unavailable_on_not_configured() {
        assert_eq!(icon_for_state(&UsageState::NotConfigured), IconKind::Unavailable);
    }

    #[test]
    fn icon_normal_when_percent_unknown() {
        // None percent_used (unlimited window) → treated as 0.0 → Normal
        let state = UsageState::Ok(vec![window(None)]);
        assert_eq!(icon_for_state(&state), IconKind::Normal);
    }
}
