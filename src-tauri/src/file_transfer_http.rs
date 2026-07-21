use std::collections::HashMap;

use tokio::io::{AsyncRead, AsyncReadExt};

const MAX_HTTP_HEAD_SIZE: usize = 16 * 1024;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RangeError {
    #[error("HTTP header is incomplete")]
    IncompleteHead,
    #[error("HTTP header is too large")]
    HeadTooLarge,
    #[error("HTTP header is invalid")]
    InvalidHead,
    #[error("HTTP range is invalid")]
    InvalidRange,
    #[error("HTTP range is not satisfiable")]
    NotSatisfiable,
    #[error("HTTP download response does not match the requested range")]
    ResponseMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpHead {
    pub first_line: String,
    pub headers: HashMap<String, String>,
    pub body_prefix: Vec<u8>,
}

impl HttpHead {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }
}

pub fn parse_open_range(value: Option<&str>) -> Result<Option<u64>, RangeError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let offset = value
        .strip_prefix("bytes=")
        .and_then(|value| value.strip_suffix('-'))
        .filter(|value| !value.is_empty() && !value.contains([',', '-']))
        .ok_or(RangeError::InvalidRange)?
        .parse::<u64>()
        .map_err(|_| RangeError::InvalidRange)?;
    Ok(Some(offset))
}

pub fn parse_head_bytes(bytes: &[u8]) -> Result<HttpHead, RangeError> {
    if bytes.len() > MAX_HTTP_HEAD_SIZE && find_header_end(bytes).is_none() {
        return Err(RangeError::HeadTooLarge);
    }
    let end = find_header_end(bytes).ok_or(RangeError::IncompleteHead)?;
    if end > MAX_HTTP_HEAD_SIZE {
        return Err(RangeError::HeadTooLarge);
    }
    let text = std::str::from_utf8(&bytes[..end]).map_err(|_| RangeError::InvalidHead)?;
    let mut lines = text.split("\r\n");
    let first_line = lines
        .next()
        .filter(|line| !line.trim().is_empty())
        .ok_or(RangeError::InvalidHead)?
        .to_string();
    let mut headers = HashMap::new();
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(RangeError::InvalidHead)?;
        let name = name.trim().to_ascii_lowercase();
        if name.is_empty() {
            return Err(RangeError::InvalidHead);
        }
        headers.insert(name, value.trim().to_string());
    }
    Ok(HttpHead {
        first_line,
        headers,
        body_prefix: bytes[end + 4..].to_vec(),
    })
}

pub async fn read_head<R>(reader: &mut R) -> Result<HttpHead, RangeError>
where
    R: AsyncRead + Unpin,
{
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .await
            .map_err(|_| RangeError::IncompleteHead)?;
        if read == 0 {
            return Err(RangeError::IncompleteHead);
        }
        bytes.extend_from_slice(&buffer[..read]);
        if find_header_end(&bytes).is_some() {
            return parse_head_bytes(&bytes);
        }
        if bytes.len() > MAX_HTTP_HEAD_SIZE {
            return Err(RangeError::HeadTooLarge);
        }
    }
}

pub fn partial_content_header(
    size: u64,
    offset: u64,
    sha256: &str,
) -> Result<String, RangeError> {
    if offset >= size {
        return Err(RangeError::NotSatisfiable);
    }
    Ok(format!(
        "HTTP/1.1 206 Partial Content\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nAccept-Ranges: bytes\r\nETag: \"sha256:{}\"\r\nConnection: close\r\n\r\n",
        size - offset,
        offset,
        size - 1,
        size,
        sha256
    ))
}

