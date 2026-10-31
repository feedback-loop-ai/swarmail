//! The agent setup guides, exercised (issue #161, epic #164): a stranger
//! wires Claude Code, Codex, Cursor or dsh to swarmail from
//! `docs/agents/setup.md` alone, and the bytes they copy are the bytes this
//! suite ran.
//!
//! What "exercised" means here, and where it stops:
//!
//! * One scanner (`extract`) reads every fence of `docs/agents/setup.md` and
//!   `docs/agents/swarmail/SKILL.md`. It fails closed: a fence shape it does
//!   not recognise, an unknown info string, an unterminated fence, a CR byte
//!   or an unmarked fence nobody executes is red. A fence that is not run
//!   carries a `not-executed` marker comment with a reason on the line above
//!   it.
//! * Each of the five wiring fences (JSON, TOML, YAML) is written verbatim
//!   into a fresh temporary project, read back, reduced to the transport it
//!   declares, compared with the binary's own defaults (parsed from
//!   `swarmail::config::Cli`, never restated), and then driven against the
//!   real `swarmail` binary: `initialize`, `tools/list`, clear, seed, wait,
//!   extract with a seeded code.
//! * The executed bytes differ from the shipped bytes in one asserted place:
//!   the HTTP snippets have their one default authority replaced by the
//!   ephemeral loopback port the server bound (eight runs share a host, so
//!   nobody owns 8025). The stdio snippet and the `swarmail serve` start
//!   command run with their argv unchanged; their addresses arrive through
//!   the documented `SWARMAIL_*` environment, set on the child, never on this
//!   process.
//! * TOML and YAML have no parser in the locked tree (and none may be
//!   added), so two subset readers validate the snippets we author. They are
//!   not parsers and refuse everything outside the subset.
//! * CI proves parse and transport only. Whether Claude Code, Codex, Cursor
//!   or dsh load these files is an operator-owned receipt (the guide says so
//!   and this suite pins that it does).
//!
//! The suite speaks real HTTP and NDJSON to the shipped binary and touches no
//! crate-private seam: a decision 0020 rule-3 test, outside the coverage
//! denominator by location.

mod common;

use clap::{CommandFactory, Parser};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::path::Path;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use swarmail::config::{Cli, Command as CliCommand};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

const GUIDE: &str = "docs/agents/setup.md";
const SKILL: &str = "docs/agents/swarmail/SKILL.md";
/// The one unmarked `sh` fence: the command the suite runs as the server.
const START: &str = "swarmail serve";
/// One exchange with a server or a bridge.
const EXCHANGE: Duration = Duration::from_secs(45);
/// The whole round trip of one snippet.
const ROUND_TRIP: Duration = Duration::from_secs(120);
/// The six agent-caused failure classes of decision 0071 the block teaches.
const CLASSES: [&str; 6] = [
    "wrong_message",
    "stale_code",
    "wrong_link",
    "wait_timeout",
    "gave_up",
    "tool_misuse",
];

// ---------------------------------------------------------------- reading

fn raw(path: &str) -> String {
    let full = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    std::fs::read_to_string(&full)
        .unwrap_or_else(|e| panic!("{} is unreadable: {e}", full.display()))
}

/// Whitespace-normalized to single spaces, so line wrapping never decides.
fn squash(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Phrases absent from `text`.
fn absent<'a>(text: &str, phrases: &[&'a str]) -> Vec<&'a str> {
    phrases
        .iter()
        .copied()
        .filter(|phrase| !text.contains(phrase))
        .collect()
}

// ---------------------------------------------------------------- scanner

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Info {
    Json,
    Toml,
    Yaml,
    Markdown,
    Sh,
    Text,
}

#[derive(Debug, Clone)]
struct Fence {
    info: Info,
    marker: Option<String>,
    body: String,
    line: usize,
}

type Refusal = String;

fn info_of(word: &str) -> Option<Info> {
    Some(match word {
        "json" => Info::Json,
        "toml" => Info::Toml,
        "yaml" => Info::Yaml,
        "markdown" => Info::Markdown,
        "sh" => Info::Sh,
        "text" => Info::Text,
        _ => return None,
    })
}

/// A marker line's reason, or the reason it is not one.
fn marker_reason(line: &str) -> Result<String, Refusal> {
    let inner = line
        .strip_prefix("<!-- not-executed:")
        .and_then(|rest| rest.strip_suffix("-->"))
        .ok_or_else(|| format!("{line:?} is not a `not-executed: reason` marker comment"))?;
    let reason = inner.trim();
    if reason.is_empty() {
        return Err("a not-executed marker needs a reason".into());
    }
    Ok(reason.to_string())
}

/// Every fence of a guide file, or the first reason the file is outside the
/// convention. A fence is a line of exactly three backticks plus an info
/// string at column 0; everything fence-shaped that is not one is refused.
fn extract(doc: &str) -> Result<Vec<Fence>, Refusal> {
    if doc.contains('\r') {
        return Err("a CR byte is in the file".into());
    }
    let mut fences = Vec::new();
    let mut marker: Option<(String, usize)> = None;
    let mut open: Option<Fence> = None;
    for (index, line) in doc.lines().enumerate() {
        let number = index + 1;
        if let Some(fence) = open.as_mut() {
            if line == "```" {
                fences.push(open.take().expect("an open fence"));
            } else if line.starts_with("```") {
                return Err(format!("line {number}: a fence opens inside a fence"));
            } else {
                fence.body.push_str(line);
                fence.body.push('\n');
            }
            continue;
        }
        if line.starts_with("<!-- not-executed") {
            if let Some((_, at)) = marker {
                return Err(format!("line {at}: a marker is not followed by a fence"));
            }
            let reason = marker_reason(line).map_err(|e| format!("line {number}: {e}"))?;
            marker = Some((reason, number));
        } else if line.contains("not-executed") {
            return Err(format!("line {number}: a malformed marker"));
        } else if let Some(rest) = line.strip_prefix("```") {
            if rest.starts_with('`') {
                return Err(format!("line {number}: four or more backticks"));
            }
            let info = info_of(rest)
                .ok_or_else(|| format!("line {number}: unknown info string {rest:?}"))?;
            let reason = marker.take().map(|(reason, _)| reason);
            if reason.is_some() && info != Info::Sh && info != Info::Text {
                return Err(format!(
                    "line {number}: wiring and the block are always executed, never marked"
                ));
            }
            open = Some(Fence {
                info,
                marker: reason,
                body: String::new(),
                line: number,
            });
        } else if line.trim_start().starts_with("```") || line.starts_with("~~~") {
            return Err(format!("line {number}: an indented or tilde fence"));
        } else if let Some((_, at)) = marker {
            return Err(format!("line {at}: a marker is not followed by a fence"));
        }
    }
    if let Some(fence) = open {
        return Err(format!("line {}: an unterminated fence", fence.line));
    }
    if let Some((_, at)) = marker {
        return Err(format!("line {at}: a marker is not followed by a fence"));
    }
    for fence in &fences {
        match (fence.info, &fence.marker) {
            (Info::Text, None) => {
                return Err(format!("line {}: an unmarked text fence", fence.line));
            }
            (Info::Sh, None) if fence.body.trim() != START => {
                return Err(format!(
                    "line {}: an unmarked sh fence must be {START:?}",
                    fence.line
                ));
            }
            _ => {}
        }
    }
    Ok(fences)
}

