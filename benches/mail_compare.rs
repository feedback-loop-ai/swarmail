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
//!       [--chunk cost.TARGET.rN --systemd VERSION]     (protocol 4)
//!   mail_compare targets --protocol P --plan v4
//!   mail_compare admit DIR
//!   mail_compare mailpit-mcp|mailhog-mcp|mailslurper-mcp --sources S [--documents DIR]
//!       (--endpoint http://127.0.0.1:PORT | --launch BIN --scratch DIR)
//!
//!   mail_compare hold --protocol P --sources S [--documents DIR]
//!       --bin TARGET=PATH [--runtime TARGET=DIR] --scratch DIR --samples FILE
//!   mail_compare footprint --protocol P --sources S [--documents DIR]
//!       --bin TARGET=PATH... [--runtime TARGET=DIR...]
//!   mail_compare admit-v4 [--seal] DIR PROTOCOL --sources S [--documents DIR]
//!       [--bin TARGET=PATH...]
//!   mail_compare client-enter --unit U.scope --handshake FILE -- CMD...
//!   mail_compare client-judge --unit U.scope --handshake FILE --out FILE [--budget-ms N]
//!       (an MCP chunk's client scope, issue #385; Linux only)
//!
//! Exit 0: the mode completed its declared checks. Exit 1: a complete,
//! sealed run whose workload failed somewhere (the report says where).
//! Exit 2: prerequisite, operational, interrupt, cleanup or accounting
//! failure — it dominates, and its output is never sealed.

// The API section and its clients are shared with the eval crate and the
// tests (accepted decision 0132 §9.2): some items serve only those.
#[allow(dead_code)]
#[path = "mail_compare/api.rs"]
mod api;
#[path = "mail_compare/api_chunk.rs"]
mod api_chunk;
#[allow(dead_code)]
#[path = "mail_compare/clients/mod.rs"]
mod clients;
// The MCP chunk's client scope (issue #385): its handshake and judgment.
#[allow(dead_code)]
#[path = "mail_compare/client_scope.rs"]
mod client_scope;
// The stream wake-up transports and their observer (decision 0132 §7, story
// C). The SSE reader and the observer's test trace have no driver caller (no
// selected row uses an event stream), so those parts compile unused.
#[path = "mail_compare/mailhog_mcp.rs"]
mod mailhog_mcp;
#[path = "mail_compare/mailpit_mcp.rs"]
mod mailpit_mcp;
#[path = "mail_compare/mailslurper_mcp.rs"]
mod mailslurper_mcp;
#[path = "mail_compare/mcp.rs"]
mod mcp;
#[allow(dead_code)]
#[path = "mail_compare/clients/observe.rs"]
mod observe;
#[path = "mail_compare/protocol.rs"]
mod protocol;
#[path = "mail_compare/records.rs"]
mod records;
#[path = "mail_compare/report.rs"]
mod report;
#[path = "mail_compare/resources.rs"]
mod resources;
#[path = "mail_compare/runner.rs"]
mod runner;
#[path = "mail_compare/stats.rs"]
mod stats;
// The test-execution section (decision 0132 §8, story D): the plan and the
// record are shared with the eval crate, which runs the suite.
#[allow(dead_code)]
#[path = "mail_compare/clients/stream.rs"]
mod stream;
#[allow(dead_code)]
#[path = "mail_compare/suite.rs"]
mod suite;
#[path = "mail_compare/suite_chunk.rs"]
mod suite_chunk;
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

