//! Story C of accepted decision 0132 (`wait-best`): the stream wake-up
//! transports of `benches/mail_compare/clients/stream.rs`. Real loopback
//! TCP exchanges prove the protocols end to end; in-memory sockets drive
//! each malformed branch, every split point and cancellation, and the
//! paused tokio clock proves the absolute cutoffs (no sleep is used as
//! synchronization, 0020 §5).

use crate::stream::{
    Limits, Sse, SseEvent, SseItem, StreamError, WebSocket, WsMessage, accept_key, fresh_key,
    parse_head,
};
use base64::Engine as _;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream, duplex};
use tokio::net::TcpListener;
use tokio::time::Instant;

const SMALL: Limits = Limits {
    head: 256,
    line: 128,
    message: 64,
    buffer: 256,
};

fn far() -> Instant {
    Instant::now() + Duration::from_secs(30)
}

/// Read one HTTP request head from the server side.
async fn read_request<S: AsyncReadExt + Unpin>(s: &mut S) -> String {
    let mut got = Vec::new();
    let mut byte = [0u8; 1];
    while !got.ends_with(b"\r\n\r\n") {
        assert_eq!(
            s.read(&mut byte).await.unwrap(),
            1,
            "the request ended early"
        );
        got.push(byte[0]);
    }
    String::from_utf8(got).unwrap()
}

fn header<'a>(request: &'a str, name: &str) -> &'a str {
    request
        .lines()
        .find_map(|l| {
            let (n, v) = l.split_once(':')?;
            n.eq_ignore_ascii_case(name).then(|| v.trim())
        })
        .unwrap_or_else(|| panic!("no {name} in {request:?}"))
}

/// The server's correct 101 answer to `request`, with `extra` fields.
fn upgrade_answer(request: &str, extra: &str) -> String {
    let key = header(request, "Sec-WebSocket-Key");
    format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Accept: {}\r\n{extra}\r\n",
        accept_key(key)
    )
}

/// An unmasked server frame with a minimal length.
fn frame(b0: u8, payload: &[u8]) -> Vec<u8> {
    let mut f = vec![b0];
    match payload.len() {
        n if n < 126 => f.push(n as u8),
        n if n <= u16::MAX as usize => {
            f.push(126);
            f.extend_from_slice(&(n as u16).to_be_bytes());
        }
        n => {
            f.push(127);
            f.extend_from_slice(&(n as u64).to_be_bytes());
        }
    }
    f.extend_from_slice(payload);
    f
}

/// Read one client frame: (FIN and opcode byte, unmasked payload). Every
/// client frame must be masked.
async fn client_frame<S: AsyncReadExt + Unpin>(s: &mut S) -> (u8, Vec<u8>) {
    let mut h = [0u8; 2];
    s.read_exact(&mut h).await.unwrap();
    assert_eq!(h[1] & 0x80, 0x80, "a client frame is masked");
    let len = match h[1] & 0x7f {
        126 => s.read_u16().await.unwrap() as usize,
        127 => s.read_u64().await.unwrap() as usize,
        n => n as usize,
    };
    let mut mask = [0u8; 4];
    s.read_exact(&mut mask).await.unwrap();
    let mut p = vec![0u8; len];
    s.read_exact(&mut p).await.unwrap();
    p.iter_mut()
        .enumerate()
        .for_each(|(i, b)| *b ^= mask[i % 4]);
    (h[0], p)
}

/// A handshake over an in-memory socket whose server answers with
/// `answer(request)`; returns the client result and the server half.
async fn ws_with(
    limits: Limits,
    answer: impl FnOnce(&str) -> Vec<u8>,
) -> (Result<WebSocket<DuplexStream>, StreamError>, DuplexStream) {
    ws_over(1 << 16, limits, answer).await
}

/// [`ws_with`] over a socket that holds at most `capacity` bytes in flight.
async fn ws_over(
    capacity: usize,
    limits: Limits,
    answer: impl FnOnce(&str) -> Vec<u8>,
) -> (Result<WebSocket<DuplexStream>, StreamError>, DuplexStream) {
    let (client, mut server) = duplex(capacity);
    let deadline = far();
    let (ws, server) = tokio::join!(
        WebSocket::handshake(client, "127.0.0.1:1", "/api/events", deadline, limits),
        async move {
            let request = read_request(&mut server).await;
            server.write_all(&answer(&request)).await.unwrap();
            server
        }
    );
    (ws, server)
}

async fn ws_open(limits: Limits) -> (WebSocket<DuplexStream>, DuplexStream) {
    let (ws, server) = ws_with(limits, |r| upgrade_answer(r, "").into_bytes()).await;
    (ws.expect("the handshake"), server)
}

/// Feed `bytes` to a fresh socket and return the first outcome.
async fn ws_outcome(bytes: &[u8]) -> Result<WsMessage, StreamError> {
    let (mut ws, mut server) = ws_open(SMALL).await;
    server.write_all(bytes).await.unwrap();
    ws.next(far()).await
}

#[test]
fn the_accept_key_is_rfc_6455s_example() {
    // RFC 6455 §1.3's worked example.
    assert_eq!(
        accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
        "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
    );
}

#[test]
fn every_handshake_key_is_fresh_sixteen_random_bytes() {
    let a = fresh_key().unwrap();
    let b = fresh_key().unwrap();
    assert_ne!(a, b);
    for k in [a, b] {
        let raw = base64::engine::general_purpose::STANDARD
            .decode(&k)
            .unwrap();
        assert_eq!(raw.len(), 16, "{k}");
    }
}

#[test]
fn limits_refuse_what_the_transports_cannot_honour() {
    assert_eq!(Limits::DEFAULT.checked(), Ok(Limits::DEFAULT));
    assert_eq!(Limits::DEFAULT.head, 32 * 1024);
    assert_eq!(Limits::DEFAULT.line, 8 * 1024);
    assert_eq!(Limits::DEFAULT.message, 1024 * 1024);
    assert_eq!(Limits::DEFAULT.buffer, 2 * 1024 * 1024);
    for bad in [
        Limits { line: 0, ..SMALL },
        Limits {
            message: 0,
            ..SMALL
        },
        Limits { line: 300, ..SMALL },
        Limits { head: 300, ..SMALL },
        Limits {
            message: 250,
            ..SMALL
        },
        Limits {
            message: usize::MAX,
            ..SMALL
        },
    ] {
        assert_eq!(bad.checked(), Err(StreamError::Limits(bad)));
    }
}

#[tokio::test]
async fn a_real_loopback_websocket_exchanges_messages_pings_and_close() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let mut keys = Vec::new();
        for round in 0..2 {
            let (mut s, _) = listener.accept().await.unwrap();
            let request = read_request(&mut s).await;
            assert!(
                request.starts_with("GET /api/events HTTP/1.1\r\n"),
                "{request}"
            );
            assert_eq!(header(&request, "Sec-WebSocket-Version"), "13");
            assert_eq!(header(&request, "Upgrade"), "websocket");
            keys.push(header(&request, "Sec-WebSocket-Key").to_string());
            // The answer, coalesced with a first message.
            let mut out = upgrade_answer(&request, "").into_bytes();
            out.extend(frame(0x81, b"first"));
            s.write_all(&out).await.unwrap();
            if round == 1 {
                // The second connection is reset at once: a disconnect.
                drop(s);
                continue;
            }
            // A fragmented binary message with a ping between its parts.
            let mut out = frame(0x02, b"ab");
            out.extend(frame(0x89, b"are you there"));
            out.extend(frame(0x80, b"cd"));
            s.write_all(&out).await.unwrap();
            assert_eq!(
                client_frame(&mut s).await,
                (0x8A, b"are you there".to_vec())
            );
            // The client's own ping and text, then our pong.
            assert_eq!(client_frame(&mut s).await, (0x89, b"ready?".to_vec()));
            assert_eq!(client_frame(&mut s).await, (0x81, b"subscribe".to_vec()));
            s.write_all(&frame(0x8A, b"ready?")).await.unwrap();
            let mut close = 1000u16.to_be_bytes().to_vec();
            close.extend_from_slice(b"bye");
            s.write_all(&frame(0x88, &close)).await.unwrap();
            assert_eq!(
                client_frame(&mut s).await,
                (0x88, 1000u16.to_be_bytes().to_vec())
            );
        }
        keys
    });

    let deadline = far();
    let tcp = crate::stream::connect(addr, deadline).await.unwrap();
    let mut ws = WebSocket::handshake(
        tcp,
        &addr.to_string(),
        "/api/events",
        deadline,
        Limits::DEFAULT,
    )
    .await
    .unwrap();
    assert_eq!(ws.next(deadline).await, Ok(WsMessage::Text("first".into())));
    assert_eq!(
        ws.next(deadline).await,
        Ok(WsMessage::Binary(b"abcd".to_vec()))
    );
    ws.ping(b"ready?", deadline).await.unwrap();
    ws.send_text("subscribe", deadline).await.unwrap();
    assert_eq!(
        ws.next(deadline).await,
        Ok(WsMessage::Pong(b"ready?".to_vec()))
    );
    let closed = ws.next(deadline).await.unwrap_err();
    assert_eq!(closed, StreamError::Closed);
    assert!(closed.is_disconnect());
    assert_eq!(ws.close_code(), Some(Some(1000)));
    assert_eq!(
        ws.send_text("late", deadline).await,
        Err(StreamError::Closed)
    );
    assert_eq!(ws.next(deadline).await, Err(StreamError::Closed));

    let tcp = crate::stream::connect(addr, deadline).await.unwrap();
    let mut ws = WebSocket::handshake(
        tcp,
        &addr.to_string(),
        "/api/events",
        deadline,
        Limits::DEFAULT,
    )
    .await
    .unwrap();
    assert_eq!(ws.next(deadline).await, Ok(WsMessage::Text("first".into())));
    let gone = ws.next(deadline).await.unwrap_err();
    assert!(gone.is_disconnect(), "{gone}");

    let keys = server.await.unwrap();
    assert_ne!(keys[0], keys[1], "each handshake draws a fresh key");
}

#[tokio::test]
async fn a_refused_connection_and_a_passed_deadline_are_named() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let err = crate::stream::connect(addr, far()).await.unwrap_err();
    assert!(matches!(err, StreamError::Io { .. }), "{err}");
    assert!(!err.is_disconnect(), "a refused connect is not a lost one");
    let past = Instant::now() - Duration::from_millis(1);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    assert_eq!(
        crate::stream::connect(addr, past).await.unwrap_err(),
        StreamError::Deadline
    );
}

/// A test server's whole answer to the handshake request it read.
type Answer = Box<dyn FnOnce(&str) -> Vec<u8>>;

