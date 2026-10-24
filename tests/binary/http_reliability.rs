//! Public helper regressions use real peers. Cold assertion diagnostics
//! belong under tests/ per accepted decision 0020.
use super::common;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn request(addr: SocketAddr, helper: usize) -> (u16, Vec<u8>) {
    let (status, text) = match helper {
        0 => common::http_get_text(addr, "/").await,
        1 => return common::http_bytes(addr, "/").await,
        2 => common::send_with_host(addr, "GET", "/", "localhost", None).await,
        3 => common::http_send_text(addr, "GET", "/", None).await,
        4 => common::http_send_text_allow_eof(addr, "GET", "/", None).await,
        _ => unreachable!(),
    };
    (status, text.into_bytes())
}

fn panic_text(e: tokio::task::JoinError) -> String {
    let payload = e.into_panic();
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
        .expect("text diagnostic")
}

async fn exchange(reply: &'static [u8], helper: usize) -> Result<(u16, Vec<u8>), String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let client = tokio::spawn(request(addr, helper));
    let (mut peer, _) = listener.accept().await.unwrap();
    let mut request = Vec::new();
    tokio::time::timeout(common::HELPER_DEADLINE, async {
        while !request.ends_with(b"\r\n\r\n") {
            request.push(peer.read_u8().await.unwrap());
        }
        peer.write_all(reply).await.unwrap();
        peer.shutdown().await.unwrap();
    })
    .await
    .expect("real peer exchange");
    client.await.map_err(panic_text)
}

#[tokio::test]
async fn strict_http_helpers_reject_missing_or_torn_responses_clearly() {
    for helper in 0..5 {
        for (reply, reason) in [
            (&b""[..], "empty response"),
            (
                &b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n"[..],
                "incomplete header/body boundary",
            ),
            (
                &b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nx"[..],
                "Content-Length 5, received 1 body bytes",
            ),
        ] {
            if helper == 4 && reply.is_empty() {
                continue;
            } // explicit no-answer observer
            let error = exchange(reply, helper)
                .await
                .expect_err("torn response must fail");
            assert!(
                error.contains("invalid HTTP response:"),
                "helper {helper}: {error}"
            );
            assert!(error.contains(reason), "helper {helper}: {error}");
        }
    }
}

#[tokio::test]
async fn complete_empty_http_bodies_keep_status_and_binary_bodies_keep_bytes() {
    for helper in 0..5 {
        for (reply, status) in [
            (
                &b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\n\r\n"[..],
                202,
            ),
            (&b"HTTP/1.1 204 No Content\r\n\r\n"[..], 204),
            (&b"HTTP/1.1 304 Not Modified\r\n\r\n"[..], 304),
            (
                &b"HTTP/1.1 503 Storage Refused\r\nContent-Length: 0\r\n\r\n"[..],
                503,
            ),
        ] {
            assert_eq!(exchange(reply, helper).await.unwrap(), (status, Vec::new()));
        }
    }
    assert_eq!(
        exchange(
            b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\n\r\n\xff\r\n\r\nx",
            1
        )
        .await
        .unwrap(),
        (200, b"\xff\r\n\r\nx".to_vec())
    );
}
