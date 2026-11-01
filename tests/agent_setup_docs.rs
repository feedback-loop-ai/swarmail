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
//! * Those five round trips seed through `swarmail_seed_email` with an
//!   explicit inbox, so they prove each snippet's transport and nothing about
//!   where an application's mail lands. The taught loop and route are proved
//!   once, over real SMTP, the way the AGENTS.md block tells the application
//!   to send (`the_taught_route_and_loop_hold_over_real_smtp`): the block's
//!   one `smtp://` URL is compared with the binary's default, substituted
//!   once for the ephemeral address and the inbox, and mail is sent with the
//!   URL's user as the AUTH name. The already-mailed entry state, two
//!   wrong-route controls and the route-domain clause are driven on the wire
//!   too. Every wait there has `timeout_ms: 0`: mail accepted with a `250` is
//!   queryable now (decision 0001) and a wait scans what is stored after it
//!   subscribes (decision 0002), so found and timed-out are deterministic and
//!   no control costs a timeout or sleeps.
//!
//! The suite speaks real HTTP, NDJSON and SMTP to the shipped binary and
//! touches no crate-private seam: a decision 0020 rule-3 test, outside the
//! coverage denominator by location.

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
/// string at column 0; everything fence-shaped that is not one is refused,
/// a tilde fence and an indented fence of either kind included.
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
        } else if ["```", "~~~"]
            .iter()
            .any(|marker| line.trim_start().starts_with(marker))
        {
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
    /// `SWARMAIL_SMTP_LISTEN`'s default, the authority the block's
    /// `smtp://` URL names.
    smtp_authority: String,
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
        smtp_authority: serve.smtp_listen,
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

/// The block's one `smtp://` URL, split.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SmtpUrl {
    token: String,
    user: String,
    authority: String,
}

/// `smtp://user:password@authority`, all three parts non-empty.
fn split_smtp(token: &str) -> Result<SmtpUrl, Refusal> {
    let rest = token
        .strip_prefix("smtp://")
        .ok_or_else(|| format!("{token:?} is not an smtp:// URL"))?;
    let (userinfo, authority) = rest
        .rsplit_once('@')
        .ok_or_else(|| format!("{token:?} names no user"))?;
    let (user, password) = userinfo
        .split_once(':')
        .ok_or_else(|| format!("{token:?} names no password"))?;
    if user.is_empty() || password.is_empty() || authority.is_empty() {
        return Err(format!("{token:?} has an empty part"));
    }
    Ok(SmtpUrl {
        token: token.into(),
        user: user.into(),
        authority: authority.into(),
    })
}

/// The block's `smtp://` URL: exactly one mention in the whole block, and it
/// is an inline-code token.
fn smtp_url(block: &str) -> Result<SmtpUrl, Refusal> {
    let mentions = block.matches("smtp://").count();
    let tokens: Vec<&str> = block
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|t| t.starts_with("smtp://"))
        .collect();
    match (mentions, tokens.as_slice()) {
        (1, [token]) => split_smtp(token),
        _ => Err(format!(
            "the block mentions smtp:// {mentions} times and carries {} inline URLs, want one of each",
            tokens.len()
        )),
    }
}

/// The URL names the inbox as the user and the binary's own default SMTP
/// authority; a second copy of `1025` lives nowhere in this suite.
fn assert_smtp_url(url: &SmtpUrl, defaults: &Defaults) -> Result<(), Refusal> {
    if url.user != "<inbox>" {
        return Err(format!("the URL's user is {:?}, want <inbox>", url.user));
    }
    if url.authority != defaults.smtp_authority {
        return Err(format!(
            "the URL's authority {:?} is not the default {:?}",
            url.authority, defaults.smtp_authority
        ));
    }
    Ok(())
}

/// The URL as the suite uses it: the default authority replaced by the
/// ephemeral SMTP address and `<inbox>` by the test's inbox, each exactly
/// once. What the suite sends is what the block, substituted, says.
fn live_url(url: &SmtpUrl, smtp: SocketAddr, inbox: &str) -> Result<SmtpUrl, Refusal> {
    let token = substitute_once(&url.token, &defaults().smtp_authority, &smtp.to_string())?;
    split_smtp(&substitute_once(&token, "<inbox>", inbox)?)
}