const USAGE: &str = "usage: mail_compare check|targets|run|admit|hold|footprint|admit-v4|client-enter|client-judge|mailpit-mcp|mailhog-mcp|mailslurper-mcp … (see scripts/compare-mail.sh)";

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
        Some("hold") => hold_mode(&args[1..]),
        Some("footprint") => footprint_mode(&args[1..]),
        Some("admit-v4") => admit_v4_mode(&args[1..]),
        Some("client-enter") => client_enter_mode(&args[1..]),
        Some("client-judge") => client_judge_mode(&args[1..]),
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
/// candidate is built by the wrapper itself and never listed. With
/// `--plan v4` it prints the protocol's chunk plan instead, one coordinate
/// per line: the wrapper never computes the plan.
fn targets_mode(args: &[String]) -> i32 {
    let mut plan = false;
    let res = flags(args).and_then(|f| {
        if let Some(k) = f
            .keys()
            .find(|k| !["protocol", "plan"].contains(&k.as_str()))
        {
            return Err(format!("unknown flag --{k}"));
        }
        if let Some(v) = f.get("plan") {
            if v.first().map(String::as_str) != Some("v4") {
                return Err("--plan takes v4".into());
            }
            plan = true;
        }
        let bytes = std::fs::read(one(&f, "protocol")?).map_err(|e| format!("protocol: {e}"))?;
        let p = Protocol::parse(&bytes).map_err(|r| r.to_string())?;
        p.validate().map_err(|r| r.to_string())?;
        Ok(p)
    });
    match res {
        Ok(p) if plan => {
            if p.protocol_version != protocol::PROTOCOL_VERSION_4 {
                eprintln!(
                    "mail_compare targets: --plan v4 needs protocol_version {}, not {}",
                    protocol::PROTOCOL_VERSION_4,
                    p.protocol_version
                );
                return 2;
            }
            for c in runner::plan_v4(&p) {
                println!("{}", c.name());
            }
            0
        }
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

/// Where a lock's captured documents live: `--documents`, else beside it.
fn docs_dir(f: &BTreeMap<String, Vec<String>>) -> Result<PathBuf, String> {
    Ok(match f.get("documents").and_then(|v| v.first()) {
        Some(d) => PathBuf::from(d),
        None => Path::new(one(f, "sources")?)
            .parent()
            .filter(|d| !d.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
            .to_path_buf(),
    })
}

/// `mail_compare footprint`: the fixed facts of 0132 §4.5, printed as one
/// JSON document. Nothing is launched and every digest relied on is rechecked.
/// `mail_compare client-enter`: run inside an MCP chunk's fresh client
/// scope, before its workload exists. It records the kernel's word on this
/// process's membership in a new handshake file, then execs the workload in
/// place, so every process the chunk starts inherits the scope (issue
/// #385). Nothing is exec'd unless the handshake is written.
fn client_enter_mode(args: &[String]) -> i32 {
    let res = client_scope::platform(std::env::consts::OS)
        .and_then(|()| client_scope::parse_args(args, true))
        .and_then(|a| {
            let text = std::fs::read_to_string("/proc/self/cgroup")
                .map_err(|e| format!("/proc/self/cgroup: {e}"))?;
            let hs = client_scope::own_handshake(&a.unit, &text)?;
            client_scope::confirm_member(
                Path::new(client_scope::CGROUP_MOUNT),
                &hs.cgroup,
                hs.pid,
            )?;
            let bytes = serde_json::to_vec(&hs).map_err(|e| e.to_string())?;
            client_scope::write_exclusive(&a.handshake, &bytes)?;
            Ok(a)
        });
    let a = match res {
        Ok(a) => a,
        Err(e) => {
            eprintln!("mail_compare client-enter: {e}; nothing was started");
            return 2;
        }
    };
    use std::os::unix::process::CommandExt;
    let e = std::process::Command::new(&a.command[0])
        .args(&a.command[1..])
        .exec();
    eprintln!(
        "mail_compare client-enter: cannot exec {}: {e}",
        a.command[0]
    );
    2
}

/// `mail_compare client-judge`: judge an MCP chunk's client scope the
/// moment its workload has ended, within one deadline, and write the
/// receipt the collector puts into the chunk's ledger entry. Exit 0 only
/// for affirmative clean evidence; a survivor or an unknown is 2.
fn client_judge_mode(args: &[String]) -> i32 {
    let a = match client_scope::platform(std::env::consts::OS)
        .and_then(|()| client_scope::parse_args(args, false))
    {
        Ok(a) => a,
        Err(e) => {
            eprintln!("mail_compare client-judge: {e}");
            return 2;
        }
    };
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("mail_compare client-judge: runtime: {e}");
            return 2;
        }
    };
    let probe = client_scope::Kernel::new(&a.handshake);
    let receipt = rt.block_on(client_scope::judge(&probe, &a.unit, a.budget));
    // A read the deadline cut may still sit on a blocking thread: the
    // runtime is let go without waiting for it.
    rt.shutdown_background();
    eprintln!("mail_compare client-judge: {}", receipt.summary());
    let out = a.out.expect("parse_args requires --out");
    let written = serde_json::to_vec(&receipt)
        .map_err(|e| e.to_string())
        .and_then(|b| client_scope::write_exclusive(&out, &b));
    if let Err(e) = written {
        eprintln!("mail_compare client-judge: the receipt: {e}");
        return 2;
    }
    if receipt.verdict == client_scope::Verdict::Clean {
        0
    } else {
        2
    }
}

fn footprint_mode(args: &[String]) -> i32 {
    let res = flags(args).and_then(|f| {
        let known = ["protocol", "sources", "documents", "bin", "runtime"];
        if let Some(k) = f.keys().find(|k| !known.contains(&k.as_str())) {
            return Err(format!("unknown flag --{k}"));
        }
        let (p, _, s, _) = load(&f)?;
        let none = Vec::new();
        let bins = runner::pairs(f.get("bin").unwrap_or(&none), "bin")?;
        let runtimes = runner::pairs(f.get("runtime").unwrap_or(&none), "runtime")?;
        resources::footprint(&p, &s, &docs_dir(&f)?, &bins, &runtimes)
    });
    match res {
        Ok(v) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&v).expect("footprint serializes")
            );
            0
        }
        Err(e) => {
            eprintln!("mail_compare footprint: refused: {e}");
            2
        }
    }
}

