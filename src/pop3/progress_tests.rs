//! A progressing POP3 reader outlives the reply bound (issue #131,
//! decision 0046 §7, decision 0020 rule 2).
//!
//! The proof watches the server's own socket writes through the
//! `REPLY_WRITES` observer instead of timing its reader: a reader's
//! elapsed time proves nothing when the whole reply may already sit in
//! kernel buffers. Its cold paths are the diagnostics — a reply that fit
//! one window at the server, writes that were never delayed, a reader cut
//! off mid-message or never finishing inside the test deadline — which run
//! only while the schedule betrays or the whole-reply bound is back, so the
//! test cannot live inline.

use super::*;
use tokio::io::AsyncReadExt;

/// A 16 MiB RETR, far larger than every socket buffer, reaches a slow but
/// steady reader in full under a 1 s bound, and the server's writes show
/// why: the reply spanned more than one window at the server, at least two
/// of its writes completed 50 ms or more after they were entered (delayed
/// completion on a full socket — the observer records entry, completion
/// and count, not individual pending polls), and every one of them moved
/// bytes inside its own window. The reader drains flat out but pauses
/// 50 ms after every 256 KiB — its pace, not a synchronization; nothing is
/// ordered by the pause. (An evenly slow reader is the wrong shape: the
/// kernel wakes a blocked writer only once a share of its send buffer has
/// drained, so at a low steady rate one write could wait out a window with
/// bytes still moving.) Red on 2a325c9's one `write_all` under one bound:
/// the reply cannot finish inside a single window, so the reader is cut
/// off. An independent whole-test deadline bounds a broken read, and both
/// tasks are aborted and joined on every ending before any verdict.
#[tokio::test]
async fn pop3_retr_progress_survives_multiple_windows() {
    const PAUSE_EVERY: usize = 256 * 1024;
    let idle = Duration::from_secs(1);
    let mut raw = b"Subject: big\r\n\r\n".to_vec();
    raw.extend_from_slice(&[b"x".repeat(62), b"\r\n".to_vec()].concat().repeat(262_144));
    let mut email = crate::store::email("big-1", "big", "a@x.io");
    email.raw = raw.clone();
    let store = Arc::new(Store::new(0));
    store.insert(email).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    REPLY_WRITES.with(|replies| replies.borrow_mut().clear());
    let server = tokio::spawn(serve(
        listener,
        store,
        Arc::new(AtomicBool::new(false)),
        idle,
        1,
        std::future::pending(),
    ));
    let mut reader = tokio::spawn(async move {
        let socket = tokio::net::TcpSocket::new_v4().unwrap();
        socket.set_recv_buffer_size(64 * 1024).unwrap();
        let mut client = BufReader::new(socket.connect(addr).await.unwrap());
        client
            .write_all(b"USER big\r\nPASS x\r\nRETR 1\r\n")
            .await
            .unwrap();
        // Greeting, USER, PASS, then the RETR status line.
        let mut status = Vec::new();
        for _ in 0..4 {
            status.clear();
            client.read_until(b'\n', &mut status).await.unwrap();
            assert!(status.starts_with(b"+OK"), "{status:?}");
        }
        let want = raw.len() + b".\r\n".len();
        let mut got = Vec::with_capacity(want);
        let mut chunk = vec![0u8; 64 * 1024];
        let mut next_pause = PAUSE_EVERY;
        while got.len() < want {
            let n = client.read(&mut chunk).await.unwrap();
            assert!(n > 0, "cut off after {} of {want} bytes", got.len());
            got.extend_from_slice(&chunk[..n.min(want - got.len())]);
            if got.len() >= next_pause {
                next_pause += PAUSE_EVERY;
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
        assert!(got[..raw.len()] == raw[..], "the body arrived intact");
        assert_eq!(&got[raw.len()..], b".\r\n");
        status.len() + want
    });

    let finished = tokio::time::timeout(Duration::from_secs(120), &mut reader).await;
    reader.abort();
    server.abort();
    let reader_end = match finished {
        Ok(end) => end,
        Err(_) => reader.await,
    };
    let _ = server.await;
    let expected = match reader_end {
        Ok(expected) => expected,
        Err(e) if e.is_panic() => std::panic::resume_unwind(e.into_panic()),
        Err(e) => panic!("the reader never finished inside the test deadline: {e}"),
    };

    // The reply as the server wrote it: the largest one on this thread.
    let writes = REPLY_WRITES.with(|replies| {
        replies
            .borrow()
            .iter()
            .max_by_key(|reply| reply.iter().map(|w| w.moved).sum::<usize>())
            .cloned()
            .unwrap()
    });
    let moved: usize = writes.iter().map(|w| w.moved).sum();
    assert_eq!(moved, expected, "the server wrote the whole reply");
    let span = writes.last().unwrap().done - writes.first().unwrap().entered;
    assert!(
        span > idle,
        "the reply fit one window at the server ({span:?} over {} writes): \
         the workload proved nothing",
        writes.len()
    );
    let delayed = writes
        .iter()
        .filter(|w| w.done - w.entered >= Duration::from_millis(50))
        .count();
    assert!(
        delayed >= 2,
        "only {delayed} of {} writes completed 50 ms or more after entry",
        writes.len()
    );
}
