#[cfg(target_os = "macos")]
pub fn read_generic_password(service: &str, account: &str) -> Option<String> {
    use security_framework::passwords::get_generic_password;
    get_generic_password(service, account)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

#[cfg(not(target_os = "macos"))]
pub fn read_generic_password(_service: &str, _account: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_service_returns_none() {
        let result = read_generic_password("__aiusagebar_test_nonexistent_xyzzy__", "test");
        assert!(result.is_none());
    }
}
