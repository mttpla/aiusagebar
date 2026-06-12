use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};
use crate::provider::{LimitWindow, UsageState};

pub mod base;
pub mod claude;
pub mod copilot;

#[cfg(target_os = "macos")]
pub(crate) mod styled;

pub struct MenuBuild {
    pub menu: Menu,
    pub about: MenuId,
    pub refresh: MenuId,
    pub quit: MenuId,
}

#[derive(Debug)]
pub(crate) enum ProviderKind {
    Claude,
    Copilot,
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
    states: &[(&str, &UsageState)],
    last_updated: Option<&str>,
) -> MenuLayout {
    let mut idx: usize = 2; // About(0) + separator(1)
    let mut header_indices: Vec<(usize, ProviderKind)> = Vec::new();
    let mut window_items: Vec<(usize, LimitWindow)> = Vec::new();

    for (name, state) in states {
        match *name {
            "Claude" => {
                header_indices.push((idx, ProviderKind::Claude));
                if let UsageState::Ok(windows, _) = state {
                    for (i, w) in windows.iter().enumerate() {
                        window_items.push((idx + 1 + i, w.clone()));
                    }
                }
                idx += claude::section_item_count(state);
            }
            "Copilot" => {
                header_indices.push((idx, ProviderKind::Copilot));
                if let UsageState::Ok(windows, _) = state {
                    for (i, w) in windows.iter().enumerate() {
                        window_items.push((idx + 1 + i, w.clone()));
                    }
                }
                idx += copilot::section_item_count(state);
            }
            _ => idx += 1,
        }
    }

    MenuLayout {
        header_indices,
        window_items,
        refresh_idx: idx,
        quit_idx: idx + 1,
        last_updated: last_updated.map(str::to_owned),
    }
}

pub(crate) fn append_label(menu: &Menu, text: impl Into<String>) {
    menu.append(&MenuItem::new(text.into(), false, None))
        .expect("menu append failed");
}

pub fn build_menu(states: &[(&str, &UsageState)], last_updated: Option<&str>) -> MenuBuild {
    let menu = Menu::new();
    let item_about = MenuItem::new("About AIUsageBar", true, None);
    menu.append(&item_about).expect("menu append failed");
    menu.append(&PredefinedMenuItem::separator())
        .expect("menu append failed");
    for (name, state) in states {
        match *name {
            "Claude" => { let _ = claude::append_claude_section(&menu, state); }
            "Copilot" => { let _ = copilot::append_copilot_section(&menu, state); }
            _ => append_label(&menu, format!("{}: unknown provider", name)),
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
        about: item_about.id().clone(),
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
        // About(0) + sep(1) → refresh at 2, quit at 3
        let layout = build_layout(&[], None);
        assert_eq!(layout.refresh_idx, 2);
        assert_eq!(layout.quit_idx, 3);
        assert!(layout.header_indices.is_empty());
    }

    #[test]
    fn menu_layout_indices_claude_two_windows() {
        // About(0) + sep(1) + header(2) + win1(3) + win2(4) → refresh at 5, quit at 6
        let state = UsageState::Ok(
            vec![
                LimitWindow { name: "d".into(), ..Default::default() },
                LimitWindow { name: "m".into(), ..Default::default() },
            ],
            Some("max".into()),
        );
        let layout = build_layout(&[("Claude", &state)], None);
        assert_eq!(layout.header_indices[0].0, 2);
        assert_eq!(layout.refresh_idx, 5);
        assert_eq!(layout.quit_idx, 6);
    }

    #[test]
    fn build_layout_window_items_indices() {
        // About(0) + sep(1) + header(2) + win0(3) + win1(4) → refresh=5
        let state = UsageState::Ok(
            vec![
                LimitWindow { name: "5h session".into(), percent_used: Some(39.0), ..Default::default() },
                LimitWindow { name: "7d weekly".into(), percent_used: Some(15.0), ..Default::default() },
            ],
            Some("max".into()),
        );
        let layout = build_layout(&[("Claude", &state)], None);
        assert_eq!(layout.window_items.len(), 2);
        assert_eq!(layout.window_items[0].0, 3); // first window at index 3
        assert_eq!(layout.window_items[1].0, 4);
        assert_eq!(layout.window_items[0].1.name, "5h session");
        assert_eq!(layout.window_items[1].1.name, "7d weekly");
    }

    #[test]
    fn build_layout_non_ok_state_no_window_items() {
        let layout = build_layout(&[("Claude", &UsageState::NotConfigured)], None);
        assert!(layout.window_items.is_empty());
    }
}
