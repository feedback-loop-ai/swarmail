//! Decision 0121 §6, the opt-in link reachability probe, through real SMTP and
//! REST against local servers only. Every link in this file names a loopback
//! listener this test opened; nothing resolves a public name or touches the
//! internet.
mod common;
use common::*;
use serde_json::{Value, json};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use swarmail::deliverability::LinkReachability;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A loopback HTTP listener that counts accepted connections. `answer` maps a
/// request path to a full response, or to `None` to accept and never answer.
struct Counting {
    addr: SocketAddr,
    accepts: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Counting {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Counting {
    async fn start(answer: fn(&str) -> Option<String>) -> Counting {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let accepts = Arc::new(AtomicUsize::new(0));
        let count = accepts.clone();
        let task = tokio::spawn(async move {
            let mut connections = tokio::task::JoinSet::new();
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                count.fetch_add(1, Ordering::SeqCst);
                connections.spawn(async move {
                    let mut head = Vec::new();
                    let mut chunk = [0u8; 1024];
                    while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                        match stream.read(&mut chunk).await {
                            Ok(0) | Err(_) => return,
                            Ok(n) => head.extend_from_slice(&chunk[..n]),
                        }
                    }
                    let head = String::from_utf8_lossy(&head).into_owned();
                    let path = head.split(' ').nth(1).unwrap().to_string();
                    let reply = answer(&path);
                    let _ = stream
                        .write_all(reply.as_deref().unwrap_or_default().as_bytes())
                        .await;
                    // Held connections stay open until the client closes them.
                    while reply.is_none() && matches!(stream.read(&mut chunk).await, Ok(n) if n > 0)
                    {
                    }
                });
            }
        });
        Counting {
            addr,
            accepts,
            task,
        }
    }
    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.addr)
    }
    fn origin(&self) -> String {
        format!("http://{}", self.addr)
    }
    fn accepted(&self) -> usize {
        self.accepts.load(Ordering::SeqCst)
    }
    async fn accepted_at_least(&self, n: usize) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        while self.accepted() < n {
            assert!(
                std::time::Instant::now() < deadline,
                "{} accepts",
                self.accepted()
            );
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }
}

fn response(status: &str, headers: &str) -> Option<String> {
    Some(format!(
        "HTTP/1.1 {status}\r\n{headers}Content-Length: 0\r\nConnection: close\r\n\r\n"
    ))
}
/// Routes shared by the outcome tests.
fn routes(path: &str) -> Option<String> {
    match path {
        "/gone" => response("404 Not Found", ""),
        "/r0" => response("302 Found", "Location: /r1?secret=1\r\n"),
        "/r1?secret=1" => response("302 Found", "Location: /final\r\n"),
        "/loop" => response("302 Found", "Location: /loop\r\n"),
        silent if silent.starts_with("/silent") => None,
        _ => response("200 OK", ""),
    }
}

fn html_mail(links: &[String]) -> String {
    let body: String = links
        .iter()
        .map(|url| format!("<a href=\"{url}\">link</a>\r\n"))
        .collect();
    format!("Subject: links\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{body}")
}
fn offset_of(raw: &str, needle: &str) -> u64 {
    (raw.find(needle).unwrap() - (raw.find("\r\n\r\n").unwrap() + 4)) as u64
}
async fn reach(srv: &swarmail::RunningServer, id: &str, query: &str) -> (u16, Value) {
    http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}/deliverability{query}"),
        None,
    )
    .await
}
fn link_findings(report: &Value) -> Vec<Value> {
    let mut found: Vec<Value> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["rule_id"].as_str().unwrap().starts_with("link."))
        .cloned()
        .collect();
    found.sort_by_key(|f| f["location"]["byte_offset"].as_u64());
    found
}
fn finding(rule: &str, severity: &str, offset: u64, hint: &str) -> Value {
    // The 0122 §2.2 weight of the severity, restated (none of these ids is zero-weight).
    let weight = match severity {
        "info" => 1,
        "warning" => 3,
        other => panic!("severity {other}"),
    };
    json!({"rule_id":rule,"severity":severity,"fix_hint":hint,
        "location":{"kind":"link","part_path":[],"byte_offset":offset},"weight":weight})
}
async fn serve(mode: LinkReachability) -> swarmail::RunningServer {
    start_with(|mut cfg| {
        cfg.link_reachability = mode;
        cfg
    })
    .await
}