#[tokio::test]
async fn a_handshake_without_proof_of_upgrade_is_refused() {
    let cases: Vec<(&str, Answer, &str)> = vec![
        (
            "status",
            Box::new(|_| b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n".to_vec()),
            "unexpected status 200",
        ),
        (
            "upgrade",
            Box::new(|r| {
                upgrade_answer(r, "")
                    .replace("Upgrade: websocket\r\n", "")
                    .into_bytes()
            }),
            "no Upgrade: websocket",
        ),
        (
            "connection",
            Box::new(|r| {
                upgrade_answer(r, "")
                    .replace("Connection: Upgrade", "Connection: keep-alive")
                    .into_bytes()
            }),
            "no Connection: upgrade",
        ),
        (
            "wrong accept",
            Box::new(|_| {
                format!(
                    "HTTP/1.1 101 x\r\nUpgrade: websocket\r\nConnection: upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
                    accept_key("dGhlIHNhbXBsZSBub25jZQ==")
                )
                .into_bytes()
            }),
            "is not derived from the request's key",
        ),
        (
            "no accept",
            Box::new(|_| {
                b"HTTP/1.1 101 x\r\nUpgrade: websocket\r\nConnection: upgrade\r\n\r\n".to_vec()
            }),
            "no Sec-WebSocket-Accept",
        ),
        (
            "two accepts",
            Box::new(|r| {
                let a = header(r, "Sec-WebSocket-Key");
                upgrade_answer(r, &format!("Sec-WebSocket-Accept: {}\r\n", accept_key(a)))
                    .into_bytes()
            }),
            "sec-websocket-accept is repeated",
        ),
        (
            "extension",
            Box::new(|r| {
                upgrade_answer(r, "Sec-WebSocket-Extensions: permessage-deflate\r\n").into_bytes()
            }),
            "unsupported negotiation: sec-websocket-extensions: permessage-deflate",
        ),
        (
            "subprotocol",
            Box::new(|r| upgrade_answer(r, "Sec-WebSocket-Protocol: chat\r\n").into_bytes()),
            "unsupported negotiation: sec-websocket-protocol: chat",
        ),
        (
            "version",
            Box::new(|_| b"HTTP/1.0 101 x\r\n\r\n".to_vec()),
            "not HTTP/1.1",
        ),
        (
            "status code",
            Box::new(|_| b"HTTP/1.1 1010 x\r\n\r\n".to_vec()),
            "bad status code",
        ),
        (
            "colon",
            Box::new(|_| b"HTTP/1.1 101 x\r\nUpgrade websocket\r\n\r\n".to_vec()),
            "header without a colon",
        ),
        (
            "name",
            Box::new(|_| b"HTTP/1.1 101 x\r\nBad Name: v\r\n\r\n".to_vec()),
            "bad header name",
        ),
        (
            "fold",
            Box::new(|_| b"HTTP/1.1 101 x\r\nA: b\r\n c\r\n\r\n".to_vec()),
            "folded or broken header line",
        ),
        (
            "bare cr",
            Box::new(|_| b"HTTP/1.1 101 x\r\nA: b\rc\r\n\r\n".to_vec()),
            "folded or broken header line",
        ),
        (
            "utf-8",
            Box::new(|_| b"HTTP/1.1 101 x\r\nA: \xff\r\n\r\n".to_vec()),
            "response head is not UTF-8",
        ),
        (
            "long line",
            Box::new(|_| format!("HTTP/1.1 101 x\r\nA: {}\r\n", "v".repeat(200)).into_bytes()),
            "header line exceeds its 128-byte limit",
        ),
        (
            "long head",
            Box::new(|_| {
                "HTTP/1.1 101 x\r\n"
                    .bytes()
                    .chain("A: b\r\n".repeat(60).bytes())
                    .collect()
            }),
            "response head exceeds its 256-byte limit",
        ),
        (
            "long head ending",
            Box::new(|_| {
                let mut h = "HTTP/1.1 101 x\r\n".to_string() + &"A: b\r\n".repeat(40);
                h.push_str("\r\n");
                h.into_bytes()
            }),
            "response head exceeds its 256-byte limit",
        ),
    ];
    for (name, answer, want) in cases {
        let (ws, _server) = ws_with(SMALL, answer).await;
        let err = ws.err().unwrap_or_else(|| panic!("{name}: accepted"));
        assert!(err.to_string().contains(want), "{name}: {err}");
        assert!(!err.is_disconnect(), "{name}: {err}");
    }
    // A head that ends before its blank line is a lost connection.
    let (client, mut server) = duplex(1 << 12);
    let ((), ws) = tokio::join!(
        async move {
            read_request(&mut server).await;
            server.write_all(b"HTTP/1.1 101 x\r\n").await.unwrap();
        },
        WebSocket::handshake(client, "h", "/", far(), SMALL)
    );
    let err = ws.err().unwrap();
    assert_eq!(
        err,
        StreamError::Eof {
            at: "before the response head ended"
        }
    );
    assert!(err.is_disconnect());
    // A head line over the limit is refused before its head ends.
    let (client, mut server) = duplex(1 << 12);
    let (_server, ws) = tokio::join!(
        async move {
            read_request(&mut server).await;
            server.write_all(&[b'x'; 200]).await.unwrap();
            server
        },
        WebSocket::handshake(client, "h", "/", far(), SMALL)
    );
    assert_eq!(
        ws.err().unwrap(),
        StreamError::TooLarge {
            what: "header line",
            limit: 128
        }
    );
    // Limits are checked before anything is written.
    let (client, _server) = duplex(64);
    let bad = Limits { line: 0, ..SMALL };
    assert_eq!(
        WebSocket::handshake(client, "h", "/", far(), bad)
            .await
            .err(),
        Some(StreamError::Limits(bad))
    );
}

#[test]
fn a_head_parses_tokens_and_single_values() {
    let head = parse_head(
        b"HTTP/1.1 101 Switching\r\nConnection: keep-alive, Upgrade\r\nconnection: x\r\nA:\t v \r\n",
        Limits::DEFAULT,
    )
    .unwrap();
    assert_eq!(head.status, 101);
    assert_eq!(head.tokens("connection"), ["keep-alive", "upgrade", "x"]);
    assert_eq!(head.single("a"), Ok(Some("v")));
    assert_eq!(head.single("missing"), Ok(None));
    assert!(head.single("connection").is_err());
    assert_eq!(
        parse_head(b"HTTP/1.1 101 x", Limits::DEFAULT),
        Err(StreamError::Head("missing CRLF".into()))
    );
}

#[tokio::test]
async fn frames_that_break_rfc_6455_are_refused_by_name() {
    let mut long_ping = vec![0x89, 126];
    long_ping.extend_from_slice(&200u16.to_be_bytes());
    let mut short16 = vec![0x81, 126];
    short16.extend_from_slice(&5u16.to_be_bytes());
    let mut short64 = vec![0x81, 127];
    short64.extend_from_slice(&200u64.to_be_bytes());
    let mut top64 = vec![0x82, 127];
    top64.extend_from_slice(&(1u64 << 63 | 70_000).to_be_bytes());
    let mut huge = vec![0x82, 127];
    huge.extend_from_slice(&(1u64 << 40).to_be_bytes());
    let mut text_inside = frame(0x01, b"a");
    text_inside.extend(frame(0x81, b"b"));
    let mut too_big = frame(0x02, &[0; 40]);
    too_big.extend(frame(0x80, &[0; 40]));
    let cases: Vec<(&str, Vec<u8>, StreamError)> = vec![
        (
            "rsv",
            frame(0xC1, b"x"),
            StreamError::Frame("reserved bits set without an extension"),
        ),
        (
            "opcode",
            frame(0x83, b"x"),
            StreamError::Frame("unknown opcode"),
        ),
        (
            "masked",
            vec![0x81, 0x81, 0, 0, 0, 0, b'x'],
            StreamError::Frame("a server frame is masked"),
        ),
        (
            "fragmented ping",
            frame(0x09, b"x"),
            StreamError::Frame("a fragmented control frame"),
        ),
        (
            "long ping",
            long_ping,
            StreamError::Frame("a control frame over 125 bytes"),
        ),
        (
            "short 16",
            short16,
            StreamError::Frame("a non-minimal 16-bit length"),
        ),
        (
            "short 64",
            short64,
            StreamError::Frame("a non-minimal 64-bit length"),
        ),
        (
            "top bit",
            top64,
            StreamError::Frame("a 64-bit length with its top bit set"),
        ),
        // Refused from its header alone: no payload is awaited or allocated.
        (
            "huge",
            huge,
            StreamError::TooLarge {
                what: "websocket frame",
                limit: 64,
            },
        ),
        (
            "orphan continuation",
            frame(0x80, b"x"),
            StreamError::Frame("a continuation with no message"),
        ),
        (
            "text inside",
            text_inside,
            StreamError::Frame("a new message inside a fragmented one"),
        ),
        (
            "assembled",
            too_big,
            StreamError::TooLarge {
                what: "websocket message",
                limit: 64,
            },
        ),
        (
            "utf-8",
            frame(0x81, b"\xc3"),
            StreamError::Utf8("text message"),
        ),
        (
            "close byte",
            frame(0x88, b"\x03"),
            StreamError::Frame("a one-byte close payload"),
        ),
        (
            "close 1005",
            frame(0x88, &1005u16.to_be_bytes()),
            StreamError::Frame("an invalid close code"),
        ),
        (
            "close 999",
            frame(0x88, &999u16.to_be_bytes()),
            StreamError::Frame("an invalid close code"),
        ),
        (
            "close reason",
            frame(0x88, b"\x03\xe8\xff"),
            StreamError::Utf8("close reason"),
        ),
    ];
    for (name, bytes, want) in cases {
        let err = ws_outcome(&bytes).await.unwrap_err();
        assert_eq!(err, want, "{name}");
        assert!(
            !err.is_disconnect(),
            "{name}: malformed is not a disconnect"
        );
    }
}

#[tokio::test]
async fn valid_edges_of_the_frame_grammar_are_accepted() {
    // A 16-bit length at its minimum, a text split inside a UTF-8 sequence
    // across fragments, an empty close, and a binary message.
    let big = vec![b'z'; 126];
    let limits = Limits {
        message: 200,
        buffer: 512,
        ..SMALL
    };
    let (mut ws, mut server) = ws_open(limits).await;
    let mut out = frame(0x82, &big);
    out.extend(frame(0x01, b"caf\xc3"));
    out.extend(frame(0x80, b"\xa9"));
    out.extend(frame(0x81, b""));
    out.extend(frame(0x88, b""));
    server.write_all(&out).await.unwrap();
    assert_eq!(ws.next(far()).await, Ok(WsMessage::Binary(big)));
    assert_eq!(ws.next(far()).await, Ok(WsMessage::Text("café".into())));
    assert_eq!(ws.next(far()).await, Ok(WsMessage::Text(String::new())));
    assert_eq!(ws.next(far()).await, Err(StreamError::Closed));
    assert_eq!(ws.close_code(), Some(None));
    assert_eq!(client_frame(&mut server).await, (0x88, Vec::new()));
    // The 64-bit form at its minimum, under a limit that admits it.
    let limits = Limits {
        message: 70_000,
        buffer: 70_100,
        head: 256,
        line: 128,
    };
    let (mut ws, mut server) = ws_open(limits).await;
    let payload = vec![7u8; 65_536];
    let writer = tokio::spawn(async move {
        server.write_all(&frame(0x82, &payload)).await.unwrap();
        server
    });
    assert_eq!(
        ws.next(far()).await,
        Ok(WsMessage::Binary(vec![7u8; 65_536]))
    );
    drop(writer.await.unwrap());
    // Client frames of every length form are masked and decode back.
    let (mut ws, mut server) = ws_open(limits).await;
    for n in [5usize, 300, 66_000] {
        let text = "y".repeat(n);
        let (sent, got) = tokio::join!(ws.send_text(&text, far()), client_frame(&mut server));
        sent.unwrap();
        assert_eq!(got, (0x81, text.into_bytes()));
    }
    assert_eq!(
        ws.send_text(&"y".repeat(70_001), far()).await,
        Err(StreamError::TooLarge {
            what: "websocket message",
            limit: 70_000
        })
    );
    assert_eq!(
        ws.ping(&[0; 126], far()).await,
        Err(StreamError::Frame("a control frame over 125 bytes"))
    );
}

#[tokio::test(start_paused = true)]
async fn every_split_point_survives_a_cancelled_read() {
    // One frame with a 16-bit length, fed a byte at a time; after each
    // byte but the last, a pending read is dropped by a timer. Nothing
    // read before the drop is lost.
    let payload: Vec<u8> = (0..130u8).collect();
    let bytes = frame(0x82, &payload);
    let (mut ws, mut server) = ws_open(Limits::DEFAULT).await;
    for (i, b) in bytes.iter().enumerate() {
        server.write_all(&[*b]).await.unwrap();
        let read = tokio::time::timeout(Duration::from_millis(5), ws.next(far())).await;
        if i + 1 < bytes.len() {
            assert!(read.is_err(), "byte {i} completed a message early");
        } else {
            assert_eq!(read.unwrap(), Ok(WsMessage::Binary(payload.clone())));
        }
    }
    // Coalesced: two messages and a ping in one write come out in order.
    let mut out = frame(0x81, b"one");
    out.extend(frame(0x89, b"p"));
    out.extend(frame(0x81, b"two"));
    server.write_all(&out).await.unwrap();
    assert_eq!(ws.next(far()).await, Ok(WsMessage::Text("one".into())));
    assert_eq!(ws.next(far()).await, Ok(WsMessage::Text("two".into())));
    assert_eq!(client_frame(&mut server).await, (0x8A, b"p".to_vec()));
}

#[tokio::test(start_paused = true)]
async fn trickled_input_cannot_renew_the_deadline() {
    let (mut ws, mut server) = ws_open(SMALL).await;
    let trickle = tokio::spawn(async move {
        // A 60-byte frame, one byte every 100 ms: complete only at 6 s.
        for b in frame(0x82, &[1; 58]) {
            if server.write_all(&[b]).await.is_err() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });
    let start = Instant::now();
    let deadline = start + Duration::from_secs(1);
    assert_eq!(ws.next(deadline).await, Err(StreamError::Deadline));
    let now = Instant::now();
    assert!(
        now >= deadline && now < deadline + Duration::from_millis(5),
        "the cutoff is absolute"
    );
    assert!(ws.buffered() > 0, "the trickled bytes are held, not lost");
    drop(ws);
    trickle.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn a_ping_flood_against_a_peer_that_never_reads_stays_bounded() {
    // The peer floods pings and reads nothing back: each pong is written
    // before parsing on, so once the socket's 100 bytes are full the
    // client waits for its deadline rather than queueing pongs unbounded.
    let (ws, mut server) = ws_over(100, SMALL, |r| upgrade_answer(r, "").into_bytes()).await;
    let mut ws = ws.unwrap();
    let flood = tokio::spawn(async move {
        for _ in 0..20 {
            if server.write_all(&frame(0x89, &[0; 60])).await.is_err() {
                return server;
            }
        }
        server
    });
    let deadline = Instant::now() + Duration::from_secs(1);
    assert_eq!(ws.next(deadline).await, Err(StreamError::Deadline));
    assert!(ws.buffered() <= SMALL.buffer, "{}", ws.buffered());
    drop(ws);
    drop(flood.await.unwrap());
}

#[tokio::test]
async fn a_connection_lost_mid_frame_or_between_messages_is_a_disconnect() {
    let (mut ws, mut server) = ws_open(SMALL).await;
    server.write_all(&frame(0x81, b"whole")[..3]).await.unwrap();
    drop(server);
    let err = ws.next(far()).await.unwrap_err();
    assert_eq!(
        err,
        StreamError::Eof {
            at: "inside a frame or message"
        }
    );
    assert!(err.is_disconnect());
    let (mut ws, server) = ws_open(SMALL).await;
    drop(server);
    let err = ws.next(far()).await.unwrap_err();
    assert_eq!(
        err,
        StreamError::Eof {
            at: "between messages"
        }
    );
    assert!(err.is_disconnect());
    // A pong that cannot be written to a vanished peer.
    let (mut ws, mut server) = ws_open(SMALL).await;
    server.write_all(&frame(0x89, b"p")).await.unwrap();
    server.shutdown().await.unwrap();
    drop(server);
    let err = ws.next(far()).await.unwrap_err();
    assert!(err.is_disconnect(), "{err}");
}

#[test]
fn errors_name_their_kind() {
    let reset = StreamError::Io {
        kind: std::io::ErrorKind::ConnectionReset,
        message: "reset".into(),
    };
    assert!(reset.is_disconnect());
    for (e, text) in [
        (StreamError::Deadline, "deadline passed"),
        (reset, "ConnectionReset: reset"),
        (StreamError::Status(500), "unexpected status 500"),
        (
            StreamError::ContentType("x".into()),
            "not an event stream: x",
        ),
        (StreamError::Framing("y".into()), "body framing: y"),
    ] {
        assert!(e.to_string().contains(text), "{e}");
        let _: &dyn std::error::Error = &e;
    }
    assert!(!StreamError::Deadline.is_disconnect());
}

// ---- server-sent events -----------------------------------------------------

const SSE_OK: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\n";

/// An event stream over an in-memory socket whose server answers `head`
/// (which must end with its blank line).
async fn sse_with(
    limits: Limits,
    head: &str,
) -> (Result<Sse<DuplexStream>, StreamError>, DuplexStream) {
    let (client, mut server) = duplex(1 << 16);
    let head = head.to_string();
    let (sse, server) = tokio::join!(
        Sse::open(client, "127.0.0.1:1", "/api/v1/events", far(), limits),
        async move {
            let request = read_request(&mut server).await;
            assert_eq!(header(&request, "Accept"), "text/event-stream");
            server.write_all(head.as_bytes()).await.unwrap();
            server
        }
    );
    (sse, server)
}

async fn sse_open(limits: Limits, framing: &str) -> (Sse<DuplexStream>, DuplexStream) {
    let (sse, server) = sse_with(limits, &format!("{SSE_OK}{framing}\r\n")).await;
    (sse.expect("the event stream"), server)
}

fn event(name: &str, data: &str, id: Option<&str>) -> SseItem {
    SseItem::Event(SseEvent {
        event: name.into(),
        data: data.into(),
        id: id.map(Into::into),
    })
}

fn chunk(bytes: &[u8]) -> Vec<u8> {
    let mut c = format!("{:x}\r\n", bytes.len()).into_bytes();
    c.extend_from_slice(bytes);
    c.extend_from_slice(b"\r\n");
    c
}

#[tokio::test]
async fn a_real_loopback_event_stream_is_read_through_chunked_framing() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (go_on, wait_go) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        let (mut s, _) = listener.accept().await.unwrap();
        let request = read_request(&mut s).await;
        assert!(request.starts_with("GET /events HTTP/1.1\r\n"), "{request}");
        assert_eq!(header(&request, "Accept"), "text/event-stream");
        let mut out = format!("{SSE_OK}Transfer-Encoding: chunked\r\n\r\n").into_bytes();
        // A comment, then an event whose UTF-8 and CRLF are split across
        // chunks, with a chunk extension on the way.
        out.extend(chunk(b": keep-alive\r\n"));
        out.extend(chunk(b"event: new\r\ndata: caf\xc3"));
        out.extend(b"5;ext=1\r\n\xa9\r\n\r\n\r\n".iter());
        s.write_all(&out).await.unwrap();
        wait_go.await.unwrap();
        // A multi-line event split exactly between CR and LF, an empty
        // dispatch, a data-only event with an id, then an incomplete one.
        let mut out = chunk(b"data: one\r");
        out.extend(chunk(
            b"\ndata:two\r\n\r\n\r\nid: 7\ndata\n\nretry: 5\nevent: x\n\n",
        ));
        out.extend(chunk(b"data: never dispatched\n"));
        out.extend(b"0\r\nTrailer: t\r\n\r\n".iter());
        s.write_all(&out).await.unwrap();
    });
    let deadline = far();
    let tcp = crate::stream::connect(addr, deadline).await.unwrap();
    let mut sse = Sse::open(tcp, &addr.to_string(), "/events", deadline, Limits::DEFAULT)
        .await
        .unwrap();
    assert_eq!(sse.next(deadline).await, Ok(event("new", "café", None)));
    go_on.send(()).unwrap();
    assert_eq!(
        sse.next(deadline).await,
        Ok(event("message", "one\ntwo", None))
    );
    assert_eq!(
        sse.next(deadline).await,
        Ok(event("message", "", Some("7")))
    );
    assert_eq!(sse.next(deadline).await, Ok(SseItem::Ended));
    assert_eq!(sse.next(deadline).await, Ok(SseItem::Ended));
    server.await.unwrap();
}