/// `mail_compare admit-v4 [--seal] DIR PROTOCOL --sources S`: admit a
/// collected run directory, or seal it.
fn admit_v4_mode(args: &[String]) -> i32 {
    let (mut seal, mut positional, mut rest) = (false, Vec::new(), Vec::new());
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--seal" {
            seal = true;
        } else if a.starts_with("--") {
            rest.push(a.clone());
            rest.extend(it.next().cloned());
        } else {
            positional.push(a.clone());
        }
    }
    let [dir, protocol_path] = positional.as_slice() else {
        eprintln!(
            "usage: mail_compare admit-v4 [--seal] DIR PROTOCOL --sources S [--documents DIR] [--bin TARGET=PATH...]"
        );
        return 2;
    };
    rest.push("--protocol".into());
    rest.push(protocol_path.clone());
    let admitted = flags(&rest).and_then(|f| {
        let known = ["protocol", "sources", "documents", "bin"];
        if let Some(k) = f.keys().find(|k| !known.contains(&k.as_str())) {
            return Err(format!("unknown flag --{k}"));
        }
        let (p, pb, s, sb) = load(&f)?;
        let none = Vec::new();
        let bins = runner::pairs(f.get("bin").unwrap_or(&none), "bin")?;
        let a = report::Admission {
            protocol: &p,
            protocol_bytes: &pb,
            lock: &s,
            lock_bytes: &sb,
            bins: &bins,
        };
        report::admit_v4(Path::new(dir), &a, seal)
    });
    match admitted {
        Ok(a) => {
            println!(
                "mail_compare admit-v4: run {} {} with {} chunks",
                a.run_id,
                if seal { "sealed" } else { "admitted" },
                a.chunks
            );
            0
        }
        Err(e) => {
            eprintln!("mail_compare admit-v4: refused: {e}");
            2
        }
    }
}

/// `mail_compare hold`: launch one target under the enforced budget, print
/// its endpoints as one JSON line, sample it, and wait. On stdin EOF, SIGTERM
/// or SIGINT it tears down and prints the receipts as one JSON line. It exits
/// 0 only when cleanup is proved.
fn hold_mode(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "--control") {
        return hold_api_mode(args);
    }
    let prepared = runner::hold_args(args).and_then(|h| {
        let mut f = BTreeMap::new();
        f.insert(
            "protocol".to_owned(),
            vec![h.protocol.display().to_string()],
        );
        f.insert("sources".to_owned(), vec![h.sources.display().to_string()]);
        if let Some(d) = &h.documents {
            f.insert("documents".to_owned(), vec![d.display().to_string()]);
        }
        let (p, _, s, _) = load(&f)?;
        runner::hold_protocol(&p)?;
        let (bins, runtimes) = (
            vec![format!("{}={}", h.target, h.bin.display())],
            h.runtime
                .iter()
                .map(|r| format!("{}={}", h.target, r.display()))
                .collect::<Vec<_>>(),
        );
        let mut specs = runner::launch_specs_of(&p, &s, &bins, &runtimes, &[h.target])?;
        let parent = h
            .scratch
            .parent()
            .filter(|d| !d.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let scope = std::sync::Arc::new(resources::prepare_host(parent)?);
        let mut spec = specs.remove(&h.target).ok_or("no launch spec")?;
        spec.extra.scope = Some(scope);
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&h.scratch)
            .map_err(|e| format!("--scratch must be a new directory: {e}"))?;
        Ok((p, spec, h))
    });
    let (p, spec, h) = match prepared {
        Ok(v) => v,
        Err(e) => {
            eprintln!("mail_compare hold: refused before launch: {e}");
            return 2;
        }
    };
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("mail_compare hold: runtime: {e}");
            return 2;
        }
    };
    let limits = runner::limits(&p);
    let interval = std::time::Duration::from_millis(p.resources.sample_interval_ms.unwrap_or(100));
    let clock = wire::Clock(std::time::Instant::now());
    let result = rt.block_on(async {
        use tokio::signal::unix::{SignalKind, signal};
        let (mut term, mut int) = (
            signal(SignalKind::terminate()),
            signal(SignalKind::interrupt()),
        );
        let interrupt = async {
            tokio::select! {
                _ = async { match term.as_mut() { Ok(s) => s.recv().await, Err(_) => std::future::pending().await } } => {}
                _ = async { match int.as_mut() { Ok(s) => s.recv().await, Err(_) => std::future::pending().await } } => {}
            }
        };
        runner::hold(
            &spec,
            (&h.scratch.join("hold"), &h.samples),
            &limits,
            &clock,
            interval,
            tokio::io::stdin(),
            &mut std::io::stdout(),
            interrupt,
        )
        .await
    });
    let gone = std::fs::remove_dir(&h.scratch).is_ok();
    match result {
        Ok(receipt) => match runner::hold_verdict(&receipt) {
            Ok(()) if gone => 0,
            Ok(()) => {
                eprintln!("mail_compare hold: the scratch was not empty after cleanup");
                2
            }
            Err(e) => {
                eprintln!("mail_compare hold: refused: {e}");
                2
            }
        },
        Err(e) => {
            eprintln!("mail_compare hold: {e}");
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
        .and_then(|()| mcp::adapter_flags(kind, args))
        .and_then(
            |(f, s)| match (f.contains_key("endpoint"), f.contains_key("launch")) {
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
            },
        );
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
        Ok((cfg, scratch, chunk)) => execute(cfg, scratch, chunk),
        Err(e) => {
            eprintln!("mail_compare run: refused before launch: {e}");
            2
        }
    }
}

