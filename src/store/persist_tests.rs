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
            store.insert(e);
        }
        assert!(store.delete("2")); // a delete must be mirrored too
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
            store.insert(email(id, "box", "a@x.io"));
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
        store.insert(email("c1", "box-a", "a@x.io"));
        store.insert(email("c2", "box-b", "a@x.io"));
        assert_eq!(store.clear("box-a"), 1);
    }
    {
        let store = Store::open(0, &path).unwrap();
        assert_eq!(store.count("box-a", &Filter::default()), 0);
        assert_eq!(store.count("box-b", &Filter::default()), 1);
        assert_eq!(store.clear_all(), 1);
    }
    let store = Store::open(0, &path).unwrap();
    assert_eq!(store.count("box-b", &Filter::default()), 0);
    assert_eq!(store.emails_inserted(), 2); // counters outlive clears
}

#[test]
fn a_failing_data_file_never_blocks_the_store() {
    let store = Store::open(0, &db_path("failing")).unwrap();
    store.persist.as_ref().unwrap().fail_writes();
    // Insert still returns the mail and the memory store stays truthful.
    let stored = store.insert(email("f1", "box", "a@x.io"));
    assert_eq!(stored.id, "f1");
    assert_eq!(store.count("box", &Filter::default()), 1);
    assert!(store.delete("f1"));
    assert_eq!(store.clear("box"), 0);
    store.insert(email("f2", "box", "a@x.io"));
    assert_eq!(store.clear_all(), 1);
}

#[test]
fn open_refuses_an_unusable_data_file() {
    assert!(Store::open(0, &db_path("nope").join("missing-dir").join("x.db")).is_err());
}

/// The clear/insert persistence race (review finding in run
/// sqlite-persistence-data-file-swa-dc3a9a2a), forced deterministically:
/// the gate runs a live insert inside the drain→mirror window of
/// `clear`. That insert is answered (synchronous — decision 0001) and
/// stays queryable in memory, so it must survive the restart. The
/// inbox-wide mirrored `DELETE` erased exactly that row.
#[test]
fn a_clear_racing_an_insert_cannot_erase_the_inserts_row() {
    let path = db_path("clear-race");
    let store = Arc::new(Store::open(0, &path).unwrap());
    store.insert(email("r1", "box", "a@x.io"));
    let s = store.clone();
    store.set_pre_mirror_gate(Some(Arc::new(move || {
        // The racing insert: lands after the drain, before the mirror.
        s.insert(email("r2", "box", "a@x.io"));
    })));
    assert_eq!(store.clear("box"), 1);
    // Queryable in memory: the clear drained r1, not the later r2.
    assert_eq!(store.count("box", &Filter::default()), 1);
    drop(store);
    let reopened = Store::open(0, &path).unwrap();
    assert!(
        reopened.get("r2").is_some(),
        "a mail answered 250 and still held in memory was erased from the data file by the racing clear"
    );
    assert!(
        reopened.get("r1").is_none(),
        "the drained mail stays drained"
    );
}

/// Same race against `clear_all`: the insert lands after every drain,
/// before the one mirrored wipe.
#[test]
fn a_clear_all_racing_an_insert_cannot_erase_the_inserts_row() {
    let path = db_path("clear-all-race");
    let store = Arc::new(Store::open(0, &path).unwrap());
    store.insert(email("r1", "box", "a@x.io"));
    let s = store.clone();
    store.set_pre_mirror_gate(Some(Arc::new(move || {
        s.insert(email("r2", "other", "a@x.io"));
    })));
    assert_eq!(store.clear_all(), 1);
    assert_eq!(store.count("other", &Filter::default()), 1);
    drop(store);
    let reopened = Store::open(0, &path).unwrap();
    assert!(
        reopened.get("r2").is_some(),
        "a mail answered 250 and still held in memory was erased from the data file by the racing clear_all"
    );
    assert!(
        reopened.get("r1").is_none(),
        "the drained mail stays drained"
    );
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
