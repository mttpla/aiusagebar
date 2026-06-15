use std::sync::OnceLock;
use std::time::Duration;
use ureq::tls::{TlsConfig, TlsProvider};

#[derive(Debug, PartialEq)]
pub enum HttpError {
    Unauthorized,
    RateLimited,
    Other(String),
}

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

pub fn get(url: &str, token: &str, extra_headers: &[(&str, &str)]) -> Result<String, HttpError> {
    let mut req = agent()
        .get(url)
        .header("Authorization", &format!("Bearer {}", token));
    for (name, value) in extra_headers {
        req = req.header(*name, *value);
    }
    let resp = req.call().map_err(|e| HttpError::Other(e.to_string()))?;
    match resp.status().as_u16() {
        200 => resp
            .into_body()
            .read_to_string()
            .map_err(|e| HttpError::Other(e.to_string())),
        401 => Err(HttpError::Unauthorized),
        429 => Err(HttpError::RateLimited),
        code => Err(HttpError::Other(format!("HTTP {}", code))),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn shared_agent_is_reused() {
        let a = super::agent() as *const ureq::Agent;
        let b = super::agent() as *const ureq::Agent;
        assert_eq!(a, b, "agent() must return the same instance across calls");
    }
}