// ---------------------------------------------------------------- server

/// A `serve` child on ephemeral loopback ports, its addresses read from its
/// own startup line. Dropped without `shutdown` (a failed assertion), the
/// child is killed by `kill_on_drop`; the success path kills and reaps it.
struct Server {
    child: Child,
    smtp: SocketAddr,
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

/// The addresses a server bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Addrs {
    smtp: SocketAddr,
    http: SocketAddr,
}

/// One `key=addr` field of the startup line's tracing fields.
fn field_addr(line: &str, key: &str) -> Option<SocketAddr> {
    line.split(key)
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// The SMTP and HTTP addresses from the startup line's tracing fields
/// (`smtp=… pop3=… http=…`), as `tests/binary.rs` reads them.
fn parse_addrs(line: &str) -> Option<Addrs> {
    line.split("pop3=").nth(1)?;
    Some(Addrs {
        smtp: field_addr(line, "smtp=")?,
        http: field_addr(line, "http=")?,
    })
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
    start_server_with(command, &[]).await
}

/// `start_server` with extra variables on the child's environment (never on
/// this process's). The route domain is removed first, so an ambient value
/// can never turn the default server into a routed one.
async fn start_server_with(command: &str, extra_env: &[(&str, &str)]) -> Server {
    let program = defaults().program;
    let mut words = command.split_whitespace();
    assert_eq!(words.next(), Some(program.as_str()), "{command}");
    let mut child = Command::new(env!("CARGO_BIN_EXE_swarmail"))
        .args(words)
        .env("SWARMAIL_SMTP_LISTEN", "127.0.0.1:0")
        .env("SWARMAIL_POP3_LISTEN", "127.0.0.1:0")
        .env("SWARMAIL_HTTP_LISTEN", "127.0.0.1:0")
        .env_remove("SWARMAIL_DATA_FILE")
        .env_remove("SWARMAIL_ROUTE_DOMAIN")
        .envs(extra_env.iter().copied())
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
                    if let Some(addrs) = parse_addrs(&line) {
                        return Ok(addrs);
                    }
                }
                Err(e) => return Err(format!("reading stdout: {e}")),
            }
        }
    })
    .await;
    let addrs = match found {
        Ok(Ok(addrs)) => addrs,
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
        smtp: addrs.smtp,
        http: addrs.http,
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

/// An HTTP client that numbers its own requests, for the wire tests.
struct Wire {
    client: Client,
    id: u64,
}

impl Wire {
    fn http(server: &Server) -> Wire {
        Wire {
            client: Client::Http {
                addr: server.http,
                path: "/mcp".into(),
            },
            id: 0,
        }
    }

    /// A non-error tool's JSON text result.
    async fn tool(&mut self, name: &str, arguments: Value) -> Value {
        self.id += 1;
        self.client.tool(self.id, name, arguments).await
    }

    /// A tool's whole `result`, `isError` included.
    async fn tool_raw(&mut self, name: &str, arguments: Value) -> Value {
        self.id += 1;
        self.client
            .call(
                self.id,
                "tools/call",
                json!({"name": name, "arguments": arguments}),
            )
            .await
    }

    /// `swarmail_wait_for_email` with `timeout_ms: 0`. Mail accepted with a
    /// `250` is already stored (0001) and the wait scans after it subscribes
    /// (0002), so it answers found or timed out at once, with no deadline to
    /// wait out and no sleep.
    async fn wait0(&mut self, inbox: &str, filter: Value) -> Value {
        let mut arguments = json!({"inbox": inbox, "timeout_ms": 0});
        if let (Some(into), Some(more)) = (arguments.as_object_mut(), filter.as_object()) {
            into.extend(more.clone());
        }
        self.tool("swarmail_wait_for_email", arguments).await
    }

    async fn clear(&mut self, inbox: &str) {
        self.tool("swarmail_clear_inbox", json!({"inbox": inbox}))
            .await;
    }

    /// The codes `swarmail_extract_codes` reads from a stored mail.
    async fn codes(&mut self, id: &str) -> Value {
        self.tool("swarmail_extract_codes", json!({"id": id})).await["codes"].clone()
    }
}

/// The id of the first mail of a wait that found something.
fn found(answer: &Value) -> String {
    assert!(answer.get("timed_out").is_none(), "timed out: {answer}");
    answer["emails"][0]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("no email in {answer}"))
        .to_string()
}