#[tokio::test]
async fn length_and_close_delimited_bodies_end_cleanly_and_drop_an_incomplete_event() {
    let body = "\u{feff}data: a\r\rdata: b\r\n\r\ndata: tail";
    let (mut sse, mut server) = sse_open(
        Limits::DEFAULT,
        &format!("Content-Length: {}\r\n", body.len()),
    )
    .await;
    server.write_all(body.as_bytes()).await.unwrap();
    server
        .write_all(b"data: beyond the length\n\n")
        .await
        .unwrap();
    assert_eq!(sse.next(far()).await, Ok(event("message", "a", None)));
    assert_eq!(sse.next(far()).await, Ok(event("message", "b", None)));
    assert_eq!(sse.next(far()).await, Ok(SseItem::Ended));

    let (mut sse, mut server) = sse_open(Limits::DEFAULT, "").await;
    server
        .write_all(b"data: x\n\ndata: unterminated")
        .await
        .unwrap();
    drop(server);
    assert_eq!(sse.next(far()).await, Ok(event("message", "x", None)));
    assert_eq!(sse.next(far()).await, Ok(SseItem::Ended));

    // A close-delimited body of only part of a BOM ends with no event.
    let (mut sse, mut server) = sse_open(Limits::DEFAULT, "").await;
    server.write_all(b"\xef\xbb").await.unwrap();
    drop(server);
    assert_eq!(sse.next(far()).await, Ok(SseItem::Ended));
    // A body that only starts like a BOM is read from its first byte.
    let (mut sse, mut server) = sse_open(Limits::DEFAULT, "").await;
    server.write_all(b"\xefdata: x\n\n").await.unwrap();
    assert_eq!(
        sse.next(far()).await,
        Err(StreamError::Utf8("event-stream line"))
    );
}

#[tokio::test(start_paused = true)]
async fn split_bytes_and_cancelled_reads_lose_nothing() {
    // A BOM, a split UTF-8 sequence and a CRLF, a byte at a time, with a
    // read dropped by a timer after each byte.
    let (mut sse, mut server) = sse_open(Limits::DEFAULT, "").await;
    let bytes = "\u{feff}event: é\r\ndata: ü\r\n\r\n".as_bytes();
    // The final CR already ends the blank line and dispatches; the LF
    // after it, read separately, is the same break and dispatches nothing.
    let dispatch_at = bytes.len() - 2;
    for (i, b) in bytes.iter().enumerate() {
        server.write_all(&[*b]).await.unwrap();
        let read = tokio::time::timeout(Duration::from_millis(5), sse.next(far())).await;
        if i == dispatch_at {
            assert_eq!(read.unwrap(), Ok(event("é", "ü", None)));
        } else {
            assert!(read.is_err(), "byte {i} dispatched");
        }
    }
}

