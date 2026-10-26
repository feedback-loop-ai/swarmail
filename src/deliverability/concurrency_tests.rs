//! Cold-path deadlines and panic-safe worker release belong in a harness (0020).
use super::*;
use crate::store::{Filter, WaitOutcome};
use std::sync::{
    Condvar,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

struct Held {
    release: Arc<(Mutex<bool>, Condvar)>,
    entered: tokio::sync::mpsc::UnboundedReceiver<()>,
}
impl Held {
    fn new(service: &Service) -> Self {
        let release = Arc::new((Mutex::new(false), Condvar::new()));
        let worker = release.clone();
        let (tx, entered) = tokio::sync::mpsc::unbounded_channel();
        *service.gate.lock().unwrap() = Some(Arc::new(move || {
            tx.send(()).unwrap();
            let (flag, cv) = &*worker;
            let mut ready = flag.lock().unwrap();
            while !*ready {
                let (next, expired) = cv.wait_timeout(ready, Duration::from_secs(10)).unwrap();
                ready = next;
                if expired.timed_out() && !*ready {
                    drop(ready);
                    panic!("report worker hold exhausted: evaluation may occupy an async worker");
                }
            }
        }));
        Self { release, entered }
    }
    async fn entry(&mut self) {
        if tokio::time::timeout(Duration::from_secs(10), self.entered.recv())
            .await
            .unwrap()
            .is_none()
        {
            panic!("report worker disappeared before entering evaluation");
        }
    }
    fn open(&self) {
        let (flag, cv) = &*self.release;
        *flag.lock().unwrap() = true;
        cv.notify_all();
    }
}
impl Drop for Held {
    fn drop(&mut self) {
        self.open();
    }
}
pub(super) const GOOD: &[u8] = b"From: sender@example.test\r\nMessage-ID: <bulk@example.test>\r\nDate: Mon, 26 Oct 2026 12:00:00 +0000\r\nList-Unsubscribe: <https://example.test/unsubscribe>\r\nList-Unsubscribe-Post: List-Unsubscribe=One-Click\r\n\r\nbody\r\n";
pub(super) fn email(id: &str, inbox: &str, to: &str) -> Email {
    let mut mail = crate::store::email(id, inbox, to);
    mail.raw = GOOD.to_vec();
    mail
}
pub(super) fn trusted(service: &Service) {
    service.auth.set(passing()).unwrap();
}
pub(super) fn passing() -> AuthEvidence {
    AuthEvidence {
        policy_id: "fixture:p=none".into(),
        observation_id: "fixture:verified".into(),
        policy_present: Fact::True,
        valid_from: Fact::True,
        dkim_pass: Fact::True,
        dkim_aligned: Fact::True,
        spf_pass: Fact::False,
        spf_aligned: Fact::False,
    }
}
pub(super) fn mail(id: &str) -> Arc<Email> {
    Arc::new(email(id, "box", "a"))
pub(super) fn html_email(id: &str, inbox: &str, to: &str) -> Email {
    let mut e = crate::store::email(id, inbox, to);
    e.raw = b"Content-Type: text/html\r\n\r\n<p>Hello</p><img alt=\"\" src=\"cid:x\">".to_vec();
    e
}
}

#[tokio::test]
async fn coalesced_miss_survives_caller_cancel() {
    let service = Arc::new(Service::default());
    let mut held = Held::new(&service);
    let email = mail("same");
    let task = {
        let s = service.clone();
        let e = email.clone();
        tokio::spawn(async move { s.report(e, false).await })
    };
    held.entry().await;
    assert!(matches!(
        service.report(email.clone(), true).await,
        Err(AnalysisError::Unavailable)
    ));
    {
        let waiting = service.report(email.clone(), false);
        tokio::pin!(waiting);
        assert!(
            std::future::Future::poll(
                waiting.as_mut(),
                &mut std::task::Context::from_waker(std::task::Waker::noop())
            )
            .is_pending()
        );
        assert_eq!(Arc::strong_count(&service.slots.lock().unwrap()[0]), 3);
    } // cancel a same-key waiter before submission
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert_eq!(service.jobs.available_permits(), 3);
    assert_eq!(Arc::strong_count(&service.slots.lock().unwrap()[0]), 2);
    held.open();
    let result = service.report(email.clone(), false).await.unwrap();
    assert!(Arc::ptr_eq(
        &result,
        &service.report(email, true).await.unwrap()
    ));
    assert_eq!(service.jobs.available_permits(), 4);
    assert!(
        held.entered.try_recv().is_err(),
        "same-key evaluation ran more than once"
    );
}

#[tokio::test]
async fn report_capacity_refuses_and_recovers() {
    let service = Arc::new(Service::default());
    let mut held = Held::new(&service);
    let mut tasks = Vec::new();
    for n in 0..JOBS {
        let s = service.clone();
        tasks.push(tokio::spawn(async move {
            s.report(mail(&n.to_string()), false).await
        }));
        held.entry().await;
    }
    assert_eq!(service.jobs.available_permits(), 0);
    assert_eq!(
        service.report(mail("refused"), false).await,
        Err(AnalysisError::Unavailable)
    );
    assert!(
        Service::default()
            .report(mail("independent"), false)
            .await
            .is_ok()
    );
    for t in tasks {
        t.abort();
        assert!(t.await.unwrap_err().is_cancelled());
    }
    assert_eq!(service.jobs.available_permits(), 0);
    held.open();
    let slots: Vec<_> = service
        .slots
        .lock()
        .unwrap()
        .iter()
        .take(JOBS)
        .cloned()
        .collect();
    for slot in slots {
        let _completed = slot.report.lock().await;
    }
    assert_eq!(service.jobs.available_permits(), JOBS);
    assert!(service.report(mail("retry"), false).await.is_ok());
}

#[tokio::test]
async fn report_wait_deadline_and_zero_timeout() {
    let store = Arc::new(Store::new(0));
    trusted(&store.reports);
    let f = Filter::default();
    let threshold = Some(Severity::Warning);
    assert!(matches!(
        store
            .wait_for_report("box", &f, 1, Duration::ZERO, threshold)
            .await,
        Ok(WaitOutcome::Timeout { matched: 0 })
    ));
    let email = store.insert(html_email("one", "box", "a")).unwrap();
    assert!(matches!(
        store
            .wait_for_report("box", &f, 1, Duration::ZERO, threshold)
            .await,
        Err(AnalysisError::Unavailable)
    ));
    store.report(email.clone()).await.unwrap();
    assert!(matches!(
        store
            .wait_for_report("box", &f, 1, Duration::ZERO, threshold)
            .await,
        Ok(WaitOutcome::Found(_))
    ));
    assert!(matches!(
        store
            .wait_for_report("box", &f, 2, Duration::ZERO, threshold)
            .await,
        Ok(WaitOutcome::Timeout { matched: 1 })
    ));
    assert!(matches!(
        store
            .wait_for_report("box", &f, 2, Duration::from_millis(10), threshold)
            .await,
        Ok(WaitOutcome::Timeout { matched: 1 })
    ));
    let slot = store
        .reports
        .pin(
            &email,
            &Inputs {
                auth: passing(),
                ..Inputs::default()
            },
        )
        .unwrap();
    let guard = slot.report.lock().await;
    assert!(matches!(
        store
            .wait_for_report("box", &f, 1, Duration::ZERO, threshold)
            .await,
        Err(AnalysisError::Unavailable)
    ));
    assert!(matches!(
        store
            .wait_for_report("box", &f, 1, Duration::from_millis(10), threshold)
            .await,
        Err(AnalysisError::Unavailable)
    ));
    drop(guard);
    store.insert(html_email("two", "box", "a")).unwrap();
    let newest = store.insert(html_email("warm-newest", "box", "a")).unwrap();
    store.report(newest).await.unwrap();
    let mut held = Held::new(&store.reports);
    let s = store.clone();
    let task = tokio::spawn(async move {
        s.wait_for_report(
            "box",
            &Filter::default(),
            3,
            Duration::from_millis(100),
            threshold,
        )
        .await
    });
    held.entry().await;
    assert!(matches!(
        task.await.unwrap(),
        Err(AnalysisError::Unavailable)
    ));
    assert_eq!(store.reports.jobs.available_permits(), 3);
    held.open();
}

#[tokio::test]
async fn report_wait_subscribes_before_analysis_and_resubscribes_after_clear() {
    let store = Arc::new(Store::new(0));
    trusted(&store.reports);
    let fires = Arc::new(AtomicUsize::new(0));
    let s = store.clone();
    let n = fires.clone();
    store.set_post_subscribe_gate(Some(Arc::new(move || {
        if n.fetch_add(1, Ordering::SeqCst) == 0 {
            s.clear("box").unwrap();
        } else {
            s.insert(html_email("after-closed", "box", "a")).unwrap();
        }
    })));
    let outcome = store
        .wait_for_report(
            "box",
            &Filter::default(),
            1,
            Duration::from_secs(5),
            Some(Severity::Info),
        )
        .await
        .unwrap();
    match outcome {
        WaitOutcome::Found(m) => assert_eq!(m[0].id, "after-closed"),
        _ => panic!("lost re-subscribe window"),
    }
    assert_eq!(fires.load(Ordering::SeqCst), 2);
    store.set_post_subscribe_gate(None); // release the fixture's Store Arc cycle
}

#[tokio::test]
async fn report_wait_observes_delivery_during_analysis_and_recovers_lag() {
    let store = Arc::new(Store::new(0));
    trusted(&store.reports);
    let mut dirty = email("dirty", "box", "a");
    dirty.raw = b"X-Swarmail-Demo: error\n\n".to_vec();
    store.insert(dirty).unwrap();
    let mut held = Held::new(&store.reports);
    let s = store.clone();
    let task = tokio::spawn(async move {
        s.wait_for_report(
            "box",
            &Filter::default(),
            2,
            Duration::from_secs(5),
            Some(Severity::Warning),
        )
        .await
    });
    held.entry().await;
    assert_eq!(
        store.watcher_count(),
        1,
        "subscribe must precede evaluation"
    );
    // Overflow the existing wake ring while the original snapshot is analyzed.
    for n in 0..32 {
        store
            .insert(html_email(&format!("new{n}"), "box", "a"))
            .unwrap();
    }
    held.open();
    match task.await.unwrap().unwrap() {
        WaitOutcome::Found(m) => assert_eq!(
            m.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            vec!["new31", "new30"]
        ),
        _ => panic!("queued arrivals were lost during analysis"),
    }
}

pub(super) fn config(data_file: Option<std::path::PathBuf>) -> crate::config::Config {
    crate::config::Config {
        smtp_listen: "127.0.0.1:0".into(),
        http_listen: "127.0.0.1:0".into(),
        pop3_listen: "127.0.0.1:0".into(),
        max_per_inbox: 0,
        data_file,
        tls_cert: None,
        tls_key: None,
        smtp: Default::default(),
        pop3_idle_timeout: crate::pop3::DEFAULT_IDLE_TIMEOUT,
    }
}
pub(super) fn client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().unwrap()
}
pub(super) async fn get(srv: &crate::RunningServer, path: &str) -> (u16, String) {
    let response = client()
        .get(format!("http://{}{path}", srv.http_addr))
        .send()
        .await
        .unwrap();
    (response.status().as_u16(), response.text().await.unwrap())
}
pub(super) async fn post(
    srv: &crate::RunningServer,
    path: &str,
    body: serde_json::Value,
) -> serde_json::Value {
    let response = client()
        .post(format!("http://{}{path}", srv.http_addr))
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    serde_json::from_str(&response.text().await.unwrap()).unwrap()
}
pub(super) async fn smtp(srv: &crate::RunningServer) -> String {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let stream = tokio::net::TcpStream::connect(srv.smtp_addr).await.unwrap();
    let (read, mut write) = stream.into_split();
    let mut read = tokio::io::BufReader::new(read);
    let mut line = String::new();
    read.read_line(&mut line).await.unwrap();
    assert!(line.starts_with("220"));
    for (command, code) in [
        ("HELO test\r\n", "250"),
        ("MAIL FROM:<s@x.io>\r\n", "250"),
        ("RCPT TO:<r@x.io>\r\n", "250"),
        ("DATA\r\n", "354"),
        (
            "X-Swarmail-Demo: warning\r\nContent-Type: text/html\r\n\r\n<p>body 424242 https://x.io/</p><img alt=\"\" src=\"cid:x\">\r\n.\r\n",
            "250",
        ),
    ] {
        write.write_all(command.as_bytes()).await.unwrap();
        line.clear();
        read.read_line(&mut line).await.unwrap();
        assert!(line.starts_with(code), "{line}");
    }
    line.split_whitespace().last().unwrap().into()
}

