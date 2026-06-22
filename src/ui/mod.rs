use tray_icon::menu::{Menu, MenuId, MenuItem};
use crate::provider::{LimitWindow, ProviderKind, UsageState};

pub(crate) mod base;
pub(crate) mod claude;
pub(crate) mod copilot;
pub(crate) mod time;

#[cfg(target_os = "macos")]
pub(crate) mod styled;

pub(crate) struct MenuBuild {
    pub(crate) menu: Menu,
    pub(crate) about: MenuId,
    pub(crate) refresh: MenuId,
    pub(crate) quit: MenuId,
    pub(crate) update: Option<MenuId>,
    pub(crate) setup_claude: Option<MenuId>,
    pub(crate) setup_copilot: Option<MenuId>,
    pub(crate) details_claude: Option<MenuId>,
    pub(crate) details_copilot: Option<MenuId>,
    pub(crate) copy_diag: Option<MenuId>,
}

pub(crate) struct MenuLayout {
    pub(crate) header_indices: Vec<(usize, ProviderKind)>,
    pub(crate) window_items: Vec<(usize, LimitWindow)>,
    pub(crate) refresh_idx: usize,
    pub(crate) quit_idx: usize,
    pub(crate) last_updated: Option<String>,
}

/// Pure index-tracking function — does NOT build the actual Menu.
/// Uses section_item_count from claude/copilot modules to count items per section.
pub(crate) fn build_layout(
    states: &[(ProviderKind, &UsageState)],
    last_updated: Option<&str>,
    update: Option<&str>,
) -> MenuLayout {
    let mut idx: usize = if update.is_some() { 2 } else { 0 };
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

    // "Other" submenu (always present) sits at `idx`; the footer follows it.
    // Footer: Refresh(idx+1), separator(idx+2), About(idx+3), Quit(idx+4)
    MenuLayout {
        header_indices,
        window_items,
        refresh_idx: idx + 1,
        quit_idx: idx + 4,
        last_updated: last_updated.map(str::to_owned),
    }
}

pub(crate) fn append_label(menu: &Menu, text: impl Into<String>) {
    menu.append(&MenuItem::new(text.into(), false, None))
        .expect("menu append failed");
}

pub(crate) fn build_menu(
    states: &[(ProviderKind, &UsageState)],
    last_updated: Option<&str>,
    update: Option<&str>,
    details_kinds: &[ProviderKind],
) -> MenuBuild {
    let menu = Menu::new();

    use tray_icon::menu::PredefinedMenuItem;
    let update_id: Option<MenuId> = if let Some(version) = update {
        let item = MenuItem::new(format!("↑ Update available {}", version), true, None);
        let id = item.id().clone();
        menu.append(&item).expect("menu append failed");
        menu.append(&PredefinedMenuItem::separator()).expect("menu append failed");
        Some(id)
    } else {
        None
    };

    let mut setup_claude: Option<MenuId> = None;
    let mut setup_copilot: Option<MenuId> = None;
    for (kind, state) in states {
        match kind {
            ProviderKind::Claude => {
                let (sc, _dc) = claude::append_claude_section(&menu, state);
                setup_claude = sc;
            }
            ProviderKind::Copilot => {
                let (sc, _dc) = copilot::append_copilot_section(&menu, state);
                setup_copilot = sc;
            }
        }
    }
    let other = base::append_other(&menu, details_kinds);
    let footer = base::append_footer(&menu);
    let layout = build_layout(states, last_updated, update);

    #[cfg(target_os = "macos")]
    styled::style_menu(&menu, &layout);

    #[cfg(not(target_os = "macos"))]
    let _ = layout;

    MenuBuild {
        menu,
        about: footer.about,
        refresh: footer.refresh,
        quit: footer.quit,
        update: update_id,
        setup_claude,
        setup_copilot,
        details_claude: other.details_claude,
        details_copilot: other.details_copilot,
        copy_diag: other.copy_diag,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{LimitWindow, UsageState};

    #[test]
    fn menu_layout_indices_no_providers() {
        // Other(0) + Refresh(1) + sep(2) + About(3) + Quit(4)
        let layout = build_layout(&[], None, None);
        assert_eq!(layout.refresh_idx, 1);
        assert_eq!(layout.quit_idx, 4);
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
        let layout = build_layout(&[(ProviderKind::Claude, &state)], None, None);
        assert_eq!(layout.header_indices[0].0, 0);
        assert_eq!(layout.refresh_idx, 5);
        assert_eq!(layout.quit_idx, 8);
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
        let layout = build_layout(&[(ProviderKind::Claude, &state)], None, None);
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
            None,
        );
        assert_eq!(layout.window_items.len(), 3);
        assert_eq!(layout.window_items[0].0, 1);
        assert_eq!(layout.window_items[1].0, 2);
        assert_eq!(layout.window_items[2].0, 5);
        assert_eq!(layout.window_items[2].1.name, "monthly");
        assert_eq!(layout.refresh_idx, 8);
        assert_eq!(layout.quit_idx, 11);
    }

    #[test]
    fn build_layout_non_ok_state_no_window_items() {
        let layout = build_layout(&[(ProviderKind::Claude, &UsageState::NotConfigured)], None, None);
        assert!(layout.window_items.is_empty());
    }

    #[test]
    fn build_layout_with_update_shifts_all_indices_by_2() {
        let state = UsageState::Ok(
            vec![LimitWindow { name: "d".into(), ..Default::default() }],
            Some("max".into()),
        );
        let layout = build_layout(&[(ProviderKind::Claude, &state)], None, Some("0.4.0"));
        // header was at 0 without update, now at 2
        assert_eq!(layout.header_indices[0].0, 2);
        // window item was at 1, now at 3
        assert_eq!(layout.window_items[0].0, 3);
        // refresh was at 3 without Other/update; +1 Other +2 update = 6
        assert_eq!(layout.refresh_idx, 6);
        assert_eq!(layout.quit_idx, 9);
    }

    #[test]
    fn build_layout_without_update_unchanged() {
        let state = UsageState::Ok(
            vec![LimitWindow { name: "d".into(), ..Default::default() }],
            Some("max".into()),
        );
        let layout = build_layout(&[(ProviderKind::Claude, &state)], None, None);
        assert_eq!(layout.header_indices[0].0, 0);
        assert_eq!(layout.refresh_idx, 4);
    }
}
