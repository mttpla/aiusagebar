use tray_icon::menu::{Menu, MenuId, MenuItem};
use crate::provider::{LimitWindow, ProviderKind, UsageState};

pub mod base;
pub mod claude;
pub mod copilot;
pub(crate) mod time;

#[cfg(target_os = "macos")]
pub(crate) mod styled;

pub struct MenuBuild {
    pub menu: Menu,
    pub about: MenuId,
    pub refresh: MenuId,
    pub quit: MenuId,
}

pub(crate) struct MenuLayout {
    pub header_indices: Vec<(usize, ProviderKind)>,
    pub window_items: Vec<(usize, LimitWindow)>,
    pub refresh_idx: usize,
    pub quit_idx: usize,
    pub last_updated: Option<String>,
}

/// Pure index-tracking function — does NOT build the actual Menu.
/// Uses section_item_count from claude/copilot modules to count items per section.
pub(crate) fn build_layout(
    states: &[(ProviderKind, &UsageState)],
    last_updated: Option<&str>,
) -> MenuLayout {
    let mut idx: usize = 0;
    let mut header_indices: Vec<(usize, ProviderKind)> = Vec::new();
    let mut window_items: Vec<(usize, LimitWindow)> = Vec::new();

    for (kind, state) in states {
        header_indices.push((idx, *kind));
        if let UsageState::Ok(windows, _) = state {
            for (i, w) in windows.iter().enumerate() {
                window_items.push((idx + 1 + i, w.clone()));
            }
        }
        idx += match kind {
            ProviderKind::Claude => claude::section_item_count(state),
            ProviderKind::Copilot => copilot::section_item_count(state),
        };
    }

    // Footer layout: Refresh(idx), separator(idx+1), About(idx+2), Quit(idx+3)
    MenuLayout {
        header_indices,
        window_items,
        refresh_idx: idx,
        quit_idx: idx + 3,
        last_updated: last_updated.map(str::to_owned),
    }
}

pub(crate) fn append_label(menu: &Menu, text: impl Into<String>) {
    menu.append(&MenuItem::new(text.into(), false, None))
        .expect("menu append failed");
}

pub fn build_menu(states: &[(ProviderKind, &UsageState)], last_updated: Option<&str>) -> MenuBuild {
    let menu = Menu::new();
    for (kind, state) in states {
        match kind {
            ProviderKind::Claude => { let _ = claude::append_claude_section(&menu, state); }
            ProviderKind::Copilot => { let _ = copilot::append_copilot_section(&menu, state); }
        }
    }
    let footer = base::append_footer(&menu);
    let layout = build_layout(states, last_updated);

    #[cfg(target_os = "macos")]
    styled::style_menu(&menu, &layout);

    #[cfg(not(target_os = "macos"))]
    let _ = layout;

    MenuBuild {
        menu,
        about: footer.about,
        refresh: footer.refresh,
        quit: footer.quit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{LimitWindow, UsageState};

    #[test]
    fn menu_layout_indices_no_providers() {
        // Refresh(0) + sep(1) + About(2) + Quit(3)
        let layout = build_layout(&[], None);
        assert_eq!(layout.refresh_idx, 0);
        assert_eq!(layout.quit_idx, 3);
        assert!(layout.header_indices.is_empty());
    }

    #[test]
    fn menu_layout_indices_claude_two_windows() {
        let state = UsageState::Ok(
            vec![
                LimitWindow { name: "d".into(), ..Default::default() },
                LimitWindow { name: "m".into(), ..Default::default() },
            ],
            Some("max".into()),
        );
        let layout = build_layout(&[(ProviderKind::Claude, &state)], None);
        assert_eq!(layout.header_indices[0].0, 0);
        assert_eq!(layout.refresh_idx, 3);
        assert_eq!(layout.quit_idx, 6);
    }

    #[test]
    fn build_layout_claude_window_items_indices() {
        let state = UsageState::Ok(
            vec![
                LimitWindow { name: "5h session".into(), percent_used: Some(39.0), ..Default::default() },
                LimitWindow { name: "7d weekly".into(), percent_used: Some(15.0), ..Default::default() },
            ],
            Some("max".into()),
        );
        let layout = build_layout(&[(ProviderKind::Claude, &state)], None);
        assert_eq!(layout.window_items.len(), 2);
        assert_eq!(layout.window_items[0].0, 1);
        assert_eq!(layout.window_items[1].0, 2);
        assert_eq!(layout.window_items[0].1.name, "5h session");
        assert_eq!(layout.window_items[1].1.name, "7d weekly");
    }

    #[test]
    fn build_layout_copilot_window_items_indices() {
        use crate::provider::LimitWindow;
        let claude_state = UsageState::Ok(
            vec![
                LimitWindow { name: "5h session".into(), ..Default::default() },
                LimitWindow { name: "7d weekly".into(), ..Default::default() },
            ],
            Some("max".into()),
        );
        let copilot_state = UsageState::Ok(
            vec![LimitWindow { name: "monthly".into(), ..Default::default() }],
            None,
        );
        let layout = build_layout(
            &[(ProviderKind::Claude, &claude_state), (ProviderKind::Copilot, &copilot_state)],
            None,
        );
        assert_eq!(layout.window_items.len(), 3);
        assert_eq!(layout.window_items[0].0, 1);
        assert_eq!(layout.window_items[1].0, 2);
        assert_eq!(layout.window_items[2].0, 4);
        assert_eq!(layout.window_items[2].1.name, "monthly");
        assert_eq!(layout.refresh_idx, 5);
        assert_eq!(layout.quit_idx, 8);
    }

    #[test]
    fn build_layout_non_ok_state_no_window_items() {
        let layout = build_layout(&[(ProviderKind::Claude, &UsageState::NotConfigured)], None);
        assert!(layout.window_items.is_empty());
    }
}
