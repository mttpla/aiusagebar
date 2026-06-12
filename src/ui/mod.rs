use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};
use crate::provider::UsageState;

pub mod base;
pub mod claude;
pub mod copilot;

pub struct MenuBuild {
    pub menu: Menu,
    pub about: MenuId,
    pub refresh: MenuId,
    pub quit: MenuId,
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
            "Claude" => claude::append_claude_section(&menu, state),
            "Copilot" => copilot::append_copilot_section(&menu, state),
            _ => append_label(&menu, format!("{}: unknown provider", name)),
        }
    }
    let footer = base::append_footer(&menu, last_updated);
    MenuBuild {
        menu,
        about: item_about.id().clone(),
        refresh: footer.refresh,
        quit: footer.quit,
    }
}
