use super::*;
use spacetimedb_bindings_sys::procedure::HttpPolicyCallError;

impl HttpClient {
    /// Send with mandatory host-enforced HTTPS, no redirects, no proxies and a streamed body cap.
    ///
    /// `response_limit` must be 1 through [`MAX_EXACT_ORIGIN_RESPONSE_BYTES`], inclusive.
    /// Redirect responses are returned without following them. A [`Timeout`] extension retains
    /// the usual 30-second default and 180-second maximum.
    ///
    /// This method requires host ABI 10.6. A module using it cannot load on older hosts;
    /// that is a module-load failure, not a per-call unsupported-policy result.
    /// There is no fallback to [`Self::send`]. This policy does not make arbitrary untrusted
    /// endpoints safe: callers must allowlist known origins and paths. Custom endpoint DNS
    /// validation and dial pinning require a different, native transport boundary.
    pub fn send_with_policy<B: Into<Body>>(
        &self,
        request: http::Request<B>,
        policy: HttpRequestPolicy,
    ) -> Result<Response, HttpPolicyError> {
        policy.response_limit()?;
        let (parts, body) = request.map(Into::into).into_parts();
        let frame = st_http::RequestWithPolicy {
            policy,
            request: convert_request(parts),
        };
        let frame = bsatn::to_vec(&frame).map_err(|_| HttpPolicyError::InvalidRequest)?;
        match spacetimedb_bindings_sys::procedure::http_request_with_policy(&frame, &body.into_bytes()) {
            Ok((response_source, body_source)) => {
                let response = read_policy_value::<st_http::Response>(response_source)?;
                let response = convert_response(response).map_err(|_| HttpPolicyError::HostFailure)?;
                let body = if body_source == spacetimedb_bindings_sys::raw::BytesSource::INVALID {
                    Body::empty()
                } else {
                    let mut buf = IterBuf::take();
                    read_bytes_source_into(body_source, &mut buf);
                    Body::from_bytes(buf.clone())
                };
                Ok(http::Response::from_parts(response, body))
            }
            Err(HttpPolicyCallError::Host(source)) => Err(read_policy_value::<HttpPolicyError>(source)?),
            Err(HttpPolicyCallError::Errno(errno)) => {
                if errno == spacetimedb_bindings_sys::Errno::WOULD_BLOCK_TRANSACTION {
                    Err(HttpPolicyError::WouldBlockTransaction)
                } else {
                    Err(HttpPolicyError::HostFailure)
                }
            }
            Err(HttpPolicyCallError::InputTooLarge) => Err(HttpPolicyError::InvalidRequest),
        }
    }
}

fn read_policy_value<T: spacetimedb_lib::de::DeserializeOwned>(
    source: spacetimedb_bindings_sys::raw::BytesSource,
) -> Result<T, HttpPolicyError> {
    if source == spacetimedb_bindings_sys::raw::BytesSource::INVALID {
        return Err(HttpPolicyError::HostFailure);
    }
    let mut buffer = IterBuf::take();
    read_bytes_source_into(source, &mut buffer);
    decode_policy_value(&buffer)
}

fn decode_policy_value<T: spacetimedb_lib::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, HttpPolicyError> {
    let mut remaining = bytes;
    let value = bsatn::from_reader(&mut remaining).map_err(|_| HttpPolicyError::HostFailure)?;
    if !remaining.is_empty() {
        return Err(HttpPolicyError::HostFailure);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_policy_host_values_have_only_safe_bounded_errors() {
        let marker = b"SYNTHETIC_SECRET_MARKER";
        let error = decode_policy_value::<HttpPolicyError>(marker).unwrap_err();
        assert_eq!(error, HttpPolicyError::HostFailure);
        assert!(!format!("{error:?}: {error}").contains("SYNTHETIC_SECRET_MARKER"));
        let mut bytes = bsatn::to_vec(&HttpPolicyError::Network).unwrap();
        bytes.extend_from_slice(marker);
        assert_eq!(
            decode_policy_value::<HttpPolicyError>(&bytes),
            Err(HttpPolicyError::HostFailure)
        );
    }

    #[test]
    fn sdk_policy_frame_uses_production_conversion_and_timeout() {
        let request = Request::builder()
            .uri("https://example.invalid/metadata")
            .header("authorization", "SYNTHETIC_SECRET_MARKER")
            .extension(Timeout(TimeDuration::from_micros(1234)))
            .body(())
            .unwrap();
        let (parts, _) = request.into_parts();
        let frame = st_http::RequestWithPolicy {
            policy: HttpRequestPolicy::ExactOrigin { response_limit: 1024 },
            request: convert_request(parts),
        };
        let encoded = bsatn::to_vec(&frame).unwrap();
        let decoded = st_http::policy::decode_request(&encoded).unwrap();
        assert_eq!(decoded.request.timeout, Some(TimeDuration::from_micros(1234)));
        assert_eq!(decoded.policy, frame.policy);
        assert!(!format!("{frame:?}").contains("SYNTHETIC_SECRET_MARKER"));
    }
}
