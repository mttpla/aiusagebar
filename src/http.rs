use std::sync::OnceLock;
use std::time::Duration;
use ureq::tls::{TlsConfig, TlsProvider};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum HttpError {
    Unauthorized,
    RateLimited,
    ServerError(u16),
    Other(String),
}

/// Return type for [`get`]: the parsed result plus the raw body whenever the
/// server responded (None only on network/IO errors).
pub(crate) type GetResult = (Result<String, HttpError>, Option<String>);

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(15)))
            .http_status_as_error(false)
            .tls_config(TlsConfig::builder().provider(TlsProvider::NativeTls).build())
            .build()
            .new_agent()
    })
}

pub(crate) fn get(url: &str, token: &str, extra_headers: &[(&str, &str)]) -> (Result<String, HttpError>, Option<String>) {
    let mut req = agent()
        .get(url)
        .header("Authorization", &format!("Bearer {}", token));
    for (name, value) in extra_headers {
        req = req.header(*name, *value);
    }
    let resp = match req.call() {
        Ok(r) => r,
        Err(e) => {
            crate::diag!(crate::diag::Level::Err, "HTTP request to {} failed: {}", url, e);
            return (Err(HttpError::Other(e.to_string())), None);
        }
    };
    let status = resp.status().as_u16();
    let raw = resp.into_body().read_to_string().ok();
    let result = match status {
        200 => raw.clone().map(Ok).unwrap_or_else(|| Err(HttpError::Other("body read error".into()))),
        401 => Err(HttpError::Unauthorized),
        429 => Err(HttpError::RateLimited),
        c @ 500..=599 => Err(HttpError::ServerError(c)),
        code => Err(HttpError::Other(format!("HTTP {}", code))),
    };
    if result.is_err() {
        crate::diag!(
            crate::diag::Level::Err,
            "HTTP {} from {}: {}",
            status,
            url,
            crate::diag::truncate(raw.as_deref().unwrap_or(""), 512)
        );
    }
    (result, raw)
}

pub(crate) fn get_public(url: &str) -> Result<String, HttpError> {
    let resp = agent()
        .get(url)
        .header("User-Agent", concat!("aiusagebar/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| {
            crate::diag!(crate::diag::Level::Err, "HTTP request to {} failed: {}", url, e);
            HttpError::Other(e.to_string())
        })?;
    let status = resp.status().as_u16();
    let result = match status {
        200 => resp.into_body().read_to_string().map_err(|e| {
            crate::diag!(crate::diag::Level::Err, "Reading body from {} failed: {}", url, e);
            HttpError::Other(e.to_string())
        }),
        401 => Err(HttpError::Unauthorized),
        429 => Err(HttpError::RateLimited),
        c @ 500..=599 => Err(HttpError::ServerError(c)),
        code => Err(HttpError::Other(format!("HTTP {}", code))),
    };
    if result.is_err() && status != 200 {
        crate::diag!(crate::diag::Level::Err, "HTTP {} from {}", status, url);
    }
    result
}

#[cfg(test)]
mod tests {
    #[test]
    fn shared_agent_is_reused() {
        let a = super::agent() as *const ureq::Agent;
        let b = super::agent() as *const ureq::Agent;
        assert_eq!(a, b, "agent() must return the same instance across calls");
    }

    #[test]
    fn get_public_function_exists_and_compiles() {
        // structural: verifies the function signature is correct
        let _: fn(&str) -> Result<String, super::HttpError> = super::get_public;
    }

    #[test]
    fn get_returns_tuple() {
        let _: fn(&str, &str, &[(&str, &str)]) -> super::GetResult = super::get;
    }
}