pub fn range_not_satisfiable_header(size: u64) -> String {
    format!(
        "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{size}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}

pub fn validate_download_head(
    head: &HttpHead,
    expected_size: u64,
    expected_offset: u64,
    expected_sha256: &str,
) -> Result<(), RangeError> {
    let status = head
        .first_line
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or(RangeError::InvalidHead)?;
    if expected_offset > 0 && status != 206 {
        return Err(RangeError::ResponseMismatch);
    }
    if expected_offset == 0 && !matches!(status, 200 | 206) {
        return Err(RangeError::ResponseMismatch);
    }
    if expected_offset > expected_size {
        return Err(RangeError::NotSatisfiable);
    }

    let expected_length = expected_size - expected_offset;
    let content_length = head
        .header("content-length")
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or(RangeError::ResponseMismatch)?;
    if content_length != expected_length {
        return Err(RangeError::ResponseMismatch);
    }

    if status == 206 {
        if expected_offset >= expected_size {
            return Err(RangeError::NotSatisfiable);
        }
        let expected_content_range = format!(
            "bytes {}-{}/{}",
            expected_offset,
            expected_size - 1,
            expected_size
        );
        if head.header("content-range") != Some(expected_content_range.as_str()) {
            return Err(RangeError::ResponseMismatch);
        }
        let expected_etag = format!("\"sha256:{expected_sha256}\"");
        if head.header("etag") != Some(expected_etag.as_str()) {
            return Err(RangeError::ResponseMismatch);
        }
    } else if let Some(etag) = head.header("etag") {
        let expected_etag = format!("\"sha256:{expected_sha256}\"");
        if etag != expected_etag {
            return Err(RangeError::ResponseMismatch);
        }
    }
    Ok(())
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_range_accepts_only_a_single_start_offset() {
        assert_eq!(parse_open_range(None).unwrap(), None);
        assert_eq!(parse_open_range(Some("bytes=0-")).unwrap(), Some(0));
        assert_eq!(
            parse_open_range(Some("bytes=734003200-")).unwrap(),
            Some(734003200)
        );
        assert!(parse_open_range(Some("bytes=-500")).is_err());
        assert!(parse_open_range(Some("bytes=0-499")).is_err());
        assert!(parse_open_range(Some("bytes=0-,500-")).is_err());
    }

    #[test]
    fn http_head_normalizes_names_and_preserves_body_prefix() {
        let raw = b"HTTP/1.1 206 Partial Content\r\nContent-Length: 4\r\nETag: \"sha256:abcd\"\r\n\r\ndata";
        let head = parse_head_bytes(raw).unwrap();

        assert_eq!(head.first_line, "HTTP/1.1 206 Partial Content");
        assert_eq!(head.header("content-length"), Some("4"));
        assert_eq!(head.header("etag"), Some("\"sha256:abcd\""));
        assert_eq!(head.body_prefix, b"data");
    }

    #[test]
    fn partial_response_matches_exact_resume_metadata() {
        let header = partial_content_header(100, 40, "abcd").unwrap();
        let head = parse_head_bytes(format!("{header}data").as_bytes()).unwrap();

        validate_download_head(&head, 100, 40, "abcd").unwrap();
        assert!(validate_download_head(&head, 100, 39, "abcd").is_err());
        assert!(validate_download_head(&head, 100, 40, "different").is_err());
    }

    #[test]
    fn nonzero_resume_rejects_a_full_response() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\n";
        let head = parse_head_bytes(raw).unwrap();

        assert!(validate_download_head(&head, 100, 40, "abcd").is_err());
    }

    #[test]
    fn range_not_satisfiable_reports_the_source_size() {
        let header = range_not_satisfiable_header(100);
        assert!(header.starts_with("HTTP/1.1 416 Range Not Satisfiable\r\n"));
        assert!(header.contains("Content-Range: bytes */100\r\n"));
    }

    #[tokio::test]
    async fn async_head_reader_keeps_bytes_already_read_from_the_body() {
        use tokio::io::AsyncWriteExt;

        let (mut client, mut server) = tokio::io::duplex(1024);
        client
            .write_all(b"GET /file-transfer HTTP/1.1\r\nRange: bytes=4-\r\n\r\ndata")
            .await
            .unwrap();
        drop(client);

        let head = read_head(&mut server).await.unwrap();

        assert_eq!(head.first_line, "GET /file-transfer HTTP/1.1");
        assert_eq!(parse_open_range(head.header("range")).unwrap(), Some(4));
        assert_eq!(head.body_prefix, b"data");
    }
}