#[tokio::test(start_paused = true)]
async fn a_trickled_event_stream_cannot_renew_the_deadline() {
    let (mut sse, mut server) = sse_open(SMALL, "").await;
    let trickle = tokio::spawn(async move {
        for b in b"data: slow\n\n" {
            if server.write_all(&[*b]).await.is_err() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
    });
    let deadline = Instant::now() + Duration::from_secs(1);
    assert_eq!(sse.next(deadline).await, Err(StreamError::Deadline));
    assert!(Instant::now() >= deadline);
    assert!(sse.buffered() > 0);
    drop(sse);
    trickle.await.unwrap();
}

#[tokio::test]
async fn event_stream_responses_without_honoured_framing_are_refused() {
    let cases = [
        ("HTTP/1.1 204 No Content\r\n\r\n", "unexpected status 204"),
        ("HTTP/1.1 200 OK\r\n\r\n", "no Content-Type"),
        (
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n",
            "text/plain",
        ),
        (
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Type: text/event-stream\r\n\r\n",
            "content-type is repeated",
        ),
        (
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Encoding: gzip\r\n\r\n",
            "content coding gzip",
        ),
        (
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: gzip, chunked\r\n\r\n",
            "unsupported transfer coding gzip, chunked",
        ),
        (
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nContent-Length: 3\r\n\r\n",
            "both Transfer-Encoding and Content-Length",
        ),
        (
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 3\r\nContent-Length: 3\r\n\r\n",
            "repeated Content-Length",
        ),
        (
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: +3\r\n\r\n",
            "bad Content-Length",
        ),
        (
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: 99999999999999999999999\r\n\r\n",
            "Content-Length overflows",
        ),
    ];
    for (head, want) in cases {
        let (sse, _server) = sse_with(Limits::DEFAULT, head).await;
        let err = sse.err().unwrap_or_else(|| panic!("{head:?} accepted"));
        assert!(err.to_string().contains(want), "{head:?}: {err}");
        assert!(!err.is_disconnect());
    }
    // An identity coding is no coding.
    let (sse, _server) = sse_with(
        Limits::DEFAULT,
        &format!("{SSE_OK}Content-Encoding: identity\r\n\r\n"),
    )
    .await;
    assert!(sse.is_ok());
    let bad = Limits {
        message: 0,
        ..SMALL
    };
    let (client, _server) = duplex(64);
    assert_eq!(
        Sse::open(client, "h", "/", far(), bad).await.err(),
        Some(StreamError::Limits(bad))
    );
}

#[tokio::test]
async fn malformed_or_oversized_event_stream_bodies_are_errors_not_silence() {
    let chunked = "Transfer-Encoding: chunked\r\n";
    let long_line = format!("data: {}\n", "x".repeat(200));
    let mut big_event = String::new();
    for _ in 0..3 {
        big_event.push_str(&format!("data: {}\n", "y".repeat(30)));
    }
    let cases: Vec<(&str, Vec<u8>, StreamError)> = vec![
        (
            chunked,
            b"zz\r\n".to_vec(),
            StreamError::Framing("bad chunk size \"zz\"".into()),
        ),
        (
            chunked,
            b"\r\n".to_vec(),
            StreamError::Framing("bad chunk size \"\"".into()),
        ),
        (
            chunked,
            b"11111111111111111111\r\n".to_vec(),
            StreamError::Framing("chunk size overflows".into()),
        ),
        (
            chunked,
            b"2\r\nabXY".to_vec(),
            StreamError::Framing("chunk data not ended by CRLF".into()),
        ),
        (
            chunked,
            vec![b'1'; 200],
            StreamError::TooLarge {
                what: "chunk framing line",
                limit: 128,
            },
        ),
        (
            chunked,
            [&[b'1'; 200][..], b"\r\n"].concat(),
            StreamError::TooLarge {
                what: "chunk framing line",
                limit: 128,
            },
        ),
        (
            chunked,
            b"\xff\r\n".to_vec(),
            StreamError::Utf8("chunk framing line"),
        ),
        (
            "",
            long_line.clone().into_bytes(),
            StreamError::TooLarge {
                what: "event-stream line",
                limit: 128,
            },
        ),
        (
            "",
            "x".repeat(200).into_bytes(),
            StreamError::TooLarge {
                what: "event-stream line",
                limit: 128,
            },
        ),
        (
            "",
            big_event.into_bytes(),
            StreamError::TooLarge {
                what: "event data",
                limit: 64,
            },
        ),
        (
            "",
            b"data: \xc3\x28\n".to_vec(),
            StreamError::Utf8("event-stream line"),
        ),
    ];
    for (framing, body, want) in cases {
        let (mut sse, mut server) = sse_open(SMALL, framing).await;
        server.write_all(&body).await.unwrap();
        let got = sse.next(far()).await;
        assert_eq!(got, Err(want), "{framing:?} {body:?}");
    }
    // Lost connections inside a framed body are disconnects.
    for framing in [chunked, "Content-Length: 50\r\n"] {
        let (mut sse, mut server) = sse_open(SMALL, framing).await;
        server.write_all(b"5\r\nda").await.unwrap();
        drop(server);
        let err = sse.next(far()).await.unwrap_err();
        assert_eq!(
            err,
            StreamError::Eof {
                at: "inside the event-stream body"
            }
        );
        assert!(err.is_disconnect());
    }
}

// ============================================================================
// T1: the wait evidence of lock v5 and the closed native_wait cells
// ============================================================================

mod evidence {
    use crate::protocol::{self, CapabilityStatus, Fact};
    use crate::{TargetKind, evals_copy, json, lock_from, protocol_from, refusals};
    use serde_json::Value;

    const DRAFT_PROTOCOL: &[u8] = include_bytes!("../../docs/evals/draft/protocol.v4.json");
    const DRAFT_LOCK: &[u8] = include_bytes!("../../docs/evals/draft/sources.lock.v5.json");

    /// The rows whose pinned sources show a push channel.
    const PUSH_CHANNELS: [&str; 4] = ["mailpit", "mailhog", "smtp4dev", "inbucket"];
    /// The one push channel with readiness evidenced from captured bytes: a
    /// candidate until its real-binary proof (T10) selects it.
    const STREAM_CANDIDATES: [&str; 1] = ["mailhog"];
    /// Push channels whose readiness or registration cannot be evidenced
    /// from captured bytes (T5a, T5b): cited polls naming the gap.
    const GAP_ROWS: [&str; 3] = ["mailpit", "smtp4dev", "inbucket"];
    /// The rows whose pinned sources show neither a wait nor a push route,
    /// and the gap rows.
    const POLL_ROWS: [&str; 7] = [
        "mailslurper",
        "mailslurper-source-build",
        "greenmail",
        "alkemio-mailslurper",
        "mailpit",
        "smtp4dev",
        "inbucket",
    ];

    fn pin_of<'a>(l: &'a mut Value, target: &str) -> &'a mut Value {
        l["entries"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|e| e["target"] == target)
            .unwrap()
            .get_mut("pin")
            .unwrap()
    }

    fn doc_mut<'a>(v: &'a mut Value, t: &str, path: &str) -> &'a mut Value {
        pin_of(v, t)["documents"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|d| d["path"] == path)
            .unwrap()
    }

    /// The wait documents lock v5 adds: source bodies and their licenses.
    fn wait_documents(l: &Value) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for e in l["entries"].as_array().unwrap() {
            for d in e["pin"]["documents"].as_array().into_iter().flatten() {
                let path = d["path"].as_str().unwrap();
                let added = !json(crate::SOURCES_V4)["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|o| {
                        o["target"] == e["target"]
                            && o["pin"]["documents"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .any(|x| x["path"] == path)
                    });
                if added {
                    out.push((e["target"].as_str().unwrap().to_owned(), path.to_owned()));
                }
            }
        }
        out
    }

    #[test]
    fn every_native_wait_cell_is_closed_with_a_citation_of_its_pin() {
        let p = protocol_from(json(DRAFT_PROTOCOL)).unwrap();
        let l = lock_from(json(DRAFT_LOCK)).unwrap();
        p.validate().unwrap();
        l.validate().unwrap();
        let caps = p.capabilities.as_ref().expect("v4 states capabilities");
        assert_eq!(caps.len(), TargetKind::ALL.len());
        for c in caps {
            let t = c.target;
            assert_ne!(
                c.native_wait.status,
                CapabilityStatus::NotInspected,
                "{t} native_wait is still not_inspected"
            );
            let cite = c
                .native_wait
                .cite
                .as_deref()
                .unwrap_or_else(|| panic!("{t} cites nothing"));
            if t == TargetKind::Swarmail {
                assert_eq!(c.native_wait.status, CapabilityStatus::Supported);
                assert!(cite.starts_with("repo:docs/decisions/0002"), "{cite}");
                continue;
            }
            // A blocking wait and a push channel are distinct facts: no
            // competitor has a blocking wait route.
            assert_eq!(c.native_wait.status, CapabilityStatus::NotSupported, "{t}");
            let pin = l.pin(t).unwrap();
            assert!(
                pin.documents.iter().any(|d| d.path == cite),
                "{t} native_wait cites {cite}, which is not one of its documents"
            );
        }
        protocol::capabilities_match(&p, &l).unwrap();
        // Official MailSlurper stays blocked.
        let blocked: Vec<_> = p
            .does_not_start
            .iter()
            .flatten()
            .map(|d| d.target)
            .collect();
        assert_eq!(blocked, [TargetKind::Mailslurper]);
    }

    #[test]
    fn every_best_wait_states_its_method_or_its_open_proof() {
        let l = lock_from(json(DRAFT_LOCK)).unwrap();
        for t in TargetKind::ALL {
            let Some(pin) = l.pin(t) else {
                assert_eq!(t, TargetKind::Swarmail, "only the candidate has no pin");
                continue;
            };
            let name = t.name();
            match &pin.wait.best_wait {
                Fact::Observed(m) => {
                    assert!(
                        POLL_ROWS.contains(&name),
                        "{t} selects {m:?} before its proof"
                    );
                    assert!(
                        m.starts_with("poll: ") && m.contains("no documented wait"),
                        "{t}: {m}"
                    );
                    // The fallback cites the document that shows the absence.
                    assert!(
                        pin.documents
                            .iter()
                            .any(|d| m.contains(d.path.rsplit('/').next().unwrap())),
                        "{t}: the poll reason cites no captured document: {m}"
                    );
                }
                Fact::Unresolved(why) => {
                    assert!(
                        STREAM_CANDIDATES.contains(&name),
                        "{t} is unresolved: {why}"
                    );
                    assert!(
                        why.starts_with("stream candidate, not yet selected"),
                        "{t}: {why}"
                    );
                    assert!(
                        why.contains("T10") && why.contains("100 ms poll"),
                        "{t}: {why}"
                    );
                }
            }
            assert!(!pin.wait.finding.trim().is_empty(), "{t}");
        }
        // A gap row names what is missing and says it is a gap, never proof
        // that no channel exists.
        for t in GAP_ROWS {
            let kind = TargetKind::ALL.into_iter().find(|k| k.name() == t).unwrap();
            let Fact::Observed(m) = &l.pin(kind).unwrap().wait.best_wait else {
                panic!("{t} is a cited poll");
            };
            assert!(
                m.contains("gap, not proof that no channel exists"),
                "{t}: {m}"
            );
            assert!(m.contains("not in the tagged tree"), "{t}: {m}");
        }
        // Every push channel's finding names the route, producer,
        // registration and readiness it rests on, and what it loses.
        for t in PUSH_CHANNELS {
            let kind = TargetKind::ALL.into_iter().find(|k| k.name() == t).unwrap();
            let f = &l.pin(kind).unwrap().wait.finding;
            for part in ["Producer:", "Registration", "Readiness evidence:", "Loss"] {
                assert!(f.contains(part), "{t} finding has no {part}");
            }
        }
    }

    #[test]
    fn the_wait_documents_verify_and_each_is_rechecked_at_use() {
        let root = tempfile::tempdir().unwrap();
        let dir = evals_copy(root.path());
        let v = json(DRAFT_LOCK);
        let l = lock_from(v.clone()).unwrap();
        l.verify_files(&dir).expect("the copy verifies");
        let added = wait_documents(&v);
        // Every target the review touched carries new documents.
        for t in [
            "mailpit",
            "mailhog",
            "smtp4dev",
            "inbucket",
            "alkemio-mailslurper",
        ] {
            assert!(
                added.iter().any(|(at, _)| at == t),
                "{t} gained no document"
            );
        }
        // A changed byte in any one of them refuses that document by name.
        for (_, path) in &added {
            let file = dir.join(path);
            let good = std::fs::read(&file).unwrap();
            let mut bad = good.clone();
            let at = bad.len() / 2;
            bad[at] ^= 1;
            std::fs::write(&file, &bad).unwrap();
            let why = refusals(l.verify_files(&dir));
            assert!(
                why.contains(&format!("{path} does not match its digest")),
                "{why}"
            );
            std::fs::write(&file, &good).unwrap();
        }
        l.verify_files(&dir).expect("restored");
        // A missing one refuses.
        let gone = "sources/mailpit/mailpit-server.go";
        let saved = std::fs::read(dir.join(gone)).unwrap();
        std::fs::remove_file(dir.join(gone)).unwrap();
        assert!(refusals(l.verify_files(&dir)).contains(&format!("{gone} is missing")));
        std::fs::write(dir.join(gone), &saved).unwrap();
    }

    /// Lock edits the bytes still have to agree with: size, git blob,
    /// attribution path and the retained reviewed body.
    #[test]
    fn a_wait_document_whose_facts_disagree_with_its_bytes_refuses() {
        let root = tempfile::tempdir().unwrap();
        let dir = evals_copy(root.path());
        let check = |edit: &dyn Fn(&mut Value)| {
            let mut v = json(DRAFT_LOCK);
            edit(&mut v);
            let l = lock_from(v).unwrap();
            let text = |r: Result<(), protocol::Refusal>| r.err().map(|e| e.0.join("\n"));
            let why = [text(l.validate()), text(l.verify_files(&dir))]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join("\n");
            assert!(!why.is_empty(), "the edit must refuse");
            why
        };
        let why = check(&|v| {
            doc_mut(v, "mailhog", "sources/mailhog/mailhog-server-api-v2.go")["size"] = 1.into();
        });
        assert!(
            why.contains("mailhog-server-api-v2.go is not 1 bytes"),
            "{why}"
        );
        let why = check(&|v| {
            doc_mut(v, "inbucket", "sources/inbucket/inbucket-msghub-hub.go")["attribution"]["observed"]
                ["git_blob"] = "0123456789abcdef0123456789abcdef01234567".into();
        });
        assert!(
            why.contains("inbucket-msghub-hub.go is not the git blob its attribution names"),
            "{why}"
        );
        let why = check(&|v| {
            doc_mut(v, "smtp4dev", "sources/smtp4dev/smtp4dev-startup.cs")["attribution"]["observed"]
                ["upstream_path"] = "Rnwood.Smtp4dev/Program.cs".into();
        });
        assert!(
            why.contains("smtp4dev-startup.cs is not attributed to its path"),
            "{why}"
        );
        // A reviewed body retained under a digest the document does not carry.
        let why = check(&|v| {
            let r = &mut pin_of(v, "mailpit")["wait"]["reviewed"][2];
            r["sha256"] = "0".repeat(64).into();
        });
        assert!(
            why.contains("names no retained source document with its digest"),
            "{why}"
        );
        // A reviewed path the tagged tree does not hold.
        let why = check(&|v| {
            let r = &mut pin_of(v, "inbucket")["wait"]["reviewed"][0];
            r["upstream_path"] = "pkg/rest/elsewhere.go".into();
            r["retained_as"] = serde_json::json!({"unresolved": "moved for the test"});
        });
        assert!(
            why.contains("reviewed source pkg/rest/elsewhere.go is not in the tagged tree"),
            "{why}"
        );
    }

    #[test]
    fn a_recursive_listing_binds_every_nested_entry_to_the_root_tree() {
        let dir = crate::evals_dir();
        let listing = |p: &str| json(&std::fs::read(dir.join(p)).unwrap());
        for p in [
            "sources/smtp4dev/smtp4dev-tree-recursive.json",
            "sources/inbucket/inbucket-tree-recursive.json",
        ] {
            let v = listing(p);
            let entries = v["tree"].as_array().unwrap();
            assert!(
                entries
                    .iter()
                    .any(|e| e["path"].as_str().unwrap().contains('/'))
            );
            assert_eq!(
                protocol::listing_object_id(entries).as_deref(),
                v["sha"].as_str(),
                "{p}"
            );
        }
        // A flat listing is all opaque directories: the same id as before.
        for p in [
            "sources/smtp4dev/smtp4dev-tree.json",
            "sources/inbucket/inbucket-tree.json",
            "sources/greenmail/greenmail-tree.json",
        ] {
            let v = listing(p);
            let entries = v["tree"].as_array().unwrap();
            assert_eq!(
                protocol::listing_object_id(entries),
                protocol::tree_object_id(entries)
            );
            assert_eq!(
                protocol::listing_object_id(entries).as_deref(),
                v["sha"].as_str()
            );
        }
        let v = listing("sources/inbucket/inbucket-tree-recursive.json");
        let entries = v["tree"].as_array().unwrap().clone();
        let nested = entries
            .iter()
            .position(|e| e["path"] == "pkg/msghub/hub.go")
            .unwrap();
        // A forged nested blob id breaks its directory's rebuilt id.
        let mut forged = entries.clone();
        forged[nested]["sha"] = "0123456789abcdef0123456789abcdef01234567".into();
        assert_eq!(protocol::listing_object_id(&forged), None);
        // So does an entry added inside a listed directory.
        let mut added = entries.clone();
        let mut extra = entries[nested].clone();
        extra["path"] = "pkg/msghub/zz.go".into();
        added.insert(nested + 1, extra);
        assert_eq!(protocol::listing_object_id(&added), None);
        // An entry under a directory the listing never names is refused.
        let mut orphan = entries.clone();
        let mut extra = entries[nested].clone();
        extra["path"] = "nowhere/hub.go".into();
        orphan.push(extra);
        assert_eq!(protocol::listing_object_id(&orphan), None);

        // Through the lock: a forged recursive capture whose digest and size
        // are made to agree still refuses, because it no longer rebuilds.
        let root = tempfile::tempdir().unwrap();
        let copy = evals_copy(root.path());
        let path = "sources/inbucket/inbucket-tree-recursive.json";
        let mut bad = v.clone();
        bad["tree"] = Value::from(forged);
        let bytes = serde_json::to_vec(&bad).unwrap();
        std::fs::write(copy.join(path), &bytes).unwrap();
        let mut l = json(DRAFT_LOCK);
        for d in pin_of(&mut l, "inbucket")["documents"]
            .as_array_mut()
            .unwrap()
        {
            if d["path"] == path {
                d["size"] = bytes.len().into();
                d["sha256"] = protocol::sha256_hex(&bytes).into();
            }
        }
        let why = refusals(lock_from(l).unwrap().verify_files(&copy));
        assert!(
            why.contains("inbucket: the captured tree listing does not rebuild to its own id"),
            "{why}"
        );
    }
}

// ============================================================================
// T2: the three wait profiles, Observation::Stream and lock v5's wait
// documents (0132 §1.2, §7)
// ============================================================================

mod profiles {
    use crate::protocol::{self, Observation};
    use crate::{TargetKind, json, lock_from, protocol_from, refusals};
    use serde_json::{Value, json};

    const DRAFT_PROTOCOL: &[u8] = include_bytes!("../../docs/evals/draft/protocol.v4.json");
    const DRAFT_LOCK: &[u8] = include_bytes!("../../docs/evals/draft/sources.lock.v5.json");

    type Edit = Box<dyn Fn(&mut Value)>;

    fn draft() -> Value {
        json(DRAFT_PROTOCOL)
    }

    fn why(v: Value) -> String {
        refusals(protocol_from(v).expect("parses").validate())
    }

