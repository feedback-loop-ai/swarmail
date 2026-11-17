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