#[tokio::test]
async fn report_workers_do_not_hold_store_guards() {
    let mut cfg = config(None);
    cfg.max_per_inbox = 1;
    let srv = crate::run_on(&cfg).await.unwrap();
    let mut held = Held::new(&srv.store.reports);
    let mut tasks = Vec::new();
    for n in 0..JOBS {
        let email = srv
            .store
            .insert(html_email(&n.to_string(), "held", "a"))
            .unwrap();
        let store = srv.store.clone();
        tasks.push(tokio::spawn(async move { store.report(email).await }));
        held.entry().await;
    }
    let id = smtp(&srv).await;
    assert_eq!(get(&srv, &format!("/api/v1/messages/{id}")).await.0, 200);
    let (status, value) = get(&srv, &format!("/api/v1/messages/{id}/deliverability")).await;
    assert_eq!(status, 503);
    assert_eq!(value, "{\"error\":\"deliverability analysis unavailable\"}");
    let reply = post(&srv, "/mcp", serde_json::json!({"id":1,"method":"tools/call","params":{"name":"swarmail_get_deliverability_report","arguments":{"id":id}}})).await;
    assert_eq!(reply["result"]["isError"], true);
    assert_eq!(
        reply["result"]["content"][0]["text"],
        AnalysisError::Unavailable.message()
    );
    let next = smtp(&srv).await;
    assert!(
        srv.store.get(&id).is_none(),
        "SMTP pruning evicts the oldest while analysis is held"
    );
    assert!(srv.store.get(&next).is_some());
    srv.store.delete(&next).unwrap();
    srv.store.clear("held").unwrap();
    held.open();
    for task in tasks {
        task.await.unwrap().unwrap();
    }
    srv.stop().await;
}