    /// The observation of `target` in profile `i` (0 common, 1 best, 2 tight).
    fn obs<'a>(v: &'a mut Value, i: usize, target: &str) -> &'a mut Value {
        v["waits"][i]["observation"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|o| o["target"] == target)
            .unwrap()
    }

    fn stream(o: &mut Value) {
        o["mode"] = json!("stream");
        o["interval_ms"] = Value::Null;
        o["backstop_ms"] = json!(1000);
    }

    /// MailHog's wait-best row as the implemented stream.
    fn mailhog_streams(v: &mut Value) {
        stream(obs(v, 1, "mailhog"));
    }

    fn refuses_each(base: Value, cases: Vec<(&str, Edit)>) {
        for (want, edit) in cases {
            let mut v = base.clone();
            edit(&mut v);
            let w = why(v);
            assert!(w.contains(want), "{want}: {w}");
        }
    }

    #[test]
    fn the_draft_carries_the_three_accepted_profiles_and_the_restored_budget() {
        let p = protocol_from(draft()).unwrap();
        p.validate().unwrap();
        let names: Vec<_> = p.waits.iter().map(|w| w.name.as_str()).collect();
        assert_eq!(names, protocol::WAIT_PROFILES);
        for w in &p.waits {
            assert_eq!(
                (w.repetitions, w.present, w.future, w.empty),
                (3, 100, 100, 20)
            );
            assert_eq!((w.deadline_ms, w.empty_deadline_ms), (30_000, 1_000));
            assert_eq!(w.observation.len(), TargetKind::ALL.len());
        }
        let interval = |i: usize| -> Vec<Option<u64>> {
            p.waits[i]
                .observation
                .iter()
                .map(|o| o.interval_ms)
                .collect()
        };
        assert!(interval(0).iter().all(|i| *i == Some(100)));
        assert!(interval(2).iter().all(|i| *i == Some(10)));
        for o in &p.waits[1].observation {
            let want = if o.target == TargetKind::Swarmail {
                Observation::Native
            } else {
                // No stream is selected before its proof runs on the host.
                Observation::Poll
            };
            assert_eq!(o.mode, want, "{}", o.target);
            assert!(o.evidence.as_ref().is_some_and(|e| !e.is_empty()));
            assert!(o.justification.starts_with("wait-best: "));
        }
        let b = p.budgets.as_ref().unwrap();
        assert_eq!(
            b.wait_operation_ms,
            30_000 + p.resources.request_deadline_ms
        );
        // The draft agrees with the lock: every citation is a document of
        // the pin, and every mode is the pin's best wait.
        let l = lock_from(json(DRAFT_LOCK)).unwrap();
        protocol::waits_match(&p, &l).unwrap();
        // A tight poll is labelled as no tool's documented method.
        assert!(
            p.waits[2].observation[0]
                .justification
                .contains("no tool's documented method")
        );
    }

    #[test]
    fn mailhogs_stream_row_validates_and_its_variants_refuse() {
        let mut v = draft();
        mailhog_streams(&mut v);
        protocol_from(v.clone()).unwrap().validate().unwrap();
        let cases: Vec<(&str, Edit)> = vec![
            (
                "stream carries a poll interval",
                Box::new(|v| obs(v, 1, "mailhog")["interval_ms"] = json!(100)),
            ),
            (
                "stream's backstop_ms is 1000, not None",
                Box::new(|v| {
                    obs(v, 1, "mailhog")
                        .as_object_mut()
                        .unwrap()
                        .remove("backstop_ms");
                }),
            ),
            (
                "stream's backstop_ms is 1000, not Some(500)",
                Box::new(|v| obs(v, 1, "mailhog")["backstop_ms"] = json!(500)),
            ),
            (
                "carries a backstop but does not stream",
                Box::new(|v| obs(v, 1, "mailpit")["backstop_ms"] = json!(1000)),
            ),
            (
                "no stream with evidenced readiness is implemented for mailpit",
                Box::new(|v| stream(obs(v, 1, "mailpit"))),
            ),
            (
                "wait-common: mailhog streams, and only protocol 4's wait-best may",
                Box::new(|v| stream(obs(v, 0, "mailhog"))),
            ),
        ];
        refuses_each(v, cases);
    }

    #[test]
    fn wrong_modes_intervals_and_citations_refuse_by_name() {
        let native = |o: &mut Value| {
            o["mode"] = json!("native");
            o["interval_ms"] = Value::Null;
        };
        let cases: Vec<(&str, Edit)> = vec![
            (
                "wait-common: swarmail observes by Native",
                Box::new(move |v| native(obs(v, 0, "swarmail"))),
            ),
            (
                "wait-common: mailpit polls at Some(10) ms, not the common 100 ms",
                Box::new(|v| obs(v, 0, "mailpit")["interval_ms"] = json!(10)),
            ),
            (
                "wait-tight: mailpit observes by Poll at Some(100) ms",
                Box::new(|v| obs(v, 2, "mailpit")["interval_ms"] = json!(100)),
            ),
            (
                "wait-best: swarmail observes by Poll",
                Box::new(|v| {
                    let o = obs(v, 1, "swarmail");
                    o["mode"] = json!("poll");
                    o["interval_ms"] = json!(100);
                }),
            ),
            (
                "wait-best: greenmail observes by Poll at Some(10) ms",
                Box::new(|v| obs(v, 1, "greenmail")["interval_ms"] = json!(10)),
            ),
            (
                "no source-verified native wait is implemented for mailpit",
                Box::new(move |v| native(obs(v, 1, "mailpit"))),
            ),
            (
                "wait-best: inbucket cites no evidence for its method",
                Box::new(|v| {
                    obs(v, 1, "inbucket")
                        .as_object_mut()
                        .unwrap()
                        .remove("evidence");
                }),
            ),
            (
                "wait-tight: inbucket cites evidence, but only wait-best chooses a method",
                Box::new(|v| obs(v, 2, "inbucket")["evidence"] = json!(["sources/x.go"])),
            ),
            (
                "wait-best: inbucket cites an empty evidence list or entry",
                Box::new(|v| obs(v, 1, "inbucket")["evidence"] = json!([])),
            ),
            (
                "wait-best: inbucket cites an empty evidence list or entry",
                Box::new(|v| obs(v, 1, "inbucket")["evidence"] = json!([" "])),
            ),
            (
                "wait-best: inbucket cites sources/x.go twice",
                Box::new(|v| {
                    obs(v, 1, "inbucket")["evidence"] = json!(["sources/x.go", "sources/x.go"])
                }),
            ),
            (
                "wait-best: smtp4dev needs exactly one observation mode",
                Box::new(|v| {
                    v["waits"][1]["observation"]
                        .as_array_mut()
                        .unwrap()
                        .retain(|o| o["target"] != "smtp4dev");
                }),
            ),
            (
                "wait-best: smtp4dev needs exactly one observation mode",
                Box::new(|v| {
                    let o = obs(v, 1, "smtp4dev").clone();
                    v["waits"][1]["observation"].as_array_mut().unwrap().push(o);
                }),
            ),
            (
                "profile name \"wait-best\" is malformed or duplicated",
                Box::new(|v| {
                    let w = v["waits"][1].clone();
                    v["waits"].as_array_mut().unwrap().push(w);
                }),
            ),
            (
                "wait-later: protocol 4's wait profiles are",
                Box::new(|v| v["waits"][2]["name"] = json!("wait-later")),
            ),
            (
                "budgets: wait_operation_ms 10000 is not the longest wait 30000 plus one request bound",
                Box::new(|v| v["budgets"]["wait_operation_ms"] = json!(10_000)),
            ),
        ];
        refuses_each(draft(), cases);
    }

    #[test]
    fn a_frozen_protocol_four_holds_exactly_the_three_exact_profiles() {
        let frozen = |edit: &dyn Fn(&mut Value)| {
            let mut v = draft();
            v["status"] = json!("frozen");
            edit(&mut v);
            refusals(protocol_from(v).unwrap().validate_owing(&[]))
        };
        let clean = frozen(&|_| {});
        assert!(!clean.contains("wait profile"), "{clean}");
        let w = frozen(&|v| {
            v["waits"].as_array_mut().unwrap().remove(2);
        });
        assert!(
            w.contains("a frozen protocol 4's wait profiles are [\"wait-common\", \"wait-best\", \"wait-tight\"] in that order, not [\"wait-common\", \"wait-best\"]"),
            "{w}"
        );
        let w = frozen(&|v| v["waits"].as_array_mut().unwrap().swap(0, 2));
        assert!(w.contains("in that order"), "{w}");
        for (field, value) in [
            ("repetitions", json!(2)),
            ("present", json!(99)),
            ("future", json!(10)),
            ("empty", json!(19)),
            ("deadline_ms", json!(20_000)),
            ("empty_deadline_ms", json!(2_000)),
        ] {
            let w = frozen(&|v| v["waits"][1][field] = value.clone());
            assert!(
                w.contains("wait-best: a frozen protocol 4's wait profile is exact"),
                "{field}: {w}"
            );
        }
        // A draft (a smoke fixture) may hold fewer, smaller profiles.
        let mut v = draft();
        v["waits"].as_array_mut().unwrap().truncate(1);
        v["waits"][0]["present"] = json!(2);
        protocol_from(v).unwrap().validate().unwrap();
    }

    #[test]
    fn earlier_protocols_refuse_the_v4_only_wait_fields_and_streams() {
        let v3 = json(crate::PROTOCOL_V3);
        protocol_from(v3.clone()).unwrap().validate().unwrap();
        for (field, value) in [("backstop_ms", json!(1000)), ("evidence", json!(["x"]))] {
            let mut v = v3.clone();
            v["waits"][0]["observation"][1][field] = value;
            let w = refusals(protocol_from(v).unwrap().validate());
            let want = format!("protocol_version 3 refuses waits[].observation[].{field}");
            assert!(w.contains(&want), "{w}");
        }
        let mut v = v3;
        let o = v["waits"][0]["observation"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|o| o["target"] == "mailhog")
            .unwrap();
        o["mode"] = json!("stream");
        o["interval_ms"] = Value::Null;
        let w = refusals(protocol_from(v).unwrap().validate());
        assert!(
            w.contains("wait-common: mailhog streams, and only protocol 4's wait-best may"),
            "{w}"
        );
        // The frozen v2 and v3 bytes still validate and serialize unchanged.
        for bytes in [crate::PROTOCOL, crate::PROTOCOL_V3] {
            let p = protocol::Protocol::parse(bytes).unwrap();
            p.validate().unwrap();
            assert_eq!(serde_json::to_value(&p).unwrap(), json(bytes));
        }
    }

    fn pin_mut<'a>(l: &'a mut Value, t: &str) -> &'a mut Value {
        &mut l["entries"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|e| e["target"] == t)
            .unwrap()["pin"]
    }

    #[test]
    fn wait_best_agrees_with_the_pins_best_wait_and_cites_its_documents() {
        let lock = json(DRAFT_LOCK);
        let check = |pv: Value, lv: Value| {
            refusals(protocol::waits_match(
                &protocol_from(pv).unwrap(),
                &lock_from(lv).unwrap(),
            ))
        };
        // MailHog streams only once its pin's best wait says so.
        let mut streaming = draft();
        mailhog_streams(&mut streaming);
        let w = check(streaming.clone(), lock.clone());
        assert!(
            w.contains("wait-best: mailhog observes by Stream, but its pin's best wait is Poll"),
            "{w}"
        );
        let mut selected = lock.clone();
        pin_mut(&mut selected, "mailhog")["wait"]["best_wait"] =
            json!({"observed": "stream: the api/v2/websocket proof passed"});
        protocol::waits_match(
            &protocol_from(streaming).unwrap(),
            &lock_from(selected.clone()).unwrap(),
        )
        .unwrap();
        let w = check(draft(), selected.clone());
        assert!(
            w.contains("wait-best: mailhog observes by Poll, but its pin's best wait is Stream"),
            "{w}"
        );
        let mut odd = selected;
        pin_mut(&mut odd, "mailhog")["wait"]["best_wait"] = json!({"observed": "carrier pigeon"});
        let w = check(draft(), odd);
        assert!(
            w.contains("mailhog: best wait \"carrier pigeon\" names neither a poll nor a stream"),
            "{w}"
        );
        // A citation must be one of the pin's documents, or the candidate's
        // repo: path.
        let mut v = draft();
        obs(&mut v, 1, "greenmail")["evidence"] = json!(["sources/greenmail/nothing.yml"]);
        let w = check(v.clone(), lock.clone());
        assert!(
            w.contains("wait-best: greenmail cites \"sources/greenmail/nothing.yml\", which is no document of its pin"),
            "{w}"
        );
        // Collection reports the disagreement too.
        let w = refusals(protocol::collection_ready(
            &protocol_from(v).unwrap(),
            &lock_from(lock.clone()).unwrap(),
        ));
        assert!(w.contains("nothing.yml"), "{w}");
        let mut v = draft();
        obs(&mut v, 1, "swarmail")["evidence"] = json!(["docs/decisions/0002.md"]);
        let w = check(v, lock);
        assert!(w.contains("wait-best: swarmail cites"), "{w}");
    }

    #[test]
    fn lock_four_refuses_the_v5_only_wait_documents_by_name() {
        // A version-only downgrade, with every cap placeholder made literal
        // so that only the wait documents can be refused.
        let text = String::from_utf8(DRAFT_LOCK.to_vec()).unwrap();
        let mut down: Value =
            serde_json::from_str(&text.replace(protocol::CAP_PLACEHOLDER, "150000")).unwrap();
        down["lock_version"] = json!(4);
        let w = refusals(lock_from(down).unwrap().validate());
        assert!(!w.contains("placeholder"), "{w}");
        for (t, doc) in [
            (
                "mailhog",
                "vendor/github.com/mailhog/MailHog-Server/api/v2.go",
            ),
            ("mailhog", "sources/mailhog/mailhog-server-api-v2.go"),
            ("mailpit", "server/server.go"),
            ("smtp4dev", "Rnwood.Smtp4dev/Hubs/NotificationsHub.cs"),
            ("inbucket", "sources/inbucket/inbucket-msghub-hub.go"),
            (
                "alkemio-mailslurper",
                "sources/alkemio-mailslurper/alkemio-setup-services-listener.go",
            ),
        ] {
            let want = format!(
                "{t}: lock_version 4 refuses the v5-only wait document {doc}; only lock_version 5 carries it (0132 §1.2)"
            );
            assert!(w.contains(&want), "{want}: {w}");
        }
        // What lock 4 froze is not refused.
        assert!(!w.contains("document server/websockets/hub.go;"), "{w}");
        assert!(
            !w.contains("document sources/mailpit/mailpit-websockets-hub.go;"),
            "{w}"
        );

        // One wait document inserted into the valid historical lock 4.
        let valid = json(crate::SOURCES_V4);
        lock_from(valid.clone()).unwrap().validate().unwrap();
        let mut v5 = json(DRAFT_LOCK);
        let pin5 = pin_mut(&mut v5, "mailhog").clone();
        let reviewed = pin5["wait"]["reviewed"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["upstream_path"] == "vendor/github.com/mailhog/MailHog-Server/api/v2.go")
            .unwrap()
            .clone();
        let doc = pin5["documents"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["path"] == "sources/mailhog/mailhog-server-api-v2.go")
            .unwrap()
            .clone();
        let edit = |f: &dyn Fn(&mut Value)| {
            let mut l = valid.clone();
            f(pin_mut(&mut l, "mailhog"));
            refusals(lock_from(l).unwrap().validate())
        };
        let w = edit(&|pin| {
            pin["wait"]["reviewed"]
                .as_array_mut()
                .unwrap()
                .push(reviewed.clone())
        });
        assert!(
            w.contains("mailhog: lock_version 4 refuses the v5-only wait document vendor/github.com/mailhog/MailHog-Server/api/v2.go"),
            "{w}"
        );
        let w = edit(&|pin| pin["documents"].as_array_mut().unwrap().push(doc.clone()));
        assert!(
            w.contains("mailhog: lock_version 4 refuses the v5-only wait document sources/mailhog/mailhog-server-api-v2.go"),
            "{w}"
        );
        assert!(!w.contains("placeholder"), "{w}");
        // Every historical lock still validates, and lock 5 carries them all.
        lock_from(json(crate::SOURCES)).unwrap().validate().unwrap();
        lock_from(json(crate::ACTIVE_SOURCES))
            .unwrap()
            .validate()
            .unwrap();
        lock_from(json(DRAFT_LOCK)).unwrap().validate().unwrap();
    }
}

