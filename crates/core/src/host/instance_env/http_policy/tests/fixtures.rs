use super::*;
use openssl::asn1::{Asn1Integer, Asn1Time};
use openssl::bn::BigNum;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::ssl::{SslAcceptor, SslMethod};
use openssl::x509::extension::{BasicConstraints, SubjectAlternativeName};
use openssl::x509::{X509NameBuilder, X509};
use std::io::{Read, Write};
use std::process::Command;

const FIXTURE_HOST: &str = "strict-http.invalid";
const CHILD_ENV: &str = "SPACETIMEDB_STRICT_HTTP_FIXTURE_CHILD";

struct Fixture {
    address: SocketAddr,
    root: reqwest::Certificate,
    done: tokio::task::JoinHandle<bool>,
}

async fn tls_fixture(response: Vec<u8>) -> Fixture {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
    let mut name = X509NameBuilder::new().unwrap();
    name.append_entry_by_text("CN", &format!("fixture-{}", address.port()))
        .unwrap();
    let name = name.build();
    let mut certificate = X509::builder().unwrap();
    certificate.set_version(2).unwrap();
    certificate
        .set_serial_number(&Asn1Integer::from_bn(&BigNum::from_u32(1).unwrap()).unwrap())
        .unwrap();
    certificate.set_subject_name(&name).unwrap();
    certificate.set_issuer_name(&name).unwrap();
    certificate.set_pubkey(&key).unwrap();
    certificate
        .set_not_before(&Asn1Time::days_from_now(0).unwrap())
        .unwrap();
    certificate.set_not_after(&Asn1Time::days_from_now(1).unwrap()).unwrap();
    certificate
        .append_extension(BasicConstraints::new().critical().ca().build().unwrap())
        .unwrap();
    let san = SubjectAlternativeName::new()
        .dns(FIXTURE_HOST)
        .build(&certificate.x509v3_context(None, None))
        .unwrap();
    certificate.append_extension(san).unwrap();
    certificate.sign(&key, MessageDigest::sha256()).unwrap();
    let certificate = certificate.build();
    let root = reqwest::Certificate::from_der(&certificate.to_der().unwrap()).unwrap();
    let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    acceptor.set_private_key(&key).unwrap();
    acceptor.set_certificate(&certificate).unwrap();
    acceptor.check_private_key().unwrap();
    let acceptor = acceptor.build();
    let done = tokio::spawn(async move {
        let Ok(Ok((stream, _))) = tokio::time::timeout(Duration::from_secs(5), listener.accept()).await else {
            return false;
        };
        tokio::task::spawn_blocking(move || {
            let stream = stream.into_std().unwrap();
            stream.set_nonblocking(false).unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            stream.set_write_timeout(Some(Duration::from_secs(3))).unwrap();
            let Ok(mut tls) = acceptor.accept(stream) else {
                return false;
            };
            let mut request = Vec::new();
            let mut buffer = [0u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let Ok(count) = tls.read(&mut buffer) else { return false };
                if count == 0 || request.len() + count > 16 * 1024 {
                    return false;
                }
                request.extend_from_slice(&buffer[..count]);
            }
            tls.write_all(&response).is_ok() && tls.flush().is_ok()
        })
        .await
        .unwrap()
    });
    Fixture { address, root, done }
}

fn fixture_builder(fixture: &Fixture) -> reqwest::ClientBuilder {
    client_builder()
        .resolve(FIXTURE_HOST, fixture.address)
        .add_root_certificate(fixture.root.clone())
}

fn fixture_request(fixture: &Fixture, limit: u32) -> StrictRequest {
    prepare_request(
        frame(
            &format!("https://{FIXTURE_HOST}:{}/metadata", fixture.address.port()),
            limit,
        ),
        Bytes::new(),
    )
    .unwrap()
}

fn isolated_child(test: &str, proxy: Option<SocketAddr>) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    let path = module_path!().split_once("::").unwrap().1;
    command
        .arg("--exact")
        .arg(format!("{path}::{test}"))
        .env(CHILD_ENV, "1");
    for name in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ] {
        if let Some(address) = proxy {
            command.env(name, format!("http://{address}"));
        } else {
            command.env_remove(name);
        }
    }
    command.env("NO_PROXY", "").env("no_proxy", "");
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "fixture subprocess failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "fixture subprocess did not execute its test"
    );
}

