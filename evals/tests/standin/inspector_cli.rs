//! The client matrix's offline stand-in for the MCP Inspector CLI (issue
//! #392): test equipment, named by no production path. It takes the
//! Inspector's own CLI grammar (`--cli <url> --transport http --method
//! tools/list|tools/call [--tool-name N] [--tool-arg k=v]...`) and speaks
//! real MCP over Streamable HTTP: one `initialize`, the `initialized`
//! notification, then the one requested method, printing its result as
//! JSON on stdout. `--tool-arg` values are read as JSON when they parse,
//! else as strings, as the Inspector reads them.
//!
//! Before `--cli` it takes its own controls: `--revision R` asks for that
//! protocol revision, and `--omit M` (a method or tool name) skips that
//! request yet prints `{}` and exits 0, the successful exit with missing
//! wire evidence a receipt must not count. `--linger FILE` leaves a
//! `sleep` child in its process group and writes that child's pid to FILE,
//! so a test can prove the runner reaps descendants; `--flood` prints more
//! than the runner's output bound.
use serde_json::{Value, json};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    match runtime.block_on(run(&args)) {
        Ok(result) => {
            println!("{}", serde_json::to_string_pretty(&result).expect("json"));
        }
        Err(error) => {
            eprintln!("inspector-cli-standin: {error}");
            std::process::exit(1);
        }
    }
}

#[derive(Default)]
struct Args {
    revision: Option<String>,
    omit: Vec<String>,
    linger: Option<String>,
    flood: bool,
    url: String,
    method: String,
    tool: Option<String>,
    arguments: serde_json::Map<String, Value>,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let mut out = Args::default();
    let mut it = args.iter();
    let value = |it: &mut std::slice::Iter<String>, flag: &str| {
        it.next().cloned().ok_or(format!("{flag} needs a value"))
    };
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--revision" => out.revision = Some(value(&mut it, arg)?),
            "--omit" => out.omit.push(value(&mut it, arg)?),
            "--linger" => out.linger = Some(value(&mut it, arg)?),
            "--flood" => out.flood = true,
            "--cli" => out.url = value(&mut it, arg)?,
            "--transport" => {
                if value(&mut it, arg)? != "http" {
                    return Err("only --transport http".into());
                }
            }
            "--method" => out.method = value(&mut it, arg)?,
            "--tool-name" => out.tool = Some(value(&mut it, arg)?),
            "--tool-arg" => {
                let pair = value(&mut it, arg)?;
                let (key, raw) = pair.split_once('=').ok_or("--tool-arg needs key=value")?;
                let parsed = serde_json::from_str(raw).unwrap_or(Value::String(raw.into()));
                out.arguments.insert(key.into(), parsed);
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if out.url.is_empty() || out.method.is_empty() {
        return Err("--cli <url> and --method are required".into());
    }
    Ok(out)
}

struct Session {
    http: reqwest::Client,
    url: String,
    session: Option<String>,
    revision: Option<String>,
}

impl Session {
    async fn post(&mut self, body: Value) -> Result<Option<Value>, String> {
        let mut req = self
            .http
            .post(&self.url)
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream");
        if let Some(id) = &self.session {
            req = req.header("mcp-session-id", id);
        }
        if let Some(revision) = &self.revision {
            req = req.header("mcp-protocol-version", revision);
        }
        let reply = req
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if let Some(id) = reply.headers().get("mcp-session-id") {
            self.session = id.to_str().ok().map(str::to_string);
        }
        let status = reply.status();
        let bytes = reply.bytes().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("HTTP {status}"));
        }
        if bytes.is_empty() {
            return Ok(None);
        }
        let value: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        if let Some(error) = value.get("error") {
            return Err(format!("JSON-RPC error {error}"));
        }
        Ok(value.get("result").cloned())
    }
}

async fn run(args: &[String]) -> Result<Value, String> {
    let args = parse(args)?;
    if let Some(file) = &args.linger {
        let child = std::process::Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .map_err(|e| e.to_string())?;
        std::fs::write(file, child.id().to_string()).map_err(|e| e.to_string())?;
    }
    let http = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let mut session = Session {
        http,
        url: args.url.clone(),
        session: None,
        revision: None,
    };
    let asked = args.revision.clone().unwrap_or_else(|| "2025-11-25".into());
    let init = session
        .post(
            json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
                "protocolVersion": asked,
                "capabilities": {},
                "clientInfo": {"name": "inspector-cli-standin", "version": "0.0.0"},
            }}),
        )
        .await?
        .ok_or("initialize had no result")?;
    session.revision = init["protocolVersion"].as_str().map(str::to_string);
    session
        .post(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        .await?;
    let skipped = args
        .omit
        .iter()
        .any(|o| o == &args.method || Some(o) == args.tool.as_ref());
    if skipped {
        return Ok(json!({}));
    }
    let params = match args.method.as_str() {
        "tools/call" => json!({
            "name": args.tool.clone().ok_or("tools/call needs --tool-name")?,
            "arguments": Value::Object(args.arguments),
        }),
        _ => json!({}),
    };
    let mut result = session
        .post(json!({"jsonrpc": "2.0", "id": 1, "method": args.method, "params": params}))
        .await?
        .ok_or_else(|| "the method had no result".to_string())?;
    if args.flood {
        result["flood"] = json!("x".repeat(2 << 20));
    }
    Ok(result)
}
