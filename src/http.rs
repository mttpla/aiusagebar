use std::sync::OnceLock;
use std::time::Duration;

#[derive(Debug, PartialEq)]
pub enum HttpError {
    Unauthorized,
    RateLimited,
    Other(String),
}

pub fn client() -> &'static reqwest::blocking::Client {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("failed to build HTTP client")
    })
}

pub fn get(url: &str, token: &str, extra_headers: &[(&str, &str)]) -> Result<String, HttpError> {
    let mut builder = client()
        .get(url)
        .header("Authorization", format!("Bearer {}", token));
    for (name, value) in extra_headers {
        builder = builder.header(*name, *value);
    }
    let resp = builder.send().map_err(|e| HttpError::Other(e.to_string()))?;
    match resp.status().as_u16() {
        200 => resp.text().map_err(|e| HttpError::Other(e.to_string())),
        401 => Err(HttpError::Unauthorized),
        429 => Err(HttpError::RateLimited),
        code => Err(HttpError::Other(format!("HTTP {}", code))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_error_variants_are_distinct() {
        assert_ne!(HttpError::Unauthorized, HttpError::RateLimited);
    }

    #[test]
    fn http_error_other_carries_message() {
        let e = HttpError::Other("boom".to_string());
        assert_eq!(e, HttpError::Other("boom".to_string()));
    }

    #[test]
    fn shared_client_is_reused() {
        let a = super::client() as *const reqwest::blocking::Client;
        let b = super::client() as *const reqwest::blocking::Client;
        assert_eq!(a, b, "client() must return the same instance across calls");
    }
}
