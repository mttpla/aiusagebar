/// OSStatus for "no such Keychain item" — the expected NotConfigured path.
#[cfg(target_os = "macos")]
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

/// True for Keychain errors worth logging — genuine read failures, not the
/// expected `errSecItemNotFound` that simply means the provider is unconfigured.
#[cfg(target_os = "macos")]
fn keychain_error_is_loggable(code: i32) -> bool {
    code != ERR_SEC_ITEM_NOT_FOUND
}

#[cfg(target_os = "macos")]
pub(crate) fn read_generic_password(service: &str, account: &str) -> Option<String> {
    use security_framework::passwords::get_generic_password;
    let bytes = match get_generic_password(service, account) {
        Ok(b) => b,
        Err(e) => {
            if keychain_error_is_loggable(e.code()) {
                crate::diag!(
                    crate::diag::Level::Err,
                    "Keychain read failed for service {} (status {}): {}",
                    service,
                    e.code(),
                    e
                );
            }
            return None;
        }
    };
    match String::from_utf8(bytes) {
        Ok(s) => Some(s),
        Err(e) => {
            crate::diag!(
                crate::diag::Level::Err,
                "Keychain item for service {} is not valid UTF-8: {}",
                service,
                e
            );
            None
        }
    }
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

    let results = match ItemSearchOptions::new()
        .class(ItemClass::generic_password())
        .service(service)
        .limit(Limit::All)
        .load_attributes(true)
        .search()
    {
        Ok(r) => r,
        Err(e) => {
            if keychain_error_is_loggable(e.code()) {
                crate::diag!(
                    crate::diag::Level::Err,
                    "Keychain enumerate failed for service {} (status {}): {}",
                    service,
                    e.code(),
                    e
                );
            }
            Vec::new()
        }
    };

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
            let password = match get_generic_password(service, &account) {
                Ok(p) => p,
                Err(e) => {
                    if keychain_error_is_loggable(e.code()) {
                        crate::diag!(
                            crate::diag::Level::Err,
                            "Keychain read failed for service {} account {} (status {}): {}",
                            service, account, e.code(), e
                        );
                    }
                    return None;
                }
            };
            match String::from_utf8(password) {
                Ok(p) => Some((account, p)),
                Err(e) => {
                    crate::diag!(
                        crate::diag::Level::Err,
                        "Keychain item for service {} account {} is not valid UTF-8: {}",
                        service, account, e
                    );
                    None
                }
            }
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

    #[cfg(target_os = "macos")]
    #[test]
    fn item_not_found_is_not_loggable() {
        // errSecItemNotFound is the expected "no credential" path — must not log.
        assert!(!super::keychain_error_is_loggable(super::ERR_SEC_ITEM_NOT_FOUND));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn genuine_read_error_is_loggable() {
        // errSecInteractionNotAllowed (-25308): a real failure worth logging.
        assert!(super::keychain_error_is_loggable(-25308));
    }

}
