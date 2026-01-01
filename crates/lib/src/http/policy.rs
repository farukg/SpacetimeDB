//! Mandatory request policy for the versioned 10.6 procedure HTTP ABI.

use super::Request;
use crate::bsatn;
use spacetimedb_sats::SpacetimeType;

/// Maximum collected response body for exact-origin requests (16 MiB).
pub const MAX_EXACT_ORIGIN_RESPONSE_BYTES: u32 = 16 * 1024 * 1024;

/// Transport requirements enforced by the host, not HTTP headers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, SpacetimeType)]
#[sats(crate = crate, name = "HttpRequestPolicy")]
pub enum HttpRequestPolicy {
    /// HTTPS with certificate validation, no redirects, no proxies, and a streamed body limit.
    /// This does not validate or pin every DNS answer for arbitrary untrusted endpoints.
    ExactOrigin { response_limit: u32 },
}

impl HttpRequestPolicy {
    pub fn response_limit(self) -> Result<u32, HttpPolicyError> {
        match self {
            Self::ExactOrigin { response_limit } => {
                if response_limit == 0 || response_limit > MAX_EXACT_ORIGIN_RESPONSE_BYTES {
                    return Err(HttpPolicyError::InvalidResponseLimit);
                }
                Ok(response_limit)
            }
        }
    }
}

/// The 10.6 frame. Never send this to the legacy 10.3 import.
#[derive(Clone, SpacetimeType)]
#[sats(crate = crate, name = "HttpRequestWithPolicy")]
pub struct RequestWithPolicy {
    pub policy: HttpRequestPolicy,
    pub request: Request,
}

impl std::fmt::Debug for RequestWithPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RequestWithPolicy([REDACTED])")
    }
}

/// Decode the entire policy frame without exposing malformed input in errors.
pub fn decode_request(bytes: &[u8]) -> Result<RequestWithPolicy, HttpPolicyError> {
    let mut remaining = bytes;
    let request: RequestWithPolicy = bsatn::from_reader(&mut remaining).map_err(|_| HttpPolicyError::InvalidPolicy)?;
    if !remaining.is_empty() {
        return Err(HttpPolicyError::InvalidPolicy);
    }
    request.policy.response_limit()?;
    Ok(request)
}

/// Bounded failures for mandatory-policy HTTP. No variant carries remote data or URLs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, SpacetimeType)]
#[sats(crate = crate, name = "HttpPolicyError")]
pub enum HttpPolicyError {
    InvalidPolicy,
    InvalidResponseLimit,
    InvalidRequest,
    HttpsRequired,
    BlockedAddress,
    ResponseTooLarge,
    Timeout,
    Network,
    WouldBlockTransaction,
    HostFailure,
}

impl std::fmt::Display for HttpPolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidPolicy => "invalid or unsupported HTTP policy frame",
            Self::InvalidResponseLimit => "invalid HTTP response limit",
            Self::InvalidRequest => "invalid strict HTTP request",
            Self::HttpsRequired => "strict HTTP requires HTTPS",
            Self::BlockedAddress => "HTTP address is blocked",
            Self::ResponseTooLarge => "HTTP response exceeds the body limit",
            Self::Timeout => "strict HTTP request timed out",
            Self::Network => "strict HTTP transport failed",
            Self::WouldBlockTransaction => "HTTP cannot run inside a transaction",
            Self::HostFailure => "strict HTTP host failure",
        })
    }
}

impl std::error::Error for HttpPolicyError {}

#[cfg(test)]
mod tests;
