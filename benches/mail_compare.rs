//! Fair wire comparison driver (issue #163, proposed decision 0091): one
//! executable that runs the frozen throughput and wait profiles against
//! real Swarmail, Mailpit, MailHog, MailSlurper, smtp4dev, Inbucket,
//! GreenMail and (separately labelled) Alkemio's patched MailSlurper
//! servers it launches and owns (proposed decision 0130). It is
//! experiment infrastructure, independent of the eval crate; the one entry
//! point is `scripts/compare-mail.sh`, which builds it and passes the
//! modes below. A plain `cargo bench` invocation measures nothing.
//!
//!   mail_compare check --protocol P --sources S [--documents DIR]
//!   mail_compare targets --protocol P
//!   mail_compare run --mode smoke|collect --protocol P --sources S
//!       [--documents DIR] --output DIR --scratch DIR --run-id ID --bin TARGET=PATH...
//!       [--runtime TARGET=DIR...] [--commit SHA --dirty true|false]
//!   mail_compare admit DIR
//!   mail_compare mailpit-mcp|mailhog-mcp|mailslurper-mcp --sources S
//!       (--endpoint http://127.0.0.1:PORT | --launch BIN --scratch DIR)
//!
//! Exit 0: the mode completed its declared checks. Exit 1: a complete,
//! sealed run whose workload failed somewhere (the report says where).
//! Exit 2: prerequisite, operational, interrupt, cleanup or accounting
//! failure — it dominates, and its output is never sealed.

#[path = "mail_compare/mailhog_mcp.rs"]
mod mailhog_mcp;
#[path = "mail_compare/mailpit_mcp.rs"]
mod mailpit_mcp;
#[path = "mail_compare/mailslurper_mcp.rs"]
mod mailslurper_mcp;
#[path = "mail_compare/mcp.rs"]
mod mcp;
#[path = "mail_compare/protocol.rs"]
mod protocol;
#[path = "mail_compare/records.rs"]
mod records;
#[path = "mail_compare/report.rs"]
mod report;
#[path = "mail_compare/runner.rs"]
mod runner;
#[path = "mail_compare/stats.rs"]
mod stats;
#[path = "mail_compare/targets.rs"]
mod targets;
#[path = "mail_compare/wire.rs"]
mod wire;

use protocol::{Protocol, SourceLock};
use runner::flags;
use std::collections::BTreeMap;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: mail_compare check|targets|run|admit|mailpit-mcp|mailhog-mcp|mailslurper-mcp … (see scripts/compare-mail.sh)";

fn main() -> ExitCode {
    // cargo bench appends `--bench`; it selects nothing here.
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| a != "--bench")
        .collect();
    let code = match args.first().map(String::as_str) {
        None => {
            eprintln!("mail_compare: no mode given, nothing measured. {USAGE}");
            0
        }
        Some("check") => check(&args[1..]),
        Some("targets") => targets_mode(&args[1..]),
        Some("run") => run(&args[1..]),
        Some("admit") => admit(&args[1..]),
        Some("mailpit-mcp") => adapter_mode::<mailpit_mcp::Mailpit>("mailpit-mcp", &args[1..]),
        Some("mailhog-mcp") => adapter_mode::<mailhog_mcp::Mailhog>("mailhog-mcp", &args[1..]),
        Some("mailslurper-mcp") => {
            adapter_mode::<mailslurper_mcp::Mailslurper>("mailslurper-mcp", &args[1..])
        }
        Some(_) => {
            eprintln!("{USAGE}");
            2
        }
    };
    ExitCode::from(code as u8)
}

fn one<'a>(f: &'a BTreeMap<String, Vec<String>>, name: &str) -> Result<&'a str, String> {
    f.get(name)
        .and_then(|v| v.first())
        .map(String::as_str)
        .ok_or_else(|| format!("--{name} is required"))
}

fn load(
    f: &BTreeMap<String, Vec<String>>,
) -> Result<(Protocol, Vec<u8>, SourceLock, Vec<u8>), String> {
    let pb = std::fs::read(one(f, "protocol")?).map_err(|e| format!("protocol: {e}"))?;
    let sb = std::fs::read(one(f, "sources")?).map_err(|e| format!("sources: {e}"))?;
    let p = Protocol::parse(&pb).map_err(|r| r.to_string())?;
    let s = SourceLock::parse(&sb).map_err(|r| r.to_string())?;
    // Version pairing is checked first: neither version reads the other's
    // records (proposed decision 0130 §3).
    protocol::pairing(&p, &s).map_err(|r| r.to_string())?;
    // The captured documents live beside the lock, or where `--documents`
    // says (a historical lock under `history/` is verified against the
    // documents in `docs/evals`); their bytes are rechecked every time the
    // lock is used, not trusted from last time.
    let dir = match f.get("documents").and_then(|v| v.first()) {
        Some(d) => Path::new(d),
        None => Path::new(one(f, "sources")?)
            .parent()
            .unwrap_or(Path::new(".")),
    };
    s.validate().map_err(|r| r.to_string())?;
    s.verify_files(if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    })
    .map_err(|r| format!("source documents: {r}"))?;
    Ok((p, pb, s, sb))
}