/// The guide's shape: one start command, five wiring fences in the order
/// Claude HTTP, Claude stdio, Codex, Cursor, dsh, and one block.
fn guide_shape(fences: &[Fence]) -> Result<(), Refusal> {
    let infos: Vec<Info> = executed(fences).iter().map(|f| f.info).collect();
    let want = [
        Info::Sh,
        Info::Json,
        Info::Json,
        Info::Toml,
        Info::Json,
        Info::Yaml,
        Info::Markdown,
    ];
    if infos != want {
        return Err(format!("executed fences are {infos:?}, want {want:?}"));
    }
    Ok(())
}

fn executed(fences: &[Fence]) -> Vec<&Fence> {
    fences.iter().filter(|f| f.marker.is_none()).collect()
}

fn guide_fences() -> Vec<Fence> {
    let fences = extract(&raw(GUIDE)).unwrap_or_else(|e| panic!("{GUIDE}: {e}"));
    guide_shape(&fences).unwrap_or_else(|e| panic!("{GUIDE}: {e}"));
    fences
}

/// The wiring fence at a position (0 Claude HTTP … 4 dsh).
fn wiring_fence(position: usize) -> Fence {
    guide_fences()
        .into_iter()
        .filter(|f| f.marker.is_none())
        .nth(1 + position)
        .expect("a wiring fence")
}

/// The AGENTS.md block: the guide's one `markdown` fence.
fn block() -> String {
    guide_fences()
        .into_iter()
        .find(|f| f.info == Info::Markdown)
        .expect("one markdown fence")
        .body
}

// ---------------------------------------------------------------- readers

#[derive(Debug, Clone, PartialEq, Eq)]
enum Wiring {
    Http { url: String },
    Stdio { command: String, args: Vec<String> },
}

fn json_value(text: &str, what: &str) -> Result<Value, Refusal> {
    serde_json::from_str(text.trim()).map_err(|e| format!("{what} is not JSON: {e}"))
}

fn object<'a>(
    value: &'a Value,
    what: &str,
    allowed: &[&str],
) -> Result<&'a serde_json::Map<String, Value>, Refusal> {
    let map = value
        .as_object()
        .ok_or_else(|| format!("{what} is not an object"))?;
    if let Some(extra) = map.keys().find(|k| !allowed.contains(&k.as_str())) {
        return Err(format!("{what} carries the unexpected key {extra:?}"));
    }
    Ok(map)
}

/// A server entry (`url`, or `command` and `args`) as a transport.
fn entry_wiring(entry: &Value) -> Result<Wiring, Refusal> {
    let has_url = entry.get("url").is_some();
    if has_url {
        let map = object(
            entry,
            "the swarmail entry",
            &["type", "url", "startup_timeout_sec", "tool_timeout_sec"],
        )?;
        if map.get("type").is_some_and(|t| t != "http") {
            return Err("the swarmail entry's type is not http".into());
        }
        let url = map["url"].as_str().ok_or("url is not a string")?;
        return Ok(Wiring::Http { url: url.into() });
    }
    let map = object(entry, "the swarmail entry", &["command", "args"])?;
    let command = map
        .get("command")
        .and_then(Value::as_str)
        .ok_or("the swarmail entry has neither url nor command")?;
    let args = map
        .get("args")
        .and_then(Value::as_array)
        .ok_or("args is not an array")?
        .iter()
        .map(|a| {
            a.as_str()
                .map(str::to_string)
                .ok_or("an arg is not a string")
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Wiring::Stdio {
        command: command.into(),
        args,
    })
}

/// `{"mcpServers": {"swarmail": …}}`, nothing else.
fn reduce_json(body: &str) -> Result<Wiring, Refusal> {
    let root = json_value(body, "the snippet")?;
    let root = object(&root, "the snippet", &["mcpServers"])?;
    let servers = root.get("mcpServers").ok_or("no mcpServers")?;
    let servers = object(servers, "mcpServers", &["swarmail"])?;
    entry_wiring(servers.get("swarmail").ok_or("no swarmail server")?)
}

/// The TOML subset the guide uses: blank lines, `#` comments, the one
/// `[mcp_servers.swarmail]` header and `key = <JSON value>` rows.
fn reduce_toml(body: &str) -> Result<Wiring, Refusal> {
    let mut seen_header = false;
    let mut entry = serde_json::Map::new();
    for line in body.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            if line != "[mcp_servers.swarmail]" || seen_header {
                return Err(format!("{line:?} is not the one swarmail table"));
            }
            seen_header = true;
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("{line:?} is outside the TOML subset"))?;
        let key = key.trim();
        let bare = !key.is_empty()
            && key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if !seen_header || !bare {
            return Err(format!("{line:?} is outside the TOML subset"));
        }
        let value = json_value(value, key)?;
        if entry.insert(key.into(), value).is_some() {
            return Err(format!("{key:?} is set twice"));
        }
    }
    if !seen_header {
        return Err("no [mcp_servers.swarmail] table".into());
    }
    entry_wiring(&Value::Object(entry))
}

/// The YAML subset the guide uses: blank lines, `#` comments and
/// `- <JSON flow node>` rows (JSON is YAML 1.2), exactly one of them the
/// `insert` row carrying the swarmail MCP client.
fn reduce_yaml(body: &str) -> Result<Wiring, Refusal> {
    let mut rows = Vec::new();
    for line in body.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let node = line
            .strip_prefix("- ")
            .ok_or_else(|| format!("{line:?} is outside the YAML subset"))?;
        rows.push(json_value(node, "a row")?);
    }
    let [row] = rows.as_slice() else {
        return Err(format!("{} rows, want exactly one", rows.len()));
    };
    let row = object(row, "the row", &["insert"])?;
    let inserts = row
        .get("insert")
        .and_then(Value::as_array)
        .ok_or("no insert array")?;
    let [client] = inserts.as_slice() else {
        return Err("the insert array holds not exactly one client".into());
    };
    let client = object(client, "the client", &["id", "name", "config"])?;
    if client.get("id") != Some(&json!("mcp-swarmail"))
        || client.get("name") != Some(&json!("@deepseek-ai/dsh-mcp-client"))
    {
        return Err("the client is not mcp-swarmail".into());
    }
    let config = client.get("config").ok_or("no config")?;
    let config = object(
        config,
        "config",
        &[
            "serverName",
            "transport",
            "url",
            "failOnStartupError",
            "toolCallTimeoutMs",
        ],
    )?;
    if config.get("serverName") != Some(&json!("swarmail"))
        || config.get("transport") != Some(&json!("streamable-http"))
        || config.get("toolCallTimeoutMs") != Some(&json!(60000))
    {
        return Err("config is not the swarmail streamable-http client".into());
    }
    let url = config
        .get("url")
        .and_then(Value::as_str)
        .ok_or("config has no url")?;
    Ok(Wiring::Http { url: url.into() })
}

