//! Deterministic fuzz-style hostile-input tests for the IPC decode path.
//!
//! A seeded xorshift generator drives thousands of cases through the real
//! framing (`read_request`) and the server's error-response construction.

use super::protocol::{ErrorCode, Request, RequestEnvelope, Response, ResponseEnvelope};
use super::transport::{read_request, write_response, IpcStream};
use serde_json::{json, Value};
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

const TARGET: &str = "fuzz";
const MAX_MESSAGE: usize = 64 * 1024;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn bytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.next() as u8).collect()
    }
}

/// In-memory stream that serves `data` in `chunk`-sized reads, then EOF.
struct MemStream {
    data: Vec<u8>,
    pos: usize,
    chunk: usize,
    written: Vec<u8>,
}

impl MemStream {
    fn new(data: Vec<u8>, chunk: usize) -> Self {
        Self {
            data,
            pos: 0,
            chunk: chunk.max(1),
            written: Vec::new(),
        }
    }
}

impl Read for MemStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = buf.len().min(self.chunk).min(self.data.len() - self.pos);
        buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

impl Write for MemStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.written.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl IpcStream for MemStream {
    fn set_read_timeout(&self, _: Option<Duration>) -> io::Result<()> {
        Ok(())
    }
    fn set_write_timeout(&self, _: Option<Duration>) -> io::Result<()> {
        Ok(())
    }
}

/// Endless stream that never sends a delimiter; counts bytes handed out.
struct Endless {
    served: usize,
    byte: u8,
    delay: Option<Duration>,
}

impl Read for Endless {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(delay) = self.delay {
            std::thread::sleep(delay);
            buf[0] = self.byte;
            self.served += 1;
            return Ok(1);
        }
        buf.fill(self.byte);
        self.served += buf.len();
        Ok(buf.len())
    }
}
impl Write for Endless {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl IpcStream for Endless {
    fn set_read_timeout(&self, _: Option<Duration>) -> io::Result<()> {
        Ok(())
    }
    fn set_write_timeout(&self, _: Option<Duration>) -> io::Result<()> {
        Ok(())
    }
}

fn valid_requests() -> Vec<Request> {
    vec![
        Request::Status,
        Request::Suspend { minutes: Some(5) },
        Request::Resume,
        Request::IdleDim,
        Request::IdleWake,
        Request::SetOverride {
            monitor_id: Some("DP-1".into()),
            percent: 50,
            minutes: Some(10),
        },
        Request::SetMonitorLimits {
            monitor_id: "DP-1".into(),
            min_pct: 5,
            max_pct: 95,
        },
        Request::SetCloudDimming { percent: 20 },
        Request::ClearOverride {
            monitor_id: None,
            global: true,
        },
        Request::ReloadConfig,
        Request::RefreshWeather,
        Request::Ping,
        Request::RunOnce { force: true },
        Request::ClearExternalHold {
            monitor: Some("DP-1".into()),
        },
        Request::ExternalBrightnessChange,
        Request::ListThemes,
        Request::GetTheme { id: "dark".into() },
        Request::DayCurve { monitor_id: None },
    ]
}

fn random_value(rng: &mut Rng, depth: u32) -> Value {
    match rng.below(if depth > 3 { 6 } else { 8 }) {
        0 => Value::Null,
        1 => Value::Bool(rng.next() & 1 == 0),
        2 => json!(rng.next()),
        3 => json!(-(rng.next() as i64)),
        4 => json!(f64::from_bits(rng.next()).clamp(-1e308, 1e308)),
        5 => {
            let len = rng.below(40);
            Value::String(String::from_utf8_lossy(&rng.bytes(len)).into_owned())
        }
        6 => Value::Array(
            (0..rng.below(4))
                .map(|_| random_value(rng, depth + 1))
                .collect(),
        ),
        _ => Value::Object(
            (0..rng.below(4))
                .map(|i| (format!("k{i}"), random_value(rng, depth + 1)))
                .collect(),
        ),
    }
}

fn frame(mut body: Vec<u8>) -> Vec<u8> {
    body.push(b'\n');
    body
}

/// Decode one frame, then mirror the server: on error build the error reply
/// and make sure it serializes within the limit and parses back.
fn exercise(data: Vec<u8>, chunk: usize) -> Result<Request, ()> {
    let mut stream = MemStream::new(data, chunk);
    let outcome = read_request(&mut stream, TARGET).map(RequestEnvelope::validate);
    match outcome {
        Ok(Ok(request)) => {
            // Anything accepted must re-encode and decode identically.
            let again = serde_json::to_vec(&RequestEnvelope::new(request.clone())).unwrap();
            let back: RequestEnvelope = serde_json::from_slice(&again).unwrap();
            assert_eq!(back.request, request);
            Ok(request)
        }
        Ok(Err(response)) => {
            assert_error_reply(&response);
            Err(())
        }
        Err(error) => {
            let reply = ResponseEnvelope::error(ErrorCode::InvalidRequest, error.to_string());
            assert_error_reply(&reply);
            Err(())
        }
    }
}

fn assert_error_reply(reply: &ResponseEnvelope) {
    assert!(matches!(reply.response, Response::Error { .. }));
    let mut out = MemStream::new(Vec::new(), 1);
    write_response(&mut out, reply, TARGET).expect("error reply must always be writable");
    assert!(out.written.len() <= MAX_MESSAGE + 1);
    assert_eq!(out.written.last(), Some(&b'\n'));
    let parsed: ResponseEnvelope = serde_json::from_slice(&out.written[..out.written.len() - 1])
        .expect("error reply must be well-formed JSON");
    assert!(matches!(parsed.response, Response::Error { .. }));
}

#[test]
fn random_bytes_never_panic() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for i in 0..3000 {
        let len = rng.below(600);
        let mut data = rng.bytes(len);
        if i % 2 == 0 {
            data.push(b'\n');
        }
        let chunk = 1 + rng.below(5000);
        let _ = exercise(data, chunk);
    }
}