#[tokio::test]
async fn report_wait_shutdown_interrupts_worker_and_slot_waits() {
    let srv = crate::run_on(&config(None)).await.unwrap();
    srv.store.insert(html_email("one", "box", "a")).unwrap();
    let mut held = Held::new(&srv.store.reports);
    let url = format!(
        "http://{}/api/v1/inboxes/box/await?no_findings_at_or_above=error&timeout_ms=60000",
        srv.http_addr
    );
    let rest = tokio::spawn(async move {
        let r = client().get(url).send().await.unwrap();
        (r.status(), r.text().await.unwrap())
    });
    held.entry().await;
    let url = format!("http://{}/mcp", srv.http_addr);
    let mcp = tokio::spawn(async move {
        let r = client().post(url).header("Content-Type", "application/json").body(serde_json::json!({"id":1,"method":"tools/call","params":{"name":"swarmail_wait_for_email","arguments":{"inbox":"box","timeout_ms":60000,"no_findings_at_or_above":"error"}}}).to_string()).send().await.unwrap();
        r.text().await.unwrap()
    });
    // Both real wait owners must be parked; the second owns no worker.
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if get(&srv, "/metrics")
                .await
                .1
                .contains("swarmail_awaits_active{inbox=\"box\"} 2")
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("both report waits did not park");
    let store = srv.store.clone();
    let stopping = tokio::spawn(srv.stop());
    let (status, text) = rest.await.unwrap();
    assert_eq!(status, 503);
    assert_eq!(text, "{\"error\":\"shutting down\"}");
    let text: serde_json::Value = serde_json::from_str(&mcp.await.unwrap()).unwrap();
    assert_eq!(text["result"]["isError"], true);
    assert_eq!(text["result"]["content"][0]["text"], "shutting down");
    assert_eq!(store.reports.jobs.available_permits(), JOBS - 1);
    held.open();
    stopping.await.unwrap();
}