/// `mail_compare targets --protocol P`: the competitors the protocol
/// declares, one name per line, for the wrapper's required-binary list. The
/// candidate is built by the wrapper itself and never listed.
fn targets_mode(args: &[String]) -> i32 {
    let res = flags(args).and_then(|f| {
        if let Some(k) = f.keys().find(|k| k.as_str() != "protocol") {
            return Err(format!("unknown flag --{k}"));
        }
        let bytes = std::fs::read(one(&f, "protocol")?).map_err(|e| format!("protocol: {e}"))?;
        let p = Protocol::parse(&bytes).map_err(|r| r.to_string())?;
        p.validate().map_err(|r| r.to_string())?;
        Ok(p)
    });
    match res {
        Ok(p) => {
            for t in protocol::declared_competitors(&p) {
                println!("{t}");
            }
            0
        }
        Err(e) => {
            eprintln!("mail_compare targets: {e}");
            2
        }
    }
}

fn check(args: &[String]) -> i32 {
    let res = flags(args).and_then(|f| load(&f));
    let (p, _, s, _) = match res {
        Ok(v) => v,
        Err(e) => {
            eprintln!("mail_compare check: {e}");
            return 2;
        }
    };
    match protocol::collection_ready(&p, &s) {
        Ok(()) => {
            println!(
                "mail_compare check: protocol {} is ready for collection",
                p.id
            );
            0
        }
        Err(r) => {
            println!(
                "mail_compare check: protocol {} is NOT ready for collection:",
                p.id
            );
            for line in r.0 {
                println!("  - {line}");
            }
            2
        }
    }
}

fn admit(args: &[String]) -> i32 {
    let [dir, protocol] = args else {
        eprintln!("usage: mail_compare admit DIR PROTOCOL");
        return 2;
    };
    let admitted = std::fs::read(protocol)
        .map_err(|e| format!("protocol: {e}"))
        .and_then(|bytes| report::admit_run(Path::new(dir), &bytes));
    match admitted {
        Ok(a) => {
            println!(
                "mail_compare admit: run {} ({}) admitted with {} rows",
                a.manifest.run_id,
                a.manifest.mode,
                a.rows.len()
            );
            0
        }
        Err(e) => {
            eprintln!("mail_compare admit: refused: {e}");
            2
        }
    }
}

