//! The focused waits over the real wire (decision 0094): SMTP in, MCP out,
//! against a real server — no mocks. Each filter is proven on each tool by
//! a counterexample whose answer changes when that filter is dropped;
//! deliveries after the park that are not the target never complete it;
//! and local applications act on the returned code or URL, so a stale code
//! or a look-alike link fails the action itself, not only a status field.

mod common;

use common::{SmtpConn, http_json, start, wait_for_metric_row};
use serde_json::{Value, json};
use std::net::SocketAddr;

/// One delivery with its own envelope: AUTH names the inbox.
async fn deliver(addr: SocketAddr, inbox: &str, mail_from: &str, rcpt: &str, raw: &str) -> String {
    let mut conn = SmtpConn::connect(addr).await;
    conn.send("EHLO focused-test").await;
    assert!(conn.reply().await.starts_with("250"));
    let auth = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        format!("\0{inbox}\0x"),
    );
    conn.send(&format!("AUTH PLAIN {auth}")).await;
    assert!(conn.reply().await.starts_with("235"));
    for line in [
        format!("MAIL FROM:<{mail_from}>"),
        format!("RCPT TO:<{rcpt}>"),
    ] {
        conn.send(&line).await;
        assert!(conn.reply().await.starts_with("250"));
    }
    conn.send("DATA").await;
    assert!(conn.reply().await.starts_with("354"));
    conn.send_bytes(raw.replace('\n', "\r\n").as_bytes()).await;
    conn.send(".").await;
    let accepted = conn.reply().await;
    assert!(accepted.starts_with("250"), "{accepted}");
    accepted.split_whitespace().last().unwrap().into()
}

/// A plain mail whose body carries `body`; headers are the caller's.
fn mail(headers: &str, body: &str) -> String {
    format!("{headers}\n\n{body}\n")
}