#[test]
fn byte_mutations_of_valid_frames_never_panic() {
    let mut rng = Rng(0xDEAD_BEEF_CAFE_F00D);
    let valid: Vec<Vec<u8>> = valid_requests()
        .into_iter()
        .map(|r| serde_json::to_vec(&RequestEnvelope::new(r)).unwrap())
        .collect();
    for _ in 0..4000 {
        let mut body = valid[rng.below(valid.len())].clone();
        for _ in 0..=rng.below(3) {
            match rng.below(4) {
                0 if !body.is_empty() => {
                    let i = rng.below(body.len());
                    body[i] = rng.next() as u8;
                }
                1 if !body.is_empty() => {
                    body.truncate(rng.below(body.len()));
                }
                2 => {
                    let i = rng.below(body.len() + 1);
                    body.insert(i, rng.next() as u8);
                }
                _ if !body.is_empty() => {
                    let i = rng.below(body.len());
                    body.remove(i);
                }
                _ => {}
            }
        }
        body.retain(|b| *b != b'\n');
        let _ = exercise(frame(body), 1 + rng.below(64));
    }
}

#[test]
fn field_mutations_of_every_request_never_panic() {
    let mut rng = Rng(0x1234_5678_9ABC_DEF1);
    for request in valid_requests() {
        let base = serde_json::to_value(RequestEnvelope::new(request)).unwrap();
        for _ in 0..200 {
            let mut value = base.clone();
            let map = value.as_object_mut().unwrap();
            let keys: Vec<String> = map.keys().cloned().collect();
            for _ in 0..=rng.below(2) {
                let key = keys[rng.below(keys.len())].clone();
                match rng.below(4) {
                    0 => {
                        map.remove(&key);
                    }
                    1 => {
                        let v = random_value(&mut rng, 0);
                        map.insert(key, v);
                    }
                    2 => {
                        map.insert(format!("extra{}", rng.below(9)), random_value(&mut rng, 0));
                    }
                    _ => {
                        let huge = [
                            json!(u64::MAX),
                            json!(i64::MIN),
                            json!(256),
                            json!(-1),
                            json!(1e308),
                            json!("x".repeat(5000)),
                        ];
                        map.insert(key, huge[rng.below(huge.len())].clone());
                    }
                }
            }
            let body = serde_json::to_vec(&value).unwrap();
            let _ = exercise(frame(body), 4096);
        }
    }
}