// ============================================================================
// T5/T6: the stream observer's state machine, on a controlled clock. The
// channel and the store are scripted in-process; the real wire is proved by
// the loopback tests below and by the pinned MailHog proof (T10).
// ============================================================================

mod observer {
    use crate::observe::{Cause, Channel, Scanner, Seen, StreamBounds, Trace, Wake, stream_wait};
    use crate::stream::StreamError;
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::rc::Rc;
    use std::time::Duration;
    use tokio::sync::{Notify, mpsc};
    use tokio::time::{Instant, sleep, sleep_until};

    type Item = Result<Wake, StreamError>;

    enum Ready {
        Now(mpsc::UnboundedReceiver<Item>),
        Fail(StreamError),
        /// Never ready: the attempt ends at its own deadline.
        Stall,
    }

    #[derive(Default)]
    struct Fake {
        readies: RefCell<VecDeque<Ready>>,
        /// Mail that becomes visible at the instant a readiness is proved
        /// (between registration and the first scan).
        on_ready: RefCell<Vec<String>>,
        store: Rc<RefCell<Vec<String>>>,
    }

    impl Channel for Fake {
        type Conn = mpsc::UnboundedReceiver<Item>;

        async fn ready(&self, deadline: Instant) -> Result<Self::Conn, StreamError> {
            let step = self.readies.borrow_mut().pop_front();
            match step {
                Some(Ready::Now(rx)) => {
                    let mut add = self.on_ready.borrow_mut();
                    self.store.borrow_mut().append(&mut add);
                    Ok(rx)
                }
                Some(Ready::Fail(e)) => Err(e),
                Some(Ready::Stall) | None => {
                    sleep_until(deadline).await;
                    Err(StreamError::Deadline)
                }
            }
        }

        async fn next(conn: &mut Self::Conn, deadline: Instant) -> Item {
            match tokio::time::timeout_at(deadline, conn.recv()).await {
                Ok(Some(item)) => item,
                Ok(None) => Err(StreamError::Eof {
                    at: "between messages",
                }),
                Err(_) => Err(StreamError::Deadline),
            }
        }
    }

    enum Step {
        Sleep(Duration),
        Gate(Rc<Notify>),
        Fail,
        Forever,
    }

    /// A complete scan answers what the store held when it started.
    struct Store {
        tokens: Rc<RefCell<Vec<String>>>,
        script: RefCell<VecDeque<Step>>,
        started: Rc<Notify>,
        scans: Cell<u32>,
    }

    impl Store {
        fn new(tokens: Rc<RefCell<Vec<String>>>, script: Vec<Step>) -> Store {
            Store {
                tokens,
                script: RefCell::new(script.into()),
                started: Rc::new(Notify::new()),
                scans: Cell::new(0),
            }
        }
    }

    impl Scanner for Store {
        async fn scan(&self) -> Result<Vec<String>, String> {
            let snapshot = self.tokens.borrow().clone();
            self.scans.set(self.scans.get() + 1);
            self.started.notify_one();
            let step = self.script.borrow_mut().pop_front();
            match step {
                None => {}
                Some(Step::Sleep(d)) => sleep(d).await,
                Some(Step::Gate(g)) => g.notified().await,
                Some(Step::Fail) => return Err("incomplete: a page was short".into()),
                Some(Step::Forever) => std::future::pending::<()>().await,
            }
            Ok(snapshot)
        }
    }

    const WAIT: Duration = Duration::from_secs(30);

    fn bounds(wait: Duration) -> StreamBounds {
        StreamBounds {
            wait,
            request: Duration::from_secs(10),
            backstop: Duration::from_millis(1000),
            reconnect_gap: Duration::from_millis(100),
        }
    }

    /// One fake channel with one ready connection, its sender and store.
    fn rig() -> (Fake, mpsc::UnboundedSender<Item>, Rc<RefCell<Vec<String>>>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let fake = Fake::default();
        fake.readies.borrow_mut().push_back(Ready::Now(rx));
        let store = fake.store.clone();
        (fake, tx, store)
    }