async fn focused(addr: SocketAddr, tool: &str, args: Value) -> Value {
    let body = json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":format!("swarmail_wait_for_{tool}"),"arguments":args}});
    let (status, reply) = http_json(addr, "POST", "/mcp", Some(&body.to_string())).await;
    assert_eq!(status, 200);
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

/// The answer's decisive value: the selected code or URL, or the status.
fn answer(result: &Value) -> Value {
    match result["status"].as_str().unwrap() {
        "selected" => result
            .get("selected_url")
            .cloned()
            .unwrap_or_else(|| result["selected"]["code"].clone()),
        other => json!(other),
    }
}

/// Each filter on each tool: the target is accepted first, the decoy
/// second, so without the filter the decoy is the newest and wins (codes)
/// or joins the candidates (links). With it, the target alone answers.
#[tokio::test]
async fn every_filter_selects_on_each_focused_tool() {
    let srv = start().await;
    let addr = srv.smtp_addr;
    // (filter, target envelope/headers, decoy envelope/headers)
    let cases = [
        (
            "to",
            "t-env@x.io",
            "hdr@x.io",
            "sender@x.io",
            "Subject: s",
            "d-env@x.io",
            "hdr@x.io",
            "sender@x.io",
            "Subject: s",
        ),
        (
            "to",
            "env@x.io",
            "hdr@x.io",
            "sender@x.io",
            "Cc: t-cc@x.io\nSubject: s",
            "env@x.io",
            "hdr@x.io",
            "sender@x.io",
            "Cc: d-cc@x.io\nSubject: s",
        ),
        (
            "from",
            "r@x.io",
            "r@x.io",
            "t-from@x.io",
            "Subject: s",
            "r@x.io",
            "r@x.io",
            "d-from@x.io",
            "Subject: s",
        ),
        (
            "subject",
            "r@x.io",
            "r@x.io",
            "s@x.io",
            "Subject: Target code",
            "r@x.io",
            "r@x.io",
            "s@x.io",
            "Subject: Decoy code",
        ),
    ];
    for (n, (filter, t_rcpt, t_to, t_from, t_headers, d_rcpt, d_to, d_from, d_headers)) in
        cases.into_iter().enumerate()
    {
        let inbox = format!("filter-{n}");
        let needle = match filter {
            "to" if n == 0 => "t-env@",
            "to" => "t-cc@",
            "from" => "t-from@",
            _ => "target",
        };
        let headers =
            |from: &str, to: &str, extra: &str| format!("From: {from}\nTo: {to}\n{extra}");
        deliver(
            addr,
            &inbox,
            t_from,
            t_rcpt,
            &mail(
                &headers(t_from, t_to, t_headers),
                "code 111111 https://app.test/target",
            ),
        )
        .await;
        deliver(
            addr,
            &inbox,
            d_from,
            d_rcpt,
            &mail(
                &headers(d_from, d_to, d_headers),
                "code 999999 https://app.test/decoy",
            ),
        )
        .await;
        let mut args = json!({"inbox":inbox,"timeout_ms":0});
        let unfiltered = (
            answer(&focused(srv.http_addr, "code", args.clone()).await),
            answer(&focused(srv.http_addr, "link", args.clone()).await),
        );
        assert_eq!(
            unfiltered,
            (json!("999999"), json!("ambiguous")),
            "{filter}"
        );
        args[filter] = json!(needle.to_uppercase());
        let code = focused(srv.http_addr, "code", args.clone()).await;
        let link = focused(srv.http_addr, "link", args.clone()).await;
        assert_eq!(
            (answer(&code), answer(&link)),
            (json!("111111"), json!("https://app.test/target")),
            "{filter} {needle}"
        );
        assert_eq!(
            (
                code["matched"].clone(),
                code["older_candidates_total"].clone()
            ),
            (json!(1), json!(0))
        );
        // A filter matching neither is a timeout on both tools.
        args[filter] = json!("no-such-value");
        assert_eq!(
            answer(&focused(srv.http_addr, "code", args.clone()).await),
            json!("timed_out")
        );
        assert_eq!(
            answer(&focused(srv.http_addr, "link", args).await),
            json!("timed_out")
        );
    }
    // since_ms is an inclusive bound on the stored receive stamp. The decoy
    // is accepted last, with a stamp just below the target's: without the
    // bound it is the newest code and a second destination.
    let target = deliver(
        addr,
        "since",
        "s@x.io",
        "r@x.io",
        &mail(
            "From: s@x.io\nTo: r@x.io\nSubject: s",
            "code 111111 https://app.test/target",
        ),
    )
    .await;
    let stamp = srv.store.get(&target).unwrap().received_ms;
    let mut decoy = (*srv.store.get(&target).unwrap()).clone();
    decoy.id = "since-decoy".into();
    decoy.received_ms = stamp - 1;
    decoy.codes = vec!["999999".into()];
    decoy.links = vec!["https://app.test/decoy".into()];
    decoy.link_metadata = None;
    srv.store.insert(decoy).unwrap();
    for (since, expected) in [
        (None, (json!("999999"), json!("ambiguous"))),
        (Some(stamp - 1), (json!("999999"), json!("ambiguous"))),
        (
            Some(stamp),
            (json!("111111"), json!("https://app.test/target")),
        ),
        (Some(stamp + 1), (json!("timed_out"), json!("timed_out"))),
    ] {
        let mut args = json!({"inbox":"since","timeout_ms":0});
        if let Some(since) = since {
            args["since_ms"] = json!(since);
        }
        let got = (
            answer(&focused(srv.http_addr, "code", args.clone()).await),
            answer(&focused(srv.http_addr, "link", args).await),
        );
        assert_eq!(got, expected, "since_ms {since:?}");
    }
    srv.stop().await;
}

/// Decision 0106, issue #156: a timed-out focused wait names the inbox's
/// near misses. The focused waits skip no mail, so what their filter
/// admitted but carried no code, or no link the selectors leave, shows with
/// an empty `missed`; what the filter refused names the fields it failed;
/// the nearest subject is a mail that missed the subject alone. An inbox
/// that holds nothing answers today's bytes.
#[tokio::test]
async fn focused_timeouts_name_their_near_misses() {
    let srv = start().await;
    let addr = srv.smtp_addr;
    let digest = deliver(
        addr,
        "fnm",
        "s@x.io",
        "dana@x.io",
        &mail(
            "From: s@x.io\nTo: dana@x.io\nSubject: Digest",
            "nothing to see",
        ),
    )
    .await;
    let sibling = deliver(
        addr,
        "fnm",
        "s@x.io",
        "erin@x.io",
        &mail(
            "From: s@x.io\nTo: erin@x.io\nSubject: Your code",
            "code 123456 https://app.test/x",
        ),
    )
    .await;
    let stamp = |id: &str| srv.store.get(id).unwrap().received_ms;
    let entry = |id: &str, to: &str, subject: &str, missed: Value| {
        json!({"email_id":id,"received_ms":stamp(id),"from":"s@x.io","to":[to],
            "to_total":1,"subject":subject,"missed":missed})
    };
    let digest_entry = |missed: Value| entry(&digest, "dana@x.io", "Digest", missed);
    let sibling_entry = |missed: Value| entry(&sibling, "erin@x.io", "Your code", missed);
    let near = |arrivals: Vec<Value>, nearest: Value| {
        json!({"arrivals_total":arrivals.len(),"arrivals":arrivals,"arrivals_truncated":false,
            "nearest_subject":nearest,"last_arrival_ms":stamp(&sibling)})
    };
    // The filter admitted the digest, which carried no code: empty `missed`.
    let code = focused(
        addr_of(&srv),
        "code",
        json!({"inbox":"fnm","to":"dana@x.io","timeout_ms":0}),
    )
    .await;
    assert_eq!(code["status"], "timed_out", "{code}");
    assert_eq!(
        code["near_misses"],
        near(
            vec![sibling_entry(json!(["to"])), digest_entry(json!([]))],
            Value::Null
        )
    );
    // A subject the digest nearly has: it is the nearest, the sibling that
    // also failed `to` is not.
    let code = focused(
        addr_of(&srv),
        "code",
        json!({"inbox":"fnm","to":"dana@x.io","subject":"Digests","timeout_ms":0}),
    )
    .await;
    assert_eq!(
        code["near_misses"],
        near(
            vec![
                sibling_entry(json!(["to", "subject"])),
                digest_entry(json!(["subject"]))
            ],
            digest_entry(json!(["subject"]))
        )
    );
    // Link selectors are not mail filters: both mails show, none missed.
    let link = focused(
        addr_of(&srv),
        "link",
        json!({"inbox":"fnm","path":"^/nowhere$","timeout_ms":0}),
    )
    .await;
    assert_eq!(link["status"], "timed_out", "{link}");
    assert_eq!(
        link["near_misses"],
        near(
            vec![sibling_entry(json!([])), digest_entry(json!([]))],
            Value::Null
        )
    );
    let object = link["near_misses"].as_object().unwrap();
    assert_eq!(
        object.keys().collect::<Vec<_>>(),
        [
            "arrivals",
            "arrivals_total",
            "arrivals_truncated",
            "last_arrival_ms",
            "nearest_subject"
        ]
    );
    assert_eq!(object["arrivals"][0].as_object().unwrap().keys().count(), 7);
    // Nothing retained: today's shape, with no member.
    for tool in ["code", "link"] {
        let empty = focused(
            addr_of(&srv),
            tool,
            json!({"inbox":"fnm-empty","timeout_ms":0}),
        )
        .await;
        assert_eq!(empty["status"], "timed_out");
        assert!(empty.get("near_misses").is_none(), "{empty}");
    }
    // The read changed nothing.
    assert_eq!(
        srv.store.count("fnm", &swarmail::store::Filter::default()),
        2
    );
    srv.stop().await;
}

fn addr_of(srv: &swarmail::RunningServer) -> SocketAddr {
    srv.http_addr
}

/// Ingest records occurrences once, for SMTP and seed alike (decisions
/// 0005, 0010): text part first, then the HTML part in source order, with
/// labels and rel only on owned anchors. An HTML attachment contributes
/// nothing. Every existing read keeps its shape: no Email surface carries
/// the metadata, and `links` stays the deduplicated projection.
#[tokio::test]
async fn ingest_metadata_reaches_the_link_wait_and_nothing_else() {
    let srv = start().await;
    let raw = "From: App <app@x.io>\nTo: r@x.io\nSubject: Welcome\nMIME-Version: 1.0\n\
Content-Type: multipart/mixed; boundary=outer\n\n--outer\n\
Content-Type: multipart/alternative; boundary=inner\n\n--inner\n\
Content-Type: text/plain\n\nConfirm at https://app.test/confirm/1\n--inner\n\
Content-Type: text/html\n\n<p><a href=https://app.test/confirm/1 rel='noopener'>Confirm  <b>now</b></a>\
 <a href=\"https://app.test/unsub\">Unsubscribe</a></p>\n--inner--\n--outer\n\
Content-Type: text/html\nContent-Disposition: attachment; filename=x.html\n\n\
<a href=https://evil.test/attached>Confirm</a>\n--outer--\n";
    let id = deliver(srv.smtp_addr, "ingest", "app@x.io", "r@x.io", raw).await;
    let result = focused(
        srv.http_addr,
        "link",
        json!({"inbox":"ingest","timeout_ms":0}),
    )
    .await;
    let rows: Vec<Value> = result["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            json!([
                c["url"],
                c["link_index"],
                c["anchor_text"],
                c["rel"],
                c["source"],
                c["email_id"]
            ])
        })
        .collect();
    assert_eq!(
        rows,
        [
            json!(["https://app.test/confirm/1", 0, null, null, "text", id]),
            json!([
                "https://app.test/confirm/1",
                1,
                "Confirm now",
                "noopener",
                "html",
                id
            ]),
            json!(["https://app.test/unsub", 2, "Unsubscribe", null, "html", id]),
        ]
    );
    assert_eq!(result["sender"], Value::Null, "no top-level sender");
    assert_eq!(
        result["candidates"][1]["sender"],
        json!({"name":"App","address":"app@x.io"})
    );
    let confirm = focused(
        srv.http_addr,
        "link",
        json!({"inbox":"ingest","anchor_text":"^Confirm","timeout_ms":0}),
    )
    .await;
    assert_eq!(confirm["selected_url"], "https://app.test/confirm/1");
    assert_eq!(confirm["candidates_total"], 1);
    // The existing reads: legacy links only, no metadata member anywhere.
    let (_, rest) = http_json(
        srv.http_addr,
        "GET",
        &format!("/api/v1/messages/{id}"),
        None,
    )
    .await;
    assert_eq!(
        rest["links"],
        json!(["https://app.test/confirm/1", "https://app.test/unsub"])
    );
    assert!(rest.get("link_metadata").is_none());
    let (_, listed) = http_json(
        srv.http_addr,
        "GET",
        "/api/v1/inboxes/ingest/messages",
        None,
    )
    .await;
    assert!(!listed.to_string().contains("anchor_text"));
    // The seed shares the builder.
    let body = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"swarmail_seed_email",
        "arguments":{"inbox":"seeded","to":"r@x.io","html":"<a href='https://app.test/s' rel=next>Go</a> https://app.test/s"}}});
    let (_, seeded) = http_json(srv.http_addr, "POST", "/mcp", Some(&body.to_string())).await;
    let seeded: Value =
        serde_json::from_str(seeded["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(seeded["links"], json!(["https://app.test/s"]));
    let result = focused(
        srv.http_addr,
        "link",
        json!({"inbox":"seeded","anchor_text":"^Go$","timeout_ms":0}),
    )
    .await;
    assert_eq!(result["candidates"].as_array().unwrap().len(), 1);
    assert_eq!(result["candidates"][0]["rel"], "next");
    let result = focused(
        srv.http_addr,
        "link",
        json!({"inbox":"seeded","timeout_ms":0}),
    )
    .await;
    assert_eq!(
        (result["status"].clone(), result["candidates_total"].clone()),
        (json!("selected"), json!(2))
    );
    srv.stop().await;
}

/// A parked wait (the readiness row is its admission) is completed only by
/// the target: an ineligible delivery to its inbox, eligible mail for
/// another inbox and a filtered-out delivery each leave it waiting, as the
/// final answer — only the target, matched once — shows. The park is then
/// released.
#[tokio::test]
async fn deliveries_after_the_park_only_complete_on_the_target() {
    let srv = start().await;
    for tool in ["code", "link"] {
        let inbox = format!("parked-{tool}");
        let http = srv.http_addr;
        let args = json!({"inbox":inbox,"from":"app@","timeout_ms":60000});
        let waiting = tokio::spawn(async move { focused(http, tool, args).await });
        let row = format!("swarmail_awaits_active{{inbox=\"{inbox}\"}}");
        wait_for_metric_row(srv.http_addr, &row, 1).await;
        let headers = |from: &str| format!("From: {from}\nTo: r@x.io\nSubject: s");
        let smtp = srv.smtp_addr;
        deliver(
            smtp,
            &inbox,
            "app@x.io",
            "r@x.io",
            &mail(&headers("app@x.io"), "no code, no link"),
        )
        .await;
        deliver(
            smtp,
            "elsewhere",
            "app@x.io",
            "r@x.io",
            &mail(
                &headers("app@x.io"),
                "code 999999 https://app.test/elsewhere",
            ),
        )
        .await;
        deliver(
            smtp,
            &inbox,
            "spoof@x.io",
            "r@x.io",
            &mail(&headers("spoof@x.io"), "code 888888 https://app.test/spoof"),
        )
        .await;
        let target = deliver(
            smtp,
            &inbox,
            "app@x.io",
            "r@x.io",
            &mail(&headers("app@x.io"), "code 111111 https://app.test/target"),
        )
        .await;
        let result = tokio::time::timeout(std::time::Duration::from_secs(30), waiting)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result["matched"], 1, "{result}");
        assert_eq!(result["candidates"][0]["email_id"], target);
        assert_eq!(
            answer(&result),
            if tool == "code" {
                json!("111111")
            } else {
                json!("https://app.test/target")
            }
        );
        // The park is released before the reply is written: no row remains.
        let (_, metrics) = common::http_get_text(srv.http_addr, "/metrics").await;
        assert_eq!(common::metric_row_value(&metrics, &row), None);
    }
    srv.stop().await;
}

