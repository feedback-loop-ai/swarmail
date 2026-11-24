//! The client matrix runner's offline proof (issue #392, proposed 0138):
//! the public `client-matrix run` command drives the Inspector-shaped
//! stand-in (`tests/standin/inspector_cli.rs`), which speaks real MCP to
//! the candidate Swarmail binary, and the drafts it writes are read back
//! through the shared receipt checker. Nothing here is a client receipt:
//! every draft lives in a temporary directory.

mod common;

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;
use swarmail_evals::client_matrix::receipt::{self, Status};

/// Far above a run's real cost (well under a second); a run still going at
/// this bound fails the test by name.
const RUN_BOUND: Duration = Duration::from_secs(120);
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn standin() -> &'static str {
    env!("CARGO_BIN_EXE_inspector-cli-standin")
}

fn run(out: &Path, budget: &str, client: &[&str]) -> Output {
    let mut cmd = Command::new(common::exe());
    cmd.args(["client-matrix", "run", "--binary"])
        .arg(std::fs::canonicalize(common::swarmail_bin()).unwrap())
        .args(["--commit", COMMIT, "--client-version", "standin 0.0.0"])
        .args(["--os", "test OS", "--recorded-by", "dispatcher"])
        .arg("--output-dir")
        .arg(out)
        .args(["--budget-secs", budget, "--"])
        .arg(standin())
        .args(client);
    common::bounded(cmd, RUN_BOUND).wait_with_output()
}

/// The one draft of `out`, checked by the shared checker, and its JSON.
fn draft(out: &Path) -> (PathBuf, receipt::Receipt, Value) {
    let entries: Vec<PathBuf> = std::fs::read_dir(out)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(entries.len(), 1, "exactly one draft: {entries:?}");
    let path = entries[0].clone();
    let checked = receipt::validate_file(&path).unwrap_or_else(|e| panic!("{e}"));
    let value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    (path, checked, value)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Swarmail servers this runner started (their working directory is a
/// matrix scratch directory) that are still alive.
#[cfg(target_os = "linux")]
fn matrix_servers() -> Vec<i32> {
    let bin = std::fs::canonicalize(common::swarmail_bin()).unwrap();
    let mut out = Vec::new();
    for entry in std::fs::read_dir("/proc").unwrap().flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<i32>() else {
            continue;
        };
        let exe = std::fs::read_link(format!("/proc/{pid}/exe"));
        let cwd = std::fs::read_link(format!("/proc/{pid}/cwd"));
        if let (Ok(exe), Ok(cwd)) = (exe, cwd)
            && exe == bin
            && cwd.to_string_lossy().contains("/swarmail-matrix-")
            && common::pid_alive(pid)
        {
            out.push(pid);
        }
    }
    out
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn the_runner_writes_green_and_red_drafts_against_the_real_swarmail() {
    let base = common::tempdir();

    // Green: six passes, the measured digest, the server's own handshake.
    let green = base.path().join("green");
    let out = run(&green, "300", &[]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("all six steps passed (no compatibility verdict)"));
    let (path, checked, value) = draft(&green);
    assert_eq!(
        path.file_name().unwrap().to_str().unwrap(),
        format!("{}-mcp-inspector.json", checked.date)
    );
    assert!(
        checked
            .steps
            .in_order()
            .iter()
            .all(|(_, o)| o.status == Status::Pass)
    );
    assert_eq!(
        checked.executable_sha256,
        swarmail_evals::identity::file_hash(common::swarmail_bin()).unwrap()
    );
    assert_eq!(checked.swarmail_commit, COMMIT);
    assert_eq!(checked.negotiated_revision.as_deref(), Some("2025-11-25"));
    assert_eq!(
        value["initialize_response"]["result"]["serverInfo"]["name"],
        "swarmail"
    );
    assert_eq!(value["mode"], "headless");
    assert_eq!(value["transport"], "http");

    // The existing draft is kept: a second run refuses before any server.
    let before = std::fs::read(&path).unwrap();
    let again = run(&green, "300", &[]);
    assert_eq!(again.status.code(), Some(2));
    assert!(text(&again.stderr).contains("already exists"));
    assert_eq!(std::fs::read(&path).unwrap(), before);

    // Red: the client exits 0 having never sent the search call. Its
    // lingering descendant is reaped with its process group.
    let red = base.path().join("red");
    let pid_file = base.path().join("linger.pid");
    let pid_arg = pid_file.to_str().unwrap();
    let out = run(
        &red,
        "300",
        &["--omit", "swarmail_search_emails", "--linger", pid_arg],
    );
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stderr));
    let (_, checked, _) = draft(&red);
    for (step, outcome) in checked.steps.in_order() {
        if step == "structured_content" {
            assert_eq!(outcome.status, Status::Fail);
            let reason = outcome.reason.as_deref().unwrap();
            assert!(reason.contains("the client sent no tools/call of swarmail_search_emails"));
        } else {
            assert_eq!(outcome.status, Status::Pass, "{step}");
        }
    }
    let lingering: i32 = std::fs::read_to_string(&pid_file).unwrap().parse().unwrap();
    #[cfg(target_os = "linux")]
    assert!(
        !common::pid_alive(lingering),
        "the client's descendant was reaped"
    );
    #[cfg(not(target_os = "linux"))]
    let _ = lingering;

    // March negotiation: text-only search fails structured_content alone.
    let march = base.path().join("march");
    let out = run(&march, "300", &["--revision", "2025-03-26"]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stderr));
    let (_, checked, _) = draft(&march);
    assert_eq!(checked.negotiated_revision.as_deref(), Some("2025-03-26"));
    let reason = checked.steps.structured_content.reason.unwrap();
    assert!(reason.contains("text block only"), "{reason}");
    assert_eq!(checked.steps.read.status, Status::Pass);

    // Oversized client output: each call fails, the handshake stands.
    let flood = base.path().join("flood");
    let out = run(&flood, "300", &["--flood"]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stderr));
    let (_, checked, _) = draft(&flood);
    assert_eq!(checked.steps.initialize.status, Status::Pass);
    let reason = checked.steps.read.reason.unwrap();
    assert!(reason.contains("helper_output_invalid"), "{reason}");

    // An exhausted budget and an unusable output path: errors, no draft.
    let spent = base.path().join("spent");
    let out = run(&spent, "0", &[]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(std::fs::read_dir(&spent).unwrap().count(), 0);
    let file = base.path().join("a-file");
    std::fs::write(&file, b"x").unwrap();
    let out = run(&file, "300", &[]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(std::fs::read(&file).unwrap(), b"x");

    #[cfg(target_os = "linux")]
    assert_eq!(
        matrix_servers(),
        Vec::<i32>::new(),
        "no matrix server survives"
    );
}