#[tokio::test]
async fn report_complete_scan_timeout_does_not_reanalyze_after_eviction() {
    let store = Store::new(0);
    trusted(&store.reports);
    let email = store.insert(self::email("warm", "box", "a")).unwrap();
    store.report(email).await.unwrap();
    let filter = Filter::default();
    let wait = store.wait_for_report(
        "box",
        &filter,
        2,
        Duration::from_millis(10),
        Some(Severity::Error),
    );
    tokio::pin!(wait);
    assert!(
        std::future::Future::poll(
            wait.as_mut(),
            &mut std::task::Context::from_waker(std::task::Waker::noop())
        )
        .is_pending()
    );
    // All selected reports completed; only the inbox receiver is parked now.
    for n in 0..CACHE_SLOTS {
        store
            .reports
            .pin(&mail(&n.to_string()), &Inputs::default())
            .unwrap();
    }
    *store.reports.gate.lock().unwrap() = Some(Arc::new(|| {
        panic!("deadline expiry started another cold scan")
    }));
    match wait.await.unwrap() {
        WaitOutcome::Timeout { matched } => assert_eq!(matched, 1),
        _ => panic!("insufficient completed scan changed into success"),
    }
}

pub(super) async fn smtp_bytes(srv: &crate::RunningServer, raw: &[u8]) -> String {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
    let stream = tokio::net::TcpStream::connect(srv.smtp_addr).await.unwrap();
    let (read, mut write) = stream.into_split();
    let mut read = tokio::io::BufReader::new(read);
    let mut line = String::new();
    read.read_line(&mut line).await.unwrap();
    assert!(line.starts_with("220"));
    for (command, code) in [
        ("HELO bulk\r\n", "250"),
        ("MAIL FROM:<s@x.io>\r\n", "250"),
        ("RCPT TO:<r@x.io>\r\n", "250"),
        ("DATA\r\n", "354"),
    ] {
        write.write_all(command.as_bytes()).await.unwrap();
        line.clear();
        read.read_line(&mut line).await.unwrap();
        assert!(line.starts_with(code), "{line}");
    }
    write.write_all(raw).await.unwrap();
    write.write_all(b".\r\n").await.unwrap();
    line.clear();
    read.read_line(&mut line).await.unwrap();
    assert!(line.starts_with("250"), "{line}");
    line.split_whitespace().last().unwrap().into()
}