#[tokio::test]
async fn strict_factory_redirect_not_followed_and_environment_proxy_not_used() {
    if std::env::var_os(CHILD_ENV).is_none() {
        let proxy = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        isolated_child(
            "strict_factory_redirect_not_followed_and_environment_proxy_not_used",
            Some(proxy.local_addr().unwrap()),
        );
        proxy.set_nonblocking(true).unwrap();
        assert_eq!(proxy.accept().unwrap_err().kind(), std::io::ErrorKind::WouldBlock);
        return;
    }
    let redirect_target = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let response = format!(
        "HTTP/1.1 302 Found\r\nLocation: https://redirect-http.invalid:{}/private\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        redirect_target.local_addr().unwrap().port(),
    );
    let fixture = tls_fixture(response.into_bytes()).await;
    let client = fixture_builder(&fixture)
        .resolve("redirect-http.invalid", redirect_target.local_addr().unwrap())
        .build()
        .unwrap();
    let (response, body) = execute_prepared(client, fixture_request(&fixture, 16)).await.unwrap();
    assert_eq!(response.code, 302);
    assert!(body.is_empty());
    assert!(fixture.done.await.unwrap());
    redirect_target.set_nonblocking(true).unwrap();
    assert_eq!(
        redirect_target.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[tokio::test]
async fn strict_factory_requires_valid_certificate_and_hostname() {
    if std::env::var_os(CHILD_ENV).is_none() {
        isolated_child("strict_factory_requires_valid_certificate_and_hostname", None);
        return;
    }
    for wrong_host in [false, true] {
        let fixture = tls_fixture(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n".to_vec()).await;
        let (builder, host) = if wrong_host {
            (
                client_builder().add_root_certificate(fixture.root.clone()),
                "wrong-host.invalid",
            )
        } else {
            (client_builder(), FIXTURE_HOST)
        };
        let client = builder.resolve(host, fixture.address).build().unwrap();
        let request = prepare_request(
            frame(
                &format!("https://{host}:{}/SYNTHETIC_SECRET_MARKER", fixture.address.port()),
                16,
            ),
            Bytes::new(),
        )
        .unwrap();
        let error = execute_prepared(client, request).await.err().unwrap();
        assert_eq!(error, HttpPolicyError::Network);
        assert!(!format!("{error:?}: {error}").contains("SYNTHETIC_SECRET_MARKER"));
        assert!(!fixture.done.await.unwrap());
    }
}

#[tokio::test]
async fn strict_execution_limits_chunked_body_at_boundary() {
    if std::env::var_os(CHILD_ENV).is_none() {
        isolated_child("strict_execution_limits_chunked_body_at_boundary", None);
        return;
    }
    for (limit, expected) in [
        (4, Ok(Bytes::from_static(b"1234"))),
        (3, Err(HttpPolicyError::ResponseTooLarge)),
    ] {
        let fixture = tls_fixture(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n2\r\n12\r\n2\r\n34\r\n0\r\n\r\n".to_vec()).await;
        let client = fixture_builder(&fixture).build().unwrap();
        let actual = execute_prepared(client, fixture_request(&fixture, limit))
            .await
            .map(|(_, body)| body);
        assert_eq!(actual, expected);
        assert!(fixture.done.await.unwrap());
    }
}

#[tokio::test]
async fn strict_execution_rejects_announced_oversize_body() {
    if std::env::var_os(CHILD_ENV).is_none() {
        isolated_child("strict_execution_rejects_announced_oversize_body", None);
        return;
    }
    let fixture = tls_fixture(b"HTTP/1.1 200 OK\r\nContent-Length: 999999\r\nConnection: close\r\n\r\n".to_vec()).await;
    let client = fixture_builder(&fixture).build().unwrap();
    let error = execute_prepared(client, fixture_request(&fixture, 4))
        .await
        .err()
        .unwrap();
    assert_eq!(error, HttpPolicyError::ResponseTooLarge);
    assert!(fixture.done.await.unwrap());
}

#[tokio::test]
async fn legacy_factory_retains_environment_proxy_routing() {
    if std::env::var_os(CHILD_ENV).is_none() {
        let proxy = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        isolated_child(
            "legacy_factory_retains_environment_proxy_routing",
            Some(proxy.local_addr().unwrap()),
        );
        proxy.set_nonblocking(true).unwrap();
        let (mut stream, _) = proxy.accept().unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(1))).unwrap();
        let mut request = [0u8; 1024];
        let count = stream.read(&mut request).unwrap();
        assert!(request[..count].starts_with(b"CONNECT "));
        return;
    }
    let unused_target = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = unused_target.local_addr().unwrap();
    let client = legacy_client_builder().resolve(FIXTURE_HOST, address).build().unwrap();
    assert!(client
        .get(format!("https://{FIXTURE_HOST}:{}/", address.port()))
        .timeout(Duration::from_millis(200))
        .send()
        .await
        .is_err());
}

#[tokio::test]
async fn legacy_factory_still_follows_redirects() {
    if std::env::var_os(CHILD_ENV).is_none() {
        isolated_child("legacy_factory_still_follows_redirects", None);
        return;
    }
    let target = tls_fixture(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".to_vec()).await;
    let origin = tls_fixture(format!(
        "HTTP/1.1 302 Found\r\nLocation: https://{FIXTURE_HOST}:{}/final\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        target.address.port(),
    ).into_bytes()).await;
    let client = legacy_client_builder()
        .resolve(FIXTURE_HOST, origin.address)
        .add_root_certificate(origin.root.clone())
        .add_root_certificate(target.root.clone())
        .build()
        .unwrap();
    let response = client
        .get(format!("https://{FIXTURE_HOST}:{}/metadata", origin.address.port()))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), http::StatusCode::OK);
    assert_eq!(&response.bytes().await.unwrap()[..], b"ok");
    assert!(origin.done.await.unwrap());
    assert!(target.done.await.unwrap());
}
