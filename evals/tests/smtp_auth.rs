//! The fixture's `smtp_auth` fact (proposed decision 0129 §2): `none` drops
//! the AUTH step and nothing else, and `plain` is the dialogue `main` sends.

mod common;

use base64::Engine;
use std::process::Stdio;
use swarmail_evals::mail::{self, SendResult, SmtpAuth};
use swarmail_evals::mcp::McpClient;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::process::Command;

/// A scripted SMTP server that accepts everything and records every command
/// line it receives, DATA's body as one entry.
async fn recording_server() -> (std::net::SocketAddr, tokio::task::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let (r, mut w) = stream.into_split();
        let mut r = BufReader::new(r);
        let mut log = Vec::new();
        w.write_all(b"220 recorder\r\n").await.unwrap();
        loop {
            let mut line = String::new();
            if r.read_line(&mut line).await.unwrap() == 0 {
                return log;
            }
            let line = line.trim_end().to_string();
            let reply: &[u8] = if line.starts_with("EHLO") {
                b"250 ok\r\n"
            } else if line.starts_with("AUTH PLAIN") {
                b"235 ok\r\n"
            } else if line == "DATA" {
                b"354 go\r\n"
            } else if line == "QUIT" {
                log.push(line);
                w.write_all(b"221 bye\r\n").await.unwrap();
                let mut rest = Vec::new();
                let _ = r.read_to_end(&mut rest).await;
                return log;
            } else {
                b"250 ok\r\n"
            };
            let is_data = line == "DATA";
            log.push(line);
            w.write_all(reply).await.unwrap();
            if is_data {
                let mut body = String::new();
                loop {
                    let mut l = String::new();
                    r.read_line(&mut l).await.unwrap();
                    if l == ".\r\n" {
                        break;
                    }
                    body.push_str(&l);
                }
                log.push(format!("<{} body bytes>", body.len()));
                w.write_all(b"250 stored\r\n").await.unwrap();
            }
        }
    });
    (addr, task)
}

#[tokio::test]
async fn plain_is_the_dialogue_main_sends_and_none_drops_only_the_auth_step() {
    let token = base64::engine::general_purpose::STANDARD.encode("\0alice\0eval");
    let data = b"Subject: s\r\n\r\nbody\r\n";
    let common = |auth: Option<&str>| {
        let mut v = vec!["EHLO fixture.eval".to_string()];
        v.extend(auth.map(str::to_string));
        v.extend([
            "MAIL FROM:<app@fixture.eval>".to_string(),
            "RCPT TO:<alice@fixture.eval>".to_string(),
            "DATA".to_string(),
            format!("<{} body bytes>", data.len()),
            "QUIT".to_string(),
        ]);
        v
    };
    for (auth, want) in [
        (
            SmtpAuth::Plain,
            common(Some(&format!("AUTH PLAIN {token}"))),
        ),
        (SmtpAuth::None, common(None)),
    ] {
        let (addr, log) = recording_server().await;
        let result = mail::send_auth(
            addr,
            auth,
            "alice",
            "app@fixture.eval",
            "alice@fixture.eval",
            data,
        )
        .await;
        assert!(
            matches!(result, SendResult::Delivered(_)),
            "{auth:?}: {result:?}"
        );
        assert_eq!(log.await.unwrap(), want, "{auth:?}");
    }
    // `send` is `Plain`: the same bytes as before the knob existed.
    let (addr, log) = recording_server().await;
    mail::send(
        addr,
        "alice",
        "app@fixture.eval",
        "alice@fixture.eval",
        data,
    )
    .await;
    assert_eq!(
        log.await.unwrap(),
        common(Some(&format!("AUTH PLAIN {token}")))
    );
}

#[test]
fn the_fact_parses_by_name_and_refuses_anything_else() {
    for a in [SmtpAuth::Plain, SmtpAuth::None] {
        assert_eq!(SmtpAuth::parse(a.name()), Some(a));
    }
    assert_eq!(SmtpAuth::default(), SmtpAuth::Plain);
    for bad in ["", "PLAIN", "login", "none ", "digest"] {
        assert_eq!(SmtpAuth::parse(bad), None, "{bad:?}");
    }
}

#[tokio::test]
async fn an_unauthenticated_dialogue_is_stored_by_a_real_swarmail() {
    // Swarmail's listener is AUTH-optional: without AUTH the mail lands in
    // the default inbox. Mechanics of the dialogue only.
    let dir = common::tempdir();
    let mut server = Command::new(common::swarmail_bin())
        .args([
            "serve",
            "--smtp-listen",
            "127.0.0.1:0",
            "--pop3-listen",
            "127.0.0.1:0",
            "--http-listen",
            "127.0.0.1:0",
        ])
        .current_dir(dir.path())
        .env_clear()
        .env("HOME", dir.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(server.stdout.take().unwrap());
    let (smtp, http) = loop {
        let mut line = String::new();
        assert!(lines.read_line(&mut line).await.unwrap() > 0);
        if let Some(addrs) = swarmail_evals::runner::parse_swarmail_addrs(&line) {
            break addrs;
        }
    };
    let message = mail::Message {
        from: "app@fixture.eval".into(),
        to: "alice@fixture.eval".into(),
        subject: "No auth step".into(),
        body: mail::Body::Text("hello".into()),
    };
    let result = mail::send_auth(
        smtp,
        SmtpAuth::None,
        "alice",
        "app@fixture.eval",
        "alice@fixture.eval",
        &mail::render(&message),
    )
    .await;
    assert!(matches!(result, SendResult::Delivered(_)), "{result:?}");
    let mut client = McpClient::connect(&format!("http://{http}/mcp"))
        .await
        .unwrap();
    let mails = client
        .wait_emails("default", Some("No auth step"), 1)
        .await
        .expect("stored without an AUTH step");
    assert_eq!(mails.len(), 1);
}
