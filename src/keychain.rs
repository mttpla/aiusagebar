#[cfg(target_os = "macos")]
pub(crate) fn read_generic_password(service: &str, account: &str) -> Option<String> {
    use security_framework::passwords::get_generic_password;
    get_generic_password(service, account)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn read_generic_password(_service: &str, _account: &str) -> Option<String> {
    None
}

#[cfg(target_os = "macos")]
pub(crate) fn enumerate_generic_passwords(service: &str) -> Vec<(String, String)> {
    use core_foundation::base::TCFType;
    use core_foundation::string::{CFString, CFStringRef};
    use security_framework::item::{ItemClass, ItemSearchOptions, Limit, SearchResult};
    use security_framework::passwords::get_generic_password;

    let results = ItemSearchOptions::new()
        .class(ItemClass::generic_password())
        .service(service)
        .limit(Limit::All)
        .load_attributes(true)
        .search()
        .unwrap_or_default();

    results
        .into_iter()
        .filter_map(|r| {
            let SearchResult::Dict(dict) = r else { return None };
            let (keys, values) = dict.get_keys_and_values();
            // Keychain attribute dict keys are always CFString — cast is safe per Security.framework docs.
            let account = keys.iter().zip(values.iter()).find_map(|(k_ptr, v_ptr)| {
                let k = unsafe { CFString::wrap_under_get_rule(*k_ptr as CFStringRef) };
                if k == "acct" {
                    let v = unsafe { CFString::wrap_under_get_rule(*v_ptr as CFStringRef) };
                    Some(v.to_string())
                } else {
                    None
                }
            })?;
            let password = get_generic_password(service, &account).ok()?;
            String::from_utf8(password).ok().map(|p| (account, p))
        })
        .collect()
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn enumerate_generic_passwords(_service: &str) -> Vec<(String, String)> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_service_returns_none() {
        let result = read_generic_password("__aiusagebar_test_nonexistent_xyzzy__", "test");
        assert!(result.is_none());
    }

    #[test]
    fn enumerate_nonexistent_service_returns_empty() {
        let result = super::enumerate_generic_passwords("__aiusagebar_test_nonexistent_xyzzy__");
        assert!(result.is_empty());
    }

}
