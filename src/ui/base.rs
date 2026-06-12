use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};

pub struct FooterIds {
    pub about: MenuId,
    pub refresh: MenuId,
    pub quit: MenuId,
}

pub(crate) fn refresh_label(updated: Option<&str>) -> Option<String> {
    updated.map(|ts| format!("Updated: {}", ts))
}

pub fn append_footer(menu: &Menu, updated: Option<&str>) -> FooterIds {
    if let Some(label) = refresh_label(updated) {
        super::append_label(menu, label);
    }
    menu.append(&PredefinedMenuItem::separator())
        .expect("menu append failed");
    let item_refresh = MenuItem::new("Refresh", true, None);
    menu.append(&item_refresh).expect("menu append failed");
    menu.append(&PredefinedMenuItem::separator())
        .expect("menu append failed");
    let item_about = MenuItem::new("About AIUsageBar", true, None);
    let item_quit = MenuItem::new("Quit", true, None);
    menu.append(&item_about).expect("menu append failed");
    menu.append(&item_quit).expect("menu append failed");
    FooterIds {
        about: item_about.id().clone(),
        refresh: item_refresh.id().clone(),
        quit: item_quit.id().clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_label_some() {
        assert_eq!(refresh_label(Some("12:34")), Some("Updated: 12:34".to_string()));
    }

    #[test]
    fn refresh_label_none() {
        assert_eq!(refresh_label(None), None);
    }
}