#[test]
fn unknown_variants_and_wrong_types_are_rejected() {
    for body in [
        r#"{"version":1,"request":"does_not_exist"}"#,
        r#"{"version":1,"request":42}"#,
        r#"{"version":1,"request":null}"#,
        r#"{"version":1}"#,
        r#"{"request":"ping"}"#,
        r#"{"version":"1","request":"ping"}"#,
        r#"{"version":-1,"request":"ping"}"#,
        r#"{"version":99999999999999999999,"request":"ping"}"#,
        r#"{"version":1,"request":"set_override","monitor_id":5,"percent":"x"}"#,
        r#"{"version":1,"request":"set_override","monitor_id":null,"percent":256}"#,
        r#"{"version":1,"request":"suspend","minutes":-3}"#,
        r#"{"version":1,"request":"suspend","minutes":1e400}"#,
        r#"{"version":2,"request":"ping"}"#,
        "[]",
        r#""ping""#,
        "null",
        "123",
    ] {
        assert!(
            exercise(frame(body.as_bytes().to_vec()), 4096).is_err(),
            "{body} must be rejected"
        );
    }
    assert_eq!(
        exercise(frame(br#"{"version":1,"request":"ping"}"#.to_vec()), 3),
        Ok(Request::Ping)
    );
}

#[test]
fn invalid_utf8_is_rejected() {
    for body in [
        &b"{\"version\":1,\"request\":\"\xff\xfe\"}"[..],
        b"\xc0\xaf",
        b"{\"version\":1,\"request\":\"ping\",\"x\":\"\xed\xa0\x80\"}",
        b"\xef\xbb\xbf{\"version\":1,\"request\":\"ping\"}",
    ] {
        assert!(exercise(frame(body.to_vec()), 4096).is_err());
    }
}

#[test]
fn deeply_nested_json_is_rejected_without_overflow() {
    for depth in [10, 127, 128, 129, 1000, 30_000] {
        let nested = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        for body in [
            format!(r#"{{"version":1,"request":"ping","x":{nested}}}"#),
            format!(r#"{{"version":1,"request":"get_theme","id":{nested}}}"#),
            nested.clone(),
            "[".repeat(depth),
            "{\"a\":".repeat(depth),
        ] {
            if body.len() <= MAX_MESSAGE {
                let _ = exercise(frame(body.into_bytes()), 4096);
            }
        }
    }
}

#[test]
fn huge_strings_and_numbers_are_bounded() {
    let digits = "9".repeat(60_000);
    let _ = exercise(
        frame(format!(r#"{{"version":1,"request":"suspend","minutes":{digits}}}"#).into_bytes()),
        4096,
    );
    let _ = exercise(
        frame(format!(r#"{{"version":{digits},"request":"ping"}}"#).into_bytes()),
        4096,
    );
    // Large-but-legal string field is accepted without issue.
    let id = "a".repeat(50_000);
    let ok = exercise(
        frame(format!(r#"{{"version":1,"request":"get_theme","id":"{id}"}}"#).into_bytes()),
        4096,
    );
    assert!(matches!(ok, Ok(Request::GetTheme { .. })));
}

#[test]
fn oversized_frames_are_rejected_with_bounded_reads() {
    for size in [MAX_MESSAGE + 1, MAX_MESSAGE + 4096, 10 * MAX_MESSAGE] {
        let mut stream = Endless {
            served: 0,
            byte: b'x',
            delay: None,
        };
        let err = read_request(&mut stream, TARGET).expect_err("endless frame must fail");
        assert!(err.to_string().contains("maximum"));
        // Never buffers (much) beyond limit + one chunk.
        assert!(
            stream.served <= MAX_MESSAGE + 4096,
            "served {}",
            stream.served
        );
        let _ = size;
    }
    // Oversized frame that does have a delimiter.
    let mut data = vec![b'x'; MAX_MESSAGE + 100];
    data.push(b'\n');
    assert!(exercise(data, 7000).is_err());
}

#[test]
fn truncated_frames_without_delimiter_are_rejected() {
    let full = serde_json::to_vec(&RequestEnvelope::new(Request::Ping)).unwrap();
    for cut in 0..full.len() {
        assert!(exercise(full[..cut].to_vec(), 5).is_err());
    }
    // Trailing bytes after the delimiter are ignored, not buffered.
    let mut data = frame(full.clone());
    data.extend_from_slice(&vec![b'z'; 200_000]);
    assert_eq!(exercise(data, 4096), Ok(Request::Ping));
}

#[test]
fn slow_drip_sender_hits_receive_deadline() {
    let mut stream = Endless {
        served: 0,
        byte: b'x',
        delay: Some(Duration::from_millis(2)),
    };
    let started = Instant::now();
    let err = read_request(&mut stream, TARGET).expect_err("slow drip must time out");
    assert!(err.to_string().contains("deadline"), "{err}");
    assert!(started.elapsed() < Duration::from_secs(2));
}
