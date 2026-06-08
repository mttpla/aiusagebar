mod http;
mod icon;
mod keychain;
mod launch_at_login;
mod provider;

use icon::{IconKind, Icons};
use provider::claude::ClaudeProvider;
use provider::copilot::CopilotProvider;
use provider::{UsageProvider, UsageState};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIconBuilder, TrayIconEvent,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
#[cfg(target_os = "macos")]
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
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
    providers: Vec<Box<dyn UsageProvider>>,
}

impl App {
    fn build_menu(states: &[(&str, &UsageState)]) -> MenuBuild {
        let menu = Menu::new();
        for (name, state) in states {
            match state {
                UsageState::NotConfigured => {
                    append_label(&menu, format!("{}: not configured", name));
                }
                UsageState::Stale(msg) => {
                    append_label(&menu, format!("{} ⚠  {}", name, msg));
                }
                UsageState::Error(msg) => {
                    append_label(&menu, format!("{} ✕  {}", name, msg));
                }
                UsageState::Ok(windows) => {
                    append_label(&menu, name.to_string());
                    for w in windows {
                        let pct = w
                            .percent_used
                            .map(|p| format!("{:.1}%", p))
                            .unwrap_or_else(|| "—".to_string());
                        let reset = w.resets_at.as_deref().unwrap_or("?");
                        append_label(
                            &menu,
                            format!("  {} — {}  resets {}", w.name, pct, reset),
                        );
                    }
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
        let states: Vec<(&str, UsageState)> = self.providers
            .iter()
            .map(|p| (p.name(), p.fetch()))
            .collect();

        let state_refs: Vec<&UsageState> = states.iter().map(|(_, s)| s).collect();
        let icon_kind = IconKind::for_providers(&state_refs);

        let refs: Vec<(&str, &UsageState)> =
            states.iter().map(|(n, s)| (*n, s)).collect();
        let build = Self::build_menu(&refs);
        self.id_refresh = build.refresh;
        self.id_quit = build.quit;
        self.tray.set_menu(Some(Box::new(build.menu)));
        self.tray.set_icon(Some(self.icons.get(icon_kind))).ok();
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

        let _ = TrayIconEvent::receiver().try_recv();
    }
}

fn main() {
    if let Err(e) = launch_at_login::enable() {
        eprintln!("[launch_at_login] {e}");
    }

    let providers: Vec<Box<dyn UsageProvider>> = vec![
        Box::new(ClaudeProvider::new()),
        Box::new(CopilotProvider::new()),
    ];

    #[cfg(target_os = "macos")]
    let event_loop = EventLoop::builder()
        .with_activation_policy(ActivationPolicy::Accessory)
        .build()
        .expect("failed to create event loop");
    #[cfg(not(target_os = "macos"))]
    let event_loop = EventLoop::new().expect("failed to create event loop");
    let icons = Icons::load();

    let initial_state = UsageState::NotConfigured;
    let initial_refs: Vec<(&str, &UsageState)> = providers
        .iter()
        .map(|p| (p.name(), &initial_state))
        .collect();
    let build = App::build_menu(&initial_refs);

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(build.menu))
        .with_tooltip("AIUsageBar")
        .with_icon(icons.get(IconKind::Unavailable))
        .build()
        .expect("failed to create tray icon");

    let mut app = App {
        tray,
        icons,
        id_quit: build.quit,
        id_refresh: build.refresh,
        providers,
    };
    event_loop.run_app(&mut app).expect("event loop error");
}
