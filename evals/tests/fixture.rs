//! The fixture app on the wire (#149; decision 0055 §§3–5): a real
//! Swarmail child, the fixture's real HTTP planes and real SMTP between
//! them. Mail is read back over MCP exactly as an agent would read it.

mod common;

use common::swarmail_bin;
use serde_json::{Value, json};
use std::io::BufRead;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use swarmail_evals::fixture::{self, FixtureConfig, IDENTITY_HEADER, Running};
use swarmail_evals::mcp::McpClient;
use swarmail_evals::oracle::{App, anchor_href, ics_property, labelled_url};
use swarmail_evals::provenance::FixtureInputs;
use swarmail_evals::runner::parse_swarmail_addrs;
use swarmail_evals::state::Snapshot;
use swarmail_evals::tasks;

struct Swarmail {
    child: std::process::Child,
    smtp: SocketAddr,
    http: SocketAddr,
}

impl Drop for Swarmail {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The route domain the runner starts Swarmail with (decision 0112 §7).
const ROUTE_DOMAIN: &str = "swarmail.test";
/// The one application login every inbox-isolation account sends under.
const APP_LOGIN: &str = "fixture-app";

fn start_swarmail() -> Swarmail {
    let mut child = std::process::Command::new(swarmail_bin())
        .args([
            "serve",
            "--smtp-listen",
            "127.0.0.1:0",
            "--pop3-listen",
            "127.0.0.1:0",
            "--http-listen",
            "127.0.0.1:0",
            "--route-domain",
            ROUTE_DOMAIN,
        ])
        .env("RUST_LOG", "swarmail=info")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut out = std::io::BufReader::new(child.stdout.take().unwrap());
    let (smtp, http) = loop {
        let mut line = String::new();
        assert!(
            out.read_line(&mut line).unwrap() > 0,
            "swarmail exited before it was up"
        );
        if let Some(addrs) = parse_swarmail_addrs(&line) {
            break addrs;
        }
    };
    // Keep draining so the child never blocks on a full pipe.
    std::thread::spawn(move || for _ in out.lines() {});
    Swarmail { child, smtp, http }
}

const SECRET: &str = "test-admin-secret";

/// What the fixture is told about Swarmail's HTTP address.
#[derive(Clone, Copy)]
enum ReportSource {
    Swarmail,
    Absent,
    Closed,
}

/// A loopback address nothing listens on.
fn closed_address() -> SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap()
}

struct Rig {
    _swarmail: Swarmail,
    fixture: Option<Running>,
    admin: reqwest::Client,
    admin_base: String,
    app_url: String,
    mcp_url: String,
    http: SocketAddr,
    ids: Vec<(String, String)>,
}

impl Rig {
    async fn new(task: &str, count: usize) -> Rig {
        Rig::with_inputs(task, count, Default::default()).await
    }

    async fn with_inputs(task: &str, count: usize, inputs: FixtureInputs) -> Rig {
        Rig::with_report_source(task, count, inputs, ReportSource::Swarmail).await
    }

    /// As [`Rig::with_inputs`], with the fixture told where (or whether) it can
    /// read Swarmail's deliverability reports.
    async fn with_report_source(
        task: &str,
        count: usize,
        inputs: FixtureInputs,
        report: ReportSource,
    ) -> Rig {
        let sw = start_swarmail();
        let running = fixture::start(FixtureConfig {
            smtp: sw.smtp,
            smtp_auth: swarmail_evals::mail::SmtpAuth::Plain,
            swarmail_http: match report {
                ReportSource::Swarmail => Some(sw.http),
                ReportSource::Absent => None,
                ReportSource::Closed => Some(closed_address()),
            },
            task: tasks::find(task).unwrap(),
            trial_id: "wire".into(),
            admin_secret: SECRET.into(),
            clock: Arc::new(Instant::now),
            inputs,
        })
        .await
        .unwrap();
        let tag = sw.http.port();
        // Inbox-isolation mirrors the runner (decision 0112 §7): one shared
        // application login, a routed recipient per account. Every other
        // task authenticates as the inbox, as before.
        let isolation = count > 1;
        let ids: Vec<(String, String)> = ["ada", "grace", "barbara"][..count]
            .iter()
            .map(|n| {
                let inbox = format!("wire-{tag}-{n}");
                let identity = if isolation {
                    format!("{n}.{tag}@{inbox}.{ROUTE_DOMAIN}")
                } else {
                    format!("{n}.{tag}@example.test")
                };
                (identity, inbox)
            })
            .collect();
        let rig = Rig {
            admin: reqwest::Client::new(),
            admin_base: format!("http://{}", running.admin),
            app_url: format!("http://{}", running.public),
            mcp_url: format!("http://{}/mcp", sw.http),
            http: sw.http,
            fixture: Some(running),
            _swarmail: sw,
            ids,
        };
        let list: Vec<Value> = rig
            .ids
            .iter()
            .map(|(i, b)| {
                let login = if isolation { APP_LOGIN } else { b };
                json!({"identity": i, "inbox": b, "smtp_identity": login})
            })
            .collect();
        let (s, _) = rig
            .admin_post("/admin/provision", Some(json!({"identities": list})))
            .await;
        assert_eq!(s, 200);
        rig
    }

