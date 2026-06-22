use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use crate::provider::ProviderKind;

#[derive(Debug, PartialEq)]
pub(crate) enum OtherEntry {
    Provider(ProviderKind),
    Diagnostics,
    Placeholder,
}

/// Decides what appears inside the "Other ▶" submenu, in order:
/// one entry per provider that has raw JSON, then Diagnostics when the diag
/// log is non-empty, then a single Placeholder if nothing else would show.
pub(crate) fn other_entries(details_kinds: &[ProviderKind], diag_empty: bool) -> Vec<OtherEntry> {
    let mut entries: Vec<OtherEntry> =
        details_kinds.iter().map(|k| OtherEntry::Provider(*k)).collect();
    if !diag_empty {
        entries.push(OtherEntry::Diagnostics);
    }
    if entries.is_empty() {
        entries.push(OtherEntry::Placeholder);
    }
    entries
}

pub(crate) struct FooterIds {
    pub(crate) refresh: MenuId,
    pub(crate) about: MenuId,
    pub(crate) quit: MenuId,
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

pub(crate) struct OtherIds {
    pub(crate) details_claude: Option<MenuId>,
    pub(crate) details_copilot: Option<MenuId>,
    pub(crate) copy_diag: Option<MenuId>,
}

/// Appends the always-present "Other ▶" submenu. Contains a "Provider ▶ Details…"
/// entry for each provider in `details_kinds`, then "Diagnostics ▶ Copy diagnostic
/// log" when the diag log is non-empty, and a disabled "No diagnostics" placeholder
/// only when nothing else would appear.
pub(crate) fn append_other(menu: &Menu, details_kinds: &[ProviderKind]) -> OtherIds {
    let other = Submenu::new("Other", true);
    let mut details_claude: Option<MenuId> = None;
    let mut details_copilot: Option<MenuId> = None;
    let mut copy_diag: Option<MenuId> = None;

    for entry in other_entries(details_kinds, crate::diag::is_empty()) {
        match entry {
            OtherEntry::Provider(kind) => {
                let sub = Submenu::new(kind.display_name(), true);
                let item = MenuItem::new("Details…", true, None);
                let id = item.id().clone();
                sub.append(&item).expect("menu append failed");
                other.append(&sub).expect("menu append failed");
                match kind {
                    ProviderKind::Claude => details_claude = Some(id),
                    ProviderKind::Copilot => details_copilot = Some(id),
                }
            }
            OtherEntry::Diagnostics => {
                let diagnostics = Submenu::new("Diagnostics", true);
                let copy = MenuItem::new("Copy diagnostic log", true, None);
                copy_diag = Some(copy.id().clone());
                diagnostics.append(&copy).expect("menu append failed");
                other.append(&diagnostics).expect("menu append failed");
            }
            OtherEntry::Placeholder => {
                let placeholder = MenuItem::new("No diagnostics", false, None);
                other.append(&placeholder).expect("menu append failed");
            }
        }
    }

    menu.append(&other).expect("menu append failed");
    OtherIds { details_claude, details_copilot, copy_diag }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::ProviderKind;

    #[test]
    fn entries_both_providers_and_diag() {
        let got = other_entries(&[ProviderKind::Claude, ProviderKind::Copilot], false);
        assert_eq!(
            got,
            vec![
                OtherEntry::Provider(ProviderKind::Claude),
                OtherEntry::Provider(ProviderKind::Copilot),
                OtherEntry::Diagnostics,
            ]
        );
    }

    #[test]
    fn entries_provider_without_raw_json_omitted() {
        let got = other_entries(&[ProviderKind::Claude], false);
        assert_eq!(
            got,
            vec![OtherEntry::Provider(ProviderKind::Claude), OtherEntry::Diagnostics]
        );
    }

    #[test]
    fn entries_diag_empty_omits_diagnostics() {
        let got = other_entries(&[ProviderKind::Claude], true);
        assert_eq!(got, vec![OtherEntry::Provider(ProviderKind::Claude)]);
    }

    #[test]
    fn entries_nothing_present_falls_back_to_placeholder() {
        let got = other_entries(&[], true);
        assert_eq!(got, vec![OtherEntry::Placeholder]);
    }
}
