//! Strict, bounded transport collection for required response-commit hooks.
//!
//! This deliberately does not use the tolerant streaming adapters: no usage,
//! session binding or successful completion may happen before validation.

use axum::body::Bytes;
use axum::http::{header, HeaderMap, HeaderValue};
use flate2::{Decompress, FlushDecompress, Status};
use std::time::{Duration, Instant};

use crate::gateway::plugins::before_commit::MAX_COMPLETE_RESPONSE_BYTES;

const MAX_ENCODED_RESPONSE_BYTES: usize = MAX_COMPLETE_RESPONSE_BYTES + 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CollectError {
    Read,
    Decode,
    UnsupportedEncoding,
    TooLarge,
    Deadline,
    FirstByteTimeout,
    IdleTimeout,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CollectedTiming {
    pub(super) first_byte_ms: Option<u128>,
    pub(super) completed_ms: u128,
}

pub(super) struct CompleteResponse {
    pub(super) body: Bytes,
    pub(super) headers: HeaderMap,
    pub(super) timing: CollectedTiming,
}

struct Decoder {
    gzip: Option<Decompress>,
    ended: bool,
    encoded_bytes: usize,
    output: Vec<u8>,
    limit: usize,
}

impl Decoder {
    fn new(headers: &HeaderMap, limit: usize) -> Result<Self, CollectError> {
        let mut encodings = headers.get_all(header::CONTENT_ENCODING).iter();
        let encoding = encodings
            .next()
            .map(|value| value.to_str().map(str::trim))
            .transpose()
            .map_err(|_| CollectError::UnsupportedEncoding)?;
        if encodings.next().is_some() {
            return Err(CollectError::UnsupportedEncoding);
        }
        let gzip = match encoding {
            None | Some("") => None,
            Some(value) if value.eq_ignore_ascii_case("identity") => None,
            Some(value) if value.eq_ignore_ascii_case("gzip") => Some(Decompress::new_gzip(15)),
            _ => return Err(CollectError::UnsupportedEncoding),
        };
        Ok(Self {
            gzip,
            ended: false,
            encoded_bytes: 0,
            output: Vec::new(),
            limit,
        })
    }

    fn push(&mut self, mut input: &[u8], eof: bool) -> Result<(), CollectError> {
        self.encoded_bytes = self.encoded_bytes.saturating_add(input.len());
        if self.encoded_bytes > MAX_ENCODED_RESPONSE_BYTES {
            return Err(CollectError::TooLarge);
        }
        if self.gzip.is_none() {
            if input.len() > self.limit.saturating_sub(self.output.len()) {
                return Err(CollectError::TooLarge);
            }
            self.output.extend_from_slice(input);
            return Ok(());
        }
        if self.ended {
            // Single-member gzip only. Never silently ignore a second member or
            // trailing bytes containing evidence that the plugin did not see.
            return if input.is_empty() {
                Ok(())
            } else {
                Err(CollectError::Decode)
            };
        }
        loop {
            let mut output = [0u8; 8192];
            let decoder = self.gzip.as_mut().expect("gzip decoder");
            let before_in = decoder.total_in();
            let before_out = decoder.total_out();
            let status = decoder
                .decompress(
                    input,
                    &mut output,
                    if eof {
                        FlushDecompress::Finish
                    } else {
                        FlushDecompress::None
                    },
                )
                .map_err(|_| CollectError::Decode)?;
            let consumed = (decoder.total_in() - before_in) as usize;
            let written = (decoder.total_out() - before_out) as usize;
            if written > self.limit.saturating_sub(self.output.len()) {
                return Err(CollectError::TooLarge);
            }
            self.output.extend_from_slice(&output[..written]);
            input = &input[consumed..];
            if status == Status::StreamEnd {
                self.ended = true;
                return if input.is_empty() {
                    Ok(())
                } else {
                    Err(CollectError::Decode)
                };
            }
            if consumed == 0 && written == 0 {
                return if eof || !input.is_empty() {
                    Err(CollectError::Decode)
                } else {
                    Ok(())
                };
            }
            if input.is_empty() && written < output.len() && !eof {
                return Ok(());
            }
        }
    }
}

pub(super) async fn collect(
    mut response: reqwest::Response,
    started: Instant,
    attempt_started: Instant,
    deadline: Instant,
    first_byte_timeout: Option<Duration>,
    idle_timeout: Option<Duration>,
) -> Result<CompleteResponse, CollectError> {
    let mut headers = response.headers().clone();
    let mut decoder = Decoder::new(&headers, MAX_COMPLETE_RESPONSE_BYTES)?;
    let mut first_byte_ms = None;
    let mut last_chunk = Instant::now();
    loop {
        let mut next_deadline = deadline;
        let mut timeout_error = CollectError::Deadline;
        let local_deadline = if first_byte_ms.is_none() {
            first_byte_timeout
                .map(|timeout| (attempt_started + timeout, CollectError::FirstByteTimeout))
        } else {
            idle_timeout.map(|timeout| (last_chunk + timeout, CollectError::IdleTimeout))
        };
        if let Some((local, error)) = local_deadline {
            if local < next_deadline {
                next_deadline = local;
                timeout_error = error;
            }
        }
        if Instant::now() >= next_deadline {
            return Err(timeout_error);
        }
        let chunk = tokio::time::timeout_at(next_deadline.into(), response.chunk())
            .await
            .map_err(|_| timeout_error)?
            .map_err(|_| CollectError::Read)?;
        let Some(chunk) = chunk else {
            decoder.push(&[], true)?;
            headers.remove(header::CONTENT_ENCODING);
            headers.remove(header::TRANSFER_ENCODING);
            headers.insert(
                header::CONTENT_LENGTH,
                HeaderValue::from_str(&decoder.output.len().to_string())
                    .expect("byte length header"),
            );
            return Ok(CompleteResponse {
                body: Bytes::from(decoder.output),
                headers,
                timing: CollectedTiming {
                    first_byte_ms,
                    completed_ms: started.elapsed().as_millis(),
                },
            });
        };
        if !chunk.is_empty() {
            last_chunk = Instant::now();
            first_byte_ms.get_or_insert_with(|| started.elapsed().as_millis());
            decoder.push(&chunk, false)?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::GzEncoder, Compression};
    use std::io::Write;

    fn gzip(input: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(input).unwrap();
        encoder.finish().unwrap()
    }

    fn decode(parts: &[&[u8]], limit: usize) -> Result<Vec<u8>, CollectError> {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
        let mut decoder = Decoder::new(&headers, limit)?;
        for part in parts {
            decoder.push(part, false)?;
        }
        decoder.push(&[], true)?;
        Ok(decoder.output)
    }

    #[test]
    fn gzip_checks_footer_and_consumes_transport_tail() {
        let compressed = gzip(b"complete response");
        let parts: Vec<&[u8]> = compressed.chunks(1).collect();
        assert_eq!(decode(&parts, 100).unwrap(), b"complete response");
        assert_eq!(
            decode(&[&compressed[..compressed.len() - 1]], 100),
            Err(CollectError::Decode)
        );
        let mut bad_crc = compressed.clone();
        let crc_offset = bad_crc.len() - 8;
        bad_crc[crc_offset] ^= 1;
        assert_eq!(decode(&[&bad_crc], 100), Err(CollectError::Decode));
        assert_eq!(
            decode(&[&compressed, b"trailing"], 100),
            Err(CollectError::Decode)
        );
        assert_eq!(
            decode(&[&compressed, &gzip(b"late conflict")], 100),
            Err(CollectError::Decode)
        );
    }

    #[test]
    fn decoded_limit_is_exact_even_for_compression_bombs() {
        let payload = vec![b'x'; 32 * 1024];
        let compressed = gzip(&payload);
        assert_eq!(decode(&[&compressed], payload.len()).unwrap(), payload);
        assert_eq!(
            decode(&[&compressed], payload.len() - 1),
            Err(CollectError::TooLarge)
        );
    }

    #[tokio::test]
    async fn expired_deadline_never_becomes_unlimited() {
        let response =
            reqwest::Response::from(axum::http::Response::new(reqwest::Body::from("ok")));
        let started = Instant::now()
            .checked_sub(Duration::from_secs(2))
            .expect("past instant");
        assert!(matches!(
            collect(
                response,
                started,
                started,
                started + Duration::from_secs(1),
                None,
                None
            )
            .await,
            Err(CollectError::Deadline)
        ));
    }
    #[tokio::test]
    async fn collector_enforces_decoded_cap_for_identity_and_gzip() {
        for len in [
            MAX_COMPLETE_RESPONSE_BYTES - 1,
            MAX_COMPLETE_RESPONSE_BYTES,
            MAX_COMPLETE_RESPONSE_BYTES + 1,
        ] {
            for compressed in [false, true] {
                let payload = vec![b'x'; len];
                let body = if compressed { gzip(&payload) } else { payload };
                let mut response = axum::http::Response::new(reqwest::Body::from(body));
                if compressed {
                    response
                        .headers_mut()
                        .insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
                }
                let now = Instant::now();
                let result = collect(
                    reqwest::Response::from(response),
                    now,
                    now,
                    now + Duration::from_secs(5),
                    None,
                    None,
                )
                .await;
                if len > MAX_COMPLETE_RESPONSE_BYTES {
                    assert!(matches!(result, Err(CollectError::TooLarge)));
                } else {
                    let result = result.unwrap();
                    assert_eq!(result.body.len(), len);
                    assert!(!result.headers.contains_key(header::CONTENT_ENCODING));
                    assert_eq!(result.headers[header::CONTENT_LENGTH], len.to_string());
                }
            }
        }
    }

    #[tokio::test]
    async fn collector_first_byte_and_idle_deadlines_fire_on_silent_transport() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        for prefix in [false, true] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let task = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = [0u8; 4096];
                assert!(socket.read(&mut request).await.unwrap() > 0);
                socket
                    .write_all(b"HTTP/1.1 200 OK\r\nconnection: close\r\n\r\n")
                    .await
                    .unwrap();
                if prefix {
                    socket.write_all(b"prefix").await.unwrap();
                }
                std::future::pending::<()>().await;
            });
            let response = reqwest::get(format!("http://{address}")).await.unwrap();
            // This collector test starts after headers; client construction and
            // connection setup must not consume its short first-byte budget.
            let now = Instant::now();
            let result = collect(
                response,
                now,
                now,
                now + Duration::from_secs(5),
                Some(Duration::from_millis(100)),
                Some(Duration::from_millis(100)),
            )
            .await;
            assert_eq!(
                result.err(),
                Some(if prefix {
                    CollectError::IdleTimeout
                } else {
                    CollectError::FirstByteTimeout
                }),
                "prefix={prefix}"
            );
            task.abort();
        }
    }
}
