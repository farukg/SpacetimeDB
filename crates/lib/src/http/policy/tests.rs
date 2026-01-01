use super::*;
use crate::http::{Method, Version};

fn legacy_request() -> Request {
    Request {
        method: Method::Get,
        headers: [].into_iter().collect(),
        timeout: None,
        uri: "https://example.invalid/".into(),
        version: Version::Http11,
    }
}

fn legacy_bytes() -> Vec<u8> {
    let mut bytes = vec![0, 0, 0, 0, 0, 1, 24, 0, 0, 0];
    bytes.extend_from_slice(b"https://example.invalid/");
    bytes.push(2);
    bytes
}

#[test]
fn legacy_request_stable_bytes_and_roundtrip() {
    let bytes = bsatn::to_vec(&legacy_request()).unwrap();
    assert_eq!(bytes, legacy_bytes());
    let decoded: Request = bsatn::from_slice(&bytes).unwrap();
    assert_eq!(bsatn::to_vec(&decoded).unwrap(), bytes);
}

#[test]
fn strict_request_stable_bytes_and_roundtrip() {
    let request = RequestWithPolicy {
        policy: HttpRequestPolicy::ExactOrigin { response_limit: 1024 },
        request: legacy_request(),
    };
    let mut expected = vec![0, 0, 4, 0, 0];
    expected.extend(legacy_bytes());
    let bytes = bsatn::to_vec(&request).unwrap();
    assert_eq!(bytes, expected);
    let decoded = decode_request(&bytes).unwrap();
    assert_eq!(decoded.policy, request.policy);
    assert_eq!(bsatn::to_vec(&decoded).unwrap(), bytes);
}

#[test]
fn legacy_decoder_ignores_appended_policy_so_it_must_not_be_used() {
    let bytes = legacy_bytes();
    let mut extended = bytes.clone();
    extended.extend_from_slice(&[255, 1, 2, 3]);
    let decoded: Request = bsatn::from_slice(&extended).unwrap();
    assert_eq!(bsatn::to_vec(&decoded).unwrap(), bytes);
}

#[test]
fn unknown_truncated_and_trailing_policy_frames_fail_closed() {
    let frame = RequestWithPolicy {
        policy: HttpRequestPolicy::ExactOrigin { response_limit: 1 },
        request: legacy_request(),
    };
    let bytes = bsatn::to_vec(&frame).unwrap();
    let mut unknown = bytes.clone();
    unknown[0] = 255;
    assert_eq!(decode_request(&unknown).unwrap_err(), HttpPolicyError::InvalidPolicy);
    for end in 0..bytes.len() {
        assert!(decode_request(&bytes[..end]).is_err());
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(decode_request(&trailing).unwrap_err(), HttpPolicyError::InvalidPolicy);
}

#[test]
fn response_limit_is_validated_at_decode_boundary() {
    for response_limit in [0, MAX_EXACT_ORIGIN_RESPONSE_BYTES + 1, u32::MAX] {
        let frame = RequestWithPolicy {
            policy: HttpRequestPolicy::ExactOrigin { response_limit },
            request: legacy_request(),
        };
        assert_eq!(
            decode_request(&bsatn::to_vec(&frame).unwrap()).unwrap_err(),
            HttpPolicyError::InvalidResponseLimit,
        );
    }
    for response_limit in [1, MAX_EXACT_ORIGIN_RESPONSE_BYTES] {
        assert_eq!(
            HttpRequestPolicy::ExactOrigin { response_limit }.response_limit(),
            Ok(response_limit)
        );
    }
}

#[test]
fn policy_debug_and_all_error_encodings_are_bounded_and_redacted() {
    let marker = "SYNTHETIC_SECRET_MARKER";
    let mut request = legacy_request();
    request.uri = format!("https://{marker}.invalid/{marker}?secret={marker}");
    request.headers = [(Some("authorization".into()), marker.as_bytes().into())]
        .into_iter()
        .collect();
    let frame = RequestWithPolicy {
        policy: HttpRequestPolicy::ExactOrigin { response_limit: 1 },
        request,
    };
    assert!(!format!("{frame:?}").contains(marker));
    for error in [
        HttpPolicyError::InvalidPolicy,
        HttpPolicyError::InvalidResponseLimit,
        HttpPolicyError::InvalidRequest,
        HttpPolicyError::HttpsRequired,
        HttpPolicyError::BlockedAddress,
        HttpPolicyError::ResponseTooLarge,
        HttpPolicyError::Timeout,
        HttpPolicyError::Network,
        HttpPolicyError::WouldBlockTransaction,
        HttpPolicyError::HostFailure,
    ] {
        assert!(!format!("{error:?}: {error}").contains(marker));
        let bytes = bsatn::to_vec(&error).unwrap();
        assert_eq!(bytes.len(), 1);
        assert_eq!(bsatn::from_slice::<HttpPolicyError>(&bytes).unwrap(), error);
    }
    let malformed = format!("\u{ff}{marker}");
    let error = decode_request(malformed.as_bytes()).unwrap_err();
    assert!(!format!("{error:?}: {error}").contains(marker));
}