    fn found(seen: &Seen) -> (Cause, bool) {
        match seen {
            Seen::Answer {
                tokens,
                cause,
                late,
            } => {
                assert_eq!(tokens, &["tok".to_owned()]);
                (*cause, *late)
            }
            Seen::Timeout => panic!("no answer"),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn mail_delivered_before_registration_or_before_the_first_scan_is_seen_by_it() {
        // Before registration: already stored when the channel opens.
        let (fake, _tx, store) = rig();
        store.borrow_mut().push("tok".into());
        let s = Store::new(store.clone(), vec![]);
        let mut t = Trace::default();
        let start = Instant::now();
        let seen = stream_wait(&fake, &s, start, bounds(WAIT), &mut t)
            .await
            .unwrap();
        assert_eq!(found(&seen), (Cause::InitialScan, false));
        assert_eq!(start.elapsed(), Duration::ZERO);
        // Between readiness and the scan: stored after registration, with
        // no event at all, and still the first scan's.
        let (fake, _tx, store) = rig();
        fake.on_ready.borrow_mut().push("tok".into());
        let s = Store::new(store, vec![]);
        let mut t = Trace::default();
        let seen = stream_wait(&fake, &s, Instant::now(), bounds(WAIT), &mut t)
            .await
            .unwrap();
        assert_eq!(found(&seen), (Cause::InitialScan, false));
        assert_eq!(t.scans, [Cause::InitialScan]);
    }

    #[tokio::test(start_paused = true)]
    async fn an_event_during_a_scan_is_kept_and_wakes_the_next_scan() {
        let (fake, tx, store) = rig();
        let gate = Rc::new(Notify::new());
        let s = Store::new(store.clone(), vec![Step::Gate(gate.clone())]);
        let started = s.started.clone();
        let mut t = Trace::default();
        let start = Instant::now();
        let observer = stream_wait(&fake, &s, start, bounds(WAIT), &mut t);
        let deliver = async {
            // The first scan has taken its snapshot: the mail lands now and
            // its event arrives while that scan still runs.
            started.notified().await;
            store.borrow_mut().push("tok".into());
            tx.send(Ok(Wake::Delivery)).unwrap();
            tokio::task::yield_now().await;
            gate.notify_one();
        };
        let (seen, ()) = tokio::join!(observer, deliver);
        assert_eq!(found(&seen.unwrap()), (Cause::Event, false));
        assert_eq!(t.scans, [Cause::InitialScan, Cause::Event]);
        assert_eq!(t.events, 1);
        // Well before the backstop: the event, not the timer, found it.
        assert!(start.elapsed() < Duration::from_millis(1000));
    }

    #[tokio::test(start_paused = true)]
    async fn events_alone_never_answer_and_the_wait_times_out_at_its_cutoff() {
        let (fake, tx, store) = rig();
        for _ in 0..5 {
            tx.send(Ok(Wake::Delivery)).unwrap();
            tx.send(Ok(Wake::Other)).unwrap();
        }
        let s = Store::new(store, vec![]);
        let mut t = Trace::default();
        let start = Instant::now();
        let wait = Duration::from_millis(1000);
        let seen = stream_wait(&fake, &s, start, bounds(wait), &mut t)
            .await
            .unwrap();
        assert_eq!(seen, Seen::Timeout);
        assert_eq!(start.elapsed(), wait);
        // Five events were coalesced into scans; the `Other` traffic woke
        // none; nothing started at the cutoff, where the backstop also falls.
        assert_eq!(t.events, 5);
        assert!(t.scans.len() <= 6, "{:?}", t.scans);
        assert_eq!(t.scans[0], Cause::InitialScan);
        assert!(!t.scans.contains(&Cause::Backstop), "{:?}", t.scans);
        // The connection is owned by the observation and ends with it.
        assert!(tx.is_closed());
    }

    #[tokio::test(start_paused = true)]
    async fn a_lost_event_is_answered_by_the_backstop_anchored_at_readiness() {
        let (fake, _tx, store) = rig();
        let s = Store::new(store.clone(), vec![]);
        let mut t = Trace::default();
        let start = Instant::now();
        let observer = stream_wait(&fake, &s, start, bounds(WAIT), &mut t);
        let lose = async {
            sleep(Duration::from_millis(300)).await;
            // Stored, and its notification dropped by the hub.
            store.borrow_mut().push("tok".into());
        };
        let (seen, ()) = tokio::join!(observer, lose);
        assert_eq!(found(&seen.unwrap()), (Cause::Backstop, false));
        assert_eq!(start.elapsed(), Duration::from_millis(1000));
        assert_eq!(t.scans, [Cause::InitialScan, Cause::Backstop]);
    }

    #[tokio::test(start_paused = true)]
    async fn a_lost_connection_is_reconnected_and_rescanned_after_readiness() {
        let (fake, tx, store) = rig();
        let (tx2, rx2) = mpsc::unbounded_channel();
        fake.readies
            .borrow_mut()
            .push_back(Ready::Fail(StreamError::Io {
                kind: std::io::ErrorKind::ConnectionRefused,
                message: "refused".into(),
            }));
        fake.readies.borrow_mut().push_back(Ready::Now(rx2));
        // Stored while no connection exists: only the rescan after the
        // re-established readiness can see it before the backstop.
        fake.on_ready.borrow_mut().clear();
        let s = Store::new(store.clone(), vec![]);
        let mut t = Trace::default();
        let start = Instant::now();
        let observer = stream_wait(&fake, &s, start, bounds(WAIT), &mut t);
        let cut = async {
            sleep(Duration::from_millis(50)).await;
            drop(tx);
            store.borrow_mut().push("tok".into());
        };
        let (seen, ()) = tokio::join!(observer, cut);
        assert_eq!(found(&seen.unwrap()), (Cause::Reconnect, false));
        // Lost at 50 ms; the first attempt at 150 ms is refused; the second,
        // one gap later, is ready at 250 ms.
        assert_eq!(start.elapsed(), Duration::from_millis(250));
        assert_eq!(t.connects, 2);
        assert_eq!(t.reconnect_failures, 1);
        assert_eq!(t.scans, [Cause::InitialScan, Cause::Reconnect]);
        assert!(tx2.is_closed());
    }

    #[tokio::test(start_paused = true)]
    async fn a_stalled_reconnect_does_not_hold_back_the_backstop_or_the_cutoff() {
        let (fake, tx, store) = rig();
        fake.readies.borrow_mut().push_back(Ready::Stall);
        let s = Store::new(store.clone(), vec![]);
        let mut t = Trace::default();
        let start = Instant::now();
        let observer = stream_wait(&fake, &s, start, bounds(WAIT), &mut t);
        let cut = async {
            sleep(Duration::from_millis(10)).await;
            drop(tx);
            sleep(Duration::from_millis(200)).await;
            store.borrow_mut().push("tok".into());
        };
        let (seen, ()) = tokio::join!(observer, cut);
        // The reconnect attempt started at 110 ms is still pending: its
        // bound is one request bound (10 s). The backstop answers.
        assert_eq!(found(&seen.unwrap()), (Cause::Backstop, false));
        assert_eq!(start.elapsed(), Duration::from_millis(1000));
        assert_eq!(t.connects, 1);

        // With nothing to find, every attempt stalls to its bound, the
        // backstop keeps scanning, and the cutoff still ends the wait.
        let (fake, tx, store) = rig();
        drop(tx);
        for _ in 0..10 {
            fake.readies.borrow_mut().push_back(Ready::Stall);
        }
        let s = Store::new(store, vec![]);
        let mut t = Trace::default();
        let start = Instant::now();
        let wait = Duration::from_millis(3500);
        let seen = stream_wait(&fake, &s, start, bounds(wait), &mut t)
            .await
            .unwrap();
        assert_eq!(seen, Seen::Timeout);
        assert_eq!(start.elapsed(), wait);
        assert_eq!(
            t.scans,
            [
                Cause::InitialScan,
                Cause::Backstop,
                Cause::Backstop,
                Cause::Backstop
            ]
        );
        // One attempt in flight, bounded by the cutoff: the cutoff wins the
        // instant they share, and nothing starts after it.
        assert_eq!(t.reconnect_failures, 0);
        assert_eq!(t.connects, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn an_event_flood_is_coalesced_and_starves_neither_backstop_nor_cutoff() {
        let (fake, tx, store) = rig();
        // A burst far larger than any scan count, queued at once.
        for _ in 0..100_000 {
            tx.send(Ok(Wake::Delivery)).unwrap();
        }
        let s = Store::new(
            store,
            (0..64)
                .map(|_| Step::Sleep(Duration::from_millis(400)))
                .collect(),
        );
        let mut t = Trace::default();
        let start = Instant::now();
        let wait = Duration::from_millis(2500);
        let observer = stream_wait(&fake, &s, start, bounds(wait), &mut t);
        let flood = async {
            // Then one more every millisecond, for longer than the wait.
            for _ in 0..4000 {
                sleep(Duration::from_millis(1)).await;
                if tx.send(Ok(Wake::Delivery)).is_err() {
                    break;
                }
            }
        };
        let (seen, ()) = tokio::join!(observer, flood);
        assert_eq!(seen.unwrap(), Seen::Timeout);
        // The cutoff fell inside a scan (0, 400, ..., 2400): it ends at
        // 2800 ms, inside its response allowance, empty.
        assert_eq!(start.elapsed(), Duration::from_millis(2800));
        assert!(t.events > 100_000, "{}", t.events);
        // Back to back scans, never one per event, and the backstop ticks
        // still won their scans over the pending events.
        assert_eq!(t.scans.len(), 7, "{:?}", t.scans);
        assert!(t.scans.contains(&Cause::Backstop), "{:?}", t.scans);
    }

    #[tokio::test(start_paused = true)]
    async fn coincident_causes_discharge_together_in_their_precedence() {
        let (fake, tx, store) = rig();
        let (_tx2, rx2) = mpsc::unbounded_channel();
        fake.readies.borrow_mut().push_back(Ready::Now(rx2));
        // The first scan runs 1.5 s: an event, a lost and re-established
        // connection and a backstop tick all arrive while it runs.
        let s = Store::new(
            store.clone(),
            vec![Step::Sleep(Duration::from_millis(1500))],
        );
        let mut t = Trace::default();
        let start = Instant::now();
        let observer = stream_wait(&fake, &s, start, bounds(WAIT), &mut t);
        let events = async {
            sleep(Duration::from_millis(10)).await;
            store.borrow_mut().push("tok".into());
            tx.send(Ok(Wake::Delivery)).unwrap();
            sleep(Duration::from_millis(10)).await;
            drop(tx);
        };
        let (seen, ()) = tokio::join!(observer, events);
        // Reconnect outranks the backstop and the event; one scan
        // discharges all three.
        assert_eq!(found(&seen.unwrap()), (Cause::Reconnect, false));
        assert_eq!(start.elapsed(), Duration::from_millis(1500));
        assert_eq!(t.scans, [Cause::InitialScan, Cause::Reconnect]);
        assert_eq!(t.connects, 2);
    }

    #[tokio::test(start_paused = true)]
    async fn a_match_after_the_cutoff_is_late_and_an_unfinished_scan_is_an_error() {
        // The first scan ends after the cutoff, inside its response
        // allowance, with the mail: late, censored, never timely.
        let (fake, _tx, store) = rig();
        store.borrow_mut().push("tok".into());
        let s = Store::new(store, vec![Step::Sleep(Duration::from_millis(1500))]);
        let mut t = Trace::default();
        let seen = stream_wait(
            &fake,
            &s,
            Instant::now(),
            bounds(Duration::from_secs(1)),
            &mut t,
        )
        .await
        .unwrap();
        assert_eq!(found(&seen), (Cause::InitialScan, true));
        // A scan still running one request bound after the cutoff.
        let (fake, _tx, store) = rig();
        let s = Store::new(store, vec![Step::Forever]);
        let mut t = Trace::default();
        let start = Instant::now();
        let e = stream_wait(&fake, &s, start, bounds(Duration::from_secs(1)), &mut t)
            .await
            .unwrap_err();
        assert!(e.starts_with("operation deadline"), "{e}");
        assert_eq!(start.elapsed(), Duration::from_secs(11));
    }

    #[tokio::test(start_paused = true)]
    async fn a_failed_scan_readiness_or_channel_is_an_error_never_an_empty_answer() {
        // A failed scan.
        let (fake, _tx, store) = rig();
        let s = Store::new(store, vec![Step::Fail]);
        let mut t = Trace::default();
        let e = stream_wait(&fake, &s, Instant::now(), bounds(WAIT), &mut t)
            .await
            .unwrap_err();
        assert_eq!(e, "incomplete: a page was short");
        // Readiness never proved: no scan, and no silent poll.
        let fake = Fake::default();
        fake.readies
            .borrow_mut()
            .push_back(Ready::Fail(StreamError::Status(404)));
        let s = Store::new(fake.store.clone(), vec![]);
        let mut t = Trace::default();
        let e = stream_wait(&fake, &s, Instant::now(), bounds(WAIT), &mut t)
            .await
            .unwrap_err();
        assert_eq!(e, "readiness: stream: unexpected status 404");
        assert_eq!(s.scans.get(), 0);
        // Readiness that stalls is bounded by one request bound.
        let fake = Fake::default();
        fake.readies.borrow_mut().push_back(Ready::Stall);
        let s = Store::new(fake.store.clone(), vec![]);
        let start = Instant::now();
        let e = stream_wait(&fake, &s, start, bounds(WAIT), &mut t)
            .await
            .unwrap_err();
        assert_eq!(e, "readiness: stream: deadline passed");
        assert_eq!(start.elapsed(), Duration::from_secs(10));
        // A malformed channel.
        let (fake, tx, store) = rig();
        tx.send(Err(StreamError::Frame("a reserved opcode")))
            .unwrap();
        let s = Store::new(store, vec![]);
        let mut t = Trace::default();
        let e = stream_wait(&fake, &s, Instant::now(), bounds(WAIT), &mut t)
            .await
            .unwrap_err();
        assert_eq!(e, "channel: stream: websocket frame: a reserved opcode");
        // A reconnect that is answered, but not with an upgrade.
        let (fake, tx, store) = rig();
        drop(tx);
        fake.readies
            .borrow_mut()
            .push_back(Ready::Fail(StreamError::Handshake(
                "no Upgrade: websocket".into(),
            )));
        let s = Store::new(store, vec![]);
        let mut t = Trace::default();
        let e = stream_wait(&fake, &s, Instant::now(), bounds(WAIT), &mut t)
            .await
            .unwrap_err();
        assert_eq!(
            e,
            "reconnect: stream: websocket handshake: no Upgrade: websocket"
        );
    }

    #[test]
    fn causes_name_and_parse_in_precedence_order() {
        let names: Vec<_> = Cause::ALL.iter().map(|c| c.name()).collect();
        assert_eq!(names, ["initial_scan", "reconnect", "backstop", "event"]);
        for c in Cause::ALL {
            assert_eq!(Cause::parse(c.name()), Some(c));
        }
        assert_eq!(Cause::parse("replay"), None);
        let mut sorted = Cause::ALL;
        sorted.sort();
        assert_eq!(sorted, Cause::ALL);
    }
}

// ============================================================================
// T5: MailHog's channel over real loopback TCP. The server plays the
// captured hub's order: upgrade, register, then a read loop that answers a
// ping with a pong; a broadcast is the stored message's JSON as text.
// ============================================================================

mod mailhog_channel {
    use super::{client_frame, frame, header, read_request, upgrade_answer};
    use crate::observe::{Channel, MailhogChannel, Wake};
    use crate::stream::{Limits, StreamError};
    use std::time::Duration;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;
    use tokio::time::Instant;

    fn soon() -> Instant {
        Instant::now() + Duration::from_secs(10)
    }

    /// A one-connection hub: the server's frames before and after its pong
    /// are `before` and `after`; `pong` maps the client's ping payload.
    async fn hub(
        before: Vec<u8>,
        pong: fn(Vec<u8>) -> Vec<u8>,
        after: Vec<u8>,
    ) -> (MailhogChannel, tokio::task::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let request = read_request(&mut s).await;
            s.write_all(upgrade_answer(&request, "").as_bytes())
                .await
                .unwrap();
            s.write_all(&before).await.unwrap();
            let (op, payload) = client_frame(&mut s).await;
            assert_eq!(op, 0x89, "the client's first frame is its ping");
            assert_eq!(payload.len(), 24, "a fresh 16-byte key, base64");
            s.write_all(&frame(0x8A, &pong(payload))).await.unwrap();
            s.write_all(&after).await.unwrap();
            // Hold the connection until the client drops it.
            let mut rest = Vec::new();
            let _ = tokio::io::AsyncReadExt::read_to_end(&mut s, &mut rest).await;
            request
        });
        let channel = MailhogChannel {
            addr,
            path: crate::protocol::stream_route(crate::protocol::TargetKind::Mailhog).unwrap(),
            limits: Limits::DEFAULT,
        };
        (channel, server)
    }

    #[tokio::test]
    async fn readiness_is_the_pong_of_this_connections_own_ping() {
        // A broadcast that lands between registration and the pong is
        // skipped (the first scan follows readiness); after the pong, text
        // is a delivery and a pong is other traffic.
        let mut after = frame(0x81, br#"{"ID":"x@mailhog.example"}"#);
        after.extend(frame(0x8A, b"late"));
        let (channel, server) = hub(frame(0x81, b"{}"), |p| p, after).await;
        let mut conn = channel.ready(soon()).await.expect("ready");
        assert_eq!(
            MailhogChannel::next(&mut conn, soon()).await,
            Ok(Wake::Delivery)
        );
        assert_eq!(
            MailhogChannel::next(&mut conn, soon()).await,
            Ok(Wake::Other)
        );
        drop(conn);
        let request = server.await.unwrap();
        assert!(
            request.starts_with("GET /api/v2/websocket HTTP/1.1\r\n"),
            "{request}"
        );
        assert_eq!(header(&request, "Host"), channel.addr.to_string());
    }

    #[tokio::test]
    async fn a_foreign_pong_or_a_binary_message_is_refused() {
        let (channel, _server) = hub(Vec::new(), |_| b"someone else's".to_vec(), Vec::new()).await;
        let e = channel.ready(soon()).await.err().expect("refused");
        assert_eq!(
            e,
            StreamError::Handshake("a pong that does not echo this connection's ping".into())
        );
        let (channel, _server) = hub(frame(0x82, b"\x00"), |p| p, Vec::new()).await;
        let e = channel.ready(soon()).await.err().expect("refused");
        assert_eq!(e, StreamError::Frame("a binary message on a JSON channel"));
        let (channel, _server) = hub(Vec::new(), |p| p, frame(0x82, b"\x00")).await;
        let mut conn = channel.ready(soon()).await.expect("ready");
        assert_eq!(
            MailhogChannel::next(&mut conn, soon()).await,
            Err(StreamError::Frame("a binary message on a JSON channel"))
        );
    }

    #[tokio::test]
    async fn a_hub_that_never_answers_the_ping_is_not_ready_by_the_deadline() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let _server = tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let request = read_request(&mut s).await;
            s.write_all(upgrade_answer(&request, "").as_bytes())
                .await
                .unwrap();
            // Registered, but its read loop never starts.
            std::future::pending::<()>().await;
        });
        let channel = MailhogChannel {
            addr,
            path: "/api/v2/websocket",
            limits: Limits::DEFAULT,
        };
        let e = channel
            .ready(Instant::now() + Duration::from_millis(200))
            .await
            .err()
            .expect("not ready");
        assert_eq!(e, StreamError::Deadline);
    }
}

// ============================================================================
// T7/T9: v4 wait rows, their facts and the wait section's arithmetic
// ============================================================================

mod accounting {
    use crate::protocol::{Protocol, TargetKind};
    use crate::records::{Phase, Row, WindowKind, WindowOutcome};
    use crate::{json, protocol_from, report, runner};
    use serde_json::json;

    const DRAFT_PROTOCOL: &[u8] = include_bytes!("../../docs/evals/draft/protocol.v4.json");

    /// The draft with MailHog's wait-best row streaming.
    fn protocol() -> Protocol {
        let mut v = json(DRAFT_PROTOCOL);
        let o = v["waits"][1]["observation"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|o| o["target"] == "mailhog")
            .unwrap();
        o["mode"] = json!("stream");
        o["interval_ms"] = serde_json::Value::Null;
        o["backstop_ms"] = json!(1000);
        protocol_from(v).unwrap()
    }

    /// A wait row: `(expected, unrelated)` counts, times in ms.
    fn wait(phase: Phase, i: u32, outcome: &str, counts: Option<(u64, u64)>) -> Row {
        let mut r = runner::row(
            "run",
            TargetKind::Mailhog,
            "wait-best",
            0,
            phase,
            i,
            outcome,
        );
        r.token = Some(format!("tok{i}"));
        r.invoked_ns = Some(1_000_000);
        if let Some((e, u)) = counts {
            (r.expected, r.unrelated) = (Some(e), Some(u));
        }
        if matches!(outcome, "observed" | "late" | "unrelated") {
            r.cause = Some("event".into());
        }
        r
    }

    fn at(mut r: Row, start: u64, ack: Option<u64>, seen: Option<u64>) -> Row {
        r.start_ns = Some(start);
        r.ack_ns = ack;
        r.observed_ns = seen;
        r
    }

    /// The measure window of the profile, never opened: no counters.
    fn window() -> crate::records::ResourceRow {
        crate::records::ResourceRow::Window(runner::unopened_window(
            "run",
            "target-0001",
            (TargetKind::Mailhog, "wait-best", 0),
            WindowKind::Measure,
            9,
            WindowOutcome::ChunkDeadline,
        ))
    }

    fn rows() -> Vec<Row> {
        let mut c = wait(Phase::Future, 2, "observed", Some((1, 0)));
        c.cause = Some("backstop".into());
        vec![
            // Present: one exact, one duplicated, one late, one error.
            at(
                wait(Phase::Present, 0, "observed", Some((1, 0))),
                0,
                Some(5),
                Some(4_000_000),
            ),
            at(
                wait(Phase::Present, 1, "observed", Some((2, 0))),
                0,
                Some(5),
                Some(2_000_000),
            ),
            at(
                wait(Phase::Present, 2, "late", Some((1, 0))),
                0,
                Some(5),
                Some(9_000_000),
            ),
            wait(Phase::Present, 3, "error", None),
            // Future: seen before the 250 (a negative offset, kept), after
            // it, by a backstop, with no ack timestamp (unavailable), and
            // an answer with the mail and other mail (both facts).
            at(
                wait(Phase::Future, 0, "observed", Some((1, 0))),
                10,
                Some(500),
                Some(400),
            ),
            at(
                wait(Phase::Future, 1, "observed", Some((1, 0))),
                10,
                Some(500),
                Some(900),
            ),
            at(c, 10, Some(500), Some(1_700)),
            at(
                wait(Phase::Future, 3, "observed", Some((1, 0))),
                10,
                None,
                Some(800),
            ),
            at(
                wait(Phase::Future, 4, "unrelated", Some((2, 1))),
                10,
                Some(500),
                Some(600),
            ),
            // An answer with only other mail is a miss, as is a timeout.
            at(
                wait(Phase::Future, 5, "unrelated", Some((0, 3))),
                10,
                Some(500),
                Some(700),
            ),
            at(
                wait(Phase::Future, 6, "timeout", Some((0, 0))),
                10,
                Some(500),
                None,
            ),
            wait(Phase::Future, 7, "send_failed", None),
            wait(Phase::Future, 8, "unstarted", None),
            // Empty: a correct silence and an answer with foreign mail.
            wait(Phase::Empty, 0, "timeout", Some((0, 0))),
            at(
                wait(Phase::Empty, 1, "unrelated", Some((0, 1))),
                0,
                None,
                Some(3),
            ),
        ]
    }

