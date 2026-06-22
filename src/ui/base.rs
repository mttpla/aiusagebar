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
