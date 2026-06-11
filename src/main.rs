mod http;
mod icon;
mod keychain;
mod launch_at_login;
mod settings;
mod provider;

use std::time::Instant;
use chrono::{DateTime, Local};
use icon::{IconKind, Icons};
use settings::Settings;
use provider::claude::ClaudeProvider;
use provider::copilot::CopilotProvider;
use provider::{UsageProvider, UsageState};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
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
    last_refreshed_at: Option<DateTime<Local>>,
    settings: Settings,
    next_poll_at: Instant,
}

impl App {
    fn build_menu(states: &[(&str, &UsageState)], last_updated: Option<&str>) -> MenuBuild {
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
        if let Some(ts) = last_updated {
            // TODO: i18n
            append_label(&menu, format!("Updated: {}", ts));
            menu.append(&PredefinedMenuItem::separator())
                .expect("menu append failed");
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
        let icon_kind = IconKind::for_providers(&state_refs, self.settings.alert_threshold_pct);

        let refs: Vec<(&str, &UsageState)> =
            states.iter().map(|(n, s)| (*n, s)).collect();
        let now = Local::now();
        let updated = now.format("%H:%M").to_string();
        let build = Self::build_menu(&refs, Some(&updated));
        self.id_refresh = build.refresh;
        self.id_quit = build.quit;
        self.tray.set_menu(Some(Box::new(build.menu)));
        self.tray.set_icon(Some(self.icons.get(icon_kind))).ok();
        self.last_refreshed_at = Some(now);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        self.refresh();
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let mut did_refresh = false;
        if now >= self.next_poll_at {
            self.refresh();
            self.next_poll_at = now + self.settings.poll_interval;
            did_refresh = true;
        }

        if let Ok(ev) = MenuEvent::receiver().try_recv() {
            if ev.id == self.id_quit {
                event_loop.exit();
            } else if ev.id == self.id_refresh && !did_refresh {
                self.refresh();
                self.next_poll_at = Instant::now() + self.settings.poll_interval;
            }
        }

        let _ = TrayIconEvent::receiver().try_recv();
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_poll_at));
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
    let build = App::build_menu(&initial_refs, None);

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(build.menu))
        .with_tooltip("AIUsageBar")
        .with_icon(icons.get(IconKind::Unavailable))
        .build()
        .expect("failed to create tray icon");

    let settings = Settings::default();
    let next_poll_at = Instant::now() + settings.poll_interval;

    let mut app = App {
        tray,
        icons,
        id_quit: build.quit,
        id_refresh: build.refresh,
        providers,
        last_refreshed_at: None,
        settings,
        next_poll_at,
    };
    event_loop.run_app(&mut app).expect("event loop error");
}
