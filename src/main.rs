mod backoff;
mod diag;
mod clipboard;
mod details;
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
use backoff::BackoffState;
use chrono::{DateTime, Local};
use http::HttpError;
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

const CLAUDE_SETUP_URL: &str =
    "https://github.com/mttpla/aiusagebar/blob/master/claude-setup.md";
const COPILOT_SETUP_URL: &str =
    "https://github.com/mttpla/aiusagebar/blob/master/copilot-setup.md";

/// True when any provider in the batch returned a 429/5xx — the only outcomes
/// that extend the global backoff. All other outcomes (network/parse error,
/// `Unauthorized`, `NotConfigured`) advance the timer normally via `on_success`.
fn should_back_off(http_errs: &[Option<HttpError>]) -> bool {
    http_errs
        .iter()
        .any(|e| matches!(e, Some(HttpError::RateLimited | HttpError::ServerError(_))))
}

struct App {
    tray: tray_icon::TrayIcon,
    icons: Icons,
    id_about: tray_icon::menu::MenuId,
    id_quit: tray_icon::menu::MenuId,
    id_refresh: tray_icon::menu::MenuId,
    id_update: Option<tray_icon::menu::MenuId>,
    id_setup_claude: Option<tray_icon::menu::MenuId>,
    id_setup_copilot: Option<tray_icon::menu::MenuId>,
    id_details_claude: Option<tray_icon::menu::MenuId>,
    id_details_copilot: Option<tray_icon::menu::MenuId>,
    id_copy_diag: Option<tray_icon::menu::MenuId>,
    providers: Vec<Box<dyn UsageProvider>>,
    last_refreshed_at: Option<DateTime<Local>>,
    settings: Settings,
    backoff: BackoffState,
    next_update_check_after: DateTime<Local>,
    update_available: Option<String>,
}

impl App {
    fn refresh_all(&mut self, force: bool) {
        if !force && !self.backoff.is_allowed() {
            return;
        }
        let count = self.providers.len();
        let mut states: Vec<(ProviderKind, UsageState)> = Vec::with_capacity(count);
        let mut http_errs: Vec<Option<HttpError>> = Vec::with_capacity(count);
        for i in 0..count {
            let kind = self.providers[i].kind();
            let (state, http_err) = self.providers[i].fetch_with_http_error();
            if let Some(msg) = crate::provider::state_diag_message(kind.display_name(), &state) {
                crate::diag!(crate::diag::Level::Err, "{}", msg);
            }
            states.push((kind, state));
            http_errs.push(http_err);
        }
        if should_back_off(&http_errs) {
            self.backoff.on_error();
            let reasons: Vec<String> = states
                .iter()
                .zip(&http_errs)
                .filter_map(|((kind, _), err)| match err {
                    Some(HttpError::RateLimited) => Some(format!("{} 429", kind.display_name())),
                    Some(HttpError::ServerError(c)) => Some(format!("{} HTTP {c}", kind.display_name())),
                    _ => None,
                })
                .collect();
            crate::diag!(
                crate::diag::Level::Err,
                "Backoff extended to {}s after {}",
                self.backoff.current_interval().as_secs(),
                reasons.join(", ")
            );
        } else {
            self.backoff.on_success();
        }
        let state_refs: Vec<&UsageState> = states.iter().map(|(_, s)| s).collect();
        let icon_kind = IconKind::for_providers(&state_refs, self.settings.alert_threshold_pct);
        let refs: Vec<(ProviderKind, &UsageState)> =
            states.iter().map(|(k, s)| (*k, s)).collect();
        let details_kinds: Vec<ProviderKind> = refs
            .iter()
            .map(|(k, _)| *k)
            .filter(|k| {
                self.providers
                    .iter()
                    .any(|p| p.kind() == *k && p.raw_json().is_some())
            })
            .collect();
        let now = Local::now();
        let updated = now.format("%H:%M").to_string();
        let build = ui::build_menu(
            &refs,
            Some(&updated),
            self.update_available.as_deref(),
            &details_kinds,
        );
        self.id_about = build.about;
        self.id_refresh = build.refresh;
        self.id_quit = build.quit;
        self.id_update = build.update;
        self.id_setup_claude = build.setup_claude;
        self.id_setup_copilot = build.setup_copilot;
        self.id_details_claude = build.details_claude;
        self.id_details_copilot = build.details_copilot;
        self.id_copy_diag = build.copy_diag;
        self.tray.set_menu(Some(Box::new(build.menu)));
        if let Err(e) = self.tray.set_icon(Some(self.icons.get(icon_kind))) {
            crate::diag!(crate::diag::Level::Err, "Tray set_icon failed: {}", e);
        }
        self.last_refreshed_at = Some(now);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        self.refresh_all(true);
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let mut did_refresh = false;
        if self.backoff.is_allowed() {
            self.refresh_all(false);
            did_refresh = true;
        }

        if Local::now() >= self.next_update_check_after {
            self.update_available = update_check::check();
            self.next_update_check_after = Local::now() + chrono::Duration::hours(24);
            if !did_refresh {
                self.refresh_all(false);
                did_refresh = true;
            }
        }

        if let Ok(ev) = MenuEvent::receiver().try_recv() {
            if ev.id == self.id_quit {
                event_loop.exit();
            } else if ev.id == self.id_about {
                about::show();
            } else if ev.id == self.id_refresh && !did_refresh {
                self.refresh_all(true);
            } else if self.id_update.as_ref().is_some_and(|id| ev.id == *id) {
                if let Err(e) = std::process::Command::new("open")
                    .arg("https://github.com/mttpla/aiusagebar/releases/latest")
                    .spawn()
                {
                    crate::diag!(crate::diag::Level::Err, "Failed to open releases page: {}", e);
                }
            } else if self.id_setup_claude.as_ref().is_some_and(|id| ev.id == *id) {
                if let Err(e) = std::process::Command::new("open").arg(CLAUDE_SETUP_URL).spawn() {
                    crate::diag!(crate::diag::Level::Err, "Failed to open {}: {}", CLAUDE_SETUP_URL, e);
                }
            } else if self.id_setup_copilot.as_ref().is_some_and(|id| ev.id == *id) {
                if let Err(e) = std::process::Command::new("open").arg(COPILOT_SETUP_URL).spawn() {
                    crate::diag!(crate::diag::Level::Err, "Failed to open {}: {}", COPILOT_SETUP_URL, e);
                }
            } else if self.id_details_claude.as_ref().is_some_and(|id| ev.id == *id) {
                let raw = self.providers.iter()
                    .find(|p| p.kind() == crate::provider::ProviderKind::Claude)
                    .and_then(|p| p.raw_json());
                crate::details::show("Claude", raw.as_deref());
            } else if self.id_details_copilot.as_ref().is_some_and(|id| ev.id == *id) {
                let raw = self.providers.iter()
                    .find(|p| p.kind() == crate::provider::ProviderKind::Copilot)
                    .and_then(|p| p.raw_json());
                crate::details::show("Copilot", raw.as_deref());
            } else if self.id_copy_diag.as_ref().is_some_and(|id| ev.id == *id) {
                crate::clipboard::copy(&crate::diag::format_all());
            }
        }

        let _ = TrayIconEvent::receiver().try_recv();
        let next_provider = self.backoff.next_allowed_at();
        let update_deadline = self.next_update_check_after
            .signed_duration_since(Local::now())
            .to_std()
            .map(|d| Instant::now() + d)
            .unwrap_or_else(|_| Instant::now());
        event_loop.set_control_flow(ControlFlow::WaitUntil(next_provider.min(update_deadline)));
    }
}

