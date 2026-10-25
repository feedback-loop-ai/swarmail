use super::*;
use crate::webhook::TargetPreparationError;
use std::time::Duration;

fn empty_job() -> Job {
    Job::Get(Vec::new())
}
fn local() -> Arc<Executor> {
    Executor::start(spawn_worker).unwrap()
}
fn usage(executor: &Executor) -> (usize, usize) {
    let state = executor.state.lock().unwrap();
    (state.busy.iter().filter(|b| **b).count(), state.bytes)
}
async fn idle(executor: &Executor) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while usage(executor).0 != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("finite work refunds independently of its waiter");
}

#[tokio::test]
async fn ca_envelope_cancellation_transfers_once() {
    let executor = local();
    let scope = Scope::default();
    let first = executor.reserve(&scope, Some(BYTE_CAP / 2)).unwrap();
    let second = executor.reserve(&scope, Some(BYTE_CAP / 2)).unwrap();
    assert_eq!(usage(&executor), (2, BYTE_CAP));
    assert!(executor.reserve(&scope, Some(0)).is_err());
    let retained = first.run(&scope, empty_job()).await.unwrap();
    assert!(matches!(&retained.value, Ok(Output::Get(v)) if v.is_empty()));
    assert_eq!(usage(&executor), (2, BYTE_CAP));
    drop(retained);
    assert_eq!(usage(&executor), (1, BYTE_CAP / 2));
    drop(second);
    assert_eq!(usage(&executor), (0, 0));
    assert!(executor.reserve(&scope, Some(BYTE_CAP + 1)).is_err());
    assert!(executor.reserve(&scope, None).is_err());
    let full = executor.reserve(&scope, Some(BYTE_CAP)).unwrap();
    assert!(executor.reserve(&scope, Some(1)).is_err());
    assert!(executor.reserve(&scope, Some(usize::MAX)).is_err());
    drop(full);
    let state = executor.state.lock().unwrap();
    assert!(executor.reserve(&scope, Some(1)).is_err());
    drop(state);
    let live = scope.live().unwrap();
    assert!(executor.reserve(&scope, Some(1)).is_err());
    drop(live);
    Owner(scope.clone()).close();
    assert!(scope.live().is_none());
    assert!(executor.reserve(&scope, Some(1)).is_err());
    drop(executor);
}

#[tokio::test]
async fn ca_closed_handoff_refunds_once() {
    let (tx, rx) = mpsc::sync_channel(1);
    let executor = Arc::new(Executor {
        state: Mutex::default(),
        workers: vec![tx],
    });
    let scope = Scope::default();
    let reservation = executor.reserve(&scope, Some(19)).unwrap();
    drop(rx);
    assert!(matches!(
        reservation.run(&scope, empty_job()).await,
        Err(TargetPreparationError::Worker)
    ));
    assert_eq!(usage(&executor), (0, 0));
}

#[tokio::test]
async fn cancelled_and_closed_unentered_envelopes_are_disposed() {
    for close_scope in [false, true] {
        let (tx, rx) = mpsc::sync_channel(1);
        let executor = Arc::new(Executor {
            state: Mutex::default(),
            workers: vec![tx],
        });
        let scope = Scope::default();
        let reservation = executor.reserve(&scope, Some(29)).unwrap();
        let (reply, receive) = tokio::sync::oneshot::channel();
        executor.workers[0]
            .try_send(Envelope {
                job: empty_job(),
                scope: scope.clone(),
                reply,
                reservation,
            })
            .unwrap_or_else(|_| panic!("reserved handoff"));
        if close_scope {
            Owner(scope).close();
        } else {
            drop(receive);
        }
        assert_eq!(usage(&executor), (1, 29));
        let thread = std::thread::spawn(move || worker(rx));
        idle(&executor).await;
        drop(executor);
        thread.join().unwrap();
    }
}

#[tokio::test]
async fn completed_unreceived_reply_keeps_its_reservation() {
    let (tx, rx) = mpsc::sync_channel(1);
    let executor = Arc::new(Executor {
        state: Mutex::default(),
        workers: vec![tx],
    });
    let scope = Scope::default();
    let reservation = executor.reserve(&scope, Some(83)).unwrap();
    let (reply, receive) = tokio::sync::oneshot::channel();
    executor.workers[0]
        .try_send(Envelope {
            job: empty_job(),
            scope,
            reply,
            reservation,
        })
        .unwrap_or_else(|_| panic!("reserved handoff"));
    std::thread::spawn(move || execute_envelope(rx.recv().unwrap()))
        .join()
        .unwrap();
    assert_eq!(usage(&executor), (1, 83));
    drop(receive);
    idle(&executor).await;
}