/// Everything that can refuse before a single process starts: inputs,
/// pins, binaries, output and scratch.
fn prepare(args: &[String]) -> Result<(runner::Config, PathBuf, Option<runner::Chunk>), String> {
    prepare_for(runner::Owner::Run, args)
}

/// `hold --control api-v1 --mode M --chunk api.mcp.TARGET.rN.fF ...` with
/// `run`'s other flags: the MCP API chunk's owner. It launches the target
/// under the budget, prints its endpoints, obeys the eval crate's `api-v1`
/// window lines on stdin and seals the chunk from the record sent last
/// (accepted decision 0132 §6.2, story B). The receipt line follows teardown.
fn hold_api_mode(args: &[String]) -> i32 {
    let mut rest = Vec::new();
    let mut it = args.iter();
    let mut control = None;
    while let Some(a) = it.next() {
        if a == "--control" {
            control = it.next().cloned();
        } else {
            rest.push(a.clone());
        }
    }
    let owner = match control.as_deref() {
        Some("api-v1") => runner::Owner::HoldApi,
        Some("testexec-v1") => runner::Owner::HoldSuite,
        _ => {
            eprintln!(
                "mail_compare hold: refused before launch: --control takes api-v1 or testexec-v1"
            );
            return 2;
        }
    };
    let (cfg, scratch, chunk) = match prepare_for(owner, &rest) {
        Ok((cfg, scratch, Some(chunk))) => (cfg, scratch, chunk),
        Ok(_) => {
            eprintln!("mail_compare hold: refused before launch: --control needs --chunk");
            return 2;
        }
        Err(e) => {
            eprintln!("mail_compare hold: refused before launch: {e}");
            return 2;
        }
    };
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("mail_compare hold: runtime: {e}");
            return 2;
        }
    };
    let clock = wire::Clock(std::time::Instant::now());
    let keepers = cfg.keepers.clone();
    let cleanup_bound = runner::limits(&cfg.protocol).cleanup;
    let output = cfg.output.clone();
    let outcome = rt.block_on(async {
        let mut term =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).ok();
        let term = async {
            match term.as_mut() {
                Some(t) => t.recv().await,
                None => std::future::pending().await,
            }
        };
        let input = tokio::io::BufReader::new(tokio::io::stdin());
        let mut out = std::io::stdout();
        tokio::select! {
            r = async {
                match owner {
                    runner::Owner::HoldSuite => {
                        suite_chunk::hold_suite(cfg, chunk, clock, input, &mut out).await
                    }
                    _ => api_chunk::hold_api(cfg, chunk, clock, input, &mut out).await,
                }
            } => Some(r),
            _ = tokio::signal::ctrl_c() => None,
            _ = term => None,
        }
    });
    rt.block_on(keepers.settle(cleanup_bound * 2));
    let gone = std::fs::remove_dir(&scratch).is_ok();
    match outcome {
        None => {
            eprintln!(
                "mail_compare hold: interrupted; {} stays unsealed",
                output.display()
            );
            2
        }
        Some((runner::Verdict::Infrastructure(e), _, _)) => {
            eprintln!(
                "mail_compare hold: infrastructure failure: {e}; {} is incomplete",
                output.display()
            );
            2
        }
        Some(_) if !gone => {
            eprintln!("mail_compare hold: scratch was not empty after cleanup");
            2
        }
        Some((verdict, _, sealed)) => {
            if let runner::Verdict::WorkloadFailures(f) = &verdict {
                for line in f {
                    eprintln!("workload failure: {line}");
                }
            }
            if let Some(p) = sealed {
                eprintln!("sealed {}", p.display());
            }
            verdict.exit_code()
        }
    }
}

