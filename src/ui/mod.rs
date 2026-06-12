use tray_icon::menu::{Menu, MenuId, MenuItem};
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

pub fn build_menu(_states: &[(&str, &UsageState)], _last_updated: Option<&str>) -> MenuBuild {
    todo!()
}