/// A local application the traps act against: it accepts only its current
/// code and its genuine confirmation link, and records every action. Any
/// URL is delivered to it with the URL's own Host — a fixed resolver, so a
/// look-alike host reaches it and is judged rather than resolved away.
#[derive(Default)]
struct App {
    code: String,
    stale: Vec<String>,
    token: String,
    confirmed: bool,
    verified: bool,
    events: Vec<String>,
}

async fn serve(app: std::sync::Arc<std::sync::Mutex<App>>) -> SocketAddr {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            let mut request = String::new();
            stream.read_line(&mut request).await.unwrap();
            let mut host = String::new();
            loop {
                let mut line = String::new();
                stream.read_line(&mut line).await.unwrap();
                if line.trim().is_empty() {
                    break;
                }
                if let Some(value) = line.strip_prefix("Host: ") {
                    host = value.trim().into();
                }
            }
            let target = request.split_whitespace().nth(1).unwrap().to_string();
            let status = {
                let mut app = app.lock().unwrap();
                app.events.push(format!("{host}{target}"));
                if let Some(code) = target.strip_prefix("/verify?code=") {
                    if host != "app.test" {
                        403
                    } else if code == app.code {
                        app.verified = true;
                        200
                    } else if app.stale.iter().any(|s| s == code) {
                        410
                    } else {
                        404
                    }
                } else if host == "app.test" && target == format!("/confirm/{}", app.token) {
                    app.confirmed = true;
                    200
                } else {
                    403
                }
            };
            let reply =
                format!("HTTP/1.1 {status} X\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            stream.get_mut().write_all(reply.as_bytes()).await.unwrap();
        }
    });
    addr
}