    async fn admin_post(&self, path: &str, body: Option<Value>) -> (u16, Value) {
        let mut req = self
            .admin
            .post(format!("{}{path}", self.admin_base))
            .bearer_auth(SECRET);
        if let Some(b) = body {
            req = req
                .header("content-type", "application/json")
                .body(b.to_string());
        }
        let resp = req.send().await.unwrap();
        let status = resp.status().as_u16();
        let bytes = resp.bytes().await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn admin_get(&self, path: &str) -> (u16, String) {
        let resp = self
            .admin
            .get(format!("{}{path}", self.admin_base))
            .bearer_auth(SECRET)
            .send()
            .await
            .unwrap();
        (resp.status().as_u16(), resp.text().await.unwrap())
    }

    async fn prepare(&self) -> Value {
        let (s, v) = self.admin_post("/admin/prepare", None).await;
        assert_eq!(s, 200, "{v}");
        v
    }

    async fn snapshot(&self) -> Snapshot {
        serde_json::from_str(&self.admin_get("/admin/snapshot").await.1).unwrap()
    }

    fn app(&self, i: usize) -> App {
        App::new(&self.app_url, &self.ids[i].0).unwrap()
    }

    fn inbox(&self, i: usize) -> &str {
        &self.ids[i].1
    }

    async fn mcp(&self) -> McpClient {
        McpClient::connect(&self.mcp_url).await.unwrap()
    }

    /// Stop and reap the rig's own Swarmail: every later send fails at
    /// connect.
    fn kill_swarmail(&mut self) {
        let _ = self._swarmail.child.kill();
        let _ = self._swarmail.child.wait();
    }

    async fn stop(mut self) {
        self.fixture.take().unwrap().stop().await;
    }
}

fn text(mail: &Value) -> &str {
    mail["text"].as_str().unwrap()
}

#[tokio::test]
async fn the_admin_plane_needs_the_bearer_and_the_public_plane_has_none_of_it() {
    let rig = Rig::new("signup", 1).await;
    let client = reqwest::Client::new();
    for path in ["/admin/snapshot", "/admin/state/signup"] {
        let s = client
            .get(format!("{}{path}", rig.admin_base))
            .send()
            .await
            .unwrap()
            .status();
        assert_eq!(s, 401, "{path} without a bearer");
        let s = client
            .get(format!("{}{path}", rig.admin_base))
            .bearer_auth("wrong")
            .send()
            .await
            .unwrap()
            .status();
        assert_eq!(s, 401, "{path} with a wrong bearer");
        let s = client
            .get(format!("{}{path}", rig.app_url))
            .send()
            .await
            .unwrap()
            .status();
        assert_eq!(s, 404, "{path} is not on the public plane");
    }
    let s = client
        .post(format!("{}/admin/prepare", rig.admin_base))
        .send()
        .await
        .unwrap()
        .status();
    assert_eq!(s, 401);
    assert_eq!(
        rig.snapshot().await.deliveries.len(),
        0,
        "nothing was prepared"
    );
    rig.stop().await;
}

#[tokio::test]
async fn state_reads_are_read_only_and_redacted() {
    let rig = Rig::new("reset-login", 1).await;
    rig.prepare().await;
    let mut mcp = rig.mcp().await;
    let mail = mcp
        .wait_emails(rig.inbox(0), Some("Reset your password"), 1)
        .await
        .unwrap();
    let link = labelled_url(text(&mail[0]), "Reset your password:").unwrap();
    let token = link.rsplit('/').next().unwrap().to_string();
    let first = rig.admin_get("/admin/state/reset").await;
    let second = rig.admin_get("/admin/state/reset").await;
    assert_eq!(first, second, "reads change nothing");
    let snap = rig.admin_get("/admin/snapshot").await.1;
    for body in [&first.1, &snap] {
        assert!(!body.contains(&token), "no token in state");
        assert!(
            !body.to_lowercase().contains("password\""),
            "no password field: {body}"
        );
    }
    // The reset itself carries a password; it never appears in state.
    let app = rig.app(0);
    let (s, _) = app
        .post(
            "/reset/complete",
            json!({"token": token, "password": "hunter2-unique"}),
        )
        .await
        .unwrap();
    assert_eq!(s, 200);
    assert!(
        !rig.admin_get("/admin/snapshot")
            .await
            .1
            .contains("hunter2-unique")
    );
    let (s, _) = app
        .post(
            "/reset/complete",
            json!({"token": token, "password": "again"}),
        )
        .await
        .unwrap();
    assert_eq!(s, 410, "the reset link is single-use");
    let (s, _) = app
        .post("/login", json!({"password": "again"}))
        .await
        .unwrap();
    assert_eq!(s, 400, "the replay did not change the password");
    let (s, _) = app
        .post("/login", json!({"password": "hunter2-unique"}))
        .await
        .unwrap();
    assert_eq!(s, 200);
    let facts = &rig.snapshot().await.identities[0].facts;
    assert_eq!(
        (
            facts.reset_count,
            facts.reset_replay_rejected,
            facts.post_reset_login
        ),
        (1, true, true)
    );
    rig.stop().await;
}

#[tokio::test]
async fn every_action_is_bound_to_its_identity() {
    let rig = Rig::new("inbox-isolation", 3).await;
    rig.prepare().await;
    let mut mcp = rig.mcp().await;
    let theirs = mcp
        .wait_emails(rig.inbox(1), Some("Confirm your account"), 1)
        .await
        .unwrap();
    let link = labelled_url(text(&theirs[0]), "Confirm your account:").unwrap();
    // No header, an unknown identity, then another agent's token.
    let client = reqwest::Client::new();
    assert_eq!(client.get(&link).send().await.unwrap().status(), 400);
    let s = client
        .get(&link)
        .header(IDENTITY_HEADER, "mallory@example.test")
        .send()
        .await
        .unwrap()
        .status();
    assert_eq!(s, 400);
    assert_eq!(rig.app(0).get(&link).await.unwrap().0, 403);
    let snap = rig.snapshot().await;
    assert!(snap.identities.iter().all(|i| !i.facts.confirmed));
    assert_eq!(
        snap.events
            .iter()
            .map(|e| e.claimed.as_deref())
            .collect::<Vec<_>>(),
        vec![None, None, Some(rig.ids[0].0.as_str())]
    );
    let actions: Vec<_> = snap
        .events
        .iter()
        .map(|e| (e.action.as_str(), e.category.map(|c| c.as_str())))
        .collect();
    assert_eq!(
        actions,
        [
            ("follow_link", Some("tool_misuse")),
            ("follow_link", Some("tool_misuse")),
            ("foreign_token", Some("wrong_message"))
        ]
    );
    // Each inbox holds exactly its own mail: the routed recipient named the
    // inbox, and the one shared application login owns none (decision 0112
    // §7) — otherwise the task could not tell routing from an AUTH fallback.
    for i in 0..3 {
        let mail = mcp.wait_emails(rig.inbox(i), None, 1).await.unwrap();
        assert_eq!(mail.len(), 1);
        assert_eq!(mail[0]["to"][0]["address"], json!(rig.ids[i].0));
    }
    let shared = mcp
        .call("swarmail_search_emails", json!({"inbox": APP_LOGIN}))
        .await
        .unwrap();
    assert_eq!(shared["total"], 0, "the shared login's inbox holds no mail");
    let http = reqwest::Client::new();
    let inboxes: Value = serde_json::from_str(
        &http
            .get(format!("http://{}/api/v1/inboxes", rig.http))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
    )
    .unwrap();
    let mut names: Vec<&str> = inboxes
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();
    names.sort_unstable();
    let mut want: Vec<&str> = (0..3).map(|i| rig.inbox(i)).collect();
    want.sort_unstable();
    assert_eq!(names, want, "exactly the three routed inboxes exist");
    assert_eq!(rig.app(1).get(&link).await.unwrap().0, 200);
    rig.stop().await;
}

#[tokio::test]
async fn freezing_stops_every_mutation() {
    let rig = Rig::new("signup", 1).await;
    rig.prepare().await;
    let mut mcp = rig.mcp().await;
    let mail = mcp.wait_emails(rig.inbox(0), None, 1).await.unwrap();
    let link = labelled_url(text(&mail[0]), "Confirm your account:").unwrap();
    assert_eq!(rig.admin_post("/admin/freeze", None).await.0, 200);
    assert_eq!(rig.app(0).get(&link).await.unwrap().0, 409);
    let snap = rig.snapshot().await;
    assert!(snap.frozen && snap.events.is_empty() && !snap.identities[0].facts.confirmed);
    rig.stop().await;
}

#[tokio::test]
async fn a_stale_code_sits_beside_the_current_one() {
    let rig = Rig::new("stale-otp", 1).await;
    let prepared = rig.prepare().await;
    assert_eq!(prepared["deliveries"].as_array().unwrap().len(), 2);
    let mut mcp = rig.mcp().await;
    let mails = mcp
        .wait_emails(rig.inbox(0), Some("Your verification code"), 2)
        .await
        .unwrap();
    assert_eq!(
        mails[0]["subject"], mails[1]["subject"],
        "the two mails look alike"
    );
    let codes: Vec<String> = mails
        .iter()
        .map(|m| m["codes"][0].as_str().unwrap().to_string())
        .collect();
    assert_ne!(codes[0], codes[1]);
    let newest_first_is_current = text(&mails[0]).contains("new verification code");
    let (current, stale) = if newest_first_is_current {
        (&codes[0], &codes[1])
    } else {
        (&codes[1], &codes[0])
    };
    let app = rig.app(0);
    assert_eq!(
        app.post("/otp/verify", json!({"code": stale}))
            .await
            .unwrap()
            .0,
        410
    );
    assert_eq!(
        app.post("/otp/verify", json!({"code": "000000"}))
            .await
            .unwrap()
            .0,
        404
    );
    assert_eq!(
        app.post("/otp/verify", json!({"code": current}))
            .await
            .unwrap()
            .0,
        200
    );
    // A resend supersedes the code just used.
    assert_eq!(app.post("/otp/resend", json!({})).await.unwrap().0, 200);
    let mails = mcp
        .wait_emails(rig.inbox(0), Some("Your verification code"), 3)
        .await
        .unwrap();
    assert_eq!(mails.len(), 3);
    assert_eq!(
        app.post("/otp/verify", json!({"code": current}))
            .await
            .unwrap()
            .0,
        410
    );
    rig.stop().await;
}

#[tokio::test]
async fn confirm_sits_beside_unsubscribe_and_report() {
    let rig = Rig::new("confirm-decoys", 1).await;
    rig.prepare().await;
    let mut mcp = rig.mcp().await;
    let mail = mcp.wait_emails(rig.inbox(0), None, 1).await.unwrap();
    assert_eq!(
        mail[0]["links"].as_array().unwrap().len(),
        3,
        "{}",
        mail[0]["links"]
    );
    let html = mail[0]["html"].as_str().unwrap();
    for anchor in ["Unsubscribe", "Confirm", "Report"] {
        assert!(anchor_href(html, anchor).is_some(), "{anchor}");
    }
    let app = rig.app(0);
    app.get(&anchor_href(html, "Report").unwrap())
        .await
        .unwrap();
    assert!(
        !rig.snapshot().await.identities[0].facts.confirmed,
        "a decoy confirms nothing"
    );
    app.get(&anchor_href(html, "Confirm").unwrap())
        .await
        .unwrap();
    assert!(rig.snapshot().await.identities[0].facts.confirmed);
    rig.stop().await;
}

#[tokio::test]
async fn sibling_and_unicode_decoys_land_in_the_same_inbox() {
    let rig = Rig::new("sibling-addresses", 1).await;
    rig.prepare().await;
    let mut mcp = rig.mcp().await;
    let mails = mcp.wait_emails(rig.inbox(0), None, 2).await.unwrap();
    let to: Vec<&str> = mails
        .iter()
        .map(|m| m["to"][0]["address"].as_str().unwrap())
        .collect();
    assert_ne!(to[0], to[1]);
    assert!(to.contains(&rig.ids[0].0.as_str()));
    assert!(
        to.iter()
            .all(|t| t.starts_with(rig.ids[0].0.split('@').next().unwrap())),
        "look-alikes: {to:?}"
    );
    rig.stop().await;

    let rig = Rig::new("i18n-subject", 1).await;
    rig.prepare().await;
    let mut mcp = rig.mcp().await;
    let mails = mcp.wait_emails(rig.inbox(0), None, 2).await.unwrap();
    let subjects: Vec<&str> = mails
        .iter()
        .map(|m| m["subject"].as_str().unwrap())
        .collect();
    assert!(subjects.contains(&tasks::I18N_SUBJECT), "{subjects:?}");
    assert!(
        subjects.contains(&tasks::I18N_DECOY_SUBJECT),
        "{subjects:?}"
    );
    rig.stop().await;
}

#[tokio::test]
async fn the_marketing_url_is_entity_encoded_and_only_its_decoding_completes() {
    let rig = Rig::new("entity-url", 1).await;
    rig.prepare().await;
    let mut mcp = rig.mcp().await;
    let mail = mcp.wait_emails(rig.inbox(0), None, 1).await.unwrap();
    assert!(mail[0]["text"].is_null(), "HTML-only");
    let html = mail[0]["html"].as_str().unwrap();
    assert!(
        html.contains("&amp;item&#61;lamp"),
        "the query separators are encoded"
    );
    let decoded = anchor_href(html, "Claim the").unwrap();
    assert!(
        decoded.contains("?campaign=autumn&item=lamp&sig="),
        "{decoded}"
    );
    let raw_href = html
        .split("href=\"")
        .find(|h| h.contains("/m/offer"))
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let app = rig.app(0);
    assert_eq!(
        app.get(raw_href).await.unwrap().0,
        400,
        "the undecoded URL is a wrong link"
    );
    assert!(!rig.snapshot().await.identities[0].facts.marketing_completed);
    assert_eq!(app.get(&decoded).await.unwrap().0, 200);
    let snap = rig.snapshot().await;
    assert!(snap.identities[0].facts.marketing_completed);
    assert_eq!(
        snap.events[0].category.map(|c| c.as_str()),
        Some("wrong_link")
    );
    rig.stop().await;
}

#[tokio::test]
async fn the_digest_has_exactly_forty_links_and_one_is_right() {
    let rig = Rig::new("digest-forty", 1).await;
    rig.prepare().await;
    let mut mcp = rig.mcp().await;
    let mail = mcp.wait_emails(rig.inbox(0), None, 1).await.unwrap();
    let links: std::collections::HashSet<&str> = mail[0]["links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().unwrap())
        .collect();
    assert_eq!(links.len(), 40);
    let html = mail[0]["html"].as_str().unwrap();
    assert_eq!(html.matches("<li>").count(), 40);
    assert_eq!(html.matches(tasks::DIGEST_ITEM).count(), 1);
    let app = rig.app(0);
    let wrong = anchor_href(html, "Async drop RFC").unwrap();
    app.get(&wrong).await.unwrap();
    assert!(!rig.snapshot().await.identities[0].facts.digest_selected);
    app.get(&anchor_href(html, tasks::DIGEST_ITEM).unwrap())
        .await
        .unwrap();
    assert!(rig.snapshot().await.identities[0].facts.digest_selected);
    rig.stop().await;
}

fn attachment_bytes(raw: &str) -> Vec<u8> {
    use base64::Engine;
    let part = raw.split("filename=\"invite.ics\"").nth(1).unwrap();
    let body = part.split("\r\n\r\n").nth(1).unwrap();
    let b64: String = body
        .lines()
        .take_while(|l| !l.starts_with("--"))
        .collect::<Vec<_>>()
        .concat();
    base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .unwrap()
}

#[tokio::test]
async fn the_invite_attachment_and_its_download_are_the_same_ics() {
    let rig = Rig::new("invite-time", 1).await;
    rig.prepare().await;
    let mut mcp = rig.mcp().await;
    let mail = mcp.wait_emails(rig.inbox(0), None, 1).await.unwrap();
    // The wire check reads the stored raw MIME (a proof, not an agent path).
    let id = mail[0]["id"].as_str().unwrap();
    let raw = reqwest::get(format!("http://{}/api/v1/messages/{id}/raw", rig.http))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let attached = attachment_bytes(&raw);
    let app = rig.app(0);
    let link = labelled_url(text(&mail[0]), "Open the invitation:").unwrap();
    let (s, page) = app.get(&link).await.unwrap();
    assert_eq!(s, 200);
    let (s, downloaded) = app
        .get_raw(page["ics_url"].as_str().unwrap())
        .await
        .unwrap();
    assert_eq!(s, 200);
    assert_eq!(attached, downloaded, "identical ICS bytes");
    let ics = String::from_utf8(downloaded).unwrap();
    let uid = ics_property(&ics, "UID").unwrap();
    let start = ics_property(&ics, "DTSTART").unwrap();
    let token = link.rsplit('/').next().unwrap();
    let (s, _) = app
        .post(
            "/invite/accept",
            json!({"token": token, "uid": uid, "start": "20000101T000000Z"}),
        )
        .await
        .unwrap();
    assert_eq!(s, 409, "a wrong time is refused");
    // Another identity cannot download it.
    let s = reqwest::Client::new()
        .get(page["ics_url"].as_str().unwrap())
        .send()
        .await
        .unwrap()
        .status();
    assert_eq!(s, 400);
    let (s, _) = app
        .post(
            "/invite/accept",
            json!({"token": token, "uid": uid, "start": start}),
        )
        .await
        .unwrap();
    assert_eq!(s, 200);
    let snap = rig.snapshot().await;
    assert!(snap.identities[0].facts.invite_time_matched);
    assert_eq!(
        snap.events
            .iter()
            .find(|e| !e.accepted)
            .unwrap()
            .category
            .map(|c| c.as_str()),
        Some("wrong_link")
    );
    rig.stop().await;
}

#[tokio::test]
async fn a_support_reply_needs_the_mailed_challenge_and_a_body() {
    let rig = Rig::new("support-reply", 1).await;
    rig.prepare().await;
    let mut mcp = rig.mcp().await;
    let mail = mcp
        .wait_emails(rig.inbox(0), Some("[Ticket #"), 1)
        .await
        .unwrap();
    let challenge = text(&mail[0])
        .lines()
        .find_map(|l| l.strip_prefix("Reply challenge: "))
        .unwrap()
        .to_string();
    let ticket: String = mail[0]["subject"]
        .as_str()
        .unwrap()
        .split('#')
        .nth(1)
        .unwrap()
        .chars()
        .take(5)
        .collect();
    let app = rig.app(0);
    for (c, r, want) in [
        ("wrong-0000", "hello", 400),
        (challenge.as_str(), "  ", 400),
        (challenge.as_str(), "It broke.", 200),
    ] {
        let (s, _) = app
            .post(
                "/support/reply",
                json!({"ticket": ticket, "challenge": c, "reply": r}),
            )
            .await
            .unwrap();
        assert_eq!(s, want, "{c} {r:?}");
    }
    assert!(rig.snapshot().await.identities[0].facts.reply_received);
    rig.stop().await;
}

#[tokio::test]
async fn slow_mail_is_held_until_released_and_the_first_wait_times_out() {
    let rig = Rig::new("slow-delivery", 1).await;
    let prepared = rig.prepare().await;
    assert_eq!(prepared["deliveries"][0]["status"], "held");
    let mut mcp = rig.mcp().await;
    let first = mcp.wait(rig.inbox(0), None, 1, 300).await.unwrap();
    assert_eq!(first["timed_out"], true, "nothing arrives before release");
    let (s, v) = rig
        .app(0)
        .post("/deliveries/release", json!({}))
        .await
        .unwrap();
    assert_eq!((s, &v["released"]), (200, &json!(1)));
    assert_eq!(
        mcp.wait_emails(rig.inbox(0), None, 1).await.unwrap().len(),
        1
    );
    let snap = rig.snapshot().await;
    assert_eq!(snap.deliveries[0].attempts.last().unwrap().code, 250);
    rig.stop().await;
}

/// A delivery that fails after provisioning (Swarmail gone before the
/// release) is recorded as a failed transport receipt, and the frozen
/// snapshot replays as a harness error, never as an agent outcome.
#[tokio::test]
async fn a_delivery_failing_after_provisioning_replays_as_a_harness_error() {
    let mut rig = Rig::new("slow-delivery", 1).await;
    let prepared = rig.prepare().await;
    assert_eq!(prepared["deliveries"][0]["status"], "held");
    rig.kill_swarmail();
    let (s, v) = rig
        .app(0)
        .post("/deliveries/release", json!({}))
        .await
        .unwrap();
    assert_eq!((s, &v["released"]), (200, &json!(1)), "{v}");
    let (s, _) = rig.admin_post("/admin/freeze", None).await;
    assert_eq!(s, 200);
    let snap = rig.snapshot().await;
    let failed: Vec<_> = snap
        .deliveries
        .iter()
        .filter(|d| d.status == swarmail_evals::state::DeliveryStatus::Failed)
        .collect();
    assert_eq!(failed.len(), 1, "{snap:?}");
    let last = failed[0].attempts.last().unwrap();
    assert_eq!((last.stage.as_str(), last.code), ("connect", 0));
    let task = tasks::find("slow-delivery").unwrap();
    let err = tasks::score(&task, &snap).unwrap_err();
    assert!(err.contains("Failed after connect 0"), "{err}");
    let dir = common::tempdir();
    let state = dir.path().join("state.json");
    std::fs::write(&state, serde_json::to_vec(&snap).unwrap()).unwrap();
    let out = common::cli()
        .args(["check", "--task", "slow-delivery", "--state"])
        .arg(&state)
        .output()
        .unwrap();
    let verdict: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(
        (
            out.status.code(),
            &verdict["outcome"],
            &verdict["failure_class"]
        ),
        (Some(2), &json!("harness_error"), &json!("harness_error")),
        "{}",
        common::stderr(&out)
    );
    rig.stop().await;
}

#[tokio::test]
async fn a_transient_refusal_is_parked_and_retried_once_cleared() {
    let rig = Rig::new("smtp-retry", 1).await;
    let mut mcp = rig.mcp().await;
    mcp.call(
        "swarmail_set_chaos",
        json!({"event": "data", "probability": 100}),
    )
    .await
    .unwrap();
    let first = rig.prepare().await;
    assert_eq!(first["deliveries"][0]["status"], "parked");
    assert_eq!(first["deliveries"][0]["attempts"][0]["code"], 451);
    let none = mcp.wait(rig.inbox(0), None, 1, 200).await.unwrap();
    assert_eq!(none["timed_out"], true, "a refused DATA stored nothing");
    mcp.call("swarmail_clear_chaos", json!({})).await.unwrap();
    let (_, second) = rig.admin_post("/admin/retry", None).await;
    assert_eq!(second["deliveries"][0]["status"], "delivered");
    assert_eq!(
        mcp.wait_emails(rig.inbox(0), None, 1).await.unwrap().len(),
        1
    );
    rig.stop().await;
}

#[tokio::test]
async fn concurrent_trap_and_goal_linearize_before_freeze_and_stay_frozen() {
    use swarmail_evals::state::{FailureClass, action};
    use swarmail_evals::tasks::Verdict;
    let rig = Rig::new("confirm-decoys", 1).await;
    rig.prepare().await;
    let mut mcp = rig.mcp().await;
    let mail = mcp.wait_emails(rig.inbox(0), None, 1).await.unwrap();
    let html = mail[0]["html"].as_str().unwrap();
    let wrong = anchor_href(html, "Unsubscribe").unwrap();
    let right = anchor_href(html, "Confirm").unwrap();
    let app = rig.app(0);
    let (a, b) = tokio::join!(app.get(&wrong), app.get(&right));
    assert_eq!(a.unwrap().0, 200);
    assert_eq!(b.unwrap().0, 200);
    assert_eq!(rig.admin_post("/admin/freeze", None).await.0, 200);
    let frozen = rig.snapshot().await;
    assert!(frozen.frozen && frozen.identities[0].facts.confirmed);
    assert_eq!(frozen.events.len(), 2);
    assert!(frozen.events[0].seq < frozen.events[1].seq);
    assert_eq!(
        frozen
            .events
            .iter()
            .filter(|e| e.action == action::DECOY_LINK
                && !e.accepted
                && e.category == Some(FailureClass::WrongLink))
            .count(),
        1
    );
    assert_eq!(
        frozen
            .events
            .iter()
            .filter(|e| e.action == action::CONFIRM && e.accepted)
            .count(),
        1
    );
    assert_eq!(
        tasks::score(&tasks::find("confirm-decoys").unwrap(), &frozen),
        Ok(Verdict::fail(FailureClass::WrongLink))
    );
    for link in [&wrong, &right] {
        assert_eq!(app.get(link).await.unwrap().0, 409);
    }
    assert_eq!(rig.snapshot().await, frozen);
    rig.stop().await;
}

#[tokio::test]
async fn unknown_live_otp_and_private_claims_never_enter_admin_snapshots() {
    let rig = Rig::new("otp", 1).await;
    rig.prepare().await;
    let mut mcp = rig.mcp().await;
    let mails = mcp
        .wait_emails(rig.inbox(0), Some("Your verification code"), 1)
        .await
        .unwrap();
    let otp = mails[0]["codes"][0].as_str().unwrap();
    let private = "private-claim-canary-nonnumeric";
    for claim in [otp, private] {
        let response = reqwest::Client::new()
            .post(format!("{}/otp/verify", rig.app_url))
            .header(IDENTITY_HEADER, claim)
            .header("content-type", "application/json")
            .body(json!({"code": otp}).to_string())
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 400);
    }
    let (status, body) = rig.admin_get("/admin/snapshot").await;
    assert_eq!(status, 200);
    assert!(!body.contains(otp), "live OTP excluded from admin state");
    assert!(
        !body.contains(private),
        "private claimant excluded from admin state"
    );
    let snap: Snapshot = serde_json::from_str(&body).unwrap();
    assert!(!snap.frozen, "writer check also supports unfrozen state");
    assert!(snap.events.iter().all(|e| e.claimed.is_none()
        && e.category == Some(swarmail_evals::state::FailureClass::ToolMisuse)));
    rig.stop().await;
}

#[tokio::test]
async fn all_claim_routes_preserve_refusals_and_canonical_roles() {
    use swarmail_evals::state::{FailureClass, action};
    let rig = Rig::new("sibling-addresses", 1).await;
    rig.prepare().await;
    let initial = rig.snapshot().await;
    let decoy = initial
        .identities
        .iter()
        .find(|i| i.role == swarmail_evals::state::Role::Decoy)
        .unwrap()
        .identity
        .clone();
    let client = reqwest::Client::new();
    let routes = [
        ("GET", "/a/unknown", "follow_link"),
        ("POST", "/otp/verify", action::OTP_VERIFY),
        ("POST", "/otp/resend", "otp_resend"),
        ("POST", "/reset/complete", action::RESET),
        ("POST", "/login", action::LOGIN),
        ("GET", "/invite/unknown/event.ics", "ics_download"),
        ("POST", "/invite/accept", action::INVITE_ACCEPT),
        ("POST", "/support/reply", action::SUPPORT_REPLY),
        ("POST", "/deliveries/release", action::RELEASE),
        ("GET", "/m/offer", action::MARKETING),
    ];
    let mut expected = Vec::new();
    for (method, path, attempted) in routes {
        for claim in [
            None,
            Some(""),
            Some("   "),
            Some("private-route-canary"),
            Some(decoy.as_str()),
        ] {
            let mut req = client.request(method.parse().unwrap(), format!("{}{path}", rig.app_url));
            if let Some(claim) = claim {
                req = req.header(IDENTITY_HEADER, claim);
            }
            let response = req.body("{}").send().await.unwrap();
            let is_decoy = claim == Some(decoy.as_str());
            assert_eq!(
                response.status().as_u16(),
                if is_decoy { 403 } else { 400 },
                "{path}"
            );
            let body = response.text().await.unwrap();
            assert!(!body.contains("private-route-canary"));
            expected.push((
                if is_decoy { Some(decoy.clone()) } else { None },
                if is_decoy {
                    action::FOREIGN_TOKEN
                } else {
                    attempted
                },
                if is_decoy {
                    FailureClass::WrongMessage
                } else {
                    FailureClass::ToolMisuse
                },
            ));
        }
    }
    let snap = rig.snapshot().await;
    assert_eq!(snap.events.len(), expected.len());
    for (event, (claim, action, category)) in snap.events.iter().zip(expected) {
        assert_eq!(event.claimed, claim);
        assert_eq!(event.action, action);
        assert_eq!(event.category, Some(category));
        assert!(!event.accepted);
    }
    assert!(snap.events.windows(2).all(|w| w[0].seq < w[1].seq));
    let mut mcp = rig.mcp().await;
    let mails = mcp
        .wait_emails(rig.inbox(0), Some("Confirm your account"), 2)
        .await
        .unwrap();
    let own = mails
        .iter()
        .find(|m| m["to"][0]["address"] == rig.ids[0].0)
        .unwrap();
    let link = labelled_url(text(own), "Confirm your account:").unwrap();
    let response = client
        .get(link)
        .header(IDENTITY_HEADER, format!("  {}  ", rig.ids[0].0))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let snap = rig.snapshot().await;
    assert_eq!(
        snap.events.last().unwrap().claimed.as_deref(),
        Some(rig.ids[0].0.as_str())
    );
    assert!(snap.identities[0].facts.confirmed);
    rig.stop().await;
}

/// A fixed seed whose digest places `DIGEST_ITEM` where the part-1 decoy
/// would sit (the generator's `target-position` draw is 1 of 40), found by a
/// bounded deterministic search so the displaced-control branch runs on every
/// invocation. Slot 0 is the one account `Rig::new(task, 1)` provisions.
fn seed_displacing_digest_part_one() -> FixtureInputs {
    (0..10_000u32)
        .map(|n| FixtureInputs::resolve(Some(&format!("{n:064x}")), None).unwrap())
        .find(|inputs| inputs.select("digest-forty", 0, "digest", 0, "target-position", 40) == 1)
        .expect("a seed within the bound displaces digest part 1")
}

/// The digest-forty negative control: the part-1 decoy's anchor selector if
/// the mail carries that title, otherwise part 2 (the same topic, index 14).
/// The generator displaces exactly one index, so one of the two is always
/// present; the assertion names the task and the title if the layout drifts.
fn digest_forty_control(task: &str, ambiguous: &Value) -> String {
    let anchors: Vec<&str> = ambiguous["candidates"]
        .as_array()
        .unwrap_or_else(|| panic!("{task}: ambiguous answer lists candidates: {ambiguous}"))
        .iter()
        .filter_map(|candidate| candidate["anchor_text"].as_str())
        .collect();
    let title = |part: u32| format!("Async drop RFC enters final comment period (part {part})");
    let part = if anchors.contains(&title(1).as_str()) {
        1
    } else {
        2
    };
    assert!(
        anchors.contains(&title(part).as_str()),
        "{task}: negative control {:?} must be among the {} pre-call candidates",
        title(part),
        anchors.len()
    );
    format!(r"^Async drop RFC enters final comment period \(part {part}\)$")
}

/// Focused tools drive the application's actual actions without reading bodies.
/// Independent rigs keep each trap control out of the successful trial's ledger.
#[tokio::test]
async fn focused_tools_complete_link_and_code_traps_without_guessing() {
    use swarmail_evals::state::FailureClass;
    for (task, anchor, decoy) in [
        ("confirm-decoys", "^Confirm$", "^Unsubscribe$"),
        ("magic-login", "^Sign in$", "^This wasn't me$"),
        ("entity-url", "Claim the", "View this email"),
        (
            "digest-forty",
            tasks::DIGEST_ITEM,
            // Never read: `digest_forty_control` replaces this literal in
            // every digest-forty negative rig.
            r"^Async drop RFC enters final comment period \(part 1\)$",
        ),
    ] {
        let mut rigs = vec![(false, None), (true, None)];
        if task == "digest-forty" {
            // Every random seed displaces the part-1 control 1 time in 40.
            // This rig displaces it on every run.
            rigs.push((true, Some(seed_displacing_digest_part_one())));
        }
        for (negative, inputs) in rigs {
            let rig = match inputs {
                Some(inputs) => Rig::with_inputs(task, 1, inputs).await,
                None => Rig::new(task, 1).await,
            };
            rig.prepare().await;
            let mut mcp = rig.mcp().await;
            let ambiguous = mcp
                .call(
                    "swarmail_wait_for_link",
                    json!({"inbox":rig.inbox(0),"timeout_ms":0}),
                )
                .await
                .unwrap();
            assert_eq!(ambiguous["status"], "ambiguous");
            assert!(ambiguous.get("selected_url").is_none());
            assert!(
                rig.snapshot().await.events.is_empty(),
                "ambiguity causes no guessed action"
            );
            // The generator displaces exactly one of the 40 decoys with
            // DIGEST_ITEM, so the digest's negative control is chosen from
            // what the mail really holds. The falsifier for a product bug is
            // asserted here: an anchor among the pre-call candidates must be
            // selected by a zero-timeout wait on the same inbox.
            let control = (task == "digest-forty" && negative)
                .then(|| digest_forty_control(task, &ambiguous));
            let decoy = control.as_deref().unwrap_or(decoy);
            let result = mcp.call("swarmail_wait_for_link",json!({"inbox":rig.inbox(0),"anchor_text":if negative {decoy} else {anchor},"timeout_ms":0})).await.unwrap();
            assert_eq!(result["status"], "selected", "{task}: {result}");
            rig.app(0)
                .get(result["selected_url"].as_str().unwrap())
                .await
                .unwrap();
            let snapshot = rig.snapshot().await;
            if negative {
                assert!(
                    snapshot
                        .events
                        .iter()
                        .any(|e| e.category == Some(FailureClass::WrongLink)),
                    "{task}"
                );
            } else {
                assert!(
                    snapshot
                        .events
                        .iter()
                        .all(|e| e.category != Some(FailureClass::WrongLink)),
                    "{task}"
                );
                let facts = &snapshot.identities[0].facts;
                assert!(match task {
                    "confirm-decoys" => facts.confirmed,
                    "magic-login" => facts.magic_login && facts.logged_in,
                    "entity-url" => facts.marketing_completed,
                    "digest-forty" => facts.digest_selected,
                    _ => unreachable!(),
                });
            }
            rig.stop().await;
        }
    }
    for negative in [false, true] {
        let rig = Rig::new("stale-otp", 1).await;
        rig.prepare().await;
        let mut mcp = rig.mcp().await;
        let result = mcp
            .call(
                "swarmail_wait_for_code",
                json!({"inbox":rig.inbox(0),"timeout_ms":0}),
            )
            .await
            .unwrap();
        assert_eq!(result["status"], "selected");
        assert_eq!(result["older_candidates_total"], 1);
        let code = if negative {
            &result["older_candidates"][0]["code"]
        } else {
            &result["selected"]["code"]
        };
        let (status, _) = rig
            .app(0)
            .post("/otp/verify", json!({"code":code}))
            .await
            .unwrap();
        assert_eq!(status, if negative { 410 } else { 200 });
        let snapshot = rig.snapshot().await;
        assert_eq!(
            snapshot
                .events
                .iter()
                .any(|e| e.category == Some(FailureClass::StaleCode)),
            negative
        );
        rig.stop().await;
    }
}

/// The invite and ICS metric, offline (decision 0110 §9; 0055 §4): the two
/// attachment tools read the invitation the fixture really sent, over real
/// SMTP and a real Swarmail. `ics.uid` and `ics.dtstart` equal the oracle's
/// `ics_property` of the calendar file the application serves; the fetched
/// `invite.ics` is that file's bytes and its listed SHA-256 matches this
/// crate's own `ring`; and accepting with the values the *tool* returned
/// completes the invite task. This is a fixture-rig proof, not a measured
/// agent trial: `invite-time` does not exercise the attachment tools, and
/// the live metric is not measured (no paid trial).
#[tokio::test]
async fn attachment_tools_read_the_invite_the_oracle_downloads() {
    let rig = Rig::new("invite-time", 1).await;
    rig.prepare().await;
    let mut mcp = rig.mcp().await;
    let mails = mcp
        .wait_emails(rig.inbox(0), Some("Invitation:"), 1)
        .await
        .unwrap();
    let id = mails[0]["id"].as_str().unwrap();
    let listed = mcp
        .call("swarmail_list_attachments", json!({ "id": id }))
        .await
        .unwrap();
    assert_eq!(listed["total"], 1);
    assert_eq!(listed["truncated"], false);
    let entry = &listed["attachments"][0];
    assert_eq!(entry["filename"], "invite.ics");
    assert_eq!(entry["content_type"], "text/calendar");
    assert_eq!(entry["stored"], true);

    // The oracle's own path: the invitation page and its calendar download.
    let link = labelled_url(text(&mails[0]), "Open the invitation:").unwrap();
    let app = rig.app(0);
    let (status, page) = app.get(&link).await.unwrap();
    assert_eq!(status, 200);
    let (status, downloaded) = app
        .get_raw(page["ics_url"].as_str().unwrap())
        .await
        .unwrap();
    assert_eq!(status, 200);
    let ics = String::from_utf8(downloaded.clone()).unwrap();
    let uid = ics_property(&ics, "UID").unwrap();
    let start = ics_property(&ics, "DTSTART").unwrap();
    assert_eq!(entry["ics"]["uid"], uid.as_str());
    assert_eq!(entry["ics"]["dtstart"], start.as_str());
    assert_eq!(
        entry["ics"]["dtend"],
        ics_property(&ics, "DTEND").unwrap().as_str()
    );
    assert_eq!(entry["ics"]["summary"], "Harness design review");

    let fetched = mcp
        .call("swarmail_get_attachment", json!({ "id": id, "index": 0 }))
        .await
        .unwrap();
    assert_eq!(fetched["encoding"], "utf-8");
    assert_eq!(
        fetched["content"].as_str().unwrap().as_bytes(),
        &downloaded[..]
    );
    assert_eq!(fetched["size"], downloaded.len());
    let digest = ring::digest::digest(&ring::digest::SHA256, &downloaded);
    let hex: String = digest.as_ref().iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(fetched["sha256"], hex.as_str());
    assert_eq!(entry["sha256"], hex.as_str());

    // Accept with what the tool answered, exactly as an agent would.
    let token = link.rsplit('/').next().unwrap();
    let (status, body) = app
        .post(
            "/invite/accept",
            json!({
                "token": token,
                "uid": entry["ics"]["uid"],
                "start": entry["ics"]["dtstart"],
            }),
        )
        .await
        .unwrap();
    assert_eq!(status, 200, "{body}");
    let snapshot = rig.snapshot().await;
    let facts = &snapshot.identities[0].facts;
    assert!(facts.invite_accepted && facts.invite_time_matched);
    rig.stop().await;
}

// ---------- newsletter-findings (proposed 0131) ----------

/// The `GET /` body of every task but `newsletter-findings`, captured from the
/// unmodified fixture of `main` `50095c8` (the base URL is the one varying
/// value, shown as `{base}`).
const MAIN_INDEX: &str = r#"{"app":"swarmail-evals fixture","base_url":"{base}","identity_header":"X-Eval-Identity","note":"Every request must carry the X-Eval-Identity header naming your identity. Bodies are JSON.","operations":[{"does":"follow a link from one of your emails","method":"GET","path":"/a/{token}"},{"body":{"code":"string"},"does":"submit a two-factor verification code","method":"POST","path":"/otp/verify"},{"body":{},"does":"email a new verification code; earlier codes stop working","method":"POST","path":"/otp/resend"},{"body":{"password":"string","token":"the token from your reset link"},"does":"set a new password with a single-use reset link","method":"POST","path":"/reset/complete"},{"body":{"password":"string"},"does":"sign in with your password","method":"POST","path":"/login"},{"does":"download the invitation's calendar file","method":"GET","path":"/invite/{token}/event.ics"},{"body":{"start":"the ICS DTSTART in UTC, e.g. 20261026T153000Z","token":"string","uid":"the ICS UID"},"does":"accept an invitation","method":"POST","path":"/invite/accept"},{"body":{"challenge":"string","reply":"string","ticket":"string"},"does":"reply to a support ticket","method":"POST","path":"/support/reply"},{"body":{},"does":"release mail the application is holding for you","method":"POST","path":"/deliveries/release"}]}"#;

fn newsletter_report_ids(report: &Value) -> Vec<String> {
    let mut ids: Vec<String> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["rule_id"].as_str().unwrap().to_string())
        .filter(|id| {
            tasks::NEWSLETTER_FAMILIES
                .iter()
                .any(|family| id.starts_with(family))
        })
        .collect();
    ids.sort();
    ids
}

