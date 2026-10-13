use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

fn db_path(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "swarmail-store-{tag}-{}-{nanos}.db",
        std::process::id()
    ))
}

#[test]
fn a_reopened_store_restores_mail_order_and_counters() {
    let path = db_path("restore");
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

#[test]
fn cap_pruning_is_mirrored_to_the_data_file() {
    let path = db_path("cap");
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
    let path = db_path("clear");
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
    let path = db_path("failing");
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
    assert!(Store::open(0, &db_path("nope").join("missing-dir").join("x.db")).is_err());
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

/// One temp data-file path per test; parallel tests never share one.
fn pin_db_path(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "swarmail-pin-{tag}-{}-{nanos}.db",
        std::process::id()
    ))
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
#[tokio::test]
async fn the_awaited_mutation_does_not_resolve_while_its_commit_is_in_flight() {
    let path = pin_db_path("ordering");
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