/// The consumer: GET the URL from the application, Host from the URL.
async fn act(app: SocketAddr, url: &str) -> u16 {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let url = reqwest::Url::parse(url).unwrap();
    let mut target = url.path().to_string();
    if let Some(query) = url.query() {
        target = format!("{target}?{query}");
    }
    let mut stream = tokio::net::TcpStream::connect(app).await.unwrap();
    let request = format!(
        "GET {target} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        url.host_str().unwrap()
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    response.split_whitespace().nth(1).unwrap().parse().unwrap()
}

/// The look-alike-link trap: two links share the anchor "Confirm", one on
/// a look-alike host. Anchor alone is ambiguous and nothing is acted on;
/// an exact host selects the genuine link and confirms the account; acting
/// on the look-alike candidate instead is refused by the application.
#[tokio::test]
async fn the_look_alike_link_trap_needs_an_exact_host() {
    let srv = start().await;
    let state = std::sync::Arc::new(std::sync::Mutex::new(App {
        token: "t0k3n".into(),
        ..App::default()
    }));
    let app = serve(state.clone()).await;
    let html = "<p><a href=\"https://app.test.evil.io/confirm/t0k3n\">Confirm</a></p>\
        <p><a href='https://app.test/confirm/t0k3n'>Confirm your account</a></p>\
        <a href=https://app.test/unsubscribe>Unsubscribe</a>";
    let raw = format!(
        "From: app@app.test\nTo: r@x.io\nSubject: Confirm\nContent-Type: text/html\n\n{html}\n"
    );
    deliver(srv.smtp_addr, "lookalike", "app@app.test", "r@x.io", &raw).await;
    let by_anchor = focused(
        srv.http_addr,
        "link",
        json!({"inbox":"lookalike","anchor_text":"^Confirm","timeout_ms":0}),
    )
    .await;
    assert_eq!(
        (
            by_anchor["status"].clone(),
            by_anchor["distinct_urls"].clone()
        ),
        (json!("ambiguous"), json!(2))
    );
    assert!(by_anchor.get("selected_url").is_none());
    let loose = focused(
        srv.http_addr,
        "link",
        json!({"inbox":"lookalike","anchor_text":"^Confirm","host":"app[.]test","timeout_ms":0}),
    )
    .await;
    assert_eq!(
        loose["status"], "ambiguous",
        "an unanchored host search is not equality"
    );
    assert!(
        state.lock().unwrap().events.is_empty(),
        "ambiguity acted on nothing"
    );
    let exact = focused(
        srv.http_addr,
        "link",
        json!({"inbox":"lookalike","anchor_text":"^Confirm","host":"^app[.]test$","timeout_ms":0}),
    )
    .await;
    assert_eq!(exact["status"], "selected");
    assert_eq!(act(app, exact["selected_url"].as_str().unwrap()).await, 200);
    assert!(state.lock().unwrap().confirmed);
    // The independent negative control: the look-alike candidate fails the
    // action itself, on a fresh application state.
    *state.lock().unwrap() = App {
        token: "t0k3n".into(),
        ..App::default()
    };
    let decoy = by_anchor["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["url"].as_str().unwrap().contains("evil"))
        .unwrap();
    assert_eq!(act(app, decoy["url"].as_str().unwrap()).await, 403);
    {
        let app_state = state.lock().unwrap();
        assert!(!app_state.confirmed);
        assert_eq!(app_state.events, ["app.test.evil.io/confirm/t0k3n"]);
    }
    srv.stop().await;
}

/// The stale-code trap: the application reissues its code, so the first
/// mail's code is stale. The newest code verifies; the older candidate is
/// refused as stale. A code from an earlier session already in the inbox
/// answers at once without since_ms — stale — and with since_ms the wait
/// holds for the fresh mail. Two codes in the newest mail are ambiguous
/// and nothing is tried.
#[tokio::test]
async fn the_stale_code_trap_takes_the_newest_code() {
    let srv = start().await;
    let state = std::sync::Arc::new(std::sync::Mutex::new(App::default()));
    let app = serve(state.clone()).await;
    let verify = |code: &Value| format!("https://app.test/verify?code={}", code.as_str().unwrap());
    let smtp = srv.smtp_addr;
    let send = |inbox: &'static str, body: &'static str| async move {
        let raw = mail("From: app@app.test\nTo: r@x.io\nSubject: Your code", body);
        deliver(smtp, inbox, "app@app.test", "r@x.io", &raw).await
    };
    {
        let mut app = state.lock().unwrap();
        app.code = "222222".into();
        app.stale = vec!["111111".into()];
    }
    send("reissued", "Your code is 111111").await;
    send("reissued", "Your new code is 222222").await;
    let result = focused(
        srv.http_addr,
        "code",
        json!({"inbox":"reissued","timeout_ms":0}),
    )
    .await;
    assert_eq!(
        (
            result["status"].clone(),
            result["older_candidates_total"].clone()
        ),
        (json!("selected"), json!(1))
    );
    assert_eq!(act(app, &verify(&result["selected"]["code"])).await, 200);
    assert_eq!(
        act(app, &verify(&result["older_candidates"][0]["code"])).await,
        410,
        "the stale candidate fails"
    );
    // An earlier session's code sits in the inbox with an older stamp.
    let previous = send("session", "Your code is 333333").await;
    let earlier = srv.store.get(&previous).unwrap().received_ms - 60_000;
    let mut old = (*srv.store.get(&previous).unwrap()).clone();
    srv.store.delete(&previous).unwrap();
    old.received_ms = earlier;
    srv.store.insert(old).unwrap();
    {
        let mut app = state.lock().unwrap();
        app.code = "444444".into();
        app.stale = vec!["333333".into()];
    }
    let immediate = focused(
        srv.http_addr,
        "code",
        json!({"inbox":"session","timeout_ms":0}),
    )
    .await;
    assert_eq!(
        act(app, &verify(&immediate["selected"]["code"])).await,
        410,
        "without since_ms: stale"
    );
    let http = srv.http_addr;
    let since = json!({"inbox":"session","since_ms":earlier + 1,"timeout_ms":60000});
    let waiting = tokio::spawn(async move { focused(http, "code", since).await });
    wait_for_metric_row(
        srv.http_addr,
        "swarmail_awaits_active{inbox=\"session\"}",
        1,
    )
    .await;
    send("session", "Your code is 444444").await;
    let fresh = waiting.await.unwrap();
    assert_eq!(fresh["older_candidates_total"], 0);
    assert_eq!(act(app, &verify(&fresh["selected"]["code"])).await, 200);
    // Ambiguity: nothing is guessed, nothing is tried.
    let before = state.lock().unwrap().events.len();
    send("two", "Use 555555 or 666666").await;
    let ambiguous = focused(srv.http_addr, "code", json!({"inbox":"two","timeout_ms":0})).await;
    assert_eq!(ambiguous["status"], "ambiguous");
    assert!(ambiguous.get("selected").is_none());
    assert_eq!(state.lock().unwrap().events.len(), before);
    srv.stop().await;
}
