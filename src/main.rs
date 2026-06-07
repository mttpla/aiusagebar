mod http;
mod icon;
mod keychain;
mod launch_at_login;
mod provider;

use icon::{IconKind, Icons};
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

struct MenuBuild {
    menu: Menu,
    refresh: tray_icon::menu::MenuId,
    quit: tray_icon::menu::MenuId,
}

fn append_label(menu: &Menu, text: impl Into<String>) {
    menu.append(&MenuItem::new(text.into(), false, None))
        .expect("menu append failed");
}

struct App {
    tray: tray_icon::TrayIcon,
    icons: Icons,
    id_quit: tray_icon::menu::MenuId,
    id_refresh: tray_icon::menu::MenuId,
    claude: ClaudeProvider,
}

impl App {
    fn build_menu(name: &str, state: &UsageState) -> MenuBuild {
        let menu = Menu::new();
        match state {
            UsageState::NotConfigured => append_label(&menu, format!("{}: not configured", name)),
            UsageState::Stale(msg) => append_label(&menu, format!("{} ⚠  {}", name, msg)),
            UsageState::Error(msg) => append_label(&menu, format!("{} ✕  {}", name, msg)),
            UsageState::Ok(windows) => {
                append_label(&menu, name.to_string());
                for w in windows {
                    let pct = w
                        .percent_used
                        .map(|p| format!("{:.1}%", p))
                        .unwrap_or_else(|| "∞".to_string());
                    let reset = w.resets_at.as_deref().unwrap_or("?");
                    append_label(&menu, format!("  {} — {}  resets {}", w.name, pct, reset));
                }
            }
        }
        let item_refresh = MenuItem::new("Refresh", true, None);
        let item_quit = MenuItem::new("Quit", true, None);
        menu.append(&item_refresh).expect("menu append failed");
        menu.append(&item_quit).expect("menu append failed");
        MenuBuild {
            refresh: item_refresh.id().clone(),
            quit: item_quit.id().clone(),
            menu,
        }
    }

    fn refresh(&mut self) {
        let state = self.claude.fetch();
        let build = Self::build_menu(self.claude.name(), &state);
        self.id_refresh = build.refresh;
        self.id_quit = build.quit;
        self.tray.set_menu(Some(Box::new(build.menu)));
        let kind = IconKind::for_state(&state);
        self.tray.set_icon(Some(self.icons.get(kind))).ok();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        self.refresh();
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Cocoa wakes the loop on tray/menu clicks; try_recv drains queued events.
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
    let icons = Icons::load();
    let claude = ClaudeProvider::new();
    let initial_state = UsageState::NotConfigured;
    let build = App::build_menu(claude.name(), &initial_state);

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(build.menu))
        .with_tooltip("AIUsageBar")
        .with_icon(icons.get(IconKind::for_state(&initial_state)))
        .build()
        .expect("failed to create tray icon");

    let mut app = App {
        tray,
        icons,
        id_quit: build.quit,
        id_refresh: build.refresh,
        claude,
    };
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
