//! PASS snapshots off the async worker (issue #86, decision 0020 rule 2).
//!
//! The proof holds real PASS work inside the snapshot job and asks the same
//! current-thread runtime to serve another real POP3 peer meanwhile. Its
//! cold path is the gate's own deadline: with the snapshot inline, the held
//! gate blocks the sole worker, and only that deadline — measured on the
//! blocked thread, never by the test runtime — lets the test end red
//! instead of hanging. The line runs only while the bug is present, so the
//! test cannot live inline.

use super::*;
use std::sync::Mutex;
use std::sync::mpsc;

/// While a PASS snapshot is held on the blocking pool, a second peer is
/// greeted and the gate is still held (red on the inline snapshot: the
/// worker blocks inside the gate until its deadline releases it, and only
/// then can the second peer be served). No sleep orders anything: the gate
/// reports entry, the test spins against a real deadline.
#[tokio::test]
async fn a_held_pass_snapshot_leaves_the_worker_serving() {
    let store = Arc::new(Store::new(0));
    let _ = store.insert(crate::store::email("m1", "pop", "a@x.io"));
    let (entered_tx, entered_rx) = mpsc::channel::<()>();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let entered_tx = Mutex::new(entered_tx);
    let release_rx = Mutex::new(release_rx);
    let released = Arc::new(AtomicBool::new(false));
    let gate_released = released.clone();
    store.set_snapshot_gate(Some(Arc::new(move || {
        let _ = entered_tx.lock().unwrap().send(());
        let _ = release_rx
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(10));
        gate_released.store(true, Ordering::SeqCst);
    })));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(serve(
        listener,
        store.clone(),
        Arc::new(AtomicBool::new(false)),
        Duration::from_secs(60),
        4,
        std::future::pending(),
    ));

    let mut first = BufReader::new(TcpStream::connect(addr).await.unwrap());
    let mut line = Vec::new();
    first.read_until(b'\n', &mut line).await.unwrap();
    first.write_all(b"USER pop\r\n").await.unwrap();
    let mut line = Vec::new();
    first.read_until(b'\n', &mut line).await.unwrap();
    assert_eq!(line, b"+OK name is a valid mailbox\r\n");
    first.write_all(b"PASS x\r\n").await.unwrap();

    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while entered_rx.try_recv().is_err() {
        assert!(
            std::time::Instant::now() < deadline,
            "PASS never reached its snapshot"
        );
        tokio::task::yield_now().await;
    }

    // The snapshot is held: the worker must still greet a new peer.
    let mut second = BufReader::new(TcpStream::connect(addr).await.unwrap());
    let mut line = Vec::new();
    second.read_until(b'\n', &mut line).await.unwrap();
    assert_eq!(line, b"+OK Swarmail POP3 server ready\r\n");
    assert!(
        !released.load(Ordering::SeqCst),
        "the second peer was served only after the snapshot let go"
    );

    release_tx.send(()).unwrap();
    let mut line = Vec::new();
    first.read_until(b'\n', &mut line).await.unwrap();
    assert_eq!(line, b"+OK maildrop has 1 messages (0 octets)\r\n");
    store.set_snapshot_gate(None);
    server.abort();
}