/// The MCP adapter over stdio for one backend. Two ways to reach it, never
/// both: `--endpoint URL` (the caller owns the backend and hands its
/// loopback address) or `--launch BIN --scratch DIR` (this process launches
/// the pinned backend through `targets::Owned`, announces `smtp=ADDR
/// tools_sha256=HEX` as its first output line, serves, and exits 0 only
/// after stdin ends and its cleanup facts are all true). The lock must pin
/// the backend either way. Stdout carries only the announcement and the
/// protocol; refusals go to stderr.
fn adapter_mode<B: mcp::Backend>(mode: &str, args: &[String]) -> i32 {
    let kind = B::TARGET;
    enum Via {
        Endpoint(std::net::SocketAddr),
        Launch(Box<targets::LaunchSpec>, PathBuf),
    }
    let ready = mcp::adapter_target(kind)
        .and_then(|()| flags(args))
        .and_then(|f| {
            if let Some(k) = f
                .keys()
                .find(|k| !["endpoint", "sources", "launch", "scratch"].contains(&k.as_str()))
            {
                return Err(format!("unknown flag --{k}"));
            }
            let path = one(&f, "sources")?;
            let s = SourceLock::parse(&std::fs::read(path).map_err(|e| format!("sources: {e}"))?)
                .map_err(|r| r.to_string())?;
            s.validate().map_err(|r| r.to_string())?;
            let dir = Path::new(path)
                .parent()
                .filter(|d| !d.as_os_str().is_empty());
            s.verify_files(dir.unwrap_or(Path::new(".")))
                .map_err(|r| format!("source documents: {r}"))?;
            s.pin(kind)
                .ok_or_else(|| format!("the source lock pins no {kind}"))?;
            match (f.contains_key("endpoint"), f.contains_key("launch")) {
                (true, false) if !f.contains_key("scratch") => {
                    Ok(Via::Endpoint(mcp::parse_endpoint(one(&f, "endpoint")?)?))
                }
                (false, true) => Ok(Via::Launch(
                    Box::new(mcp::launch_spec_for(
                        kind,
                        PathBuf::from(one(&f, "launch")?),
                        &s,
                    )?),
                    PathBuf::from(one(&f, "scratch").map_err(|_| "--launch needs --scratch DIR")?),
                )),
                _ => {
                    Err("give --endpoint URL, or --launch BIN with --scratch DIR, not both".into())
                }
            }
        });
    let via = match ready {
        Ok(v) => v,
        Err(e) => {
            eprintln!("mail_compare {mode}: refused: {e}");
            return 2;
        }
    };
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("mail_compare {mode}: runtime: {e}");
            return 2;
        }
    };
    let bounds = mcp::Bounds::default();
    match via {
        Via::Endpoint(addr) => {
            let end = rt.block_on(async {
                let http = wire::Http::new(addr, bounds.request, bounds.max_body);
                mcp::serve::<B, _, _>(tokio::io::stdin(), tokio::io::stdout(), http, bounds).await
            });
            match end {
                mcp::End::Eof => 0,
                other => {
                    eprintln!("mail_compare {mode}: ended: {other:?}");
                    2
                }
            }
        }
        Via::Launch(spec, scratch) => {
            let keepers = targets::Keepers::default();
            let limits = mcp::launch_limits();
            let outcome = rt.block_on(async {
                let clock = wire::Clock(std::time::Instant::now());
                let mut owned =
                    match targets::Owned::launch(&spec, &scratch, &limits, &clock, &keepers).await
                    {
                        Ok(o) => o,
                        Err(e) => return Err(format!("{kind} did not launch: {e:?}")),
                    };
                let mut term =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                        .ok();
                let term = async {
                    match term.as_mut() {
                        Some(t) => t.recv().await,
                        None => std::future::pending().await,
                    }
                };
                tokio::select! {
                    r = mcp::serve_owned::<B, _, _>(&mut owned, tokio::io::stdin(), tokio::io::stdout(), bounds) => Ok(Some(r)),
                    // Terminated: the backend is still cleaned up, but the
                    // exit is not a receipt.
                    _ = term => {
                        eprintln!("mail_compare {mode}: terminated; cleaning up");
                        let facts = owned.shutdown().await;
                        Ok(Some((mcp::End::OutputClosed, facts)))
                    }
                }
            });
            // Every worker is settled before exit, whatever happened.
            let settled = rt.block_on(keepers.settle(limits.cleanup * 2));
            match outcome {
                Ok(Some((end, facts))) => {
                    eprintln!(
                        "mail_compare {mode}: cleanup reaped={} group_gone={} listeners_closed={} scratch_removed={} forced={}",
                        facts.reaped,
                        facts.group_gone,
                        facts.listeners_closed,
                        facts.scratch_removed,
                        facts.forced_kill
                    );
                    if mcp::receipt(end, &facts)
                        && settled.iter().all(targets::CleanupFacts::proved)
                    {
                        0
                    } else {
                        eprintln!("mail_compare {mode}: ended {end:?}; no cleanup receipt");
                        2
                    }
                }
                Ok(None) => 2,
                Err(e) => {
                    eprintln!("mail_compare {mode}: {e}");
                    2
                }
            }
        }
    }
}

fn kernel() -> String {
    // SAFETY: uname fills a caller-owned struct.
    let mut u: libc::utsname = unsafe { std::mem::zeroed() };
    if unsafe { libc::uname(&mut u) } != 0 {
        return "unavailable".into();
    }
    let field = |f: &[libc::c_char]| {
        f.iter()
            .take_while(|c| **c != 0)
            .map(|c| *c as u8 as char)
            .collect::<String>()
    };
    format!("{} {}", field(&u.sysname), field(&u.release))
}

fn run(args: &[String]) -> i32 {
    match prepare(args) {
        Ok((cfg, scratch)) => execute(cfg, scratch),
        Err(e) => {
            eprintln!("mail_compare run: refused before launch: {e}");
            2
        }
    }
}

