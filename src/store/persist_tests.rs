use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// A unique scratch directory per test, owned by a drop guard (issue #101):
/// the database lives *inside* it, so the `-wal`/`-shm` sidecars die with
/// the directory by construction and cleanup survives a panic — the old
/// pid+nanos file under `std::env::temp_dir()` leaked on a failing test.
/// The guard must be held for the test's whole life; parallel tests never
/// collide because tempfile's name is.
fn db_path(tag: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::TempDir::with_prefix(format!("swarmail-store-{tag}-")).unwrap();
    let path = dir.path().join("persist.db");
    (dir, path)
}

#[test]
fn a_reopened_store_restores_mail_order_and_counters() {
    let (_guard, path) = db_path("restore");
    {
        let store = Store::open(3, &path).unwrap();
        for id in ["1", "2", "3"] {
            let mut e = email(id, "box", "a@x.io");
            e.raw = vec![b'r', id.as_bytes()[0]];
            let _ = store.insert(e);
        }
        assert!(store.delete("2").unwrap()); // a delete must be mirrored too
    }
    let store = Store::open(3, &path).unwrap();
    // Newest-first list of what is left, in restored insertion order.
    let listed = store.list("box", &Filter::default());
    assert_eq!(
        listed.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
        ["3", "1"]
    );
    // Raw bytes survive the round trip.
    assert_eq!(store.get("1").unwrap().raw, b"r1".to_vec());
    // Lifetime counters are restored, not reset.
    assert_eq!(store.emails_inserted(), 3);
    assert_eq!(store.emails_dropped(), 0);
    assert_eq!(store.inboxes(), vec![("box".to_string(), 2)]);
}

/// The historical-data residual (decision 0040 §8): restore is verbatim,
/// so a row stored by an older extractor keeps its synthesized text, its
/// mis-cut link and its joined code across a restart, while a fresh
/// delivery of the same raw source gets the repaired derivation. Nothing
/// is re-derived on load.
#[test]
fn restore_keeps_legacy_derived_fields_while_fresh_ingest_repairs_them() {
    let (_guard, path) = db_path("legacy");
    let raw = b"From: s@x.io\r\nSubject: legacy\r\nMIME-Version: 1.0\r\n\
        Content-Type: text/html\r\n\r\n\
        <p>Unsubscribe: &lt;https://x.io/unsub?u=1&gt;</p>\
        <table><tr><td>Order 1234</td><td>5678</td></tr></table>\r\n"
        .to_vec();
    {
        let store = Store::open(0, &path).unwrap();
        let mut legacy = email("old", "box", "a@x.io");
        legacy.raw = raw.clone();
        legacy.text = Some("Unsubscribe: <https://x.io/unsub?u=1>Order 12345678".into());
        legacy.links = vec!["https://x.io/unsub?u=1>".into()];
        legacy.codes = vec!["12345678".into()];
        let _ = store.insert(legacy);
    }
    let store = Store::open(0, &path).unwrap();
    let restored = store.get("old").unwrap();
    assert_eq!(
        restored.text.as_deref(),
        Some("Unsubscribe: <https://x.io/unsub?u=1>Order 12345678")
    );
    assert_eq!(restored.links, vec!["https://x.io/unsub?u=1>"]);
    assert_eq!(restored.codes, vec!["12345678"]);
    assert_eq!(restored.raw, raw);

    let fresh = crate::smtp::build_email(&raw, "box", &["a@x.io".to_string()], "s@x.io".into());
    assert!(fresh.text.is_none(), "no text synthesized from html");
    assert_eq!(fresh.links, vec!["https://x.io/unsub?u=1"]);
    assert_eq!(fresh.codes, vec!["1234", "5678"]);
}