fn main() {
    if let Err(e) = launch_at_login::enable() {
        crate::diag!(crate::diag::Level::Err, "launch_at_login enable failed: {}", e);
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
    let build = ui::build_menu(&initial_refs, None, None, &[]);

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(build.menu))
        .with_tooltip("AIUsageBar")
        .with_icon(icons.get(IconKind::Unavailable))
        .build()
        .expect("failed to create tray icon");

    let settings = Settings::default();
    let backoff = BackoffState::new(
        settings.poll_interval,
        settings.backoff_factor,
        settings.backoff_cap,
    );

    let mut app = App {
        tray,
        icons,
        id_about: build.about,
        id_quit: build.quit,
        id_refresh: build.refresh,
        id_update: build.update,
        id_setup_claude: build.setup_claude,
        id_setup_copilot: build.setup_copilot,
        id_details_claude: build.details_claude,
        id_details_copilot: build.details_copilot,
        id_copy_diag: build.copy_diag,
        providers,
        last_refreshed_at: None,
        settings,
        backoff,
        next_update_check_after: Local::now() + chrono::Duration::hours(24),
        update_available: None,
    };
    event_loop.run_app(&mut app).expect("event loop error");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_back_off_empty_is_false() {
        assert!(!should_back_off(&[]));
    }

    #[test]
    fn should_back_off_all_none_is_false() {
        assert!(!should_back_off(&[None, None]));
    }

    #[test]
    fn should_back_off_rate_limited_is_true() {
        assert!(should_back_off(&[None, Some(HttpError::RateLimited)]));
    }

    #[test]
    fn should_back_off_server_error_is_true() {
        assert!(should_back_off(&[Some(HttpError::ServerError(503))]));
    }

    #[test]
    fn should_back_off_unauthorized_and_other_is_false() {
        assert!(!should_back_off(&[
            Some(HttpError::Unauthorized),
            Some(HttpError::Other("dns".into())),
        ]));
    }
}
