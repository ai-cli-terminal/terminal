//! 최소 HTTP 전송 추상화 (Phase 2). 백엔드 로직을 네트워크와 분리해 테스트 가능하게 한다.
//!
//! [`HttpTransport`]를 주입하면 요청 빌드/응답 파싱을 오프라인에서 검증할 수 있다.
//! [`TcpTransport`]는 의존성 없는 평문 HTTP/1.1 구현(예: 로컬 Ollama). HTTPS는 미지원.

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

/// 의존성 없는 평문 HTTP/1.1 비동기 전송(로컬호스트 등 비-TLS 전용).
///
/// 진짜 async I/O이므로 상위에서 future를 drop하면(타임아웃/취소) 연결도 함께 취소된다.
/// HTTPS(TLS)는 `tls` feature의 별도 transport에서 지원한다.
pub struct TcpTransport;

impl HttpTransport for TcpTransport {
    async fn post_json(&self, url: &str, body: &str, bearer: Option<&str>) -> Result<String> {
        let (host, port, path) = parse_http_url(url)?;
        let mut stream = TcpStream::connect((host.as_str(), port)).await?;
        let auth = match bearer {
            Some(token) => format!("Authorization: Bearer {token}\r\n"),
            None => String::new(),
        };
        let req = format!(
            "POST {path} HTTP/1.1\r\nHost: {host}:{port}\r\nContent-Type: application/json\r\n\
             {auth}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(req.as_bytes()).await?;
        let mut resp = String::new();
        stream.read_to_string(&mut resp).await?;
        // 헤더/본문 분리
        let body = resp
            .split_once("\r\n\r\n")
            .map(|(_, b)| b.to_string())
            .ok_or_else(|| anyhow!("malformed HTTP response"))?;
        Ok(body)
    }
}

/// `http://host:port/path` 를 (host, port, path)로 파싱한다. TLS(https)는 거부.
pub fn parse_http_url(url: &str) -> Result<(String, u16, String)> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| anyhow!("only plain http:// supported (got {url})"))?;
    let (authority, path) = match rest.split_once('/') {
        Some((a, p)) => (a, format!("/{p}")),
        None => (rest, "/".to_string()),
    };
    let (host, port) = match authority.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse().map_err(|_| anyhow!("bad port"))?),
        None => (authority.to_string(), 80),
    };
    Ok((host, port, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_http_url() {
        assert_eq!(
            parse_http_url("http://localhost:11434/api/generate").unwrap(),
            ("localhost".into(), 11434, "/api/generate".into())
        );
        assert_eq!(
            parse_http_url("http://example.com").unwrap(),
            ("example.com".into(), 80, "/".into())
        );
    }

    #[test]
    fn rejects_https() {
        assert!(parse_http_url("https://x.com").is_err());
    }
}
