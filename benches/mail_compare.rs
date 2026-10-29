//! Fair wire comparison driver (issue #163, proposed decision 0091): one
//! executable that runs the frozen throughput and wait profiles against
//! real Swarmail, Mailpit and MailHog servers it launches and owns. It is
//! experiment infrastructure, independent of the eval crate; the one entry
//! point is `scripts/compare-mail.sh`, which builds it and passes the
//! modes below. A plain `cargo bench` invocation measures nothing.
//!
//!   mail_compare check --protocol P --sources S
//!   mail_compare run --mode smoke|collect --protocol P --sources S
//!       --output DIR --scratch DIR --run-id ID --bin TARGET=PATH...
//!       [--commit SHA --dirty true|false]
//!   mail_compare admit DIR
//!   mail_compare mailpit-mcp --endpoint http://127.0.0.1:PORT --sources S
//!
//! Exit 0: the mode completed its declared checks. Exit 1: a complete,
//! sealed run whose workload failed somewhere (the report says where).
//! Exit 2: prerequisite, operational, interrupt, cleanup or accounting
//! failure — it dominates, and its output is never sealed.

#[path = "mail_compare/mailpit_mcp.rs"]
mod mailpit_mcp;
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

use protocol::{Protocol, Role, SourceLock, TargetKind};
use std::collections::BTreeMap;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str =
    "usage: mail_compare check|run|admit|mailpit-mcp … (see scripts/compare-mail.sh)";

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
        Some("run") => run(&args[1..]),
        Some("admit") => admit(&args[1..]),
        Some("mailpit-mcp") => mailpit_mcp_mode(&args[1..]),
        Some(_) => {
            eprintln!("{USAGE}");
            2
        }
    };
    ExitCode::from(code as u8)
}

/// `--flag value` pairs; a repeated flag other than `--bin`, a flag without
/// a value or a stray word refuses.
fn flags(args: &[String]) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let name = flag
            .strip_prefix("--")
            .ok_or_else(|| format!("unexpected argument {flag:?}"))?;
        let value = it.next().ok_or_else(|| format!("--{name} needs a value"))?;
        let slot = out.entry(name.to_owned()).or_default();
        if !slot.is_empty() && name != "bin" {
            return Err(format!("--{name} given twice"));
        }
        slot.push(value.clone());
    }
    Ok(out)
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
    // The captured documents live beside the lock; their bytes are
    // rechecked every time the lock is used, not trusted from last time.
    let dir = Path::new(one(f, "sources")?)
        .parent()
        .unwrap_or(Path::new("."));
    s.validate().map_err(|r| r.to_string())?;
    s.verify_files(if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    })
    .map_err(|r| format!("source documents: {r}"))?;
    Ok((p, pb, s, sb))
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
    let Some(dir) = args.first().filter(|_| args.len() == 1) else {
        eprintln!("usage: mail_compare admit DIR");
        return 2;
    };
    match records::admit(Path::new(dir)) {
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

/// The MCP adapter over stdio. The caller owns the Mailpit server; the
/// lock must pin one, and the endpoint must be a literal loopback socket.
/// Stdout carries only the protocol; refusals go to stderr.
fn mailpit_mcp_mode(args: &[String]) -> i32 {
    let ready = flags(args).and_then(|f| {
        if let Some(k) = f
            .keys()
            .find(|k| !["endpoint", "sources"].contains(&k.as_str()))
        {
            return Err(format!("unknown flag --{k}"));
        }
        let addr = mailpit_mcp::parse_endpoint(one(&f, "endpoint")?)?;
        let path = one(&f, "sources")?;
        let s = SourceLock::parse(&std::fs::read(path).map_err(|e| format!("sources: {e}"))?)
            .map_err(|r| r.to_string())?;
        s.validate().map_err(|r| r.to_string())?;
        let dir = Path::new(path)
            .parent()
            .filter(|d| !d.as_os_str().is_empty());
        s.verify_files(dir.unwrap_or(Path::new(".")))
            .map_err(|r| format!("source documents: {r}"))?;
        s.pin(TargetKind::Mailpit)
            .ok_or("the source lock pins no Mailpit")?;
        Ok(addr)
    });
    let addr = match ready {
        Ok(a) => a,
        Err(e) => {
            eprintln!("mail_compare mailpit-mcp: refused: {e}");
            return 2;
        }
    };
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("mail_compare mailpit-mcp: runtime: {e}");
            return 2;
        }
    };
    let bounds = mailpit_mcp::Bounds::default();
    let end = rt.block_on(async {
        let http = wire::Http::new(addr, bounds.request, bounds.max_body);
        mailpit_mcp::serve(tokio::io::stdin(), tokio::io::stdout(), http, bounds).await
    });
    match end {
        mailpit_mcp::End::Eof => 0,
        other => {
            eprintln!("mail_compare mailpit-mcp: ended: {other:?}");
            2
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
    let f = flags(args)?;
    let known = [
        "mode", "protocol", "sources", "output", "scratch", "run-id", "bin", "commit", "dirty",
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
            for t in &p.targets {
                let ok = match s.entry(*t) {
                    Some(e) if *t == TargetKind::Swarmail => e.role == Role::Candidate,
                    Some(e) => e.pin.is_some(),
                    None => false,
                };
                if !ok {
                    return Err(format!("{t} has no source pin; it cannot be launched"));
                }
            }
        }
        other => return Err(format!("unknown mode {other:?}")),
    }
    let run_id = one(&f, "run-id")?;
    if !protocol::is_name(run_id) {
        return Err("--run-id must be 1-64 [A-Za-z0-9._-]".into());
    }
    let mut bins = BTreeMap::new();
    for b in f.get("bin").into_iter().flatten() {
        let (name, path) = b.split_once('=').ok_or("--bin takes TARGET=PATH")?;
        let t = TargetKind::parse(name).ok_or_else(|| format!("unknown target {name:?}"))?;
        if bins.insert(t, PathBuf::from(path)).is_some() {
            return Err(format!("--bin {t} given twice"));
        }
    }
    let mut specs = BTreeMap::new();
    for &t in &p.targets {
        let binary = bins
            .remove(&t)
            .ok_or_else(|| format!("--bin {t}=PATH is required"))?;
        let (expected_sha256, template) = match t {
            TargetKind::Swarmail => {
                let cap = p
                    .storage
                    .retention
                    .iter()
                    .find(|r| r.target == t)
                    .and_then(|r| r.max_messages);
                (None, targets::swarmail_template(cap.unwrap_or(0)))
            }
            _ => {
                let pin = s.pin(t).ok_or_else(|| format!("{t}: no source pin"))?;
                (
                    Some(pin.artifact.executable_sha256.clone()),
                    pin.launch.clone(),
                )
            }
        };
        specs.insert(
            t,
            targets::LaunchSpec {
                kind: t,
                binary,
                expected_sha256,
                template,
            },
        );
    }
    if let Some(t) = bins.keys().next() {
        return Err(format!(
            "--bin {t} names a target the protocol does not declare"
        ));
    }
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
