use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};

pub struct FooterIds {
    pub about: MenuId,
    pub refresh: MenuId,
    pub quit: MenuId,
}

pub(crate) fn refresh_label(_updated: Option<&str>) -> Option<String> {
    todo!()
}

pub fn append_footer(_menu: &Menu, _updated: Option<&str>) -> FooterIds {
    todo!()
}
