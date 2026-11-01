//! Recipient inbox routing (decision 0112) and `swarmail_create_inbox`,
//! against real servers over real SMTP and real MCP — no mocks. Three agents
//! behind one SMTP AUTH identity stay apart because each is given its own
//! address; the rule, its refusals, its precedence over the park and the
//! minting tool's shape are each proved on the wire.

mod common;

use common::{
    SmtpConn, http_json, http_send_text_with_headers, list_all_ids, start, start_with,
    wait_for_metric_row,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

type Http = std::net::SocketAddr;

const DOMAIN: &str = "swarmail.test";

async fn routed() -> swarmail::RunningServer {
    start_with(|mut c| {
        c.smtp.route_domain = Some(DOMAIN.to_string());
        c
    })
    .await
}

/// One tool call over POST /mcp at the March revision (no header).
async fn tool(addr: Http, name: &str, arguments: Value) -> Value {
    let body = json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":name,"arguments":arguments}});
    let (status, reply) = http_json(addr, "POST", "/mcp", Some(&body.to_string())).await;
    assert_eq!(status, 200, "{name}: {reply}");
    reply
}

fn text(reply: &Value) -> Value {
    serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

struct Minted {
    inbox: String,
    address: String,
}

async fn mint(addr: Http) -> Minted {
    let reply = tool(addr, "swarmail_create_inbox", json!({})).await;
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    let value = text(&reply);
    Minted {
        inbox: value["inbox"].as_str().unwrap().to_string(),
        address: value["address"].as_str().unwrap().to_string(),
    }
}

/// An SMTP session after EHLO, authenticated as `auth` when given.
async fn session(addr: Http, auth: Option<&str>) -> SmtpConn {
    let mut conn = SmtpConn::connect(addr).await;
    conn.send("EHLO routing.test").await;
    conn.reply_multi().await;
    if let Some(user) = auth {
        conn.send(&format!(
            "AUTH PLAIN {}",
            base64_of(&format!("\0{user}\0whatever"))
        ))
        .await;
        let reply = conn.reply().await;
        assert!(reply.starts_with("235"), "{reply}");
    }
    conn
}

fn base64_of(s: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(s)
}

async fn cmd(conn: &mut SmtpConn, line: &str) -> String {
    conn.send(line).await;
    conn.reply().await
}

/// A command whose reply must be a `250`.
async fn ok(conn: &mut SmtpConn, line: &str) {
    let reply = cmd(conn, line).await;
    assert!(reply.starts_with("250"), "{line}: {reply}");
}

/// One whole transaction; the id the `250` names.
async fn deliver(addr: Http, auth: Option<&str>, rcpts: &[&str], subject: &str) -> String {
    let mut conn = session(addr, auth).await;
    ok(&mut conn, "MAIL FROM:<app@example.test>").await;
    for rcpt in rcpts {
        ok(&mut conn, &format!("RCPT TO:<{rcpt}>")).await;
    }
    let reply = conn.data(&format!("Subject: {subject}\r\n\r\nbody")).await;
    stored_id(&reply)
}

fn stored_id(reply: &str) -> String {
    reply
        .strip_prefix("250 2.0.0 OK: stored as ")
        .unwrap_or_else(|| panic!("not a stored reply: {reply}"))
        .to_string()
}

async fn inbox_names(addr: Http) -> BTreeSet<String> {
    let (status, list) = http_json(addr, "GET", "/api/v1/inboxes", None).await;
    assert_eq!(status, 200);
    list.as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap().to_string())
        .collect()
}

async fn ids(addr: Http, inbox: &str) -> BTreeSet<String> {
    list_all_ids(addr, inbox).await.into_iter().collect()
}

async fn recipients(addr: Http, inbox: &str) -> Value {
    let (_, list) = http_json(
        addr,
        "GET",
        &format!("/api/v1/inboxes/{inbox}/messages"),
        None,
    )
    .await;
    list["emails"][0]["recipients"].clone()
}

// ---------- AC1: three agents, one AUTH identity, one Swarmail ----------

#[tokio::test]
async fn three_parallel_agents_behind_one_auth_identity_never_see_each_others_mail() {
    let srv = routed().await;
    let addr = srv.http_addr;
    let smtp = srv.smtp_addr;
    let agents = [mint(addr).await, mint(addr).await, mint(addr).await];
    assert_eq!(
        agents
            .iter()
            .map(|a| a.inbox.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        3,
        "three distinct inboxes"
    );

    // Three concurrent batches, one AUTH username for all of them.
    let mut batches = Vec::new();
    for (n, agent) in agents.iter().enumerate() {
        let address = agent.address.clone();
        batches.push(tokio::spawn(async move {
            let mut sent = BTreeSet::new();
            for m in 0..4 {
                let subject = format!("agent {n} mail {m}");
                sent.insert(deliver(smtp, Some("fixture-app"), &[&address], &subject).await);
            }
            sent
        }));
    }
    let mut sent = Vec::new();
    for batch in batches {
        sent.push(batch.await.unwrap());
    }

    for (agent, sent) in agents.iter().zip(&sent) {
        // The REST read, and the MCP search an agent actually uses.
        assert_eq!(&ids(addr, &agent.inbox).await, sent, "{}", agent.inbox);
        let found = text(
            &tool(
                addr,
                "swarmail_search_emails",
                json!({"inbox": agent.inbox}),
            )
            .await,
        );
        let searched: BTreeSet<String> = found["emails"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["id"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(&searched, sent, "{}: search sees only its own", agent.inbox);
        assert_eq!(found["total"], 4);
    }

    // The AUTH identity's inbox was never created, and exactly the three
    // minted inboxes exist.
    assert_eq!(
        inbox_names(addr).await,
        agents.iter().map(|a| a.inbox.clone()).collect(),
        "the shared AUTH name owns no inbox"
    );
    assert!(ids(addr, "fixture-app").await.is_empty());
    srv.stop().await;
}

// ---------- AC3: default-off ----------

#[tokio::test]
async fn an_unconfigured_server_routes_by_auth_and_default_as_before() {
    let srv = start().await;
    let addr = srv.http_addr;
    // A recipient shaped like a route address has no vote without the flag.
    let authed = deliver(
        srv.smtp_addr,
        Some("auth-box"),
        &["x@ada.swarmail.test"],
        "authed",
    )
    .await;
    let anonymous = deliver(srv.smtp_addr, None, &["x@ada.swarmail.test"], "anon").await;
    assert_eq!(ids(addr, "auth-box").await, [authed].into());
    assert_eq!(ids(addr, "default").await, [anonymous].into());
    assert!(ids(addr, "ada").await.is_empty());

    // The tool refuses, naming the flag, and nothing else changes.
    let reply = tool(addr, "swarmail_create_inbox", json!({})).await;
    assert_eq!(reply["result"]["isError"], true, "{reply}");
    let message = reply["result"]["content"][0]["text"].as_str().unwrap();
    assert!(message.contains("--route-domain"), "{message}");
    assert!(reply["result"].get("structuredContent").is_none());
    assert_eq!(inbox_names(addr).await.len(), 2);
    srv.stop().await;
}

// ---------- AC4: the rule on the wire ----------

#[tokio::test]
async fn the_recipient_outranks_the_auth_identity_and_folds_case() {
    let srv = routed().await;
    let addr = srv.http_addr;
    let id = deliver(
        srv.smtp_addr,
        Some("fixture-app"),
        &["Agent@ADA.Swarmail.TEST"],
        "fold",
    )
    .await;
    assert_eq!(ids(addr, "ada").await, [id.clone()].into());
    assert!(ids(addr, "fixture-app").await.is_empty());
    // The local part is free: a second address of the same inbox.
    let second = deliver(
        srv.smtp_addr,
        None,
        &["other.name@ada.swarmail.test"],
        "two",
    )
    .await;
    assert_eq!(ids(addr, "ada").await, [id, second].into());
    srv.stop().await;
}

#[tokio::test]
async fn hosts_outside_the_domain_have_no_vote_and_are_recorded() {
    let srv = routed().await;
    let addr = srv.http_addr;
    // Whole-label comparison: neither a longer name, a lookalike suffix nor
    // a sibling domain is under swarmail.test; the AUTH inbox takes them.
    let outside = [
        "x@notswarmail.test",
        "x@swarmail.test.example",
        "x@ada.swarmail.testing",
        "no-at-sign",
        "x@example.test",
    ];
    for (n, rcpt) in outside.into_iter().enumerate() {
        let id = deliver(
            srv.smtp_addr,
            Some("fallback"),
            &[rcpt],
            &format!("out {n}"),
        )
        .await;
        assert!(ids(addr, "fallback").await.contains(&id), "{rcpt}");
    }
    assert_eq!(ids(addr, "fallback").await.len(), 5);
    assert_eq!(inbox_names(addr).await, ["fallback".to_string()].into());

    // A non-matching CC does not disturb routing and is still recorded.
    let all = [
        "cc@example.test",
        "to@bee.swarmail.test",
        "cc2@elsewhere.test",
    ];
    let id = deliver(srv.smtp_addr, Some("fallback"), &all, "cc").await;
    assert_eq!(ids(addr, "bee").await, [id].into());
    assert_eq!(recipients(addr, "bee").await, json!(all));
    assert_eq!(ids(addr, "fallback").await.len(), 5);
    srv.stop().await;
}

#[tokio::test]
async fn a_recipient_under_the_domain_that_names_no_inbox_is_refused() {
    let srv = routed().await;
    let addr = srv.http_addr;
    let too_long = format!("x@{}.swarmail.test", "a".repeat(64));
    let mut conn = session(srv.smtp_addr, Some("fixture-app")).await;
    ok(&mut conn, "MAIL FROM:<app@example.test>").await;
    for rcpt in [
        "agent@swarmail.test",
        "agent@SWARMAIL.TEST",
        "a@x.y.swarmail.test",
        "a@bad_label.swarmail.test",
        "a@.swarmail.test",
        "a@sp\u{e4}ce.swarmail.test",
        too_long.as_str(),
    ] {
        assert_eq!(
            cmd(&mut conn, &format!("RCPT TO:<{rcpt}>")).await,
            "550 5.1.3 Recipient names no inbox under the route domain",
            "{rcpt}"
        );
    }
    // The refusals appended nothing: the transaction is still open and
    // empty, and a DATA without a recipient says so.
    assert_eq!(
        cmd(&mut conn, "DATA").await,
        "503 5.5.1 Error: need RCPT command"
    );
    // A 63-character label is the longest admitted.
    let longest = "b".repeat(63);
    ok(&mut conn, &format!("RCPT TO:<x@{longest}.swarmail.test>")).await;
    let id = stored_id(&conn.data("Subject: ok\r\n\r\nbody").await);
    assert_eq!(ids(addr, &longest).await, [id].into());
    assert!(ids(addr, "fixture-app").await.is_empty(), "no fallback");
    srv.stop().await;
}

#[tokio::test]
async fn a_second_routed_inbox_defers_and_retries_as_its_own_message() {
    let srv = routed().await;
    let addr = srv.http_addr;
    let mut conn = session(srv.smtp_addr, Some("fixture-app")).await;
    ok(&mut conn, "MAIL FROM:<app@example.test>").await;
    ok(&mut conn, "RCPT TO:<a@ada.swarmail.test>").await;
    // The same routed inbox again, and a recipient with no vote, are fine.
    ok(&mut conn, "RCPT TO:<b@ADA.swarmail.test>").await;
    ok(&mut conn, "RCPT TO:<c@example.test>").await;
    // A different routed inbox is deferred and not appended; the
    // transaction stays open.
    assert_eq!(
        cmd(&mut conn, "RCPT TO:<d@bob.swarmail.test>").await,
        "452 4.5.3 One routed inbox per transaction"
    );
    let first = stored_id(&conn.data("Subject: first\r\n\r\nbody").await);
    // The retry in a fresh transaction lands in the other inbox as its
    // own message.
    ok(&mut conn, "MAIL FROM:<app@example.test>").await;
    ok(&mut conn, "RCPT TO:<d@bob.swarmail.test>").await;
    let second = stored_id(&conn.data("Subject: second\r\n\r\nbody").await);
    assert_eq!(ids(addr, "ada").await, [first].into());
    assert_eq!(ids(addr, "bob").await, [second].into());
    // The deferred recipient was never recorded on the first message.
    assert_eq!(
        recipients(addr, "ada").await,
        json!([
            "a@ada.swarmail.test",
            "b@ADA.swarmail.test",
            "c@example.test"
        ])
    );
    assert!(ids(addr, "fixture-app").await.is_empty());
    srv.stop().await;
}

#[tokio::test]
async fn rset_and_a_failed_rcpt_leave_the_route_unset() {
    let srv = routed().await;
    let addr = srv.http_addr;
    let mut conn = session(srv.smtp_addr, Some("fixture-app")).await;
    ok(&mut conn, "MAIL FROM:<app@example.test>").await;
    ok(&mut conn, "RCPT TO:<a@ada.swarmail.test>").await;
    // RSET clears the route with the rest of the envelope.
    ok(&mut conn, "RSET").await;
    ok(&mut conn, "MAIL FROM:<app@example.test>").await;
    ok(&mut conn, "RCPT TO:<d@bob.swarmail.test>").await;
    let id = stored_id(&conn.data("Subject: after rset\r\n\r\nbody").await);
    assert_eq!(ids(addr, "bob").await, [id].into());

    // A chaos-failed RCPT does not fix the route (0034 §3): the next
    // routed inbox is still free to be the first.
    tool(
        addr,
        "swarmail_set_chaos",
        json!({"event":"rcpt","probability":100}),
    )
    .await;
    ok(&mut conn, "MAIL FROM:<app@example.test>").await;
    let failed = cmd(&mut conn, "RCPT TO:<a@ada.swarmail.test>").await;
    assert!(failed.starts_with("451"), "{failed}");
    tool(addr, "swarmail_clear_chaos", json!({})).await;
    ok(&mut conn, "RCPT TO:<e@eve.swarmail.test>").await;
    let id = stored_id(&conn.data("Subject: after chaos\r\n\r\nbody").await);
    assert_eq!(ids(addr, "eve").await, [id].into());
    assert!(ids(addr, "ada").await.is_empty());
    assert!(ids(addr, "fixture-app").await.is_empty());
    srv.stop().await;
}

// ---------- AC5: routing precedes the park ----------

#[tokio::test]
async fn a_wait_on_a_minted_never_mailed_inbox_registered_first_gets_the_mail() {
    let srv = routed().await;
    let addr = srv.http_addr;
    let agent = mint(addr).await;
    // The inbox does not exist yet (0032).
    assert!(inbox_names(addr).await.is_empty());

    let waiter = tokio::spawn({
        let inbox = agent.inbox.clone();
        async move {
            let wait = json!({"inbox": inbox, "timeout_ms": 20000});
            tool(addr, "swarmail_wait_for_email", wait).await
        }
    });
    // The server's own readiness row, not a sleep: the watcher is registered.
    wait_for_metric_row(
        addr,
        &format!("swarmail_awaits_active{{inbox=\"{}\"}}", agent.inbox),
        1,
    )
    .await;
    let id = deliver(
        srv.smtp_addr,
        Some("fixture-app"),
        &[&agent.address],
        "late",
    )
    .await;
    let reply = waiter.await.unwrap();
    let found = text(&reply);
    assert_eq!(found["emails"][0]["id"], id.as_str(), "{reply}");
    assert_eq!(found["emails"][0]["inbox"], agent.inbox.as_str());
    srv.stop().await;
}

#[tokio::test]
async fn the_in_flight_delivery_names_the_routed_inbox_not_the_auth_identity() {
    let dir = tempfile::TempDir::with_prefix("swarmail-routing-park-").unwrap();
    let db = dir.path().join("park.db");
    let srv = start_with(|mut c| {
        c.data_file = Some(db.clone());
        c.smtp.route_domain = Some(DOMAIN.to_string());
        c
    })
    .await;
    let addr = srv.http_addr;
    let agent = mint(addr).await;
    // A real second connection holds the writer lock, so the delivery sits
    // between its DATA terminator and its commit while the row is read.
    let lock = common::hold_write_lock(&db);
    let delivery = tokio::spawn({
        let smtp = srv.smtp_addr;
        let address = agent.address.clone();
        async move { deliver(smtp, Some("held-identity"), &[&address], "held").await }
    });
    wait_for_metric_row(
        addr,
        &format!("swarmail_deliveries_in_flight{{inbox=\"{}\"}}", agent.inbox),
        1,
    )
    .await;
    let (_, metrics) = common::http_get_text(addr, "/metrics").await;
    assert!(
        !metrics.contains("swarmail_deliveries_in_flight{inbox=\"held-identity\"}"),
        "the AUTH identity owns no in-flight row:\n{metrics}"
    );
    drop(lock);
    let id = delivery.await.unwrap();
    assert_eq!(ids(addr, &agent.inbox).await, [id].into());
    assert!(ids(addr, "held-identity").await.is_empty());
    srv.stop().await;
}

// ---------- AC6: the tool's shape ----------

async fn mcp_at(addr: Http, version: &str, request: &Value) -> Value {
    let (status, raw) = http_send_text_with_headers(
        addr,
        "POST",
        "/mcp",
        &[("MCP-Protocol-Version", version)],
        Some(&request.to_string()),
    )
    .await;
    assert_eq!(status, 200, "{raw}");
    serde_json::from_str(&raw).unwrap()
}

#[tokio::test]
async fn the_tool_is_appended_after_the_focused_waits_with_its_modern_projection() {
    let srv = routed().await;
    for version in ["2025-03-26", "2025-06-18", "2025-11-25"] {
        let list = json!({"jsonrpc":"2.0","id":1,"method":"tools/list"});
        let reply = mcp_at(srv.http_addr, version, &list).await;
        let tools = reply["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 18, "{version}");
        assert_eq!(tools[14]["name"], "swarmail_wait_for_link", "{version}");
        let tool = &tools[15];
        assert_eq!(tool["name"], "swarmail_create_inbox");
        // Decision 0110 appends the two attachment tools after it.
        assert_eq!(tools[16]["name"], "swarmail_list_attachments", "{version}");
        assert_eq!(tools[17]["name"], "swarmail_get_attachment", "{version}");
        assert_eq!(
            tool["inputSchema"],
            json!({"type":"object","additionalProperties":false})
        );
        if version == "2025-03-26" {
            assert!(tool.get("outputSchema").is_none(), "March has none");
            assert!(tool.get("annotations").is_none());
        } else {
            assert_eq!(
                tool["outputSchema"],
                json!({"type":"object","properties":{"inbox":{"type":"string"},
                    "address":{"type":"string"}},"required":["inbox","address"],
                    "additionalProperties":false})
            );
            assert_eq!(tool["annotations"], json!({"readOnlyHint": true}));
        }
    }
    srv.stop().await;
}

#[tokio::test]
async fn the_tool_answers_structured_content_on_modern_revisions() {
    let srv = routed().await;
    let call = json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"swarmail_create_inbox","arguments":{}}});
    for (version, structured) in [
        ("2025-03-26", false),
        ("2025-06-18", true),
        ("2025-11-25", true),
    ] {
        let reply = mcp_at(srv.http_addr, version, &call).await;
        let minted = text(&reply);
        assert_eq!(reply["result"]["isError"], false);
        assert_eq!(
            reply["result"].get("structuredContent").is_some(),
            structured
        );
        if structured {
            assert_eq!(reply["result"]["structuredContent"], minted);
        }
        let inbox = minted["inbox"].as_str().unwrap();
        assert_eq!(
            minted["address"],
            format!("agent@{inbox}.{DOMAIN}").as_str()
        );
    }
    srv.stop().await;
}

#[tokio::test]
async fn a_sentinel_argument_refuses_before_any_effect() {
    let srv = routed().await;
    let reply = tool(
        srv.http_addr,
        "swarmail_create_inbox",
        json!({"sentinel": 1}),
    )
    .await;
    assert_eq!(reply["error"]["code"], -32602, "{reply}");
    assert_eq!(reply["error"]["message"], "unknown argument: sentinel");
    srv.stop().await;
}

#[tokio::test]
async fn concurrent_calls_mint_distinct_names_inside_the_routing_alphabet() {
    let srv = routed().await;
    let addr = srv.http_addr;
    let calls: Vec<_> = (0..24).map(|_| tokio::spawn(mint(addr))).collect();
    let mut inboxes = BTreeSet::new();
    let mut addresses = BTreeSet::new();
    for call in calls {
        let minted = call.await.unwrap();
        swarmail::model::validate_inbox(&minted.inbox).expect("passes the one inbox rule");
        assert_eq!(minted.inbox.len(), 32);
        assert!(
            minted
                .inbox
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        );
        assert_eq!(minted.address, format!("agent@{}.{DOMAIN}", minted.inbox));
        inboxes.insert(minted.inbox);
        addresses.insert(minted.address);
    }
    assert_eq!(inboxes.len(), 24);
    assert_eq!(addresses.len(), 24);
    // Minting allocates nothing (0032).
    assert!(inbox_names(addr).await.is_empty());
    srv.stop().await;
}

// ---------- Routing beside the attachment tools (decisions 0110, 0112) ----------

/// One transaction carrying a text body and one attachment; the stored id.
async fn deliver_attached(
    addr: Http,
    auth: Option<&str>,
    rcpt: &str,
    filename: &str,
    content: &str,
) -> String {
    let mut conn = session(addr, auth).await;
    ok(&mut conn, "MAIL FROM:<app@example.test>").await;
    ok(&mut conn, &format!("RCPT TO:<{rcpt}>")).await;
    let reply = conn
        .data(&format!(
            "Subject: attached\r\nMIME-Version: 1.0\r\n\
             Content-Type: multipart/mixed; boundary=\"bd\"\r\n\r\n\
             --bd\r\nContent-Type: text/plain\r\n\r\nsee attached\r\n\
             --bd\r\nContent-Type: text/plain\r\n\
             Content-Disposition: attachment; filename=\"{filename}\"\r\n\r\n\
             {content}\r\n--bd--"
        ))
        .await;
    stored_id(&reply)
}

/// A mail routed by recipient to a minted inbox keeps its attachments: the
/// two tools list and fetch them by the id the routed inbox shows, and
/// clearing the AUTH identity's inbox (which holds a mail of its own)
/// leaves them in place. Clearing the minted inbox takes them with it.
#[tokio::test]
async fn attachments_of_a_routed_mail_are_listed_and_fetched_from_the_minted_inbox() {
    let srv = routed().await;
    let addr = srv.http_addr;
    let smtp = srv.smtp_addr;
    let agent = mint(addr).await;
    let routed_id =
        deliver_attached(smtp, Some("fixture-app"), &agent.address, "a.txt", "abc").await;
    // The same AUTH identity, a recipient outside the domain: no vote, so
    // this one lands in the AUTH inbox.
    let auth_id = deliver_attached(
        smtp,
        Some("fixture-app"),
        "ops@elsewhere.test",
        "b.txt",
        "other",
    )
    .await;

    assert_eq!(
        ids(addr, &agent.inbox).await,
        BTreeSet::from([routed_id.clone()])
    );
    assert_eq!(
        ids(addr, "fixture-app").await,
        BTreeSet::from([auth_id.clone()])
    );

    let expect_routed = |listed: &Value| {
        assert_eq!(listed["id"], routed_id.as_str());
        assert_eq!(listed["total"], 1);
        assert_eq!(listed["truncated"], false);
        let entry = &listed["attachments"][0];
        assert_eq!(entry["index"], 0);
        assert_eq!(entry["filename"], "a.txt");
        assert_eq!(entry["content_type"], "text/plain");
        assert_eq!(entry["size"], 3);
        assert_eq!(entry["stored"], true);
        assert_eq!(
            entry["sha256"],
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    };
    let list = |id: &str| tool(addr, "swarmail_list_attachments", json!({ "id": id }));
    let get = |id: &str| {
        tool(
            addr,
            "swarmail_get_attachment",
            json!({ "id": id, "index": 0 }),
        )
    };

    let listed = list(&routed_id).await;
    assert_eq!(listed["result"]["isError"], false, "{listed}");
    expect_routed(&text(&listed));
    let fetched = get(&routed_id).await;
    assert_eq!(fetched["result"]["isError"], false, "{fetched}");
    let fetched = text(&fetched);
    assert_eq!(fetched["encoding"], "utf-8");
    assert_eq!(fetched["content"], "abc");
    assert_eq!(fetched["filename"], "a.txt");

    // The id an agent reads from its own inbox is the id the tools take.
    let found = text(
        &tool(
            addr,
            "swarmail_search_emails",
            json!({"inbox": agent.inbox}),
        )
        .await,
    );
    assert_eq!(found["emails"][0]["id"], routed_id.as_str());

    // Clearing the AUTH identity's inbox touches only its own mail.
    let cleared = tool(
        addr,
        "swarmail_clear_inbox",
        json!({"inbox": "fixture-app"}),
    )
    .await;
    assert_eq!(cleared["result"]["isError"], false, "{cleared}");
    let gone = list(&auth_id).await;
    assert_eq!(gone["result"]["isError"], true, "{gone}");
    expect_routed(&text(&list(&routed_id).await));
    assert_eq!(text(&get(&routed_id).await)["content"], "abc");

    // Clearing the minted inbox removes the routed mail and its attachments.
    let cleared = tool(addr, "swarmail_clear_inbox", json!({"inbox": agent.inbox})).await;
    assert_eq!(cleared["result"]["isError"], false, "{cleared}");
    let gone = list(&routed_id).await;
    assert_eq!(gone["result"]["isError"], true, "{gone}");
    assert_eq!(
        gone["result"]["content"][0]["text"],
        format!("message {routed_id} not found")
    );
    srv.stop().await;
}
