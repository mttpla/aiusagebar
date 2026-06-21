use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};

pub(crate) struct FooterIds {
    pub refresh: MenuId,
    pub about: MenuId,
    pub quit: MenuId,
}

/// Appends Refresh, separator, About, Quit. Always adds exactly 4 items.
pub(crate) fn append_footer(menu: &Menu) -> FooterIds {
    let item_refresh = MenuItem::new("↺ Refresh", true, None);
    let item_about = MenuItem::new("ℹ About AIUsageBar", true, None);
    let item_quit = MenuItem::new("Quit", true, None);
    menu.append(&item_refresh).expect("menu append failed");
    menu.append(&PredefinedMenuItem::separator())
        .expect("menu append failed");
    menu.append(&item_about).expect("menu append failed");
    menu.append(&item_quit).expect("menu append failed");
    FooterIds {
        refresh: item_refresh.id().clone(),
        about: item_about.id().clone(),
        quit: item_quit.id().clone(),
    }
}

/// Appends the always-visible "Other ▶" submenu. When the diagnostic log has
/// entries it contains "Diagnostics ▶ Copy diagnostic log" and returns the copy
/// item's id; when empty it shows a disabled "No diagnostics" placeholder and
/// returns None.
pub(crate) fn append_other(menu: &Menu) -> Option<MenuId> {
    let other = Submenu::new("Other", true);
    let copy_id = if crate::diag::is_empty() {
        let placeholder = MenuItem::new("No diagnostics", false, None);
        other.append(&placeholder).expect("menu append failed");
        None
    } else {
        let diagnostics = Submenu::new("Diagnostics", true);
        let copy = MenuItem::new("Copy diagnostic log", true, None);
        let id = copy.id().clone();
        diagnostics.append(&copy).expect("menu append failed");
        other.append(&diagnostics).expect("menu append failed");
        Some(id)
    };
    menu.append(&other).expect("menu append failed");
    copy_id
}
