pub const VALID_PROTOCOLS: [&str; 2] = ["http", "https"];

#[derive(Debug, thiserror::Error)]
pub enum ServerUrlError {
    #[error("Invalid url: {url}")]
    InvalidUrl { url: String },
    #[error("Invalid protocol: {protocol}")]
    InvalidProtocol { protocol: String },
}

pub fn contains_protocol(name_or_url: &str) -> bool {
    name_or_url.contains("://")
}

pub fn url_to_host_and_protocol(url: &str) -> Result<(&str, &str), ServerUrlError> {
    let (protocol, host) = url
        .split_once("://")
        .ok_or_else(|| ServerUrlError::InvalidUrl { url: url.to_owned() })?;
    if !VALID_PROTOCOLS.contains(&protocol) {
        return Err(ServerUrlError::InvalidProtocol { protocol: protocol.to_owned() });
    }
    if host.is_empty() || contains_protocol(host) {
        return Err(ServerUrlError::InvalidUrl { url: url.to_owned() });
    }
    Ok((host, protocol))
}

pub fn host_or_url_to_host_and_protocol(host_or_url: &str) -> Result<(&str, Option<&str>), ServerUrlError> {
    if contains_protocol(host_or_url) {
        let (host, protocol) = url_to_host_and_protocol(host_or_url)?;
        Ok((host, Some(protocol)))
    } else {
        Ok((host_or_url, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_alias_and_http_origin_parts() -> Result<(), ServerUrlError> {
        assert_eq!(host_or_url_to_host_and_protocol("local")?, ("local", None));
        assert_eq!(host_or_url_to_host_and_protocol("http://127.0.0.1:3000")?, ("127.0.0.1:3000", Some("http")));
        assert_eq!(url_to_host_and_protocol("https://example.com")?, ("example.com", "https"));
        Ok(())
    }

    #[test]
    fn rejects_invalid_protocol_and_malformed_origins() {
        assert!(matches!(host_or_url_to_host_and_protocol("ftp://example.com"), Err(ServerUrlError::InvalidProtocol { .. })));
        for url in ["local", "http://", "http://example.com://other"] {
            assert!(matches!(url_to_host_and_protocol(url), Err(ServerUrlError::InvalidUrl { .. })), "{url}");
        }
    }
}