/// Reachability is off by default and lazy: nothing but an explicit request
/// on a server the operator enabled opens a connection (AC3).
#[tokio::test]
async fn link_reachability_is_off_and_lazy() {
    for mode in [LinkReachability::Off, LinkReachability::Loopback] {
        let target = Counting::start(routes).await;
        let raw = html_mail(&[target.url("/ok"), target.url("/gone")]);
        let srv = serve(mode).await;
        let id = smtp_raw(srv.smtp_addr, "lazy", raw.as_bytes()).await;
        let path = format!("/api/v1/messages/{id}");
        assert_eq!(http_json(srv.http_addr, "GET", &path, None).await.0, 200);
        let (status, report) = reach(&srv, &id, "").await;
        assert_eq!(status, 200);
        assert!(report["findings"].as_array().unwrap().iter().all(|f| {
            let rule = f["rule_id"].as_str().unwrap();
            !rule.starts_with("link.reachability") && rule != "link.redirect-chain.v1"
        }));
        assert_eq!(reach(&srv, &id, "?reachability=false").await.1, report);
        let call = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":
            {"name":"swarmail_get_deliverability_report","arguments":{"id":id}}});
        let (status, mcp) = http_json(srv.http_addr, "POST", "/mcp", Some(&call.to_string())).await;
        assert_eq!((status, &mcp["result"]["isError"]), (200, &json!(false)));
        let (status, _) = http_get_text(srv.http_addr, &format!("/ui/message/{id}")).await;
        assert_eq!(status, 200);
        // The bulk checklist errors keep this mail from qualifying: assert
        // answers 409, a zero-timeout await 408, and neither probes.
        for (op, expected) in [
            ("assert?count=1&no_findings_at_or_above=error", 409),
            (
                "await?count=1&timeout_ms=0&no_findings_at_or_above=error",
                408,
            ),
        ] {
            let path = format!("/api/v1/inboxes/lazy/{op}");
            assert_eq!(
                http_json(srv.http_addr, "GET", &path, None).await.0,
                expected,
                "{op}"
            );
        }
        assert_eq!(target.accepted(), 0, "{mode:?}: nothing may probe");
        if mode == LinkReachability::Off {
            let (status, body) = reach(&srv, &id, "?reachability=true").await;
            assert_eq!(
                (status, body),
                (400, json!({"error":"link reachability is not enabled"}))
            );
            // A missing message is a 404 before the mode is consulted.
            assert_eq!(reach(&srv, "missing", "?reachability=true").await.0, 404);
            assert_eq!(target.accepted(), 0);
        } else {
            let (status, report) = reach(&srv, &id, "?reachability=true").await;
            assert_eq!(status, 200);
            assert_eq!(target.accepted(), 2, "one GET per distinct target");
            assert_eq!(link_findings(&report).len(), 1, "{report}");
        }
        for bad in [
            "?reachability=maybe",
            "?reachability=",
            "?probe=true",
            "?reachability=true&x=1",
        ] {
            assert_eq!(reach(&srv, &id, bad).await.0, 400, "{bad}");
        }
        srv.stop().await;
    }
}

