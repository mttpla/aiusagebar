use tray_icon::menu::{Menu, MenuId, MenuItem};

pub(crate) struct FooterIds {
    pub refresh: MenuId,
    pub quit: MenuId,
}

/// Appends Refresh and Quit items. Always adds exactly 2 items.
pub(crate) fn append_footer(menu: &Menu) -> FooterIds {
    let item_refresh = MenuItem::new("↺ Refresh", true, None);
    let item_quit = MenuItem::new("Quit", true, None);
    menu.append(&item_refresh).expect("menu append failed");
    menu.append(&item_quit).expect("menu append failed");
    FooterIds {
        refresh: item_refresh.id().clone(),
        quit: item_quit.id().clone(),
    }
}