/// The mail ids of one inbox, oldest first, through Swarmail's REST list.
async fn inbox_ids(http: SocketAddr, inbox: &str) -> Vec<String> {
    let resp = reqwest::get(format!("http://{http}/api/v1/inboxes/{inbox}/messages"))
        .await
        .unwrap();
    let body: Value = serde_json::from_str(&resp.text().await.unwrap()).unwrap();
    let mut mails: Vec<(i64, String)> = body["emails"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["received_ms"].as_i64().unwrap(),
                e["id"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    mails.sort();
    mails.into_iter().map(|(_, id)| id).collect()
}

impl Rig {
    /// The deliverability report of one stored mail, from Swarmail's REST.
    async fn report(&self, id: &str) -> Value {
        let body = reqwest::get(format!(
            "http://{}/api/v1/messages/{id}/deliverability",
            self.http
        ))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
        serde_json::from_str(&body).unwrap()
    }

    /// Every mail id the agent's inbox holds, oldest first.
    async fn mail_ids(&self) -> Vec<String> {
        inbox_ids(self.http, self.inbox(0)).await
    }

    async fn post_raw(&self, who: &str, body: String) -> (u16, Value) {
        let resp = reqwest::Client::new()
            .post(format!("{}/newsletter", self.app_url))
            .header(IDENTITY_HEADER, who)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .unwrap();
        let status = resp.status().as_u16();
        let bytes = resp.bytes().await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn post_newsletter(&self, subject: &str, html: &str) -> (u16, Value) {
        let who = self.ids[0].0.clone();
        self.post_raw(&who, json!({"subject": subject, "html": html}).to_string())
            .await
    }

    /// The events recorded so far, as `(action, accepted, category)`.
    async fn events(&self) -> Vec<(String, bool, Option<String>)> {
        self.snapshot()
            .await
            .events
            .iter()
            .map(|e| {
                (
                    e.action.clone(),
                    e.accepted,
                    e.category.map(|c| {
                        serde_json::to_value(c)
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .to_string()
                    }),
                )
            })
            .collect()
    }
}

#[tokio::test]
async fn newsletter_prepare_carries_exactly_the_three_findings() {
    let rig = Rig::new("newsletter-findings", 1).await;
    let prepared = rig.prepare().await;
    assert_eq!(prepared["deliveries"].as_array().unwrap().len(), 1);
    let ids = rig.mail_ids().await;
    assert_eq!(ids.len(), 1);
    assert_eq!(
        newsletter_report_ids(&rig.report(&ids[0]).await),
        tasks::NEWSLETTER_FINDINGS
    );
    let snap = rig.snapshot().await;
    assert_eq!(snap.deliveries.len(), 1);
    assert_eq!(snap.deliveries[0].message, "newsletter");
    assert_eq!(
        serde_json::to_value(snap.deliveries[0].flow).unwrap(),
        "marketing"
    );
    assert_eq!(
        serde_json::to_value(snap.deliveries[0].status).unwrap(),
        "delivered"
    );
    assert!(snap.events.is_empty(), "prepare records no event");
    // GET /newsletter serves the template the mail was rendered from.
    let (s, template) = rig.app(0).get("/newsletter").await.unwrap();
    assert_eq!(s, 200);
    assert_eq!(template["subject"], tasks::NEWSLETTER_SUBJECT);
    assert_eq!(template["html"], tasks::NEWSLETTER_HTML);
    rig.stop().await;
}

#[tokio::test]
async fn newsletter_prepare_without_a_readable_report_is_not_200() {
    // No address at all: nothing is sent.
    let rig = Rig::with_report_source(
        "newsletter-findings",
        1,
        Default::default(),
        ReportSource::Absent,
    )
    .await;
    let (s, body) = rig.admin_post("/admin/prepare", None).await;
    assert_ne!(s, 200, "{body}");
    assert!(rig.snapshot().await.deliveries.is_empty());
    rig.stop().await;
    // An address nothing listens on: the report cannot be read.
    let rig = Rig::with_report_source(
        "newsletter-findings",
        1,
        Default::default(),
        ReportSource::Closed,
    )
    .await;
    let (s, body) = rig.admin_post("/admin/prepare", None).await;
    assert_ne!(s, 200, "{body}");
    assert!(rig.snapshot().await.events.is_empty());
    rig.stop().await;
}

#[tokio::test]
async fn the_newsletter_routes_exist_only_in_the_newsletter_scenario() {
    // Every other task's application is byte-identical to `main`'s.
    for task in tasks::catalogue() {
        let running = fixture::start(FixtureConfig {
            smtp: "127.0.0.1:1".parse().unwrap(),
            smtp_auth: swarmail_evals::mail::SmtpAuth::Plain,
            swarmail_http: None,
            task: task.clone(),
            trial_id: "index".into(),
            admin_secret: SECRET.into(),
            clock: Arc::new(Instant::now),
            inputs: Default::default(),
        })
        .await
        .unwrap();
        let base = format!("http://{}", running.public);
        let client = reqwest::Client::new();
        let index = client.get(format!("{base}/")).send().await.unwrap();
        let index = index.text().await.unwrap();
        let ours = task.id == "newsletter-findings";
        let main = MAIN_INDEX.replace("{base}", &base);
        if ours {
            assert_ne!(index, main);
            assert_eq!(index.matches(r#""path":"/newsletter""#).count(), 2);
        } else {
            assert_eq!(index, main, "{}", task.id);
        }
        for method in [reqwest::Method::GET, reqwest::Method::POST] {
            let status = client
                .request(method, format!("{base}/newsletter"))
                .header(IDENTITY_HEADER, "nobody")
                .send()
                .await
                .unwrap()
                .status();
            // Mounted: the identity claim answers (400 unknown identity).
            assert_eq!(status == 404, !ours, "{}: {status}", task.id);
        }
        running.stop().await;
    }
}

#[tokio::test]
async fn newsletter_refusals_send_nothing_and_record_one_unaccepted_event() {
    let rig = Rig::new("newsletter-findings", 1).await;
    rig.prepare().await;
    let who = rig.ids[0].0.clone();
    let ok_html = "<p>hi</p><a href=\"https://shop.example/autumn\">x</a>".to_string();
    let raw = |v: Value| v.to_string();
    let with_subject = |subject: &str| raw(json!({"subject": subject, "html": ok_html}));
    let with_html = |html: String| raw(json!({"subject": "s", "html": html}));
    let cases: Vec<(&str, String)> = vec![
        ("not json", "not json".to_string()),
        ("an array", "[]".to_string()),
        ("a missing key", raw(json!({"subject": "s"}))),
        (
            "an unknown key",
            raw(json!({"subject": "s", "html": ok_html, "extra": 1})),
        ),
        ("a non-string", raw(json!({"subject": 1, "html": ok_html}))),
        ("a long subject", with_subject(&"s".repeat(201))),
        ("a CRLF subject", with_subject("a\r\nBcc: x@y.test")),
        ("an LF subject", with_subject("a\nb")),
        ("a tab subject", with_subject("a\tb")),
        ("a DEL subject", with_subject("a\u{7f}b")),
        ("a NEL subject", with_subject("a\u{85}b")),
        ("a line separator subject", with_subject("a\u{2028}b")),
        ("a paragraph separator subject", with_subject("a\u{2029}b")),
        (
            "long html",
            with_html(format!("{ok_html}{}", "x".repeat(16384))),
        ),
        ("html with NUL", with_html(format!("{ok_html}\u{0}"))),
        ("html with CR", with_html(format!("{ok_html}\r\n"))),
        ("html without the offer", with_html("<p>hi</p>".to_string())),
        (
            "html with a different anchor",
            with_html("<a href=\"https://other.example/autumn\">x</a>".to_string()),
        ),
    ];
    let mut expected_events = 0;
    for (name, body) in cases {
        let (status, reply) = rig.post_raw(&who, body).await;
        assert_eq!(status, 400, "{name}: {reply}");
        assert_eq!(reply["ok"], false, "{name}");
        expected_events += 1;
        let events = rig.events().await;
        assert_eq!(events.len(), expected_events, "{name}");
        assert_eq!(
            events.last().unwrap(),
            &("marketing".to_string(), false, None),
            "{name}"
        );
        let snap = rig.snapshot().await;
        assert_eq!(snap.deliveries.len(), 1, "{name}: nothing was enqueued");
        assert_eq!(rig.mail_ids().await.len(), 1, "{name}: nothing was sent");
    }
    // The template is untouched by every refusal.
    let (_, template) = rig.app(0).get("/newsletter").await.unwrap();
    assert_eq!(template["html"], tasks::NEWSLETTER_HTML);
    // A bound is a bound, not a refusal one byte early: 200 and 16384 pass.
    let html = format!("{ok_html}{}", "x".repeat(16384 - ok_html.len()));
    assert_eq!(html.len(), 16384);
    let (status, _) = rig.post_newsletter(&"s".repeat(200), &html).await;
    assert_eq!(status, 200);
    assert_eq!(rig.mail_ids().await.len(), 2);
    // An unknown identity never reaches the template or the wire.
    let (status, _) = rig.post_raw("nobody", with_subject("s")).await;
    assert_eq!(status, 400);
    assert_eq!(rig.mail_ids().await.len(), 2);
    rig.stop().await;
}

#[tokio::test]
async fn the_seventeenth_resend_is_refused_and_the_prepared_mail_does_not_count() {
    let rig = Rig::new("newsletter-findings", 1).await;
    rig.prepare().await;
    for n in 0..tasks::NEWSLETTER_MAX_SENDS {
        let (status, body) = rig
            .post_newsletter(tasks::NEWSLETTER_SUBJECT, tasks::NEWSLETTER_HTML)
            .await;
        assert_eq!(status, 200, "re-send {}: {body}", n + 1);
    }
    let (status, body) = rig
        .post_newsletter(tasks::NEWSLETTER_SUBJECT, tasks::NEWSLETTER_HTML)
        .await;
    assert_eq!(status, 429, "{body}");
    let snap = rig.snapshot().await;
    assert_eq!(snap.deliveries.len(), 1 + tasks::NEWSLETTER_MAX_SENDS);
    assert_eq!(rig.mail_ids().await.len(), 1 + tasks::NEWSLETTER_MAX_SENDS);
    // Every unchanged re-send recorded an unaccepted event, and so did the
    // refusal; the fact never turned.
    let events = rig.events().await;
    assert_eq!(events.len(), tasks::NEWSLETTER_MAX_SENDS + 1);
    assert!(
        events
            .iter()
            .all(|e| *e == ("marketing".to_string(), false, None))
    );
    assert!(!snap.identities[0].facts.marketing_completed);
    rig.stop().await;
}

#[tokio::test]
async fn a_repaired_resend_completes_and_the_fact_never_reverts() {
    let rig = Rig::new("newsletter-findings", 1).await;
    rig.prepare().await;
    // Unchanged: the report still has the three findings.
    let (status, body) = rig
        .post_newsletter(tasks::NEWSLETTER_SUBJECT, tasks::NEWSLETTER_HTML)
        .await;
    assert_eq!(status, 200, "{body}");
    assert!(!rig.snapshot().await.identities[0].facts.marketing_completed);
    // The reply never carries a finding or a verdict.
    assert_eq!(body, json!({"ok": true, "message": "newsletter sent"}));
    // Repaired.
    let html = tasks::NEWSLETTER_HTML
        .replace("http://shop.example", "https://shop.example")
        .replace(
            "<img src=\"https://shop.example/lamp.png\">",
            "<img src=\"https://shop.example/lamp.png\" alt=\"Autumn Lamp\">",
        );
    let (status, body) = rig.post_newsletter("Autumn lamp offer", &html).await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body, json!({"ok": true, "message": "newsletter sent"}));
    let snap = rig.snapshot().await;
    assert!(snap.identities[0].facts.marketing_completed);
    assert_eq!(snap.events.iter().filter(|e| e.accepted).count(), 1);
    let ids = rig.mail_ids().await;
    assert_eq!(ids.len(), 3);
    assert!(newsletter_report_ids(&rig.report(&ids[2]).await).is_empty());
    assert_eq!(
        newsletter_report_ids(&rig.report(&ids[1]).await),
        tasks::NEWSLETTER_FINDINGS
    );
    // A later unchanged re-send does not undo the completed fact.
    let (status, _) = rig
        .post_newsletter(tasks::NEWSLETTER_SUBJECT, tasks::NEWSLETTER_HTML)
        .await;
    assert_eq!(status, 200);
    let snap = rig.snapshot().await;
    assert!(snap.identities[0].facts.marketing_completed);
    assert_eq!(snap.events.iter().filter(|e| e.accepted).count(), 1);
    assert_eq!(snap.events.len(), 3);
    rig.stop().await;
}

#[tokio::test]
async fn a_cleared_inbox_cannot_make_a_resend_clean() {
    let rig = Rig::new("newsletter-findings", 1).await;
    rig.prepare().await;
    rig.mcp()
        .await
        .call("swarmail_clear_inbox", json!({"inbox": rig.inbox(0)}))
        .await
        .unwrap();
    assert!(rig.mail_ids().await.is_empty());
    let (status, _) = rig
        .post_newsletter(tasks::NEWSLETTER_SUBJECT, tasks::NEWSLETTER_HTML)
        .await;
    assert_eq!(status, 200);
    assert_eq!(rig.mail_ids().await.len(), 1);
    assert!(!rig.snapshot().await.identities[0].facts.marketing_completed);
    rig.stop().await;
}

#[tokio::test]
async fn a_frozen_newsletter_trial_answers_409_and_records_nothing() {
    let rig = Rig::new("newsletter-findings", 1).await;
    rig.prepare().await;
    let (s, _) = rig.admin_post("/admin/freeze", None).await;
    assert_eq!(s, 200);
    let (status, _) = rig
        .post_newsletter(tasks::NEWSLETTER_SUBJECT, tasks::NEWSLETTER_HTML)
        .await;
    assert_eq!(status, 409);
    let (status, _) = rig.app(0).get("/newsletter").await.unwrap();
    assert_eq!(status, 409);
    assert!(rig.snapshot().await.events.is_empty());
    assert_eq!(rig.mail_ids().await.len(), 1);
    rig.stop().await;
}
