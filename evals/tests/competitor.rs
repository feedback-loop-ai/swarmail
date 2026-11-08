//! The runner's competitor path (proposed decision 0129 §4, spec R4): the
//! adapter as an owned process behind the bridge, its announced tool list
//! checked against the product, and a cleanup receipt that only an orderly
//! exit 0 gives. The adapter here is a stand-in that speaks the real
//! announcement and stdio protocol; the real adapter against the real
//! executables is the controls' proof, not CI's.

mod common;

use serde_json::Value;
use std::path::{Path, PathBuf};
use swarmail_evals::provenance::sha256;

const PRODUCT: &str = include_str!("../../docs/evals/agent-comparison.json");
const LOCK: &str = include_str!("../../docs/evals/sources.lock.json");

fn python3() -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
        .map(|d| d.join("python3"))
        .find(|p| p.is_file())
        .expect("python3 is on PATH")
}

const STAND_IN: &str = r#"
import sys, json, hashlib, os, signal, socketserver, threading
mode = open(sys.argv[3]).read().strip()
TOOLS = [{"name": "mailpit_list", "description": "d", "inputSchema": {"type": "object", "properties": {}}}]
digest = hashlib.sha256(json.dumps(TOOLS, separators=(',', ':'), sort_keys=True).encode()).hexdigest()
class H(socketserver.StreamRequestHandler):
    def handle(self):
        def w(s):
            self.wfile.write((s + "\r\n").encode()); self.wfile.flush()
        w("220 stand-in")
        data = False
        while True:
            line = self.rfile.readline()
            if not line: return
            u = line.decode().strip().upper()
            if data:
                if line == b".\r\n": data = False; w("250 ok")
                continue
            if u.startswith("RCPT") and mode == "kill-on-rcpt":
                os.kill(os.getpid(), signal.SIGKILL)
            if u.startswith("AUTH"): w("235 ok")
            elif u == "DATA": data = True; w("354 go")
            elif u == "QUIT": w("221 bye"); return
            else: w("250 ok")
srv = socketserver.ThreadingTCPServer(("127.0.0.1", 0), H); srv.daemon_threads = True
port = srv.server_address[1]
threading.Thread(target=srv.serve_forever, daemon=True).start()
if mode == "stderr-proof":
    sys.stderr.write("adapter-stderr-marker\n"); sys.stderr.flush()
announced = "ab" * 32 if mode == "mismatch" else digest
print("smtp=127.0.0.1:%d tools_sha256=%s" % (port, announced), flush=True)
for line in sys.stdin:
    msg = json.loads(line)
    if "id" not in msg: continue
    m = msg["method"]
    if m == "initialize":
        r = {"protocolVersion": "2025-03-26", "capabilities": {"tools": {}}, "serverInfo": {"name": "stand-in", "version": "1"}}
    elif m == "tools/list":
        r = {"tools": TOOLS}
    else:
        r = {}
    print(json.dumps({"jsonrpc": "2.0", "id": msg["id"], "result": r}), flush=True)
if mode == "stderr-proof":
    with open(sys.argv[3] + ".stderr-path", "w") as f: f.write(os.readlink("/proc/self/fd/2"))
    with open("/proc/self/fd/2") as f: copy = f.read()
    with open(sys.argv[3] + ".stderr-copy", "w") as f: f.write(copy)
sys.exit(3 if mode == "exit-nonzero" else 0)
"#;

/// The stand-in's tool-list digest, as the adapter and the product agree on it.
fn stand_in_digest() -> String {
    let tools = r#"[{"description":"d","inputSchema":{"properties":{},"type":"object"},"name":"mailpit_list"}]"#;
    sha256(tools.as_bytes())
}

struct Rig {
    dir: tempfile::TempDir,
    adapter: PathBuf,
    bin: PathBuf,
    product: PathBuf,
    sources: PathBuf,
}

fn rig(mode: &str) -> Rig {
    use std::os::unix::fs::PermissionsExt;
    let dir = common::tempdir();
    let adapter = dir.path().join("adapter");
    std::fs::write(
        &adapter,
        format!("#!{} -I\n{STAND_IN}", python3().display()),
    )
    .unwrap();
    std::fs::set_permissions(&adapter, std::fs::Permissions::from_mode(0o755)).unwrap();
    // The competitor's "executable" is the mode file: its digest is the pin.
    let bin = dir.path().join("mailpit");
    std::fs::write(&bin, mode).unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    let digest = sha256(mode.as_bytes());
    let mut product: Value = serde_json::from_str(PRODUCT).unwrap();
    let mut lock: Value = serde_json::from_str(LOCK).unwrap();
    product["targets"][1]["executable_sha256"] = digest.clone().into();
    product["targets"][1]["tools_sha256"] = stand_in_digest().into();
    for e in lock["entries"].as_array_mut().unwrap() {
        if e["target"] == "mailpit" {
            e["pin"]["artifact"]["executable_sha256"] = digest.clone().into();
        }
    }
    // The competitors other than mailpit stay as committed: their binds are
    // checked only for the target a run names, and the lock's other entries
    // are read for agreement, not launched. Their digests are the pinned ones.
    let product_path = dir.path().join("product.json");
    std::fs::write(&product_path, product.to_string()).unwrap();
    let sources = dir.path().join("sources.lock.json");
    std::fs::write(&sources, lock.to_string()).unwrap();
    std::fs::write(
        dir.path().join("agents.json"),
        r#"[{"label":"null","argv":[]}]"#,
    )
    .unwrap();
    Rig {
        dir,
        adapter,
        bin,
        product: product_path,
        sources,
    }
}