/// The code cap is a fresh-ingest contract (decision 0049 §4, issue #139):
/// a freshly built mail stores its 64 capped codes and restores them
/// as-is, while a row stored before the cap — 100 codes here — restores
/// and reads verbatim, never truncated or re-derived on load.
#[test]
fn fresh_codes_stay_capped_and_historical_codes_restore_verbatim() {
    let (_guard, path) = db_path("codes");
    let numbers: Vec<String> = (0..100).map(|n| (2_000_000 + n).to_string()).collect();
    let raw = format!(
        "From: s@x.io\r\nSubject: n\r\n\r\n{}\r\n",
        numbers.join(" ")
    );
    {
        let store = Store::open(0, &path).unwrap();
        let fresh = crate::smtp::build_email(
            raw.as_bytes(),
            "box",
            &["a@x.io".to_string()],
            "s@x.io".into(),
        );
        assert_eq!(fresh.codes, numbers[..64]);
        let fresh_id = fresh.id.clone();
        store.insert(fresh).unwrap();
        let mut historical = email("historical", "box", "a@x.io");
        historical.raw = raw.clone().into_bytes();
        historical.codes = numbers.clone();
        store.insert(historical).unwrap();
        assert_eq!(store.get(&fresh_id).unwrap().codes.len(), 64);
    }
    let store = Store::open(0, &path).unwrap();
    let listed = store.list("box", &Filter::default());
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].codes, numbers, "the historical row is verbatim");
    assert_eq!(listed[1].codes, numbers[..64], "the fresh row stays capped");
}

#[test]
fn cap_pruning_is_mirrored_to_the_data_file() {
    let (_guard, path) = db_path("cap");
    {
        let store = Store::open(2, &path).unwrap();
        for id in ["1", "2", "3"] {
            let _ = store.insert(email(id, "box", "a@x.io"));
        }
    }
    let store = Store::open(2, &path).unwrap();
    // The pruned-oldest row is gone from the restart copy as well.
    assert_eq!(
        store
            .list("box", &Filter::default())
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>(),
        ["3", "2"]
    );
    assert!(store.get("1").is_none());
    assert_eq!(store.emails_inserted(), 3);
    assert_eq!(store.emails_dropped(), 1);
}

#[test]
fn clear_and_clear_all_are_mirrored_to_the_data_file() {
    let (_guard, path) = db_path("clear");
    {
        let store = Store::open(0, &path).unwrap();
        let _ = store.insert(email("c1", "box-a", "a@x.io"));
        let _ = store.insert(email("c2", "box-b", "a@x.io"));
        assert_eq!(store.clear("box-a").unwrap(), 1);
    }
    {
        let store = Store::open(0, &path).unwrap();
        assert_eq!(store.count("box-a", &Filter::default()), 0);
        assert_eq!(store.count("box-b", &Filter::default()), 1);
        assert_eq!(store.clear_all().unwrap(), 1);
    }
    let store = Store::open(0, &path).unwrap();
    assert_eq!(store.count("box-b", &Filter::default()), 0);
    assert_eq!(store.emails_inserted(), 2); // counters outlive clears
}