/// Opt-in outcomes on a loopback server, with findings written here at exact
/// source offsets (AC4). The silent target pays the real 5 s budget.
#[tokio::test]
async fn link_reachability_outcomes_on_loopback() {
    let target = Counting::start(routes).await;
    let urls = ["/ok", "/gone", "/r0", "/loop", "/silent"].map(|p| target.url(p));
    let mut links: Vec<String> = urls.to_vec();
    links.push("mailto:someone@example.test".into());
    links.push(target.url("/gone"));
    let raw = html_mail(&links);
    let srv = serve(LinkReachability::Loopback).await;
    let id = smtp_raw(srv.smtp_addr, "outcomes", raw.as_bytes()).await;
    let (status, report) = reach(&srv, &id, "?reachability=true").await;
    assert_eq!(status, 200, "{report}");
    let at = |path: &str| offset_of(&raw, &target.url(path));
    let gone = "The link answered HTTP 404 during the opt-in reachability check; fix or replace the target.";
    let second_gone =
        raw.rfind(&target.url("/gone")).unwrap() - (raw.find("\r\n\r\n").unwrap() + 4);
    let mut expected = vec![
        finding("link.reachability.v1", "warning", at("/gone"), gone),
        finding("link.reachability.v1", "warning", second_gone as u64, gone),
        finding(
            "link.redirect-chain.v1",
            "info",
            at("/r0"),
            &format!(
                "This link redirects 2 time(s) before reaching {}; link to the destination directly.",
                target.origin()
            ),
        ),
        finding(
            "link.reachability.v1",
            "warning",
            at("/loop"),
            "The link redirected more than 5 times during the opt-in reachability check; link to the destination directly.",
        ),
        finding(
            "link.reachability.v1",
            "warning",
            at("/silent"),
            "The link did not finish answering within 5 seconds during the opt-in reachability check.",
        ),
    ];
    expected.sort_by_key(|f| f["location"]["byte_offset"].as_u64());
    assert_eq!(link_findings(&report), expected);
    // The cache slot still holds only the deterministic report.
    let (_, plain) = reach(&srv, &id, "").await;
    assert!(link_findings(&plain).is_empty());
    // The probed response weighs and totals the composed union (0122 §2.3): the
    // deterministic report keeps its own total, and the probed total adds exactly
    // the observed findings' weights (a count of warnings and an info).
    let observed: u64 = expected.iter().map(|f| f["weight"].as_u64().unwrap()).sum();
    assert!(observed > 0);
    assert_eq!(
        report["total"].as_u64().unwrap(),
        plain["total"].as_u64().unwrap() + observed,
        "{report}"
    );
    assert_ne!(report["total"], plain["total"]);
    let hints = serde_json::to_string(&report).unwrap();
    assert!(!hints.contains("secret"), "no query leaks into a hint");
    // One GET each for /ok, /gone (probed once though linked twice) and
    // /silent, three for /r0 and its two redirects, six for the endless loop.
    assert_eq!(target.accepted(), 1 + 1 + 3 + 6 + 1);
    srv.stop().await;
}

/// A fragment is never sent, so two links that differ only in it are one
/// target: one connection, and the outcome at both locations (AC6).
#[tokio::test]
async fn link_reachability_ignores_the_fragment() {
    let target = Counting::start(routes).await;
    let (a, b) = (target.url("/gone#a"), target.url("/gone#b"));
    let raw = html_mail(&[a.clone(), b.clone()]);
    let srv = serve(LinkReachability::Loopback).await;
    let id = smtp_raw(srv.smtp_addr, "fragment", raw.as_bytes()).await;
    let (status, report) = reach(&srv, &id, "?reachability=true").await;
    assert_eq!(status, 200, "{report}");
    let hint = "The link answered HTTP 404 during the opt-in reachability check; fix or replace the target.";
    let expected = vec![
        finding("link.reachability.v1", "warning", offset_of(&raw, &a), hint),
        finding("link.reachability.v1", "warning", offset_of(&raw, &b), hint),
    ];
    assert_eq!(link_findings(&report), expected);
    assert_eq!(target.accepted(), 1, "one resource, fetched once");
    srv.stop().await;
}