fn reduce(fence: &Fence) -> Result<Wiring, Refusal> {
    match fence.info {
        Info::Json => reduce_json(&fence.body),
        Info::Toml => reduce_toml(&fence.body),
        Info::Yaml => reduce_yaml(&fence.body),
        other => Err(format!("{other:?} is not a wiring fence")),
    }
}

// ---------------------------------------------------------------- defaults

/// What the binary itself defaults to, read from its own CLI definition.
struct Defaults {
    /// The CLI's program name (`swarmail`).
    program: String,
    /// `SWARMAIL_URL`'s default, the bridge's base URL.
    url: String,
    /// `SWARMAIL_HTTP_LISTEN`'s default, the authority a snippet names.
    authority: String,
}

fn defaults() -> Defaults {
    let Ok(Cli {
        command: CliCommand::Serve(serve),
    }) = Cli::try_parse_from(["swarmail", "serve"])
    else {
        panic!("`swarmail serve` parses to the serve command");
    };
    let Ok(Cli {
        command: CliCommand::Mcp(mcp),
    }) = Cli::try_parse_from(["swarmail", "mcp"])
    else {
        panic!("`swarmail mcp` parses to the mcp command");
    };
    Defaults {
        program: Cli::command().get_name().to_string(),
        url: mcp.url,
        authority: serve.http_listen,
    }
}

/// The wiring names exactly what the binary defaults to: an HTTP URL is the
/// default base URL plus `/mcp` (and that base URL is the server's own
/// default listener); the stdio command is the program with arguments that
/// parse as the `mcp` command with no `--url` of their own.
fn assert_defaults(wiring: &Wiring, defaults: &Defaults) -> Result<(), Refusal> {
    match wiring {
        Wiring::Http { url } => {
            if defaults.url != format!("http://{}", defaults.authority) {
                return Err("the bridge and server defaults disagree".into());
            }
            let want = format!("{}/mcp", defaults.url);
            if *url != want {
                return Err(format!("url {url:?} is not the default {want:?}"));
            }
        }
        Wiring::Stdio { command, args } => {
            if *command != defaults.program {
                return Err(format!("command {command:?} is not {:?}", defaults.program));
            }
            let argv = std::iter::once(command.as_str()).chain(args.iter().map(String::as_str));
            match Cli::try_parse_from(argv) {
                Ok(Cli {
                    command: CliCommand::Mcp(mcp),
                }) if args == &["mcp".to_string()] && mcp.url == defaults.url => {}
                other => return Err(format!("args {args:?} are not `mcp` alone: {other:?}")),
            }
        }
    }
    Ok(())
}

/// Replace `from` by `to` only when `from` occurs exactly once.
fn substitute_once(text: &str, from: &str, to: &str) -> Result<String, Refusal> {
    match text.matches(from).count() {
        1 => Ok(text.replacen(from, to, 1)),
        n => Err(format!("{from:?} occurs {n} times, want exactly one")),
    }
}

// ---------------------------------------------------------------- server

/// A `serve` child on ephemeral loopback ports, its addresses read from its
/// own startup line. Dropped without `shutdown` (a failed assertion), the
/// child is killed by `kill_on_drop`; the success path kills and reaps it.
struct Server {
    child: Child,
    http: SocketAddr,
    stderr: Arc<Mutex<String>>,
    drains: Vec<tokio::task::JoinHandle<()>>,
}

impl Drop for Server {
    fn drop(&mut self) {
        for task in &self.drains {
            task.abort();
        }
        let _ = self.child.start_kill();
    }
}

/// The HTTP address from the startup line's tracing fields
/// (`smtp=… pop3=… http=…`), as `tests/binary.rs` reads it.
fn parse_http_addr(line: &str) -> Option<SocketAddr> {
    line.split("smtp=").nth(1)?;
    line.split("pop3=").nth(1)?;
    line.split("http=")
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// A task that fills a bounded buffer from a child's stderr; the buffer is
/// printed when a round trip fails.
fn drain_stderr(
    pipe: tokio::process::ChildStderr,
    sink: Arc<Mutex<String>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut lines = BufReader::new(pipe).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let mut sink = sink.lock().expect("stderr sink");
            if sink.len() < 64 * 1024 {
                sink.push_str(&line);
                sink.push('\n');
            }
        }
    })
}

/// The start command from the guide, argv unchanged, with its three listen
/// knobs pointed at ephemeral loopback ports by the child's environment.
async fn start_server(command: &str) -> Server {
    let program = defaults().program;
    let mut words = command.split_whitespace();
    assert_eq!(words.next(), Some(program.as_str()), "{command}");
    let mut child = Command::new(env!("CARGO_BIN_EXE_swarmail"))
        .args(words)
        .env("SWARMAIL_SMTP_LISTEN", "127.0.0.1:0")
        .env("SWARMAIL_POP3_LISTEN", "127.0.0.1:0")
        .env("SWARMAIL_HTTP_LISTEN", "127.0.0.1:0")
        .env_remove("SWARMAIL_DATA_FILE")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn swarmail serve");
    let stderr = Arc::new(Mutex::new(String::new()));
    let mut drains = vec![drain_stderr(
        child.stderr.take().expect("piped stderr"),
        stderr.clone(),
    )];
    let mut stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
    let mut transcript = String::new();
    let found = tokio::time::timeout(common::HELPER_DEADLINE, async {
        let mut line = String::new();
        loop {
            line.clear();
            match stdout.read_line(&mut line).await {
                Ok(0) => return Err("stdout closed before the startup line".to_string()),
                Ok(_) => {
                    transcript.push_str(&line);
                    if let Some(addr) = parse_http_addr(&line) {
                        return Ok(addr);
                    }
                }
                Err(e) => return Err(format!("reading stdout: {e}")),
            }
        }
    })
    .await;
    let http = match found {
        Ok(Ok(addr)) => addr,
        outcome => {
            let _ = child.start_kill();
            let _ = child.wait().await;
            for task in &drains {
                task.abort();
            }
            panic!(
                "serve never reported its addresses: {outcome:?}\nstdout: {transcript}\nstderr: {}",
                stderr.lock().expect("stderr sink")
            );
        }
    };
    // The rest of stdout is drained so the child never blocks on a full pipe.
    drains.push(tokio::spawn(async move {
        let mut sink = Vec::new();
        let _ = stdout.read_to_end(&mut sink).await;
    }));
    Server {
        child,
        http,
        stderr,
        drains,
    }
}