/// The refusal contract (decision 0014): a failed commit is a refusal with
/// zero trace — the mail is nowhere on any surface, the counters did not
/// move, no watcher or webhook channel woke, not even the inbox was minted
/// — and the committed mail the cap held is untouched. Nothing to roll
/// back: nothing was applied. The other mutations refuse the same way with
/// memory and disk preserved, so a retry after repair is all a client owes.
#[test]
fn a_failing_data_file_refuses_with_zero_trace() {
    let (_guard, path) = db_path("failing");
    let store = Store::open(1, &path).unwrap();
    // One committed mail first: the cap's incumbent, and the eviction bait.
    store.insert(email("committed", "box", "a@x.io")).unwrap();
    store.persist.as_ref().unwrap().fail_writes();
    // The watchers exist before the refused insert: fan-out is part of
    // acceptance (AC4), so a commit-failed mail must fire neither channel.
    let mut inbox_rx = store.subscribe("box");
    let mut global_rx = store.subscribe_all();
    // The refusal (AC1): insert refuses instead of returning the mail —
    // red against the tree that logged and acknowledged.
    assert!(
        store.insert(email("f1", "box", "a@x.io")).is_err(),
        "a failed commit must refuse, not acknowledge"
    );
    // Zero trace (AC2): the refused mail is nowhere, the counters did not
    // move, and the incumbent was not evicted.
    assert!(
        store.get("f1").is_none(),
        "a refused mail must not be queryable"
    );
    assert_eq!(
        store.count("box", &Filter::default()),
        1,
        "the refused mail must not be counted and the incumbent must not be evicted"
    );
    let listed: Vec<String> = store
        .list("box", &Filter::default())
        .iter()
        .map(|e| e.id.clone())
        .collect();
    assert_eq!(
        listed,
        vec!["committed".to_string()],
        "the refusal must leave the inbox as it was"
    );
    assert_eq!(
        store.emails_inserted(),
        1,
        "a refused insert must not count"
    );
    assert_eq!(
        store.emails_dropped(),
        0,
        "a refused insert must not count an eviction"
    );
    // A second refusal into a fresh inbox: not even the inbox is minted.
    assert!(store.insert(email("ghost", "fresh", "a@x.io")).is_err());
    assert_eq!(
        store.inboxes(),
        vec![("box".to_string(), 1)],
        "a refused insert must not mint an inbox"
    );
    // Fan-out is gated on commit success (AC4): neither channel saw a mail.
    assert!(
        inbox_rx.try_recv().is_err(),
        "a refused mail must not wake an inbox watcher"
    );
    assert!(
        global_rx.try_recv().is_err(),
        "a refused mail must not enter the webhook feed"
    );
    // The other mutations refuse the same way (AC3), and each refusal
    // preserves the mail: nothing half-applied.
    assert!(
        store.delete("committed").is_err(),
        "a failed delete must refuse"
    );
    assert_eq!(store.count("box", &Filter::default()), 1);
    assert!(store.clear("box").is_err(), "a failed clear must refuse");
    assert_eq!(store.count("box", &Filter::default()), 1);
    assert!(store.clear_all().is_err(), "a failed clear_all must refuse");
    assert_eq!(store.count("box", &Filter::default()), 1);
    drop(store);
    // The disk copy is exactly what was committed: the incumbent only — no
    // refused row to resurrect, no deleted row, no lost row.
    let reopened = Store::open(1, &path).unwrap();
    assert_eq!(reopened.count("box", &Filter::default()), 1);
    assert!(reopened.get("committed").is_some());
    assert!(
        reopened.get("f1").is_none(),
        "the refused mail must leave no row to resurrect"
    );
    assert_eq!(
        reopened.emails_inserted(),
        1,
        "the counters survive the refusal unchanged"
    );
}

#[test]
fn open_refuses_an_unusable_data_file() {
    // The drop guard owns the scratch dir; the db path's parent directory
    // inside it is missing, so the open refuses.
    let (guard, _path) = db_path("nope");
    assert!(Store::open(0, &guard.path().join("missing-dir").join("x.db")).is_err());
}

