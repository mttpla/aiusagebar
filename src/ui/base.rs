use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};

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
