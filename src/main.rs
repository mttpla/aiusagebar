mod http;
mod keychain;
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
                menu.append(&MenuItem::new(format!("{}: non configurato", name), false, None)).unwrap();
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
        let item_refresh = MenuItem::new("Aggiorna", true, None);
        let item_quit = MenuItem::new("Esci", true, None);
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

        if let Ok(tray_event) = TrayIconEvent::receiver().try_recv() {
            if let tray_icon::TrayIconEvent::Click {
                button: tray_icon::MouseButton::Left,
                ..
            } = tray_event
            {
                self.refresh();
            }
        }
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    set_accessory_policy();

    let event_loop = EventLoop::new().expect("Impossibile creare event loop");
    let icon = load_icon();
    let claude = ClaudeProvider::new();
    let (initial_menu, id_refresh, id_quit) = App::build_menu(claude.name(), &UsageState::NotConfigured);

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(initial_menu))
        .with_tooltip("AIUsageBar")
        .with_icon(icon)
        .build()
        .expect("Impossibile creare la tray icon");

    let mut app = App { tray, id_quit, id_refresh, claude };
    event_loop.run_app(&mut app).expect("Errore nell'event loop");
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

fn load_icon() -> tray_icon::Icon {
    let icon_path = std::path::Path::new("icons/app_icon.png");
    let (rgba, width, height) = if icon_path.exists() {
        let img = image::open(icon_path)
            .expect("Impossibile aprire icons/app_icon.png")
            .into_rgba8();
        let (w, h) = img.dimensions();
        (img.into_raw(), w, h)
    } else {
        eprintln!("icons/app_icon.png not found, using placeholder icon.");
        let size = 32u32;
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);
        for _ in 0..(size * size) {
            pixels.extend_from_slice(&[0xCC, 0x00, 0x00, 0xFF]);
        }
        (pixels, size, size)
    };
    tray_icon::Icon::from_rgba(rgba, width, height).expect("Impossibile creare l'icona")
}
