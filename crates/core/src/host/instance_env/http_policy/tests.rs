use super::*;
use spacetimedb_lib::TimeDuration;

mod fixtures;

fn frame(uri: &str, response_limit: u32) -> RequestWithPolicy {
    RequestWithPolicy {
        policy: st_http::HttpRequestPolicy::ExactOrigin { response_limit },
        request: st_http::Request {
            method: st_http::Method::Get,
            headers: [].into_iter().collect(),
            timeout: Some(TimeDuration::from_micros(2_000_000)),
            uri: uri.into(),
            version: st_http::Version::Http11,
        },
    }
}

#[test]
fn strict_conversion_marks_headers_and_redacts_request_debug() {
    let marker = "SYNTHETIC_SECRET_MARKER";
    let mut frame = frame(&format!("https://{marker}.invalid/{marker}"), 1024);
    frame.request.headers = [
        (Some("authorization".into()), marker.as_bytes().into()),
        (Some("x-private-token".into()), marker.as_bytes().into()),
    ]
    .into_iter()
    .collect();
    let request = prepare_request(frame, Bytes::copy_from_slice(marker.as_bytes())).unwrap();
    assert!(request.inner.headers().values().all(http::HeaderValue::is_sensitive));
    assert!(!format!("{:?}", request.inner.headers()).contains(marker));
    assert!(!format!("{request:?}").contains(marker));
}

#[test]
fn strict_request_rejects_insecure_blocked_and_malformed_targets_without_data_errors() {
    let marker = "SYNTHETIC_SECRET_MARKER";
    for (uri, expected) in [
        (
            format!("http://example.invalid/{marker}"),
            HttpPolicyError::HttpsRequired,
        ),
        (format!("https://10.0.0.1/{marker}"), HttpPolicyError::BlockedAddress),
        (
            format!("https://[::ffff:127.0.0.1]/{marker}"),
            HttpPolicyError::BlockedAddress,
        ),
    ] {
        let error = prepare_request(frame(&uri, 1), Bytes::new()).unwrap_err();
        assert_eq!(error, expected);
        assert!(!format!("{error:?}: {error}").contains(marker));
    }
    let mut bad = frame("https://example.invalid/", 1);
    bad.request.method = st_http::Method::Extension(format!("\n{marker}"));
    assert_eq!(
        prepare_request(bad, Bytes::new()).unwrap_err(),
        HttpPolicyError::InvalidRequest
    );
    let mut bad = frame("https://example.invalid/", 1);
    bad.request.headers = [(Some("host".into()), b"other.invalid".as_slice().into())]
        .into_iter()
        .collect();
    assert_eq!(
        prepare_request(bad, Bytes::new()).unwrap_err(),
        HttpPolicyError::InvalidRequest
    );
}

#[test]
fn strict_timeout_defaults_and_clamps_are_preserved() {
    let mut request = frame("https://example.invalid/", 1);
    request.request.timeout = None;
    assert_eq!(
        prepare_request(request.clone(), Bytes::new())
            .unwrap()
            .inner
            .timeout()
            .copied(),
        Some(HTTP_DEFAULT_TIMEOUT)
    );
    request.request.timeout = Some(TimeDuration::from_micros(900_000_000));
    assert_eq!(
        prepare_request(request.clone(), Bytes::new())
            .unwrap()
            .inner
            .timeout()
            .copied(),
        Some(HTTP_MAX_TIMEOUT)
    );
    request.request.timeout = Some(TimeDuration::from_micros(1234));
    assert_eq!(
        prepare_request(request, Bytes::new()).unwrap().inner.timeout().copied(),
        Some(Duration::from_micros(1234))
    );
}

#[tokio::test]
async fn streamed_body_limit_accepts_boundary_and_stops_before_later_chunks() {
    use futures_util::stream;
    let body = collect_limited(
        stream::iter([Ok(Bytes::from_static(b"12")), Ok(Bytes::from_static(b"34"))]),
        4,
    )
    .await
    .unwrap();
    assert_eq!(&body[..], b"1234");
    let error = collect_limited(
        stream::iter([
            Ok(Bytes::from_static(b"1234")),
            Ok(Bytes::from_static(b"5")),
            Err(HttpPolicyError::HostFailure),
        ]),
        4,
    )
    .await
    .unwrap_err();
    assert_eq!(error, HttpPolicyError::ResponseTooLarge);
    assert_eq!(
        collect_limited(stream::iter([Ok(Bytes::new())]), 1)
            .await
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        collect_limited(stream::iter([Err(HttpPolicyError::Timeout)]), 4)
            .await
            .unwrap_err(),
        HttpPolicyError::Timeout
    );
}

#[tokio::test]
async fn policy_execution_rejects_invalid_limits_before_transport() {
    for limit in [0, st_http::MAX_EXACT_ORIGIN_RESPONSE_BYTES + 1] {
        let error = execute(frame("https://example.invalid/", limit), Bytes::new())
            .await
            .err()
            .unwrap();
        assert_eq!(error, HttpPolicyError::InvalidResponseLimit);
    }
}

#[test]
fn actual_handler_wiring_uses_policy_decoder_and_executor() {
    let handler = include_str!("../../wasmtime/wasm_instance_env/http_policy.rs");
    assert!(handler.contains("match decode_request(request_buf)"));
    assert!(handler.contains("env.instance_env.http_request_with_policy(request, body).await"));
    assert!(!handler.contains("instance_env.http_request("));
    assert!(!handler.contains("log::"));
    assert!(!handler.contains("request_buf:?"));
}