/// `public` mode never contacts loopback, and credentials are never sent
/// (AC5, wire).
#[tokio::test]
async fn link_reachability_public_mode_refuses_loopback() {
    let hint = "Not checked: the link uses credentials or resolves to an address the reachability check never contacts in this mode.";
    for mode in [LinkReachability::Public, LinkReachability::Loopback] {
        let target = Counting::start(routes).await;
        let with_credentials = format!("http://user:pass@{}/ok", target.addr);
        let raw = html_mail(&[target.url("/ok"), with_credentials.clone()]);
        let srv = serve(mode).await;
        let id = smtp_raw(srv.smtp_addr, "refused", raw.as_bytes()).await;
        let (status, report) = reach(&srv, &id, "?reachability=true").await;
        assert_eq!(status, 200, "{report}");
        let found = link_findings(&report);
        let credentials = finding(
            "link.reachability.refused.v1",
            "info",
            offset_of(&raw, &with_credentials),
            hint,
        );
        if mode == LinkReachability::Public {
            let both = vec![
                finding(
                    "link.reachability.refused.v1",
                    "info",
                    offset_of(&raw, &target.url("/ok")),
                    hint,
                ),
                credentials,
            ];
            assert_eq!(found, both);
            assert_eq!(target.accepted(), 0);
        } else {
            assert_eq!(found, vec![credentials]);
            assert_eq!(
                target.accepted(),
                1,
                "only the credential-free link connects"
            );
        }
        srv.stop().await;
    }
}

/// The bounds on the wire (AC6): 33 distinct targets fail before any
/// connection; a third concurrent probe request is unavailable; and while two
/// probes are held, SMTP, reads and a cold default report all proceed, so a
/// probe holds neither a job permit nor a Store guard.
#[tokio::test]
async fn link_reachability_bounds_on_the_wire() {
    let quick = Counting::start(routes).await;
    let held = Counting::start(routes).await;
    let srv = serve(LinkReachability::Loopback).await;
    let many: Vec<String> = (0..33).map(|n| quick.url(&format!("/{n}"))).collect();
    let id = smtp_raw(srv.smtp_addr, "bounds", html_mail(&many).as_bytes()).await;
    let (status, body) = reach(&srv, &id, "?reachability=true").await;
    assert_eq!(
        (status, body),
        (503, json!({"error":"deliverability analysis failed"}))
    );
    assert_eq!(quick.accepted(), 0, "refused before any connection");
    // Exactly 32 targets are allowed.
    let id = smtp_raw(srv.smtp_addr, "bounds", html_mail(&many[..32]).as_bytes()).await;
    assert_eq!(reach(&srv, &id, "?reachability=true").await.0, 200);
    assert_eq!(quick.accepted(), 32);

    let mut waiting = Vec::new();
    for n in 1..=3 {
        let raw = html_mail(&[held.url(&format!("/silent?{n}"))]);
        waiting.push(smtp_raw(srv.smtp_addr, "held", raw.as_bytes()).await);
    }
    let addr = srv.http_addr;
    let ask = |id: String| {
        tokio::spawn(async move {
            http_json(
                addr,
                "GET",
                &format!("/api/v1/messages/{id}/deliverability?reachability=true"),
                None,
            )
            .await
        })
    };
    let (first, second) = (ask(waiting[0].clone()), ask(waiting[1].clone()));
    held.accepted_at_least(2).await;
    let (status, body) = reach(&srv, &waiting[2], "?reachability=true").await;
    assert_eq!(
        (status, body),
        (503, json!({"error":"deliverability analysis unavailable"}))
    );
    // A fresh 250, its read and a cold default report complete meanwhile.
    let fresh = smtp_raw(srv.smtp_addr, "fresh", b"Subject: fresh\r\n\r\nhello\r\n").await;
    let (status, _) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{fresh}"),
        None,
    )
    .await;
    assert_eq!(status, 200);
    let (status, cold) = reach(&srv, &fresh, "").await;
    assert_eq!(status, 200, "{cold}");
    // Both probes are still held: a probe that took a job permit or a Store
    // guard would have stalled the reads above instead.
    assert!(!first.is_finished() && !second.is_finished());
    // Release them: each pays its budget and ends as a timeout.
    for task in [first, second] {
        let (status, report) = task.await.unwrap();
        assert_eq!(status, 200, "{report}");
        let found = link_findings(&report);
        assert_eq!(found.len(), 1);
        assert!(
            found[0]["fix_hint"]
                .as_str()
                .unwrap()
                .contains("within 5 seconds")
        );
    }
    srv.stop().await;
}
