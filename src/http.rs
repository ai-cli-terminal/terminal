//! 최소 HTTP 전송 추상화 (Phase 2). 백엔드 로직을 네트워크와 분리해 테스트 가능하게 한다.
//!
//! [`HttpTransport`]를 주입하면 요청 빌드/응답 파싱을 오프라인에서 검증할 수 있다.
//! [`TcpTransport`]는 `tokio::net::TcpStream` 기반 비동기 전송이다. `http://`는 평문,
//! `https://`는 `tls` feature(tokio-rustls/ring)에서 TLS로 처리한다(없으면 명확히 거부).

use anyhow::{anyhow, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// JSON 본문을 POST하고 응답 본문을 돌려준다. `bearer`가 있으면 Authorization 헤더 추가.
///
/// async 트레이트(AFIT). future는 current-thread 런타임에서만 쓰이므로 `Send`를
/// 요구하지 않는다([`crate::gateway::Gateway::ask`]가 `block_on`으로 구동).
#[allow(async_fn_in_trait)]
pub trait HttpTransport {
    async fn post_json(&self, url: &str, body: &str, bearer: Option<&str>) -> Result<String>;
}

/// URL 스킴.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Http,
    Https,
}

/// `tokio::net::TcpStream` 기반 비동기 HTTP/1.1 전송.
///
/// 진짜 async I/O이므로 상위에서 future를 drop하면(타임아웃/취소) 연결도 함께 취소된다.
/// `https://`는 `tls` feature에서 TLS(tokio-rustls/ring)로 처리한다.
pub struct TcpTransport;

impl HttpTransport for TcpTransport {
    async fn post_json(&self, url: &str, body: &str, bearer: Option<&str>) -> Result<String> {
        let (scheme, host, port, path) = parse_url(url)?;
        let req = build_request(&host_header(scheme, &host, port), &path, body, bearer);
        match scheme {
            Scheme::Http => {
                let mut stream = TcpStream::connect((host.as_str(), port)).await?;
                stream.write_all(req.as_bytes()).await?;
                let mut resp = String::new();
                stream.read_to_string(&mut resp).await?;
                extract_body(&resp)
            }
            Scheme::Https => post_json_tls(&host, port, &req).await,
        }
    }
}

/// HTTP/1.1 POST 요청 문자열을 만든다(`Connection: close`로 EOF까지 읽기).
fn build_request(host_header: &str, path: &str, body: &str, bearer: Option<&str>) -> String {
    let auth = match bearer {
        Some(token) => format!("Authorization: Bearer {token}\r\n"),
        None => String::new(),
    };
    format!(
        "POST {path} HTTP/1.1\r\nHost: {host_header}\r\nContent-Type: application/json\r\n\
         {auth}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

/// 응답에서 헤더를 떼고 본문만 돌려준다.
fn extract_body(resp: &str) -> Result<String> {
    resp.split_once("\r\n\r\n")
        .map(|(_, b)| b.to_string())
        .ok_or_else(|| anyhow!("malformed HTTP response"))
}

/// Host 헤더값(기본 포트면 포트 생략).
fn host_header(scheme: Scheme, host: &str, port: u16) -> String {
    let is_default =
        (scheme == Scheme::Http && port == 80) || (scheme == Scheme::Https && port == 443);
    if is_default {
        host.to_string()
    } else {
        format!("{host}:{port}")
    }
}

/// TLS(https) POST — `tls` feature에서만 동작한다.
#[cfg(feature = "tls")]
async fn post_json_tls(host: &str, port: u16, req: &str) -> Result<String> {
    use std::sync::Arc;

    use tokio_rustls::rustls::pki_types::ServerName;
    use tokio_rustls::rustls::{ClientConfig, RootCertStore};
    use tokio_rustls::TlsConnector;

    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = ClientConfig::builder_with_provider(Arc::new(
        tokio_rustls::rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| anyhow!("tls config: {e}"))?
    .with_root_certificates(root_store)
    .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(config));
    let server_name =
        ServerName::try_from(host.to_string()).map_err(|e| anyhow!("bad tls host {host}: {e}"))?;

    let tcp = TcpStream::connect((host, port)).await?;
    let mut tls = connector.connect(server_name, tcp).await?;
    tls.write_all(req.as_bytes()).await?;
    let mut resp = String::new();
    tls.read_to_string(&mut resp).await?;
    extract_body(&resp)
}

/// TLS 미지원 빌드: https 요청을 명확히 거부한다(조용한 실패 금지).
#[cfg(not(feature = "tls"))]
async fn post_json_tls(_host: &str, _port: u16, _req: &str) -> Result<String> {
    Err(anyhow!(
        "https는 `tls` feature 빌드가 필요합니다 (cargo build --features tls)"
    ))
}

/// `http(s)://host:port/path` 를 (scheme, host, port, path)로 파싱한다.
pub fn parse_url(url: &str) -> Result<(Scheme, String, u16, String)> {
    let (scheme, rest, default_port) = if let Some(r) = url.strip_prefix("https://") {
        (Scheme::Https, r, 443u16)
    } else if let Some(r) = url.strip_prefix("http://") {
        (Scheme::Http, r, 80u16)
    } else {
        return Err(anyhow!("only http:// or https:// supported (got {url})"));
    };
    let (authority, path) = match rest.split_once('/') {
        Some((a, p)) => (a, format!("/{p}")),
        None => (rest, "/".to_string()),
    };
    let (host, port) = match authority.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().map_err(|_| anyhow!("bad port"))?),
        None => (authority.to_string(), default_port),
    };
    Ok((scheme, host, port, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_http_url() {
        assert_eq!(
            parse_url("http://localhost:11434/api/generate").unwrap(),
            (
                Scheme::Http,
                "localhost".into(),
                11434,
                "/api/generate".into()
            )
        );
        assert_eq!(
            parse_url("http://example.com").unwrap(),
            (Scheme::Http, "example.com".into(), 80, "/".into())
        );
    }

    #[test]
    fn parses_https_with_default_port() {
        assert_eq!(
            parse_url("https://api.openai.com/v1/chat/completions").unwrap(),
            (
                Scheme::Https,
                "api.openai.com".into(),
                443,
                "/v1/chat/completions".into()
            )
        );
    }

    #[test]
    fn rejects_unknown_scheme() {
        assert!(parse_url("ftp://x.com").is_err());
        assert!(parse_url("x.com").is_err());
    }

    #[test]
    fn host_header_omits_default_port() {
        assert_eq!(
            host_header(Scheme::Https, "api.openai.com", 443),
            "api.openai.com"
        );
        assert_eq!(host_header(Scheme::Http, "h", 80), "h");
        assert_eq!(
            host_header(Scheme::Http, "localhost", 11434),
            "localhost:11434"
        );
    }

    #[test]
    fn build_request_includes_bearer_and_length() {
        let r = build_request("h", "/p", "{}", Some("tok"));
        assert!(r.contains("Authorization: Bearer tok"));
        assert!(r.contains("Content-Length: 2"));
        assert!(r.contains("Connection: close"));
    }
}