fn prepare_for(
    owner: runner::Owner,
    args: &[String],
) -> Result<(runner::Config, PathBuf, Option<runner::Chunk>), String> {
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
        "chunk",
        "systemd",
    ];
    if let Some(k) = f.keys().find(|k| !known.contains(&k.as_str())) {
        return Err(format!("unknown flag --{k}"));
    }
    let (p, pb, s, sb) = load(&f)?;
    let mode = one(&f, "mode")?;
    p.validate().map_err(|r| r.to_string())?;
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
    let chunk = runner::v4_preflight_owner(
        owner,
        &p,
        mode,
        f.get("chunk").and_then(|v| v.first()).map(String::as_str),
        commit.as_deref(),
        dirty,
    )?;
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
    let (bins, runtimes) = (
        f.get("bin").unwrap_or(&none),
        f.get("runtime").unwrap_or(&none),
    );
    let mut specs = match &chunk {
        Some(c) => runner::launch_specs_of(&p, &s, bins, runtimes, &[c.target()])?,
        None => runner::launch_specs(&p, &s, bins, runtimes)?,
    };
    // An API chunk rechecks every route citation of its target against the
    // documents just verified (accepted decision 0132 §6.2, story B).
    if let Some(c @ runner::Chunk::Api { .. }) = &chunk {
        let dir = match f.get("documents").and_then(|v| v.first()) {
            Some(d) => PathBuf::from(d),
            None => Path::new(one(&f, "sources")?)
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default(),
        };
        api_chunk::check_citations(c.target(), &s, &dir, Path::new("."))
            .map_err(|w| format!("api citations: {}", w.join("; ")))?;
    }
    let output = PathBuf::from(one(&f, "output")?);
    if output.exists() {
        return Err("--output already exists; runs are never overwritten or resumed".into());
    }
    let scratch = PathBuf::from(one(&f, "scratch")?);
    // Protocol 4: delegation, the scratch's backing and the CPU layout are
    // checked, the driver pinned to the client CPUs and the scope adopted,
    // all before the runtime exists and before anything is created. There is
    // no unenforced fallback, in collect or in smoke.
    let scope = match &chunk {
        Some(_) => {
            let parent = scratch
                .parent()
                .filter(|d| !d.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            Some(std::sync::Arc::new(resources::prepare_host(parent)?))
        }
        None => None,
    };
    if let Some(scope) = &scope {
        for spec in specs.values_mut() {
            spec.extra.scope = Some(scope.clone());
        }
    }
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&scratch)
        .map_err(|e| format!("--scratch must be a new directory: {e}"))?;
    let (planned, planned_windows) = match &chunk {
        Some(c) => {
            let (planned, windows) = runner::plan_chunk(&p, c);
            (planned, Some(windows))
        }
        None => (runner::plan(&p), None),
    };
    let host = scope.as_ref().map(|sc| {
        let systemd = f
            .get("systemd")
            .and_then(|v| v.first())
            .map_or("not stated", String::as_str);
        resources::host_facts(&kernel(), systemd, &sc.layout)
    });
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
        planned,
        chunk: chunk.as_ref().map(runner::Chunk::name),
        host,
        planned_windows,
    };
    let cfg = runner::Config {
        protocol: p,
        manifest,
        specs,
        output,
        scratch: scratch.clone(),
        keepers: targets::Keepers::default(),
    };
    Ok((cfg, scratch, chunk))
}

fn execute(cfg: runner::Config, scratch: PathBuf, chunk: Option<runner::Chunk>) -> i32 {
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
            r = async {
                match chunk {
                    Some(c @ runner::Chunk::Api { .. }) => api_chunk::run(cfg, c, clock).await,
                    Some(c) => runner::run_chunk(cfg, c, clock).await,
                    None => runner::run(cfg, clock).await,
                }
            } => Some(r),
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