impl Server {
    /// Kill and witness the reap.
    async fn shutdown(mut self) {
        let _ = self.child.start_kill();
        let reaped = tokio::time::timeout(common::HELPER_DEADLINE, self.child.wait()).await;
        assert!(
            matches!(reaped, Ok(Ok(_))),
            "the server was not reaped: {reaped:?}"
        );
    }

    fn stderr(&self) -> String {
        self.stderr.lock().expect("stderr sink").clone()
    }
}

// ---------------------------------------------------------------- clients

/// One MCP client over either transport the snippets declare.
enum Client {
    Http { addr: SocketAddr, path: String },
    Stdio(Box<Bridge>),
}

struct Bridge {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl Bridge {
    /// The snippet's argv unchanged; only the binary's path stands for the
    /// program name, and the server's address arrives in `SWARMAIL_URL`.
    fn spawn(args: &[String], server: SocketAddr) -> Bridge {
        let mut child = Command::new(env!("CARGO_BIN_EXE_swarmail"))
            .args(args)
            .env("SWARMAIL_URL", format!("http://{server}"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .expect("spawn swarmail mcp");
        let stdin = child.stdin.take();
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
        Bridge {
            child,
            stdin,
            stdout,
        }
    }

    async fn send(&mut self, request: &Value) {
        let line = format!("{request}\n");
        let stdin = self.stdin.as_mut().expect("stdin is open");
        stdin.write_all(line.as_bytes()).await.expect("feed bridge");
        stdin.flush().await.expect("flush bridge");
    }

    async fn reply(&mut self, id: u64) -> Value {
        loop {
            let mut line = String::new();
            let n = self.stdout.read_line(&mut line).await.expect("read bridge");
            assert!(n > 0, "the bridge closed before answering id {id}");
            let reply: Value =
                serde_json::from_str(&line).unwrap_or_else(|e| panic!("{line:?} is not JSON: {e}"));
            if reply["id"] == id {
                return reply;
            }
        }
    }

    /// Close stdin: the bridge answers what is in flight and exits 0.
    async fn finish(mut self) {
        drop(self.stdin.take());
        let status = tokio::time::timeout(common::HELPER_DEADLINE, self.child.wait()).await;
        match status {
            Ok(Ok(status)) => assert!(status.success(), "the bridge exited {status:?}"),
            other => {
                let _ = self.child.start_kill();
                let _ = self.child.wait().await;
                panic!("the bridge did not exit at EOF: {other:?}");
            }
        }
    }
}

impl Client {
    /// A request's `result`, panicking on a JSON-RPC error.
    async fn call(&mut self, id: u64, method: &str, params: Value) -> Value {
        let request = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let reply = tokio::time::timeout(EXCHANGE, async {
            match self {
                Client::Http { addr, path } => {
                    let (status, body) = common::http_send_text_with_headers(
                        *addr,
                        "POST",
                        path,
                        &[("Accept", "application/json, text/event-stream")],
                        Some(&request.to_string()),
                    )
                    .await;
                    assert_eq!(status, 200, "{method}: {body}");
                    serde_json::from_str::<Value>(&body)
                        .unwrap_or_else(|e| panic!("{method}: {body:?} is not JSON: {e}"))
                }
                Client::Stdio(bridge) => {
                    bridge.send(&request).await;
                    bridge.reply(id).await
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("{method} did not answer within {EXCHANGE:?}"));
        assert!(reply.get("error").is_none(), "{method}: {reply}");
        reply["result"].clone()
    }

    async fn notify(&mut self, method: &str) {
        let request = json!({"jsonrpc": "2.0", "method": method});
        match self {
            Client::Http { addr, path } => {
                let (status, body) = common::http_send_text_with_headers(
                    *addr,
                    "POST",
                    path,
                    &[],
                    Some(&request.to_string()),
                )
                .await;
                assert_eq!(status, 202, "{method}: {body}");
            }
            Client::Stdio(bridge) => bridge.send(&request).await,
        }
    }

    /// A tool's JSON text result.
    async fn tool(&mut self, id: u64, name: &str, arguments: Value) -> Value {
        let result = self
            .call(
                id,
                "tools/call",
                json!({"name": name, "arguments": arguments}),
            )
            .await;
        assert_ne!(result["isError"], true, "{name}: {result}");
        let text = result["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("{name}: no text in {result}"));
        serde_json::from_str(text).unwrap_or_else(|e| panic!("{name}: {text:?}: {e}"))
    }
}

/// The names `tools/list` answers.
fn tool_names(list: &Value) -> BTreeSet<String> {
    list["tools"]
        .as_array()
        .expect("a tools array")
        .iter()
        .map(|t| t["name"].as_str().expect("a tool name").to_string())
        .collect()
}

/// The guide's loop against whichever transport the snippet declared:
/// `initialize`, `tools/list`, clear, seed, wait, extract. Returns the tool
/// names it listed.
async fn round_trip(client: &mut Client, inbox: &str) -> BTreeSet<String> {
    let init = client
        .call(
            1,
            "initialize",
            json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "agent-setup-docs", "version": "0"}
            }),
        )
        .await;
    assert_eq!(init["protocolVersion"], "2025-03-26", "{init}");
    client.notify("notifications/initialized").await;
    let names = tool_names(&client.call(2, "tools/list", json!({})).await);
    assert!(!names.is_empty(), "tools/list is empty");

    client
        .tool(3, "swarmail_clear_inbox", json!({"inbox": inbox}))
        .await;
    client
        .tool(
            4,
            "swarmail_seed_email",
            json!({
                "inbox": inbox,
                "to": format!("{inbox}@example.test"),
                "from": "noreply@example.test",
                "subject": "Your verification code",
                "text": "Your verification code is 482916. It expires soon."
            }),
        )
        .await;
    let waited = client
        .tool(
            5,
            "swarmail_wait_for_email",
            json!({"inbox": inbox, "timeout_ms": 20000}),
        )
        .await;
    assert!(
        waited.get("timed_out").is_none(),
        "a false timeout: {waited}"
    );
    let id = waited["emails"][0]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("no email in {waited}"))
        .to_string();
    let codes = client
        .tool(6, "swarmail_extract_codes", json!({"id": id}))
        .await;
    assert_eq!(codes["codes"], json!(["482916"]), "{codes}");
    names
}

// ---------------------------------------------------------------- drive

/// Where each wiring fence lands in a fresh project.
const PROJECT_FILES: [&str; 5] = [
    ".mcp.json",
    ".mcp.json",
    ".codex/config.toml",
    ".cursor/mcp.json",
    "swarmail.yml",
];

/// One wiring fence, end to end: write it verbatim into a fresh project,
/// read it back, check it against the binary's defaults, start the guide's
/// server, speak the declared transport.
async fn drive(position: usize) {
    let fences = guide_fences();
    let ran = executed(&fences);
    let (start, fence) = (ran[0], ran[1 + position]);
    let project = tempfile::tempdir().expect("a fresh project");
    let file = project.path().join(PROJECT_FILES[position]);
    std::fs::create_dir_all(file.parent().expect("a parent")).expect("project dir");
    std::fs::write(&file, &fence.body).expect("write the snippet verbatim");
    let on_disk = std::fs::read_to_string(&file).expect("read the snippet back");
    assert_eq!(on_disk, fence.body, "the bytes written are the bytes read");

    let read_back = Fence {
        body: on_disk.clone(),
        ..fence.clone()
    };
    let wiring = reduce(&read_back).unwrap_or_else(|e| panic!("{}: {e}", PROJECT_FILES[position]));
    let defaults = defaults();
    assert_defaults(&wiring, &defaults).unwrap_or_else(|e| panic!("{e}"));

    let server = start_server(start.body.trim()).await;
    let http = server.http;
    let mut client = match wiring {
        Wiring::Http { .. } => {
            // The one asserted substitution: the default authority becomes
            // the ephemeral one the server bound.
            let live = substitute_once(&on_disk, &defaults.authority, &http.to_string())
                .unwrap_or_else(|e| panic!("{e}"));
            let live = Fence {
                body: live,
                ..read_back
            };
            let Wiring::Http { url } = reduce(&live).unwrap_or_else(|e| panic!("{e}")) else {
                panic!("a substituted URL stays an HTTP wiring");
            };
            let rest = url
                .strip_prefix("http://")
                .unwrap_or_else(|| panic!("{url} is not http://"));
            let (host, path) = rest.split_once('/').expect("a path");
            assert_eq!(host.parse::<SocketAddr>().expect("an authority"), http);
            Client::Http {
                addr: http,
                path: format!("/{path}"),
            }
        }
        Wiring::Stdio { args, .. } => Client::Stdio(Box::new(Bridge::spawn(&args, http))),
    };
    let inbox = format!("setup-{position}");
    let task = tokio::spawn(async move {
        let names = round_trip(&mut client, &inbox).await;
        if let Client::Stdio(bridge) = client {
            bridge.finish().await;
        }
        names
    });
    match tokio::time::timeout(ROUND_TRIP, task).await {
        Ok(Ok(names)) => assert!(names.contains("swarmail_wait_for_email"), "{names:?}"),
        outcome => panic!(
            "the round trip for {} failed: {outcome:?}\nserver stderr: {}",
            PROJECT_FILES[position],
            server.stderr()
        ),
    }
    server.shutdown().await;
}

#[tokio::test]
async fn claude_code_http_snippet_wires_a_round_trip() {
    drive(0).await;
}

#[tokio::test]
async fn claude_code_stdio_snippet_wires_a_round_trip() {
    drive(1).await;
}

#[tokio::test]
async fn codex_toml_snippet_wires_a_round_trip() {
    drive(2).await;
}

#[tokio::test]
async fn cursor_json_snippet_wires_a_round_trip() {
    drive(3).await;
}

#[tokio::test]
async fn dsh_yaml_snippet_wires_a_round_trip() {
    drive(4).await;
}

// ---------------------------------------------------------------- AC-4

#[test]
fn every_fence_of_both_guide_files_is_executed_or_marked() {
    let fences = guide_fences();
    assert_eq!(fences.len(), 9, "two install fences and seven executed");
    let marked: Vec<_> = fences.iter().filter(|f| f.marker.is_some()).collect();
    assert_eq!(marked.len(), 2);
    assert!(marked.iter().all(|f| f.info == Info::Sh));
    assert_eq!(
        executed(&fences)
            .iter()
            .filter(|f| f.info == Info::Sh)
            .map(|f| f.body.trim())
            .collect::<Vec<_>>(),
        [START]
    );
    let skill = extract(&raw(SKILL)).unwrap_or_else(|e| panic!("{SKILL}: {e}"));
    assert!(skill.is_empty(), "the skill is its body: no fences");
}

#[test]
fn the_scanner_refuses_what_it_cannot_run() {
    let fine = "intro\n\n```json\n{}\n```\n";
    assert_eq!(extract(fine).expect("fine").len(), 1);
    let cases: [(&str, &str, &str); 13] = [
        ("CR byte", "```json\r\n{}\n```\n", "CR byte"),
        ("tilde fence", "~~~json\n{}\n~~~\n", "tilde fence"),
        ("indented fence", "  ```json\n{}\n  ```\n", "indented"),
        ("four backticks", "````json\n{}\n````\n", "four or more"),
        ("unterminated", "```json\n{}\n", "unterminated"),
        ("unknown info", "```ruby\nx\n```\n", "unknown info"),
        ("no info", "```\nx\n```\n", "unknown info"),
        ("unmarked text", "```text\nx\n```\n", "unmarked text"),
        (
            "unmarked non-start sh",
            "```sh\nrm -rf x\n```\n",
            "unmarked sh",
        ),
        (
            "empty reason",
            "<!-- not-executed: -->\n```sh\nx\n```\n",
            "needs a reason",
        ),
        (
            "marker then blank line",
            "<!-- not-executed: why -->\n\n```sh\nx\n```\n",
            "not followed by a fence",
        ),
        (
            "marker on json",
            "<!-- not-executed: why -->\n```json\n{}\n```\n",
            "never marked",
        ),
        (
            "orphan marker at the end",
            "```json\n{}\n```\n<!-- not-executed: why -->\n",
            "not followed by a fence",
        ),
    ];
    for (name, doc, why) in cases {
        let refusal = extract(doc).expect_err(name);
        assert!(refusal.contains(why), "{name}: {refusal}");
    }
    // The valid shapes the guide uses stay accepted.
    for ok in [
        "<!-- not-executed: needs the network -->\n```sh\ndocker pull x\n```\n",
        "<!-- not-executed: why -->\n```text\nx\n```\n",
        "```sh\nswarmail serve\n```\n",
    ] {
        assert_eq!(extract(ok).expect(ok).len(), 1);
    }
    // A malformed marker (no closer) or one that shares its line is refused,
    // as is two markers in a row and a fence opened inside a fence.
    for bad in [
        "<!-- not-executed: why\n```sh\nx\n```\n",
        "text <!-- not-executed: why -->\n",
        "<!-- not-executed: a -->\n<!-- not-executed: b -->\n```sh\nx\n```\n",
        "```json\n```json\n```\n",
    ] {
        extract(bad).expect_err(bad);
    }
}

#[test]
fn the_guide_shape_refuses_a_missing_or_second_block() {
    let fences = guide_fences();
    let mut without = fences.clone();
    without.retain(|f| f.info != Info::Markdown);
    guide_shape(&without).expect_err("no markdown block");
    let mut twice = fences.clone();
    twice.push(
        fences
            .iter()
            .find(|f| f.info == Info::Markdown)
            .expect("the block")
            .clone(),
    );
    guide_shape(&twice).expect_err("two markdown blocks");
    let mut missing_wiring = fences;
    missing_wiring.retain(|f| f.info != Info::Yaml);
    guide_shape(&missing_wiring).expect_err("no dsh row");
}

// ---------------------------------------------------------------- AC-2 red

#[test]
fn the_snippets_reduce_to_the_binarys_defaults() {
    let defaults = defaults();
    let want = Wiring::Http {
        url: format!("{}/mcp", defaults.url),
    };
    for position in [0, 2, 3, 4] {
        let wiring = reduce(&wiring_fence(position)).expect("reduces");
        assert_eq!(wiring, want, "snippet {position}");
        assert_defaults(&wiring, &defaults).expect("the defaults");
    }
    let stdio = reduce(&wiring_fence(1)).expect("reduces");
    assert_eq!(
        stdio,
        Wiring::Stdio {
            command: defaults.program.clone(),
            args: vec!["mcp".into()],
        }
    );
    assert_defaults(&stdio, &defaults).expect("the defaults");
}

#[test]
fn a_changed_port_host_path_command_or_argument_is_refused() {
    let defaults = defaults();
    let http = wiring_fence(0).body;
    for (what, from, to) in [
        ("port", "8025", "9999"),
        ("host", "127.0.0.1", "localhost"),
        ("path", "/mcp", "/rpc"),
        ("scheme", "http://", "https://"),
    ] {
        let mutated = http.replace(from, to);
        assert_ne!(mutated, http, "{what} mutation applied");
        let wiring = reduce_json(&mutated).expect("still reduces");
        assert_defaults(&wiring, &defaults).expect_err(what);
    }
    let stdio = wiring_fence(1).body;
    for (what, from, to) in [
        (
            "command",
            "\"command\": \"swarmail\"",
            "\"command\": \"swarmail-x\"",
        ),
        ("argument", "\"mcp\"", "\"serve\""),
        (
            "extra argument",
            "[\"mcp\"]",
            "[\"mcp\", \"--url\", \"http://127.0.0.1:1\"]",
        ),
    ] {
        let mutated = stdio.replace(from, to);
        assert_ne!(mutated, stdio, "{what} mutation applied");
        let wiring = reduce_json(&mutated).expect("still reduces");
        assert_defaults(&wiring, &defaults).expect_err(what);
    }
    // The same refusal through the TOML and YAML readers.
    for position in [2, 4] {
        let fence = wiring_fence(position);
        let mutated = fence.body.replace("8025", "9999");
        assert_ne!(mutated, fence.body);
        let wiring = reduce(&Fence {
            body: mutated,
            ..fence
        })
        .expect("still reduces");
        assert_defaults(&wiring, &defaults).expect_err("port");
    }
}

#[test]
fn the_subset_readers_refuse_everything_outside_the_subset() {
    for (name, body) in [
        ("not json", "{"),
        ("no mcpServers", "{}"),
        (
            "extra root key",
            r#"{"mcpServers":{"swarmail":{"url":"u"}},"x":1}"#,
        ),
        ("another server", r#"{"mcpServers":{"other":{"url":"u"}}}"#),
        ("no swarmail", r#"{"mcpServers":{}}"#),
        ("not an object", r#"{"mcpServers":{"swarmail":3}}"#),
        (
            "neither url nor command",
            r#"{"mcpServers":{"swarmail":{}}}"#,
        ),
        (
            "an env",
            r#"{"mcpServers":{"swarmail":{"command":"c","args":[],"env":{}}}}"#,
        ),
        (
            "wrong type",
            r#"{"mcpServers":{"swarmail":{"type":"sse","url":"u"}}}"#,
        ),
        (
            "url not a string",
            r#"{"mcpServers":{"swarmail":{"url":1}}}"#,
        ),
        (
            "args not an array",
            r#"{"mcpServers":{"swarmail":{"command":"c","args":"x"}}}"#,
        ),
        (
            "an arg not a string",
            r#"{"mcpServers":{"swarmail":{"command":"c","args":[1]}}}"#,
        ),
        ("mcpServers not an object", r#"{"mcpServers":[]}"#),
    ] {
        reduce_json(body).expect_err(name);
    }
    for (name, body) in [
        ("no table", "url = \"u\"\n"),
        ("another table", "[mcp_servers.other]\nurl = \"u\"\n"),
        (
            "two tables",
            "[mcp_servers.swarmail]\n[mcp_servers.swarmail]\n",
        ),
        ("not a pair", "[mcp_servers.swarmail]\nurl\n"),
        ("a dotted key", "[mcp_servers.swarmail]\na.b = 1\n"),
        ("a non-JSON value", "[mcp_servers.swarmail]\nurl = 'u'\n"),
        (
            "a repeated key",
            "[mcp_servers.swarmail]\nurl = \"u\"\nurl = \"v\"\n",
        ),
        ("an empty file", "# nothing\n"),
        (
            "a key before the table",
            "url = \"u\"\n[mcp_servers.swarmail]\n",
        ),
    ] {
        reduce_toml(body).expect_err(name);
    }
    let client = |config: &str| {
        format!(
            "- {{\"insert\": [{{\"id\": \"mcp-swarmail\", \"name\": \"@deepseek-ai/dsh-mcp-client\"{config}}}]}}\n"
        )
    };
    let config = |transport: &str, url: &str| {
        client(&format!(
            ", \"config\": {{\"serverName\": \"swarmail\", \"transport\": \"{transport}\", \"toolCallTimeoutMs\": 60000{url}}}"
        ))
    };
    for (name, body) in [
        ("not a list row", "insert: []\n".to_string()),
        ("a block mapping", "- insert:\n    - id: x\n".to_string()),
        (
            "two rows",
            "- {\"insert\": []}\n- {\"insert\": []}\n".to_string(),
        ),
        ("no rows", "# nothing\n".to_string()),
        ("a row without insert", "- {\"id\": \"x\"}\n".to_string()),
        ("two clients", "- {\"insert\": [{}, {}]}\n".to_string()),
        (
            "another client",
            "- {\"insert\": [{\"id\": \"x\", \"name\": \"y\", \"config\": {}}]}\n".to_string(),
        ),
        ("no config", client("")),
        ("wrong transport", config("stdio", ", \"url\": \"u\"")),
        ("no url", config("streamable-http", "")),
    ] {
        reduce_yaml(&body).expect_err(name);
    }
    reduce(&Fence {
        info: Info::Sh,
        marker: None,
        body: START.into(),
        line: 1,
    })
    .expect_err("sh is not wiring");
    // Comments and blank lines are the subset's only slack.
    let toml = "# a comment\n\n[mcp_servers.swarmail]\nurl = \"http://x/mcp\"\n";
    assert_eq!(
        reduce_toml(toml).expect("subset"),
        Wiring::Http {
            url: "http://x/mcp".into()
        }
    );
}

#[test]
fn the_port_substitution_demands_exactly_one_occurrence() {
    let body = wiring_fence(0).body;
    let authority = defaults().authority;
    let live = substitute_once(&body, &authority, "127.0.0.1:41000").expect("once");
    assert!(live.contains("127.0.0.1:41000") && !live.contains(&authority));
    substitute_once(&body, "127.0.0.1:1", "x").expect_err("zero occurrences");
    let twice = format!("{body}{body}");
    substitute_once(&twice, &authority, "x").expect_err("two occurrences");
}

// ---------------------------------------------------------------- AC-5

/// Every `swarmail_…` token of lowercase letters and underscores that is
/// not the tail of a longer identifier.
fn tool_tokens(text: &str) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(at) = text[from..].find("swarmail_") {
        let start = from + at;
        let mut end = start + "swarmail_".len();
        while end < bytes.len() && (bytes[end].is_ascii_lowercase() || bytes[end] == b'_') {
            end += 1;
        }
        let glued =
            start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_');
        if !glued {
            tokens.insert(text[start..end].to_string());
        }
        from = end;
    }
    tokens
}

async fn live_tool_names() -> BTreeSet<String> {
    let server = start_server(START).await;
    let mut client = Client::Http {
        addr: server.http,
        path: "/mcp".into(),
    };
    let list = client.call(1, "tools/list", json!({})).await;
    server.shutdown().await;
    tool_names(&list)
}

#[tokio::test]
async fn every_tool_name_in_the_guides_is_a_live_tool() {
    let live = live_tool_names().await;
    assert!(live.len() >= 15, "{live:?}");
    let mut seen = BTreeSet::new();
    for path in [GUIDE, SKILL] {
        let tokens = tool_tokens(&raw(path));
        let unknown: Vec<_> = tokens.difference(&live).collect();
        assert!(
            unknown.is_empty(),
            "{path} names unknown tools: {unknown:?}"
        );
        seen.extend(tokens);
    }
    for tool in [
        "swarmail_clear_inbox",
        "swarmail_seed_email",
        "swarmail_wait_for_email",
        "swarmail_wait_for_code",
        "swarmail_wait_for_link",
        "swarmail_extract_codes",
        "swarmail_extract_links",
    ] {
        assert!(seen.contains(tool), "the guides never name {tool}");
    }
}

#[test]
fn a_misspelt_tool_name_is_a_token_the_live_list_lacks() {
    let live: BTreeSet<String> = ["swarmail_clear_inbox".to_string()].into();
    let tokens = tool_tokens(
        "call `swarmail_clear_inbox`, not `swarmail_clear_inbx`; SWARMAIL_URL; xswarmail_a",
    );
    assert_eq!(
        tokens.difference(&live).collect::<Vec<_>>(),
        [&"swarmail_clear_inbx".to_string()]
    );
    assert!(tool_tokens("a bare swarmail_ stub").contains("swarmail_"));
}

// ---------------------------------------------------------------- AC-6

/// `FailureClass`'s variants from `evals/src/state.rs`, as snake_case. The
/// root suite reads the file as text and depends on no evals code.
fn enum_members(source: &str) -> Result<BTreeSet<String>, Refusal> {
    let (_, rest) = source
        .split_once("pub enum FailureClass {")
        .ok_or("evals/src/state.rs no longer declares `pub enum FailureClass {`")?;
    let (body, _) = rest
        .split_once("\n}")
        .ok_or("the enum has no closing brace")?;
    let members: BTreeSet<String> = body
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with('#'))
        .map(|l| snake(l.trim_end_matches(',')))
        .collect();
    if members.is_empty() {
        return Err("the enum has no variants".into());
    }
    Ok(members)
}

fn snake(camel: &str) -> String {
    let mut out = String::new();
    for (i, c) in camel.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// The step labels in order, each after the last.
fn steps_in_order(block: &str) -> Result<(), Refusal> {
    let mut from = 0;
    for step in ["**Clear.**", "**Act.**", "**Wait.**", "**Extract.**"] {
        let at = block[from..]
            .find(step)
            .ok_or_else(|| format!("{step} is missing or out of order"))?;
        from += at + step.len();
    }
    Ok(())
}

/// The backticked heads of the bullets under `### Failure classes`.
fn failure_classes(block: &str) -> Result<Vec<String>, Refusal> {
    let (_, section) = block
        .split_once("### Failure classes\n")
        .ok_or("no `### Failure classes` section")?;
    let mut classes = Vec::new();
    for line in section.lines() {
        if line.starts_with('#') {
            break;
        }
        if let Some(rest) = line.strip_prefix("- `") {
            let (name, _) = rest.split_once('`').ok_or("an unclosed class name")?;
            classes.push(name.to_string());
        }
    }
    Ok(classes)
}

fn check_block(block: &str, members: &BTreeSet<String>) -> Result<(), Refusal> {
    steps_in_order(block)?;
    let classes = failure_classes(block)?;
    for class in &classes {
        if !members.contains(class) {
            return Err(format!("{class:?} is not a FailureClass member"));
        }
    }
    let named: BTreeSet<&str> = classes.iter().map(String::as_str).collect();
    let want: BTreeSet<&str> = CLASSES.into();
    if named != want || classes.len() != CLASSES.len() {
        return Err(format!("the block names {classes:?}, want {CLASSES:?}"));
    }
    Ok(())
}

fn members() -> BTreeSet<String> {
    enum_members(&raw("evals/src/state.rs")).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn the_block_teaches_four_steps_and_six_failure_classes() {
    let members = members();
    assert_eq!(members.len(), 8, "{members:?}");
    for class in CLASSES {
        assert!(members.contains(class), "{class} is not in the enum");
    }
    check_block(&block(), &members).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn the_block_checks_go_red_on_a_removed_step_or_class_or_an_invented_one() {
    let members = members();
    let block = block();
    check_block(&block.replace("**Wait.**", "**Pause.**"), &members).expect_err("no wait step");
    let swapped = block
        .replace("**Clear.**", "**TMP.**")
        .replace("**Act.**", "**Clear.**")
        .replace("**TMP.**", "**Act.**");
    check_block(&swapped, &members).expect_err("act before clear");
    let removed: String = block
        .lines()
        .filter(|l| !l.starts_with("- `gave_up`"))
        .map(|l| format!("{l}\n"))
        .collect();
    check_block(&removed, &members).expect_err("no gave_up");
    let invented = format!("{block}- `made_up_class`: not in the enum.\n");
    let refusal = check_block(&invented, &members).expect_err("invented class");
    assert!(refusal.contains("made_up_class"), "{refusal}");
    let extra = format!("{block}- `budget`: not the agent's fault.\n");
    check_block(&extra, &members).expect_err("budget is not a block class");
    check_block("**Clear.** **Act.** **Wait.** **Extract.**", &members).expect_err("no section");
    enum_members("fn main() {}").expect_err("no enum");
    enum_members("pub enum FailureClass {\n    A,").expect_err("no closing brace");
    enum_members("pub enum FailureClass {\n}").expect_err("no variants");
    assert_eq!(snake("WrongMessage"), "wrong_message");
}

/// The text after `---`, `name: swarmail`, a one-line `description:` and
/// `---` plus one blank line.
fn skill_body(skill: &str) -> Result<String, Refusal> {
    let rest = skill
        .strip_prefix("---\nname: swarmail\ndescription: ")
        .ok_or("frontmatter is not `name: swarmail` then `description:`")?;
    let (description, body) = rest
        .split_once("\n---\n\n")
        .ok_or("the frontmatter does not close with a blank line after")?;
    if description.is_empty() || description.contains('\n') || description.contains(": ") {
        return Err(format!(
            "description {description:?} is not a plain one-line scalar"
        ));
    }
    Ok(body.to_string())
}

#[test]
fn the_skill_is_the_block_behind_its_frontmatter() {
    let skill = raw(SKILL);
    let body = skill_body(&skill).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(body, block(), "SKILL.md's body is the guide's block");
    // Byte for byte: one changed byte is red.
    let mutated = skill.replacen("Clear.", "Clean.", 1);
    assert_ne!(skill_body(&mutated).expect("frontmatter"), block());
    for bad in [
        "no frontmatter\n",
        "---\nname: swarmail\n---\n\nbody",
        "---\nname: other\ndescription: d\n---\n\nbody",
        "---\nname: swarmail\ndescription: a: b\n---\n\nbody",
        "---\nname: swarmail\ndescription: d\n---\nbody",
    ] {
        skill_body(bad).expect_err(bad);
    }
}

// ---------------------------------------------------------------- AC-7/8

const GUARANTEES: [&str; 4] = [
    "A `250` (or a successful seed) means the mail is queryable now by the running server; when a data file is configured the mail is on disk before that reply; without one, a restart loses mail. Never re-send after a `250`",
    "The wait registers before it scans, so a mail that arrives between the two can never be missed. Wait after act, and never poll-then-decide",
    "Codes and links are extracted at ingest, so extract reads stored results and a cleared or pruned mail is gone from them too",
    "Clear before you act, never between act and extract: the clear deletes the mail you are waiting for.",
];

const BLOCK_RULES: [&str; 5] = [
    "Clear before you act, never between act and extract: the clear deletes the mail you are waiting for.",
    "The wait registers before it scans, so wait after act and never poll-then-decide.",
    "Codes and links are extracted at ingest, so extract reads stored results and a cleared or pruned mail is gone from them too.",
    "Never re-send after it.",
    "Report it, then look at the inbox before you re-trigger; a second trigger makes the first code stale.",
];

const LABELS: [&str; 7] = [
    "**Unverified:** the file path.",
    "**Unverified:** Cursor's file name and shape.",
    "**Unverified:** the install path and the frontmatter.",
    "CI proves two things about each snippet above: the bytes parse, and the server answers on the transport the snippet declares.",
    "CI does not prove that Claude Code, Codex, Cursor or dsh load these files. That is an operator-owned receipt.",
    "schema-evidenced, not live-proven",
    "has not been run against a real model",
];

#[test]
fn the_guides_state_the_guarantees_the_loop_depends_on() {
    let guide = squash(&raw(GUIDE));
    assert_eq!(absent(&guide, &GUARANTEES), Vec::<&str>::new());
    let block = squash(&block());
    assert_eq!(absent(&block, &BLOCK_RULES), Vec::<&str>::new());
    // The skill carries the same rules, being the same bytes.
    assert_eq!(
        absent(&squash(&raw(SKILL)), &BLOCK_RULES),
        Vec::<&str>::new()
    );
    // A check-then-subscribe order is never taught.
    for text in [&guide, &block] {
        for forbidden in ["check, then subscribe", "poll until", "sleep and retry"] {
            assert!(!text.contains(forbidden), "{forbidden:?} is taught");
        }
    }
}

#[test]
fn removing_a_pinned_sentence_goes_red() {
    let guide = squash(&raw(GUIDE));
    for phrase in GUARANTEES.iter().chain(LABELS.iter()) {
        let without = guide.replace(phrase, "");
        assert_eq!(absent(&without, &[phrase]), [*phrase], "{phrase}");
    }
    let block = squash(&block());
    for phrase in BLOCK_RULES {
        let without = block.replace(phrase, "");
        assert_eq!(absent(&without, &[phrase]), [phrase], "{phrase}");
    }
}

#[test]
fn the_guide_labels_what_no_source_backs_and_what_ci_cannot_prove() {
    let guide = squash(&raw(GUIDE));
    assert_eq!(absent(&guide, &LABELS), Vec::<&str>::new());
}

#[test]
fn the_readme_points_at_the_guide_and_not_at_an_unexercised_block() {
    let readme = raw("README.md");
    assert!(
        readme.contains("(docs/agents/setup.md)"),
        "README links the guide"
    );
    assert!(
        !readme.contains("mcpServers"),
        "the unexercised .mcp.json snippet is gone from the README"
    );
    assert!(readme.contains("| `tests/agent_setup_docs.rs` |"));
}