/// Everything that can refuse before a single process starts: inputs,
/// pins, binaries, output and scratch.
fn prepare(args: &[String]) -> Result<(runner::Config, PathBuf), String> {
    let f = runner::flags(args)?;
    let known = [
        "mode",
        "protocol",
        "sources",
        "documents",
        "output",
        "scratch",
        "run-id",
        "bin",
        "runtime",
        "commit",
        "dirty",
    ];
    if let Some(k) = f.keys().find(|k| !known.contains(&k.as_str())) {
        return Err(format!("unknown flag --{k}"));
    }
    let (p, pb, s, sb) = load(&f)?;
    let mode = one(&f, "mode")?;
    p.validate().map_err(|r| r.to_string())?;
    match mode {
        "collect" => protocol::collection_ready(&p, &s)
            .map_err(|r| format!("not ready for collection: {r}"))?,
        // A smoke run is labelled and never evidence, but it still launches
        // nothing without a pinned executable digest, and never with a
        // retention the protocol did not state.
        "smoke" => {
            protocol::retention_matches(&p, &s).map_err(|r| r.to_string())?;
            protocol::launchable(&p, &s).map_err(|r| r.to_string())?;
        }
        other => return Err(format!("unknown mode {other:?}")),
    }
    let run_id = one(&f, "run-id")?;
    if !protocol::is_name(run_id) {
        return Err("--run-id must be 1-64 [A-Za-z0-9._-]".into());
    }
    let none = Vec::new();
    let specs = runner::launch_specs(
        &p,
        &s,
        f.get("bin").unwrap_or(&none),
        f.get("runtime").unwrap_or(&none),
    )?;
    let output = PathBuf::from(one(&f, "output")?);
    if output.exists() {
        return Err("--output already exists; runs are never overwritten or resumed".into());
    }
    let scratch = PathBuf::from(one(&f, "scratch")?);
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&scratch)
        .map_err(|e| format!("--scratch must be a new directory: {e}"))?;
    let commit = f.get("commit").and_then(|v| v.first()).cloned();
    if commit.as_deref().is_some_and(|c| !protocol::is_hex(c, 40)) {
        return Err("--commit must be a full commit id".into());
    }
    let dirty = match f.get("dirty").and_then(|v| v.first()).map(String::as_str) {
        None => None,
        Some("true") => Some(true),
        Some("false") => Some(false),
        Some(_) => return Err("--dirty takes true or false".into()),
    };
    let manifest = records::Manifest {
        manifest_version: records::MANIFEST_VERSION,
        run_id: run_id.to_owned(),
        mode: mode.to_owned(),
        protocol_id: p.id.clone(),
        protocol_sha256: targets::sha256_hex(&pb),
        sources_sha256: targets::sha256_hex(&sb),
        candidate_commit: commit,
        candidate_dirty: dirty,
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        kernel: kernel(),
        cpus: std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(0),
        prelaunch_utc: chrono::Utc::now().to_rfc3339(),
        planned: runner::plan(&p),
    };
    let cfg = runner::Config {
        protocol: p,
        manifest,
        specs,
        output,
        scratch: scratch.clone(),
        keepers: targets::Keepers::default(),
    };
    Ok((cfg, scratch))
}

fn execute(cfg: runner::Config, scratch: PathBuf) -> i32 {
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("mail_compare run: runtime: {e}");
            return 2;
        }
    };
    let clock = wire::Clock(std::time::Instant::now());
    let output = cfg.output.clone();
    let keepers = cfg.keepers.clone();
    let cleanup_bound = runner::limits(&cfg.protocol).cleanup;
    let outcome = rt.block_on(async {
        let mut term =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).ok();
        let term = async {
            match term.as_mut() {
                Some(t) => t.recv().await,
                None => std::future::pending().await,
            }
        };
        tokio::select! {
            r = runner::run(cfg, clock) => Some(r),
            // Interrupted: dropping the run tells every owner's cleanup
            // worker to finish on its own; they are awaited below. The
            // output stays unsealed whatever they prove.
            _ = tokio::signal::ctrl_c() => None,
            _ = term => None,
        }
    });
    // Every worker, interrupted or not, is settled before exit; a runtime
    // dropped with one still running would be forced death.
    let settled = rt.block_on(keepers.settle(cleanup_bound * 2));
    let Some((verdict, life, sealed)) = outcome else {
        let proved = settled.iter().all(targets::CleanupFacts::proved);
        eprintln!(
            "mail_compare run: interrupted; {} stays incomplete and unsealed; cleanup of {} launch(es) {}",
            output.display(),
            settled.len(),
            if proved { "proved" } else { "NOT proved" }
        );
        return 2;
    };
    let scratch_gone = std::fs::remove_dir(&scratch).is_ok();
    for c in &life.cleanups {
        println!(
            "cleanup {}: reaped={} group_gone={} listeners_closed={} scratch_removed={} forced={}",
            c.target, c.reaped, c.group_gone, c.listeners_closed, c.scratch_removed, c.forced_kill
        );
    }
    if !scratch_gone {
        eprintln!("mail_compare run: scratch was not empty after cleanup; the run is incomplete");
        return 2;
    }
    match (&verdict, sealed) {
        (runner::Verdict::Infrastructure(e), _) => {
            eprintln!(
                "mail_compare run: infrastructure failure: {e}; {} is incomplete",
                output.display()
            );
        }
        (runner::Verdict::WorkloadFailures(f), Some(path)) => {
            for line in f {
                println!("workload failure: {line}");
            }
            println!("sealed {}", path.display());
        }
        (_, path) => println!("sealed {}", path.unwrap_or_default().display()),
    }
    verdict.exit_code()
}