fn matrix(rig: &Rig, trials: &str) -> (Vec<Value>, std::process::Output) {
    let _scope = common::trial_scope();
    let out = common::cli()
        .args([
            "matrix",
            "--target",
            "mailpit",
            "--tasks",
            "otp-backlog",
            "--trials",
            trials,
        ])
        .arg("--agents")
        .arg(rig.dir.path().join("agents.json"))
        .arg("--product")
        .arg(&rig.product)
        .arg("--adapter")
        .arg(&rig.adapter)
        .arg("--target-bin")
        .arg(&rig.bin)
        .arg("--sources")
        .arg(&rig.sources)
        .args([
            "--seed",
            &format!("{:064x}", 339),
            "--fixture-time",
            "2026-11-07T00:00:00Z",
        ])
        .current_dir(rig.dir.path())
        .output()
        .expect("the CLI runs");
    let rows = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).expect("a comparison row"))
        .collect();
    (rows, out)
}

fn outcomes(rows: &[Value]) -> Vec<String> {
    rows.iter()
        .map(|r| r["outcome"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn a_tool_list_the_product_did_not_freeze_refuses_every_trial_by_name() {
    let rig = rig("mismatch");
    let (rows, out) = matrix(&rig, "2");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(rows.len(), 2, "every slot keeps its row: {stderr}");
    assert_eq!(outcomes(&rows), ["harness_error", "harness_error"]);
    assert_eq!(out.status.code(), Some(2));
    // The matrix drains a worker's diagnostics, so the cause is shown where
    // the runner makes it: the same start refuses with its name.
    let plan = swarmail_evals::runner::CompetitorPlan {
        target: swarmail_evals::target::Target::Mailpit,
        adapter: rig.adapter.clone(),
        target_bin: rig.bin.clone(),
        sources: rig.sources.clone(),
        tools_sha256: stand_in_digest(),
        smtp_auth: swarmail_evals::mail::SmtpAuth::Plain,
    };
    let why = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(swarmail_evals::runner::probe_contract(
            plan.target,
            None,
            Some(&plan),
        ))
        .unwrap_err();
    assert_eq!(why, "adapter_tools_mismatch");
    // The rows are schema 3, bound to the product and the pinned bytes.
    assert_eq!(rows[0]["schema_version"], 3);
    assert_eq!(rows[0]["target"], "mailpit-mcp");
    assert_eq!(rows[0]["executable_sha256"], sha256(b"mismatch"));
    assert_eq!(
        rows[0]["protocol_sha256"],
        sha256(&std::fs::read(&rig.product).unwrap())
    );
}

#[test]
// The stand-in proves its stderr through /proc/self/fd/2, which only Linux has.
#[cfg(target_os = "linux")]
fn a_clean_exit_after_stdin_closes_is_the_cleanup_receipt_and_stderr_is_a_private_file() {
    let rig = rig("stderr-proof");
    let (rows, out) = matrix(&rig, "2");
    let stderr = String::from_utf8_lossy(&out.stderr);
    // The null agent fails by design; a receipt means the failure is the
    // agent's, never the harness's, and the next slot was admitted.
    assert_eq!(outcomes(&rows), ["fail", "fail"], "{stderr}");
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    let path = std::fs::read_to_string(rig.dir.path().join("mailpit.stderr-path")).unwrap();
    assert!(path.ends_with("/adapter/stderr.log"), "{path}");
    let copy = std::fs::read_to_string(rig.dir.path().join("mailpit.stderr-copy")).unwrap();
    assert!(copy.contains("adapter-stderr-marker"), "{copy:?}");
    assert!(
        !Path::new(&path).exists(),
        "the trial's private scratch is gone with the trial"
    );
}

#[test]
fn a_non_zero_adapter_exit_is_no_receipt_and_stops_admission_but_keeps_every_row() {
    let rig = rig("exit-nonzero");
    let (rows, out) = matrix(&rig, "3");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(rows.len(), 3, "the denominator is kept: {stderr}");
    assert_eq!(
        outcomes(&rows),
        ["harness_error", "harness_error", "harness_error"]
    );
    assert!(stderr.contains("trial_cleanup_unproved"), "{stderr}");
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn a_killed_adapter_mid_trial_keeps_the_slots_row_and_stops_admission() {
    let rig = rig("kill-on-rcpt");
    let (rows, out) = matrix(&rig, "3");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(rows.len(), 3, "the denominator is kept: {stderr}");
    assert!(
        outcomes(&rows).iter().all(|o| o == "harness_error"),
        "{rows:?}"
    );
    assert!(stderr.contains("trial_cleanup_unproved"), "{stderr}");
    assert_eq!(out.status.code(), Some(2));
    // The refused slots were never started: the matrix says how many.
    assert!(
        stderr.contains("refused 2 unstarted") || stderr.contains("cleanup"),
        "{stderr}"
    );
}

#[test]
fn matrix_target_swarmail_runs_the_hard_tier_end_to_end_with_oracle_and_null() {
    let dir = common::tempdir();
    let agents = dir.path().join("agents.json");
    std::fs::write(
        &agents,
        r#"[{"label":"oracle","argv":[]},{"label":"null","argv":[]}]"#,
    )
    .unwrap();
    let product = dir.path().join("product.json");
    std::fs::write(&product, PRODUCT).unwrap();
    let _scope = common::trial_scope();
    let out = common::cli()
        .args([
            "matrix", "--target", "swarmail", "--tier", "hard", "--trials", "1",
        ])
        .arg("--agents")
        .arg(&agents)
        .arg("--product")
        .arg(&product)
        .args([
            "--seed",
            &format!("{:064x}", 339),
            "--fixture-time",
            "2026-11-07T00:00:00Z",
        ])
        .current_dir(dir.path())
        .output()
        .expect("the CLI runs");
    let rows: Vec<Value> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).expect("a comparison row"))
        .collect();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(rows.len(), 6, "{stderr}");
    for row in &rows {
        assert_eq!(row["schema_version"], 3, "no native row reaches the output");
        assert_eq!(row["target"], "swarmail-mcp");
        assert_eq!(row["settings"]["connection"], "native-mcp");
        assert_eq!(
            row["executable_sha256"],
            sha256(&std::fs::read(common::swarmail_bin()).unwrap())
        );
        let want = if row["coordinate"]["agent"] == "oracle" {
            "pass"
        } else {
            "fail"
        };
        assert_eq!(row["outcome"], want, "{row}");
    }
    assert_eq!(
        out.status.code(),
        Some(1),
        "the null agent fails by design: {stderr}"
    );
}

#[test]
fn every_refusal_happens_before_anything_runs_and_names_itself() {
    let rig = rig("serve");
    let marker = rig.dir.path().join("ran");
    let spy = rig.dir.path().join("spy");
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(&spy, format!("#!/bin/sh\ntouch {}\n", marker.display())).unwrap();
        std::fs::set_permissions(&spy, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let run = |args: &[&str]| {
        let out = common::cli()
            .args(["matrix", "--tier", "hard", "--trials", "1"])
            .arg("--agents")
            .arg(rig.dir.path().join("agents.json"))
            .args(args)
            .current_dir(rig.dir.path())
            .output()
            .unwrap();
        (
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };
    let (code, err, out) = run(&[
        "--target",
        "bogus",
        "--product",
        rig.product.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(2));
    assert!(err.contains("unknown --target \"bogus\""), "{err}");
    assert!(out.is_empty(), "no row: {out}");
    let (code, err, _) = run(&[
        "--target",
        "swarmail",
        "--brokkr",
        "/bin/true",
        "--product",
        rig.product.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(2));
    assert!(err.contains("competitor_brokkr_unsupported"), "{err}");
    // A dsh agent refuses a competitor before anything starts.
    let dsh = rig.dir.path().join("dsh.json");
    std::fs::write(
        &dsh,
        r#"[{"label":"dsh-agent","argv":["/bin/true"],"model":"dashscope/glm-5.3","wrapper_profile":"dsh_0_1_5_rc_1"}]"#,
    )
    .unwrap();
    let out = common::cli()
        .args([
            "matrix",
            "--target",
            "mailpit",
            "--tasks",
            "otp-backlog",
            "--trials",
            "1",
            "--agents",
        ])
        .arg(&dsh)
        .arg("--product")
        .arg(&rig.product)
        .arg("--adapter")
        .arg(&spy)
        .arg("--target-bin")
        .arg(&rig.bin)
        .arg("--sources")
        .arg(&rig.sources)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("competitor_dsh_unsupported"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty(), "no row, nothing started");
    // A task the product does not request.
    let mut v: Value =
        serde_json::from_str(&std::fs::read_to_string(&rig.product).unwrap()).unwrap();
    v["applicability"]["otp-backlog"]["mailpit"] = "not-requested".into();
    let skipped = rig.dir.path().join("skipped.json");
    std::fs::write(&skipped, v.to_string()).unwrap();
    let out = common::cli()
        .args([
            "matrix",
            "--target",
            "mailpit",
            "--tasks",
            "otp-backlog",
            "--trials",
            "1",
            "--agents",
        ])
        .arg(rig.dir.path().join("agents.json"))
        .args([
            "--product",
            skipped.to_str().unwrap(),
            "--adapter",
            spy.to_str().unwrap(),
            "--target-bin",
        ])
        .arg(&rig.bin)
        .arg("--sources")
        .arg(&rig.sources)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("task_not_applicable"));
    assert!(
        !marker.exists(),
        "the adapter spy never ran: nothing was launched"
    );
}