    #[test]
    fn duplicates_unrelated_misses_and_signed_offsets_are_counted_independently() {
        let p = protocol();
        let rows = rows();
        report::check_wait_rows(&p, &rows).unwrap();
        let reps = report::wait_figures(&p, &rows, &[window()]);
        assert_eq!(reps.len(), 1);
        let r = &reps[0];
        assert_eq!((r.method, r.interval_ms), ("stream", None));
        // 4 present + 7 future were sent; send_failed and unstarted were not.
        assert_eq!(r.sent, 11);
        assert_eq!(r.timely, 7, "present 0, 1 and future 0 to 4");
        assert_eq!(r.missed, 4, "late, error, foreign-only answer, timeout");
        assert_eq!(r.duplicate, 2, "present 1 and future 4");
        assert_eq!(r.unrelated, 3, "future 4, future 5 and empty 1");
        assert_eq!(r.duplicate_and_unrelated, 1);
        assert_eq!(r.empty_correct, 1);
        // Every answer of a stream names its cause; nothing else does.
        let causes: Vec<(&str, u64)> = r.causes.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        assert_eq!(causes, [("backstop", 1), ("event", 9)]);
        // Signed, never clamped; the ack-less row is unavailable, not zero;
        // the misses are censored.
        assert_eq!(r.ack_to_return.samples, 4);
        assert_eq!(r.ack_to_return.censored, 2);
        let p50 = r
            .ack_to_return
            .values
            .iter()
            .find(|(q, _)| *q == 50)
            .unwrap()
            .1;
        assert_eq!(p50, Some(100));
        let mut acks: Vec<i64> = rows
            .iter()
            .filter(|x| x.phase == Phase::Future && x.outcome != "timeout")
            .filter_map(|x| Some(x.observed_ns? as i64 - x.ack_ns? as i64))
            .collect();
        acks.sort();
        assert_eq!(acks[0], -100, "seen before the 250");
        assert_eq!(r.present_latency.samples, 2);
        assert_eq!(r.present_latency.censored, 2);
        // A never-opened window has no CPU: null, never zero.
        assert_eq!(r.window_outcome, Some(WindowOutcome::ChunkDeadline));
        assert_eq!(r.cpu_total_usec, None);
        let cells = report::wait_cells(&p, &rows, &reps);
        assert_eq!(cells.len(), 1);
        let c = &cells[0];
        assert_eq!((c.sent, c.timely, c.missed), (11, 7, 4));
        assert_eq!(
            (c.duplicate, c.unrelated, c.duplicate_and_unrelated),
            (2, 3, 1)
        );
        assert_eq!(c.ack_to_return.samples, 4);
        assert_eq!(c.cpu_total_usec, None);
        // Rows of a repetition whose windows another chunk holds are that
        // chunk's: nothing is derived without its window.
        assert!(report::wait_figures(&p, &rows, &[]).is_empty());
    }

    #[test]
    fn a_row_whose_facts_disagree_with_its_outcome_is_refused_by_name() {
        let p = protocol();
        type RowEdit = Box<dyn Fn(&mut Row)>;
        let edits: Vec<(&str, RowEdit)> = vec![
            (
                "disagree with its outcome \"observed\"",
                Box::new(|r| r.unrelated = Some(1)),
            ),
            (
                "disagree with its outcome \"observed\"",
                Box::new(|r| r.expected = Some(0)),
            ),
            (
                "disagree with its outcome \"observed\"",
                Box::new(|r| r.observed_ns = None),
            ),
            (
                "disagree with its outcome \"timeout\"",
                Box::new(|r| {
                    r.outcome = "timeout".into();
                    r.observed_ns = None;
                }),
            ),
            (
                "disagree with its outcome \"error\"",
                Box::new(|r| r.outcome = "error".into()),
            ),
            (
                "disagree with its outcome \"unrelated\"",
                Box::new(|r| r.outcome = "unrelated".into()),
            ),
            (
                "outcome \"vanished\" is not a wait outcome",
                Box::new(|r| r.outcome = "vanished".into()),
            ),
            (
                "a stream answer names no scan cause",
                Box::new(|r| r.cause = None),
            ),
            (
                "cause \"replay\" is not a scan cause",
                Box::new(|r| r.cause = Some("replay".into())),
            ),
            (
                "no wait profile observes mailhog by that name",
                Box::new(|r| r.profile = "wait-later".into()),
            ),
        ];
        for (want, edit) in edits {
            let mut r = at(
                wait(Phase::Future, 0, "observed", Some((1, 0))),
                1,
                Some(2),
                Some(3),
            );
            edit(&mut r);
            let e = report::check_wait_rows(&p, &[r]).unwrap_err();
            assert!(e.contains(want), "{want}: {e}");
            assert!(e.starts_with("wait row mailhog wait-"), "{e}");
        }
        // A poll's answer names no cause.
        let mut r = at(
            wait(Phase::Future, 0, "observed", Some((1, 0))),
            1,
            Some(2),
            Some(3),
        );
        r.profile = "wait-common".into();
        let e = report::check_wait_rows(&p, &[r]).unwrap_err();
        assert!(
            e.contains("cause \"event\" on a row that is no stream answer"),
            "{e}"
        );
        // Cost rows are not wait rows.
        let send = runner::row(
            "run",
            TargetKind::Mailhog,
            "cost-c1-reused",
            0,
            Phase::Send,
            0,
            "accepted",
        );
        report::check_wait_rows(&p, &[send]).unwrap();
    }

    #[test]
    fn earlier_protocols_refuse_rows_that_carry_protocol_fours_facts() {
        let clean = runner::row(
            "run",
            TargetKind::Swarmail,
            "wait-common",
            0,
            Phase::Future,
            0,
            "observed",
        );
        report::refuse_v4_row_facts(std::slice::from_ref(&clean)).unwrap();
        for f in [
            |r: &mut Row| r.expected = Some(1),
            |r: &mut Row| r.unrelated = Some(0),
            |r: &mut Row| r.cause = Some("event".into()),
        ] {
            let mut r = clean.clone();
            f(&mut r);
            let e = report::refuse_v4_row_facts(&[r.clone()]).unwrap_err();
            assert!(e.contains("carries protocol 4's wait facts"), "{e}");
            // Serialized, an old row has none of the keys at all.
            let v = serde_json::to_value(&clean).unwrap();
            for k in ["expected", "unrelated", "cause"] {
                assert!(v.get(k).is_none(), "{k}");
            }
        }
    }
}

// ============================================================================
// T10: the real-binary proof of each implemented stream candidate. Ignored
// by default; run on the dispatcher host, one at a time, under
// `timeout 600 taskset -c 0-31` (docs/evals/README.md names the command).
// It fails, never skips, without its digest-checked bytes.
// ============================================================================

mod real {
    use crate::observe::{
        Cause, Channel, MailhogChannel, Scanner, Seen, StreamBounds, Trace, Wake, stream_wait,
    };
    use crate::protocol::{SourceLock, TargetKind};
    use crate::{json, targets, wire};
    use std::ffi::OsString;
    use std::time::Duration;
    use tokio::sync::Notify;

    const DRAFT_LOCK: &[u8] = include_bytes!("../../docs/evals/draft/sources.lock.v5.json");

    /// The variable that names a stream candidate's pinned executable.
    pub fn variable(t: TargetKind) -> String {
        format!("MAIL_COMPARE_{}", t.name().to_uppercase().replace('-', "_"))
    }

    /// The launch of `t`'s pinned executable from lock v5, whose digest the
    /// launch checks before it spawns; an absent variable is a refusal by
    /// name, never a skip.
    pub fn pinned_v5(
        t: TargetKind,
        value: Option<OsString>,
    ) -> Result<targets::LaunchSpec, String> {
        let var = variable(t);
        let binary = value.ok_or_else(|| {
            format!(
                "{var} must name the pinned {t} executable (lock v5's digest is checked at launch)"
            )
        })?;
        let lock = SourceLock::parse(DRAFT_LOCK).map_err(|r| r.to_string())?;
        let pin = lock.pin(t).ok_or_else(|| format!("{t} has no pin"))?;
        targets::LaunchSpec::from_pin(t, binary.into(), None, pin, None)
            .map_err(|e| format!("{t}: {e:?}"))
    }

    #[test]
    fn a_missing_pinned_executable_is_a_refusal_by_name() {
        let e = pinned_v5(TargetKind::Mailhog, None).unwrap_err();
        assert_eq!(
            e,
            "MAIL_COMPARE_MAILHOG must name the pinned mailhog executable (lock v5's digest is checked at launch)"
        );
        // Lock v5's pin is MailHog's, with no cap placeholder to fill.
        let lock = json(DRAFT_LOCK);
        assert!(lock.to_string().contains("mailhog"));
    }

    /// The first complete scan, then a signal: what the proof sends after
    /// it can only be found by a later scan.
    struct Signalled<'a> {
        owned: &'a targets::Owned,
        token: &'a str,
        first_done: &'a Notify,
        scans: std::cell::Cell<u32>,
    }

    impl Scanner for Signalled<'_> {
        async fn scan(&self) -> Result<Vec<String>, String> {
            let got = self
                .owned
                .find(self.token)
                .await
                .map_err(|e| e.reason().to_owned());
            self.scans.set(self.scans.get() + 1);
            if self.scans.get() == 1 {
                self.first_done.notify_one();
            }
            got
        }
    }

    /// MailHog's api/v2/websocket against the pinned executable: readiness
    /// by the matched pong, then a real SMTP delivery, then an event, then
    /// a complete authoritative scan that finds the mail. The backstop is set
    /// beyond the wait, so neither the initial scan nor the backstop can
    /// answer: only the event can.
    #[cfg(target_os = "linux")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[ignore = "needs MAIL_COMPARE_MAILHOG (the pinned MailHog executable); dispatcher host, wait-best T10"]
    async fn waits_real_mailhog_stream_event_wakes_an_authoritative_match() {
        let spec = pinned_v5(
            TargetKind::Mailhog,
            std::env::var_os(variable(TargetKind::Mailhog)),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let root = tempfile::tempdir().unwrap();
        let clock = wire::Clock(tokio::time::Instant::now().into_std());
        let mut owned = targets::Owned::launch(
            &spec,
            &root.path().join("s"),
            &crate::limits(),
            &clock,
            &targets::Keepers::default(),
        )
        .await
        .unwrap_or_else(|e| panic!("mailhog did not launch: {e:?}"));
        let channel = MailhogChannel {
            addr: owned.http_addr,
            path: crate::protocol::stream_route(TargetKind::Mailhog).unwrap(),
            limits: crate::stream::Limits::DEFAULT,
        };
        let soon = || tokio::time::Instant::now() + Duration::from_secs(10);

        // 1. Readiness, then a delivery, then its event, then the match.
        let mut conn = channel
            .ready(soon())
            .await
            .expect("readiness by the matched pong");
        let first = wire::authored(7, "stream-proof", 0, &[crate::protocol::MessageKind::Plain]);
        assert!(
            owned.find(&first.token).await.unwrap().is_empty(),
            "nothing yet"
        );
        crate::send_authored(&owned, &first).await;
        let wake = MailhogChannel::next(&mut conn, soon()).await;
        assert_eq!(wake, Ok(Wake::Delivery), "the delivery's event fired");
        assert_eq!(
            owned.find(&first.token).await.unwrap(),
            std::slice::from_ref(&first.token)
        );
        drop(conn);

        // 2. The observer end to end: the answer is an event's scan.
        let second = wire::authored(7, "stream-proof", 1, &[crate::protocol::MessageKind::Plain]);
        let first_done = Notify::new();
        let scanner = Signalled {
            owned: &owned,
            token: &second.token,
            first_done: &first_done,
            scans: std::cell::Cell::new(0),
        };
        let bounds = StreamBounds {
            wait: Duration::from_secs(30),
            request: Duration::from_secs(10),
            backstop: Duration::from_secs(60),
            reconnect_gap: crate::observe::RECONNECT_GAP,
        };
        let mut trace = Trace::default();
        let start = tokio::time::Instant::now();
        let (seen, ()) = tokio::join!(
            stream_wait(&channel, &scanner, start, bounds, &mut trace),
            async {
                first_done.notified().await;
                crate::send_authored(&owned, &second).await;
            }
        );
        let seen = seen.expect("the stream observation");
        let Seen::Answer {
            tokens,
            cause,
            late,
        } = seen
        else {
            panic!("no answer: {seen:?} after {trace:?}");
        };
        assert_eq!(tokens, std::slice::from_ref(&second.token));
        assert_eq!(cause, Cause::Event, "{trace:?}");
        assert!(!late);
        assert_eq!(trace.scans[0], Cause::InitialScan);
        assert!(!trace.scans.contains(&Cause::Backstop), "{trace:?}");
        assert!(trace.events >= 1);
        eprintln!(
            "mailhog stream proof: answered by {:?} in {:?}; scans {:?}, events {}, connects {}",
            cause,
            start.elapsed(),
            trace.scans,
            trace.events,
            trace.connects
        );

        // 3. Cleanup, proved.
        let facts = owned.shutdown().await;
        assert!(facts.proved(), "{facts:?}");
        eprintln!("mailhog stream proof: cleanup {facts:?}");
    }
}