fn assert_timed_out(answer: &Value) {
    assert_eq!(answer["timed_out"], true, "{answer}");
    assert_eq!(answer["matched"], 0, "{answer}");
}

/// What the application under test does: one message over real SMTP, with
/// the AUTH name the block's URL carries (`None` sends no AUTH).
async fn mail_code(smtp: SocketAddr, auth: Option<&str>, to: &str, code: &str) {
    let reply = common::smtp_send(
        smtp,
        auth,
        "app@example.test",
        to,
        "Your verification code",
        &format!("Your verification code is {code}. It expires soon."),
    )
    .await
    .unwrap_or_else(|e| panic!("the server refused the mail: {e}"));
    assert!(reply.starts_with("250"), "{reply}");
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

// ---------------------------------------------------------------- the route

/// The field the block's `wait_timeout` line tells the reader to look at:
/// its one inline-code token besides the class name.
fn near_miss_field(block: &str) -> Result<String, Refusal> {
    let line = bullet(block, "wait_timeout")?;
    let fields: Vec<&str> = code_spans(&line)
        .into_iter()
        .filter(|t| *t != "wait_timeout")
        .collect();
    match fields.as_slice() {
        [field] => Ok((*field).to_string()),
        other => Err(format!("the wait_timeout line names fields {other:?}")),
    }
}

/// The block's field is a member of a live timed-out answer.
fn check_near_miss(field: &str, keys: &BTreeSet<String>) -> Result<(), Refusal> {
    if keys.contains(field) {
        return Ok(());
    }
    Err(format!("{field:?} is not among the live keys {keys:?}"))
}

fn keys_of(answer: &Value) -> BTreeSet<String> {
    answer
        .as_object()
        .expect("an object answer")
        .keys()
        .cloned()
        .collect()
}

#[test]
fn the_blocks_smtp_url_is_the_binarys_default_and_occurs_once() {
    let block = block();
    let defaults = defaults();
    let url = smtp_url(&block).unwrap_or_else(|e| panic!("{e}"));
    assert_smtp_url(&url, &defaults).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(url.user, "<inbox>");
    // The substitution names the ephemeral address and the inbox, once each.
    let smtp: SocketAddr = "127.0.0.1:41025".parse().expect("an address");
    let live = live_url(&url, smtp, "route-x").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(live.user, "route-x");
    assert_eq!(live.authority, "127.0.0.1:41025");
    // Red controls: another authority, two URLs, none, another user.
    let moved = block.replace(&defaults.smtp_authority, "127.0.0.1:1026");
    assert_ne!(moved, block);
    let moved = smtp_url(&moved).expect("still one URL");
    assert_smtp_url(&moved, &defaults).expect_err("authority 127.0.0.1:1026");
    let twice = format!("{block}\nAlso `smtp://<inbox>:x@127.0.0.1:1025`.\n");
    smtp_url(&twice).expect_err("two inline URLs");
    let bare = format!("{block}\nAlso smtp://<inbox>:x@127.0.0.1:1025 bare.\n");
    smtp_url(&bare).expect_err("a bare second mention");
    smtp_url(&block.replace("smtp://", "smtp:/")).expect_err("no URL");
    let other_user = block.replace("smtp://<inbox>:x@", "smtp://agent:x@");
    assert_ne!(other_user, block);
    let other_user = smtp_url(&other_user).expect("still one URL");
    assert_smtp_url(&other_user, &defaults).expect_err("user agent");
    // The URL splitter and the substitution refuse what they cannot use.
    for bad in [
        "http://u:p@h",
        "smtp://h",
        "smtp://u@h",
        "smtp://:p@h",
        "smtp://u:@h",
        "smtp://u:p@",
    ] {
        split_smtp(bad).expect_err(bad);
    }
    live_url(&url, smtp, "<inbox>").expect("an inbox may be any text, once");
    let no_inbox = SmtpUrl {
        token: "smtp://agent:x@127.0.0.1:1025".into(),
        ..url
    };
    live_url(&no_inbox, smtp, "a").expect_err("no <inbox> token to substitute");
}

#[tokio::test]
async fn a_started_server_reports_a_loopback_smtp_address_distinct_from_http() {
    let server = start_server(START).await;
    assert!(server.smtp.ip().is_loopback(), "{}", server.smtp);
    assert_ne!(server.smtp, server.http);
    assert_eq!(
        parse_addrs(&format!(
            "up smtp={} pop3=127.0.0.1:2 http={}",
            server.smtp, server.http
        )),
        Some(Addrs {
            smtp: server.smtp,
            http: server.http
        })
    );
    for bad in [
        "smtp=127.0.0.1:1 http=127.0.0.1:2",
        "pop3=x smtp=y http=z",
        "",
    ] {
        assert_eq!(parse_addrs(bad), None, "{bad}");
    }
    server.shutdown().await;
}

/// The taught loop and route, as the block tells the application to send:
/// mail over real SMTP with the URL's user as the AUTH name, read by the
/// taught tools. Every wait has `timeout_ms: 0` (see `Wire::wait0`).
#[tokio::test]
async fn the_taught_route_and_loop_hold_over_real_smtp() {
    let block = block();
    let url = smtp_url(&block).unwrap_or_else(|e| panic!("{e}"));
    assert_smtp_url(&url, &defaults()).unwrap_or_else(|e| panic!("{e}"));
    let near = near_miss_field(&block).unwrap_or_else(|e| panic!("{e}"));
    let server = start_server(START).await;
    let mut wire = Wire::http(&server);
    let to = "user@example.test";
    let inbox_of = |name: &str| {
        let live = live_url(&url, server.smtp, name).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(live.authority, server.smtp.to_string());
        live.user
    };

    // The loop: clear, act (the application mails the inbox the URL names),
    // wait, extract. The seed tool is not on this path.
    let a = inbox_of("route-loop");
    assert_eq!(a, "route-loop");
    wire.clear(&a).await;
    mail_code(server.smtp, Some(&a), to, "135790").await;
    let id = found(&wire.wait0(&a, json!({})).await);
    assert_eq!(wire.codes(&id).await, json!(["135790"]));

    // Already mailed: the code was delivered before the agent started and
    // nothing is cleared. The wait returns it at once and extract reads it.
    let b = inbox_of("route-mailed");
    mail_code(server.smtp, Some(&b), to, "246801").await;
    let id = found(&wire.wait0(&b, json!({})).await);
    assert_eq!(wire.codes(&id).await, json!(["246801"]));
    // A timed-out wait on an inbox that holds other mail lists it under the
    // field the block names; the field is a member of the live answer.
    let missed = wire.wait0(&b, json!({"subject": "no-such-subject"})).await;
    assert_timed_out(&missed);
    let keys = keys_of(&missed);
    check_near_miss(&near, &keys).unwrap_or_else(|e| panic!("{e}"));
    let made_up = block.replace(&format!("`{near}`"), "`near_miss`");
    assert_ne!(made_up, block);
    let field = near_miss_field(&made_up).expect("still one field");
    check_near_miss(&field, &keys).expect_err("a made-up near-miss field");
    // The control that shows why the rule exists: a clear deletes it.
    wire.clear(&b).await;
    let emptied = wire.wait0(&b, json!({})).await;
    assert_timed_out(&emptied);
    assert!(
        !keys_of(&emptied).contains(&near),
        "an empty inbox lists no near misses: {emptied}"
    );

    // Wrong routes. Mail sent with no AUTH is in `default`, mail sent with
    // another AUTH name is in that inbox, and neither is in the named one.
    let c = inbox_of("route-control");
    wire.clear(&c).await;
    mail_code(server.smtp, None, to, "357913").await;
    assert_timed_out(&wire.wait0(&c, json!({})).await);
    let id = found(&wire.wait0("default", json!({})).await);
    assert_eq!(wire.codes(&id).await, json!(["357913"]));
    mail_code(server.smtp, Some("other"), to, "468024").await;
    assert_timed_out(&wire.wait0(&c, json!({})).await);
    let id = found(&wire.wait0("other", json!({})).await);
    assert_eq!(wire.codes(&id).await, json!(["468024"]));
    server.shutdown().await;
}

/// The conditional clause: `swarmail_create_inbox` answers an error naming
/// `--route-domain` on the guide's server, and with `SWARMAIL_ROUTE_DOMAIN`
/// in the child's environment (the knob the guide names beside the flag) it
/// mints an address that routes whatever the AUTH name.
#[tokio::test]
async fn the_route_domain_clause_mints_and_routes() {
    let server = start_server(START).await;
    let mut wire = Wire::http(&server);
    let refused = wire.tool_raw("swarmail_create_inbox", json!({})).await;
    assert_eq!(refused["isError"], true, "{refused}");
    let text = refused["content"][0]["text"].as_str().expect("a message");
    assert!(text.contains("--route-domain"), "{text}");
    server.shutdown().await;

    let server = start_server_with(START, &[("SWARMAIL_ROUTE_DOMAIN", "swarmail.test")]).await;
    let mut wire = Wire::http(&server);
    let minted = wire.tool("swarmail_create_inbox", json!({})).await;
    let inbox = minted["inbox"].as_str().expect("an inbox").to_string();
    let address = minted["address"].as_str().expect("an address").to_string();
    assert!(
        address.ends_with(&format!("@{inbox}.swarmail.test")),
        "{minted}"
    );
    mail_code(server.smtp, Some("other"), &address, "579135").await;
    let id = found(&wire.wait0(&inbox, json!({})).await);
    assert_eq!(wire.codes(&id).await, json!(["579135"]));
    assert_timed_out(&wire.wait0("other", json!({})).await);
    server.shutdown().await;
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
    let cases: [(&str, &str, &str); 15] = [
        ("CR byte", "```json\r\n{}\n```\n", "CR byte"),
        ("tilde fence", "~~~json\n{}\n~~~\n", "tilde fence"),
        ("indented fence", "  ```json\n{}\n  ```\n", "indented"),
        (
            "indented tilde fence",
            "  ~~~json\n{}\n  ~~~\n",
            "indented or tilde",
        ),
        ("indented bare tilde", "  ~~~\n", "indented or tilde"),
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

/// What a running `tools/list` answers.
async fn live_list() -> Value {
    let server = start_server(START).await;
    let mut client = Client::Http {
        addr: server.http,
        path: "/mcp".into(),
    };
    let list = client.call(1, "tools/list", json!({})).await;
    server.shutdown().await;
    list
}

async fn live_tool_names() -> BTreeSet<String> {
    tool_names(&live_list().await)
}

/// The property names of one tool's live `inputSchema`.
fn property_names(list: &Value, tool: &str) -> BTreeSet<String> {
    list["tools"]
        .as_array()
        .expect("a tools array")
        .iter()
        .find(|t| t["name"] == tool)
        .unwrap_or_else(|| panic!("{tool} is not a live tool"))["inputSchema"]["properties"]
        .as_object()
        .expect("a properties object")
        .keys()
        .cloned()
        .collect()
}

/// The first-column tool names of the table under the guide's `The tools`
/// heading.
fn inventory_names(guide: &str) -> Result<BTreeSet<String>, Refusal> {
    let mut lines = guide
        .lines()
        .skip_while(|l| !(l.starts_with("## ") && l.contains("The tools")));
    lines.next().ok_or("no `## … The tools` heading")?;
    let mut names = BTreeSet::new();
    for line in lines {
        if line.starts_with("## ") {
            break;
        }
        let Some(rest) = line.strip_prefix("| `") else {
            continue;
        };
        let (name, _) = rest.split_once('`').ok_or("an unclosed tool name")?;
        if !names.insert(name.to_string()) {
            return Err(format!("{name} has two rows"));
        }
    }
    if names.is_empty() {
        return Err("the tools table has no rows".into());
    }
    Ok(names)
}

/// The table and the live list are the same set: a vanished tool and an
/// unlisted new one are both red, and no number is pinned.
fn check_inventory(table: &BTreeSet<String>, live: &BTreeSet<String>) -> Result<(), Refusal> {
    let missing: Vec<_> = live.difference(table).collect();
    let extra: Vec<_> = table.difference(live).collect();
    if missing.is_empty() && extra.is_empty() {
        return Ok(());
    }
    Err(format!(
        "the table lacks the live tools {missing:?} and lists non-tools {extra:?}"
    ))
}

#[tokio::test]
async fn the_tool_table_equals_the_live_list() {
    let live = live_tool_names().await;
    let guide = raw(GUIDE);
    let table = inventory_names(&guide).unwrap_or_else(|e| panic!("{e}"));
    check_inventory(&table, &live).unwrap_or_else(|e| panic!("{e}"));
    // A copy with a row removed, and a copy with a made-up row, are refused.
    let victim = "swarmail_clear_chaos";
    let without: String = guide
        .lines()
        .filter(|l| !l.starts_with(&format!("| `{victim}`")))
        .map(|l| format!("{l}\n"))
        .collect();
    let refusal = check_inventory(&inventory_names(&without).expect("a table"), &live)
        .expect_err("a row is missing");
    assert!(refusal.contains(victim), "{refusal}");
    let invented = guide.replace(
        &format!("| `{victim}` |"),
        "| `swarmail_made_up` | x |\n| `swarmail_clear_chaos` |",
    );
    let refusal = check_inventory(&inventory_names(&invented).expect("a table"), &live)
        .expect_err("a non-tool row");
    assert!(refusal.contains("swarmail_made_up"), "{refusal}");
    // The reader's own refusals.
    inventory_names("no heading\n").expect_err("no heading");
    inventory_names("## 8. The tools\n\nnothing\n").expect_err("no rows");
    inventory_names("## The tools\n| `a` |\n| `a` |\n").expect_err("a repeated row");
    inventory_names("## The tools\n| `a |\n").expect_err("an unclosed name");
}

/// The bullet under `### Failure classes` that starts with a class, its
/// wrapped lines joined.
fn bullet(block: &str, class: &str) -> Result<String, Refusal> {
    let head = format!("- `{class}`");
    let mut lines = block.lines().skip_while(|l| !l.starts_with(&head));
    let mut out = lines
        .next()
        .ok_or_else(|| format!("no bullet for {class}"))?
        .to_string();
    for line in lines.take_while(|l| l.starts_with("  ")) {
        out.push(' ');
        out.push_str(line.trim());
    }
    Ok(out)
}

/// The inline-code tokens of a line.
fn code_spans(line: &str) -> Vec<&str> {
    line.split('`').skip(1).step_by(2).collect()
}

/// The `wrong_link` bullet sends the reader to the tool that has the
/// selectors, and only to it: `swarmail_extract_links` returns plain URLs.
fn check_wrong_link(
    bullet: &str,
    wait_props: &BTreeSet<String>,
    extract_props: &BTreeSet<String>,
) -> Result<(), Refusal> {
    let spans = code_spans(bullet);
    if !spans.contains(&"swarmail_wait_for_link") || bullet.contains("swarmail_extract_links") {
        return Err("the wrong_link line must name swarmail_wait_for_link alone".into());
    }
    for selector in ["anchor_text", "host", "path"] {
        if !wait_props.contains(selector) || extract_props.contains(selector) {
            return Err(format!(
                "{selector} is not a selector of swarmail_wait_for_link alone"
            ));
        }
        if !spans.contains(&selector) {
            return Err(format!("the wrong_link line never names `{selector}`"));
        }
    }
    Ok(())
}

#[tokio::test]
async fn the_wrong_link_line_names_the_tool_that_has_the_selectors() {
    let list = live_list().await;
    let wait = property_names(&list, "swarmail_wait_for_link");
    let extract = property_names(&list, "swarmail_extract_links");
    let line = bullet(&block(), "wrong_link").unwrap_or_else(|e| panic!("{e}"));
    check_wrong_link(&line, &wait, &extract).unwrap_or_else(|e| panic!("{e}"));
    // The old advice (choose by selectors that extract_links does not have)
    // and a line missing a selector are refused.
    let on_extract = line.replace("swarmail_wait_for_link", "swarmail_extract_links");
    check_wrong_link(&on_extract, &wait, &extract).expect_err("selectors on extract_links");
    let both = format!("{line} Or `swarmail_extract_links`.");
    check_wrong_link(&both, &wait, &extract).expect_err("extract_links beside the selectors");
    let no_host = line.replace("`host`", "host");
    check_wrong_link(&no_host, &wait, &extract).expect_err("a selector unnamed");
    let mut fake = wait.clone();
    fake.remove("path");
    check_wrong_link(&line, &fake, &extract).expect_err("a selector the live schema lacks");
    let mut leaked = extract.clone();
    leaked.insert("host".into());
    check_wrong_link(&line, &wait, &leaked).expect_err("a selector extract_links has");
    bullet(&block(), "made_up").expect_err("no such bullet");
}

#[tokio::test]
async fn every_tool_name_in_the_guides_is_a_live_tool() {
    let live = live_tool_names().await;
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
        "swarmail_create_inbox",
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

const GUARANTEES: [&str; 5] = [
    "A `250` (or a successful seed) means the mail is queryable now by the running server; when a data file is configured the mail is on disk before that reply; without one, a restart loses mail. Never re-send after a `250`",
    "The wait registers before it scans, so a mail that arrives between the two can never be missed. Wait after act, and never poll-then-decide",
    "Codes and links are extracted at ingest, so extract reads stored results and a cleared or pruned mail is gone from them too",
    "Unless the flow has already mailed you, clear before you act, never between act and extract: the clear deletes the mail you are waiting for.",
    "If the flow has already mailed you, do not clear: the wait returns mail that is already stored, and a clear would delete it with no way back.",
];

const BLOCK_RULES: [&str; 11] = [
    "If your task or environment names your swarmail inbox, use that name and pass it as `\"inbox\"` to every tool.",
    "the SMTP AUTH username names the inbox and any password is accepted. Mail sent without AUTH lands in the inbox `default`",
    "On a server started with `--route-domain`, you may instead call `swarmail_create_inbox` and give the flow the address it returns.",
    "**If the flow has already mailed you, do not clear: wait, or read, straight away.** The wait returns mail that is already in your inbox.",
    "Otherwise work in four steps, in this order: clear, act, wait, extract.",
    "Clear before you act, never between act and extract: the clear deletes the mail you are waiting for.",
    "The wait registers before it scans, so wait after act and never poll-then-decide.",
    "Codes and links are extracted at ingest, so extract reads stored results and a cleared or pruned mail is gone from them too.",
    "Never re-send after it.",
    "then look at the inbox before you re-trigger; a second trigger makes the first code stale.",
    "take the code from the newest mail.",
];

/// Wording the review retracted (F1, F2): none of it may come back.
const FORBIDDEN_IN_BLOCK: [&str; 4] = [
    "Pick an inbox name that is yours alone",
    "with a recipient in your inbox",
    "always in this order",
    "after your clear",
];

const LABELS: [&str; 9] = [
    "**Unverified:** the file path.",
    "**Unverified:** Cursor's file name and shape.",
    "**Unverified:** the install path and the frontmatter.",
    "CI proves two things: the bytes parse, and the server answers on the transport the snippet declares.",
    "Those five round trips prove each snippet's transport and nothing about where an application's mail lands",
    "One more test drives the loop and the route the block teaches over real SMTP",
    "CI does not prove that Claude Code, Codex, Cursor or dsh load these files. That is an operator-owned receipt.",
    "schema-evidenced, not live-proven",
    "has not been run against a real model",
];

const README_CLAIM: &str = "The wiring snippets in `docs/agents/setup.md` are written into a fresh project and driven against the real binary by `tests/agent_setup_docs.rs` in CI; install commands are marked not-executed.";

/// The retracted wording that survives in `text`.
fn retracted(text: &str) -> Vec<&'static str> {
    FORBIDDEN_IN_BLOCK
        .iter()
        .copied()
        .filter(|phrase| text.contains(phrase))
        .collect()
}

/// The inbox paragraph comes before the already-mailed rule, which comes
/// before the four steps; the environment-assigned name is the first rule.
fn block_order(block: &str) -> Result<(), Refusal> {
    let text = squash(block);
    let mut from = 0;
    for (name, mark) in [
        (
            "the environment-assigned inbox",
            "If your task or environment names your swarmail inbox",
        ),
        ("the AUTH URL", "smtp://"),
        (
            "the unauthenticated inbox",
            "Mail sent without AUTH lands in the inbox `default`",
        ),
        ("the routed clause", "--route-domain"),
        (
            "the already-mailed rule",
            "**If the flow has already mailed you",
        ),
        ("the order sentence", "Otherwise work in four steps"),
        ("the first step", "**Clear.**"),
    ] {
        let at = text[from..]
            .find(mark)
            .ok_or_else(|| format!("{name} is missing or out of order"))?;
        from += at + mark.len();
    }
    Ok(())
}

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
    block_order(&block).unwrap_or_else(|e| panic!("{e}"));
    // A check-then-subscribe order is never taught.
    for text in [&guide, &block] {
        for forbidden in ["check, then subscribe", "poll until", "sleep and retry"] {
            assert!(!text.contains(forbidden), "{forbidden:?} is taught");
        }
    }
}

#[test]
fn the_block_never_teaches_the_retracted_wording() {
    for (what, text) in [("block", block()), ("skill", raw(SKILL))] {
        assert_eq!(retracted(&squash(&text)), Vec::<&str>::new(), "{what}");
    }
    // Red controls: each retracted phrase, put back, is found.
    let block = block();
    for phrase in FORBIDDEN_IN_BLOCK {
        let regressed = format!("{block}\n{phrase}\n");
        assert_eq!(retracted(&squash(&regressed)), [phrase], "{phrase}");
    }
    // The order sentence says so without \"always\", and the already-mailed
    // rule and the inbox paragraph are in front of the steps.
    block_order(&block).unwrap_or_else(|e| panic!("{e}"));
    let moved = block.replace(
        "**If the flow has already mailed you",
        "**If the flow mailed you",
    );
    block_order(&moved).expect_err("no already-mailed rule");
    let unordered = block.replace("Otherwise work in four steps", "Work in four steps");
    block_order(&unordered).expect_err("no order sentence after the rule");
    let no_assignment = block.replace(
        "If your task or environment names your swarmail inbox",
        "Choose an inbox",
    );
    block_order(&no_assignment).expect_err("no environment-assigned rule");
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
    let flat = squash(&readme);
    assert!(
        flat.contains(README_CLAIM),
        "the README claims only what is proven"
    );
    let overclaim = "Every snippet in it is written into a fresh project and driven";
    assert!(!flat.contains(overclaim), "the README overclaims");
    // The suite-table row names the SMTP-driven proof beside the transport.
    let row = readme
        .lines()
        .find(|l| l.starts_with("| `tests/agent_setup_docs.rs` |"))
        .expect("the suite row");
    for phrase in ["over real SMTP", "already-mailed"] {
        assert!(
            row.contains(phrase),
            "the suite row lacks {phrase:?}: {row}"
        );
    }
    // Red controls: the overclaim, put back, is found; the claim, removed, is gone.
    assert!(format!("{flat} {overclaim}").contains(overclaim));
    assert!(!flat.replace(README_CLAIM, "").contains(README_CLAIM));
}
