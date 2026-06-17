mod http;
mod icon;
mod keychain;
mod launch_at_login;
mod settings;
mod version;
mod about;
mod provider;
mod ui;
mod update_check;

use std::time::Instant;
use chrono::{DateTime, Local};
use icon::{IconKind, Icons};
use settings::Settings;
use provider::claude::ClaudeProvider;
use provider::copilot::CopilotProvider;
use provider::{ProviderKind, UsageProvider, UsageState};
use tray_icon::{
    menu::MenuEvent,
    TrayIconBuilder, TrayIconEvent,
};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
#[cfg(target_os = "macos")]
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
use winit::window::WindowId;

struct App {
    tray: tray_icon::TrayIcon,
    icons: Icons,
    id_about: tray_icon::menu::MenuId,
    id_quit: tray_icon::menu::MenuId,
    id_refresh: tray_icon::menu::MenuId,
    id_update: Option<tray_icon::menu::MenuId>,
    id_setup_claude: Option<tray_icon::menu::MenuId>,
    id_setup_copilot: Option<tray_icon::menu::MenuId>,
    providers: Vec<Box<dyn UsageProvider>>,
    last_refreshed_at: Option<DateTime<Local>>,
    settings: Settings,
    next_poll_at: Instant,
    next_update_check_after: DateTime<Local>,
    update_available: Option<String>,
}

impl App {
    fn refresh(&mut self) {
        let states: Vec<(ProviderKind, UsageState)> = self.providers
            .iter()
            .map(|p| (p.kind(), p.fetch()))
            .collect();

        let state_refs: Vec<&UsageState> = states.iter().map(|(_, s)| s).collect();
        let icon_kind = IconKind::for_providers(&state_refs, self.settings.alert_threshold_pct);

        let refs: Vec<(ProviderKind, &UsageState)> =
            states.iter().map(|(k, s)| (*k, s)).collect();
        let now = Local::now();
        let updated = now.format("%H:%M").to_string();
        let build = ui::build_menu(&refs, Some(&updated), self.update_available.as_deref());
        self.id_about = build.about;
        self.id_refresh = build.refresh;
        self.id_quit = build.quit;
        self.id_update = build.update;
        self.id_setup_claude = build.setup_claude;
        self.id_setup_copilot = build.setup_copilot;
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

        if Local::now() >= self.next_update_check_after {
            self.update_available = update_check::check();
            self.next_update_check_after = Local::now() + chrono::Duration::hours(24);
            if !did_refresh {
                self.refresh();
                did_refresh = true;
            }
        }

        if let Ok(ev) = MenuEvent::receiver().try_recv() {
            if ev.id == self.id_quit {
                event_loop.exit();
            } else if ev.id == self.id_about {
                about::show();
            } else if ev.id == self.id_refresh && !did_refresh {
                self.refresh();
                self.next_poll_at = Instant::now() + self.settings.poll_interval;
            } else if self.id_update.as_ref().is_some_and(|id| ev.id == *id) {
                let _ = std::process::Command::new("open")
                    .arg("https://github.com/mttpla/aiusagebar/releases/latest")
                    .spawn();
            } else if self.id_setup_claude.as_ref().is_some_and(|id| ev.id == *id) {
                // Setup Claude handler
            } else if self.id_setup_copilot.as_ref().is_some_and(|id| ev.id == *id) {
                // Setup Copilot handler
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
    let initial_refs: Vec<(ProviderKind, &UsageState)> = providers
        .iter()
        .map(|p| (p.kind(), &initial_state))
        .collect();
    let build = ui::build_menu(&initial_refs, None, None);

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
        id_about: build.about,
        id_quit: build.quit,
        id_refresh: build.refresh,
        id_update: build.update,
        id_setup_claude: build.setup_claude,
        id_setup_copilot: build.setup_copilot,
        providers,
        last_refreshed_at: None,
        settings,
        next_poll_at,
        next_update_check_after: Local::now() + chrono::Duration::hours(24),
        update_available: None,
    };
    event_loop.run_app(&mut app).expect("event loop error");
}