/// Bounded wait until the pre-mirror gate reports it has been entered.
async fn await_gate_entered(entered: &AtomicBool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !entered.load(Ordering::SeqCst) {
        assert!(
            Instant::now() < deadline,
            "the mutation never reached the pre-mirror gate"
        );
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

/// One temp data-file path per test, under a drop guard; parallel tests
/// never share one (issue #101: the guard is returned beside the path and
/// must be held for the test's whole life).
fn pin_db_path(tag: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::TempDir::with_prefix(format!("swarmail-pin-{tag}-")).unwrap();
    let path = dir.path().join("pin.db");
    (dir, path)
}

/// The panic-symmetry pin: a mutation that panics inside the mirror path
/// re-raises in the awaiting caller, exactly as the synchronous call would.
/// This pin is also the deterministic cover for `offload`'s `Err` arm — the
/// `resume_unwind` path the gate executes here and nothing else does.
#[tokio::test]
#[should_panic(expected = "the held gate panics")]
async fn a_panicking_mutation_re_raises_in_the_awaiting_caller() {
    let store = Arc::new(Store::new(0));
    store.set_pre_mirror_gate(Some(Arc::new(|| panic!("the held gate panics"))));
    let _ = store.clone().clear_async("box".to_string()).await;
}

/// Same symmetry for `clear_all`: the gate fires inside its boundary too,
/// and the panic re-raises in the awaiting caller — the memory phase never
/// ran, so nothing half-applied survives it.
#[tokio::test]
#[should_panic(expected = "the held gate panics")]
async fn a_panicking_clear_all_re_raises_in_the_awaiting_caller() {
    let store = Arc::new(Store::new(0));
    store.set_pre_mirror_gate(Some(Arc::new(|| panic!("the held gate panics"))));
    let _ = store.clone().clear_all_async().await;
}

/// The ordering pin: the awaited mutation does not resolve while its commit
/// is still in flight. Red against fire-and-forget (the await would resolve
/// while the gate holds) and against any reply-before-commit reorder (the
/// resolution is the reply's only licence).
///
/// The gate releases on every exit path, not just the green one: if the
/// mid-test assertion fails, a dropped `release` would strand the boundary
/// thread in its spin — `spawn_blocking` threads are never aborted, so the
/// red test would hang the whole process at runtime teardown instead of
/// failing loudly (decision 0020 rule 2). `GateRelease` is the panic-safe
/// net; the explicit store below is the sequencing step.
struct GateRelease(Arc<AtomicBool>);

impl Drop for GateRelease {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn the_awaited_mutation_does_not_resolve_while_its_commit_is_in_flight() {
    let (_guard, path) = pin_db_path("ordering");
    let store = Arc::new(Store::open(0, &path).unwrap());
    store.insert(email("committed", "box", "a@x.io")).unwrap();
    store.persist.as_ref().unwrap().fail_writes();

    let entered = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let (en, rel) = (entered.clone(), release.clone());
    store.set_pre_mirror_gate(Some(Arc::new(move || {
        // Inside the boundary (decision 0014): the closure must not
        // re-enter the store — it would deadlock on its own lock — so it
        // signals and holds instead.
        en.store(true, Ordering::SeqCst);
        while !rel.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(1));
        }
    })));
    let _release_on_exit = GateRelease(release.clone());

    let task = tokio::spawn(store.clone().clear_async("box".to_string()));
    await_gate_entered(&entered).await;
    assert!(
        !task.is_finished(),
        "the awaited mutation resolved while the commit was still held: fire-and-forget"
    );

    release.store(true, Ordering::SeqCst);
    assert!(
        task.await.unwrap().is_err(),
        "the held commit fails: the refusal resolves the await, and nothing else does"
    );
}

// ---------- R11 (0018, 0024 §3): an admitted commit at stop ----------
//
// Moved from tests/lifecycle.rs (issue #143, re-drive 5): the integration
// copy sequenced its stop behind the delivery metric — live during the
// parse, before shutdown admission — and held an external SQLite writer
// across the 5 s busy bound, so under host load the stop could land before
// admission (refused, nothing ever lands) or the bound could expire first
// (refused, nothing ever lands). Here the interleaving is a handshake: the
// pre-mirror gate in `record` fires only after admission, and it holds
// until this test releases it — no clock decides which side wins. Its home
// is this harness (0020 rule 2): the gate is crate-private, and the step
// deadlines below are cold diagnostics that run only while a bug is present.

/// How long one wire step or one handshake may take before it is named as
/// the step that made no progress. A diagnostic, never a validity window.
const STEP: Duration = Duration::from_secs(10);

/// The bound the stop is observed against — tests/lifecycle.rs's
/// `STOP_BUDGET`: the server's 10 s stop fence plus the 2 s allowance to
/// observe it.
const STOP_BUDGET: Duration = Duration::from_secs(12);

/// `fut`, bounded by [`STEP`] and named on expiry.
async fn step<T>(what: &str, fut: impl std::future::Future<Output = T>) -> T {
    match tokio::time::timeout(STEP, fut).await {
        Ok(value) => value,
        Err(_) => panic!("{what}: no progress within {STEP:?}"),
    }
}

/// One SMTP reply, multi-line replies folded to their final line.
async fn smtp_reply(
    reader: &mut tokio::io::BufReader<tokio::net::tcp::OwnedReadHalf>,
    what: &str,
) -> String {
    use tokio::io::AsyncBufReadExt;
    loop {
        let mut line = String::new();
        step(what, reader.read_line(&mut line))
            .await
            .unwrap_or_else(|e| panic!("{what}: {e}"));
        assert!(line.len() >= 4, "{what}: short reply {line:?}");
        if line.as_bytes()[3] != b'-' {
            return line;
        }
    }
}

/// One SMTP command and its reply, which must carry `code`.
async fn smtp_step(
    reader: &mut tokio::io::BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    command: &str,
    code: &str,
) {
    use tokio::io::AsyncWriteExt;
    step(command, writer.write_all(command.as_bytes()))
        .await
        .unwrap_or_else(|e| panic!("{command}: {e}"));
    let reply = smtp_reply(reader, command).await;
    assert!(reply.starts_with(code), "{command}: {reply:?}");
}

/// What the data file holds, read on its own read-only connection (the
/// live store keeps the writer identity, 0033): every mail row as (id,
/// inbox, envelope sender, raw) and the two persisted counters.
type DiskRow = (String, String, Option<String>, Vec<u8>);

fn on_disk(path: &std::path::Path) -> (Vec<DiskRow>, String, String) {
    let ro =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("the data file opens read-only");
    let rows = ro
        .prepare("SELECT id, inbox, envelope_from, raw FROM emails ORDER BY seq, rowid")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<DiskRow>>>()
        .unwrap();
    let counter = |key: &str| -> String {
        ro.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
            .unwrap()
    };
    let (inserted, dropped) = (counter("emails_inserted"), counter("emails_dropped"));
    (rows, inserted, dropped)
}

/// The test-owned release of the held commit: a predicate under a condvar,
/// so a release before the gate is entered is never lost and a spurious
/// wake re-waits. Dropping it releases too — an assertion failing while
/// the gate holds must not strand the blocking thread and hang runtime
/// teardown (`spawn_blocking` work is never aborted).
struct HeldCommit(Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>);

impl HeldCommit {
    fn release(&self) {
        let (open, wake) = &*self.0;
        *open.lock().unwrap() = true;
        wake.notify_all();
    }
}

impl Drop for HeldCommit {
    fn drop(&mut self) {
        self.release();
    }
}

/// R11 (0018), at the boundary 0024 §3 admits: a commit already admitted
/// when stop fires is not cancelled — the blocking pool runs it to
/// completion — but it does not hold stop. While the gate holds the
/// admitted insert, the real server stops inside its budget, the client
/// never sees `250`, both listeners refuse, and neither memory, counters,
/// fan-out nor the data file shows the mail. Released, the commit lands
/// exactly once, identically in memory and on disk, though the client was
/// never told (R11: "retry is lossless").
#[tokio::test]
async fn an_in_flight_commit_neither_holds_stop_nor_half_lands() {
    use tokio::io::AsyncWriteExt;
    let (dir, path) = pin_db_path("stop-burn");
    let srv = crate::run_on(&crate::config::Config {
        smtp_listen: "127.0.0.1:0".into(),
        http_listen: "127.0.0.1:0".into(),
        pop3_listen: "127.0.0.1:0".into(),
        max_per_inbox: 0,
        data_file: Some(path.clone()),
        tls_cert: None,
        tls_key: None,
        smtp: Default::default(),
        pop3_idle_timeout: crate::pop3::DEFAULT_IDLE_TIMEOUT,
    })
    .await
    .unwrap();
    let store = srv.store.clone();
    // A committed incumbent, so the persisted counters exist and their
    // deltas are exact.
    store.insert(email("incumbent", "seed", "a@x.io")).unwrap();
    let before = on_disk(&path);
    assert_eq!(before.1, "1");

    // The gate and its release exist before the delivery can reach it.
    let entered = Arc::new(tokio::sync::Notify::new());
    let held = HeldCommit(Arc::new((
        std::sync::Mutex::new(false),
        std::sync::Condvar::new(),
    )));
    let (signal, open) = (entered.clone(), held.0.clone());
    store.set_pre_mirror_gate(Some(Arc::new(move || {
        signal.notify_one();
        let (open, wake) = &*open;
        let _open = wake
            .wait_while(open.lock().unwrap(), |open| !*open)
            .unwrap();
    })));
    // Subscribe before the send (0002): the fan-out is the completion signal.
    let mut landed = store.subscribe("burn");

    let stream = step("connect", tokio::net::TcpStream::connect(srv.smtp_addr))
        .await
        .unwrap();
    let (r, mut w) = stream.into_split();
    let mut r = tokio::io::BufReader::new(r);
    assert!(smtp_reply(&mut r, "greeting").await.starts_with("220"));
    smtp_step(&mut r, &mut w, "EHLO swarmail-test\r\n", "250").await;
    // The AUTH user names the inbox "burn": the row is this test's alone.
    let plain = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        "\u{0}burn\u{0}whatever",
    );
    smtp_step(&mut r, &mut w, &format!("AUTH PLAIN {plain}\r\n"), "235").await;
    smtp_step(&mut r, &mut w, "MAIL FROM:<f@x.io>\r\n", "250").await;
    smtp_step(&mut r, &mut w, "RCPT TO:<r@x.io>\r\n", "250").await;
    smtp_step(&mut r, &mut w, "DATA\r\n", "354").await;
    let message = "From: f@x.io\r\nTo: r@x.io\r\nSubject: held\r\n\r\nx\r\n.\r\n";
    step("the message", w.write_all(message.as_bytes()))
        .await
        .unwrap();
    // Admission is past exactly when the gate is entered: `record` fires
    // it after its stopping check, under the mutation boundary.
    step("the gate entry", entered.notified()).await;

    let (smtp_addr, http_addr) = (srv.smtp_addr, srv.http_addr);
    assert!(
        tokio::time::timeout(STOP_BUDGET, srv.stop()).await.is_ok(),
        "an admitted commit held stop past {STOP_BUDGET:?}"
    );
    assert!(
        !*held.0.0.lock().unwrap(),
        "stop must finish before commit release"
    );

    // The session died with the stop: EOF or reset, or a refusal — never
    // `250`. A reply written before the stop is buffered and read here.
    let mut tail = String::new();
    let read = step(
        "the DATA reply",
        tokio::io::AsyncBufReadExt::read_line(&mut r, &mut tail),
    )
    .await;
    assert!(
        !tail.starts_with("250"),
        "an unfinished commit was acknowledged: {tail:?} ({read:?})"
    );
    for addr in [smtp_addr, http_addr] {
        assert!(
            tokio::net::TcpStream::connect(addr).await.is_err(),
            "{addr} still accepts connections after stop"
        );
    }

    // Still held: nothing is visible anywhere — no mail, no counter, no
    // fan-out, no row. None of these reads takes the mutation lock.
    assert!(
        store.list("burn", &Filter::default()).is_empty(),
        "the held mail is visible in memory before its commit"
    );
    assert_eq!(
        store.emails_inserted(),
        1,
        "the held mail was counted before its commit"
    );
    assert!(landed.try_recv().is_err(), "the held mail fanned out early");
    assert_eq!(on_disk(&path), before, "the held commit half-landed");

    // Released, the admitted commit runs to completion: one mail, the same
    // in memory and on disk, counted once.
    held.release();
    let mail = step("the commit after release", landed.recv())
        .await
        .expect("the released commit fans out");
    assert_eq!(
        store.list("burn", &Filter::default()),
        vec![mail.clone()],
        "exactly one mail lands in memory"
    );
    let (rows, inserted, dropped) = on_disk(&path);
    assert_eq!(rows.len(), 2, "exactly one row lands beside the incumbent");
    assert_eq!(rows[0], before.0[0]);
    assert_eq!(
        rows[1],
        (
            mail.id.clone(),
            "burn".to_string(),
            Some("f@x.io".to_string()),
            mail.raw.clone()
        )
    );
    assert_eq!((inserted.as_str(), dropped.as_str()), ("2", "0"));
    assert_eq!((store.emails_inserted(), store.emails_dropped()), (2, 0));
    assert!(
        landed.try_recv().is_err(),
        "the released commit fans out exactly once"
    );
    drop(store);
    drop(dir);
}
