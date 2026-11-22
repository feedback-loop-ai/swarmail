//! One-command paid baseline proofs (proposed 0137) over fake direct CLIs:
//! stand-in executables answer the bounded version probe and log each
//! launch; no provider, credential or network beyond loopback is involved.
mod common;
use common::*;
use serde_json::{Value, json};
use std::{
    io::{BufRead, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use swarmail_evals::{baseline, provenance::WrapperProfile, runner, tasks};

const CANARY: &str = "SYNTHETIC-PAID-CANARY-NEVER-RETAINED";
const SPARK_CANARY: &str = "SYNTHETIC-SPARK-CANARY-NEVER-RETAINED";
const FAMILIES: [&str; 3] = ["claude", "codex", "dsh"];
const HOUR: u64 = 3600;

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
fn private(path: &Path, bytes: &[u8]) {
    // Replace atomically, as an operator renewing a login does.
    let tmp = path.with_extension("next");
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)
        .unwrap();
    f.write_all(bytes).unwrap();
    std::fs::rename(&tmp, path).unwrap();
}
fn banner(family: &str) -> &'static str {
    match family {
        "claude" => "2.1.310 (Claude Code)",
        "codex" => "codex-cli 0.164.0",
        _ => "0.1.5-rc.1",
    }
}
fn profile(family: &str) -> WrapperProfile {
    match family {
        "claude" => WrapperProfile::Claude2_1_310,
        "codex" => WrapperProfile::Codex0_164_0,
        _ => WrapperProfile::Dsh0_1_5Rc1,
    }
}
fn jwt(exp: u64) -> String {
    use base64::Engine as _;
    let e = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    format!(
        "{}.{}.{}",
        e.encode(br#"{"alg":"none"}"#),
        e.encode(json!({"exp":exp,"canary":CANARY}).to_string()),
        e.encode(CANARY)
    )
}

/// A private run: fake CLIs, their credentials, the v2 selection and an
/// independent launch log the harness never reads.
struct Paid {
    dir: tempfile::TempDir,
    config: PathBuf,
    log: PathBuf,
    hold: PathBuf,
    output: PathBuf,
}
impl Paid {
    fn new() -> Self {
        let dir = runner::private_tempdir("paid-baseline-").unwrap();
        let p = Self {
            config: dir.path().join("selection.json"),
            log: dir.path().join("launches.log"),
            hold: dir.path().join("hold"),
            output: dir.path().join("run"),
            dir,
        };
        for family in FAMILIES {
            p.write_cli(family, banner(family));
        }
        p.claude(now() + 24 * HOUR);
        p.codex(now() + 24 * HOUR);
        p.dsh(now() + 24 * HOUR, now() + 24 * HOUR);
        p.select(|_| {});
        p
    }
    fn bin(&self, family: &str) -> PathBuf {
        self.dir.path().join(format!("{family}-cli"))
    }
    fn cred(&self, family: &str) -> PathBuf {
        self.dir.path().join(format!("{family}-credential.json"))
    }
    /// The stand-in answers `--version` with `version`; any other launch logs
    /// its family and model (dsh: its route), holds (bounded) while the hold
    /// file names this launch as `<family> <model> <n>`, and exits: a
    /// measured failure.
    fn write_cli(&self, family: &str, version: &str) {
        let script = format!(
            r#"#!/bin/sh
if [ "$1" = "--version" ]; then printf '%s\n' '{version}'; exit 0; fi
case "$*" in *{CANARY}*|*{SPARK_CANARY}*) echo LEAK >> '{log}';; esac
m=none
while [ $# -gt 0 ]; do [ "$1" = --model ] && m="$2"; shift; done
[ -n "$DASHSCOPE_API_KEY" ] && m=dashscope
[ -n "$SPARK_API_KEY" ] && m=spark-glm
echo "{family} $m" >> '{log}'
n=$(grep -c "^{family} $m\$" '{log}')
i=0
while [ "$(cat '{hold}' 2>/dev/null)" = "{family} $m $n" ] && [ $i -lt 600 ]; do sleep 0.05; i=$((i+1)); done
exit 0
"#,
            log = self.log.display(),
            hold = self.hold.display(),
        );
        let path = self.bin(family);
        let tmp = path.with_extension("next");
        std::fs::write(&tmp, script).unwrap();
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::rename(&tmp, &path).unwrap();
    }
    fn claude(&self, until: u64) {
        private(
            &self.cred("claude"),
            json!({"claudeAiOauth":{"accessToken":CANARY,"expiresAt":until * 1000,"scopes":["user:inference"]}})
                .to_string()
                .as_bytes(),
        );
    }
    fn codex(&self, until: u64) {
        private(
            &self.cred("codex"),
            json!({"auth_mode":"chatgpt","OPENAI_API_KEY":null,"tokens":{"id_token":CANARY,"access_token":jwt(until),"refresh_token":CANARY,"account_id":"synthetic-account"},"last_refresh":"2026-11-22T00:00:00Z"})
                .to_string()
                .as_bytes(),
        );
    }
    fn dsh(&self, dashscope: u64, spark: u64) {
        private(
            &self.cred("dsh"),
            json!({"DASHSCOPE_API_KEY":{"key":CANARY,"valid_through":dashscope},"SPARK_API_KEY":{"key":SPARK_CANARY,"valid_through":spark}})
                .to_string()
                .as_bytes(),
        );
    }
    /// The v2 selection with each fake's measured digest; `edit` may break it.
    fn select(&self, edit: impl FnOnce(&mut Value)) {
        let mut families = json!({});
        for family in FAMILIES {
            families[family] = json!({
                "executable": self.bin(family),
                "credential_file": self.cred(family),
                "profile": profile(family),
                "cli_version": profile(family).cli_version(),
                "cli_sha256": swarmail_evals::identity::file_hash(&self.bin(family)).unwrap(),
                "auth_shape": swarmail_evals::wrappers::auth_shape(family),
            });
        }
        let mut v = json!({"version":2,"families":families});
        edit(&mut v);
        private(&self.config, v.to_string().as_bytes());
    }
    fn plan(&self) -> baseline::Plan {
        baseline::Plan::for_tier(Some(tasks::Tier::Deliverability), &self.output)
    }
    fn command(&self, extra: &[&str]) -> Command {
        let mut c = cli();
        c.args(["baseline", "--tier", "deliverability", "--config"])
            .arg(&self.config)
            .arg("--output")
            .arg(&self.output)
            .args(["--renewal-poll-seconds", "1"])
            .args(extra);
        c
    }
    fn authorized(&self, extra: &[&str]) -> Command {
        let id = self.plan().product_id;
        let mut c = self.command(&["--authorized-product", &id]);
        c.args(extra);
        c
    }
    fn launches(&self) -> Vec<String> {
        std::fs::read_to_string(&self.log)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }
    fn count(&self, line: &str) -> usize {
        self.launches().iter().filter(|l| *l == line).count()
    }
}

/// A running baseline whose stderr lines are collected as they arrive.
struct Running {
    child: Child,
    lines: Arc<Mutex<Vec<String>>>,
}
impl Running {
    fn start(mut c: Command) -> Self {
        let mut child = c
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let lines = Arc::new(Mutex::new(Vec::new()));
        let sink = lines.clone();
        let stderr = child.stderr.take().unwrap();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stderr)
                .lines()
                .map_while(Result::ok)
            {
                sink.lock().unwrap().push(line);
            }
        });
        Self { child, lines }
    }
    /// Wait, bounded, until a stderr line satisfies `f`.
    fn until(&self, what: &str, f: impl Fn(&str) -> bool) {
        wait(what, || self.lines.lock().unwrap().iter().any(|l| f(l)));
    }
    fn signal(&self, sig: i32) {
        unsafe { libc::kill(self.child.id() as i32, sig) };
    }
    fn finish(mut self) -> (Option<i32>, Vec<String>) {
        let start = Instant::now();
        let status = loop {
            if let Some(s) = self.child.try_wait().unwrap() {
                break s;
            }
            assert!(start.elapsed() < Duration::from_secs(240), "baseline hung");
            std::thread::sleep(Duration::from_millis(20));
        };
        // The reader thread ends at EOF; give it a bounded moment.
        let end = Instant::now();
        while Arc::strong_count(&self.lines) > 1 && end.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(10));
        }
        let lines = self.lines.lock().unwrap().clone();
        (status.code(), lines)
    }
}
fn wait(what: &str, f: impl Fn() -> bool) {
    let start = Instant::now();
    while !f() {
        assert!(
            start.elapsed() < Duration::from_secs(180),
            "timed out waiting for {what}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn run(c: &mut Command) -> (Option<i32>, String, String) {
    let out = c.stdin(Stdio::null()).output().unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}
/// Every byte under `root`, for leak checks.
fn all_bytes(root: &Path) -> Vec<u8> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(p) = stack.pop() {
        let m = std::fs::symlink_metadata(&p).unwrap();
        if m.is_dir() {
            for e in std::fs::read_dir(&p).unwrap() {
                stack.push(e.unwrap().path());
            }
        } else if m.is_file() {
            out.extend(std::fs::read(&p).unwrap());
        }
    }
    out
}
fn contains(hay: &[u8], needle: &str) -> bool {
    hay.windows(needle.len()).any(|w| w == needle.as_bytes())
}

#[test]
fn dry_run_prints_the_whole_product_and_reads_creates_and_launches_nothing() {
    let dir = tempdir();
    // Neither file exists: a dry run never opens a selection or credential.
    let config = dir.path().join("absent-selection.json");
    let plan_for = |tier: &[&str], output: &Path, margin: &[&str]| {
        let out = Command::new(exe())
            .args(["baseline", "--dry-run", "--config"])
            .arg(&config)
            .arg("--output")
            .arg(output)
            .args(tier)
            .args(margin)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        serde_json::from_slice::<Value>(&out.stdout).unwrap()
    };
    let a = plan_for(&[], &dir.path().join("a"), &[]);
    let b = plan_for(
        &[],
        &dir.path().join("elsewhere"),
        &["--freshness-minutes", "90"],
    );
    let del = plan_for(&["--tier", "deliverability"], &dir.path().join("a"), &[]);
    let id = a["product_id"].as_str().unwrap();
    assert!(id.starts_with("baseline-v1-") && id.len() == "baseline-v1-".len() + 64);
    // Output location and margin never change what an authorization covers.
    assert_eq!(a["product_id"], b["product_id"]);
    assert_ne!(a["product_id"], del["product_id"]);
    assert_eq!(
        a["product_id"],
        baseline::Plan::new(Path::new("/x")).product_id
    );
    assert_eq!(a["authorization"], format!("--authorized-product {id}"));
    assert_eq!(b["freshness"]["margin_minutes"], 90);
    assert_eq!(a["freshness"]["margin_minutes"], 70);
    for (plan, controls, paid, tasks) in [(&a, 32, 480, 16), (&del, 2, 30, 1)] {
        assert_eq!(plan["control_slots"].as_array().unwrap().len(), controls);
        let chunks = plan["chunks"].as_array().unwrap();
        assert_eq!(chunks.len(), tasks * 6);
        assert!(
            chunks
                .iter()
                .all(|c| c["repetitions"] == json!([0, 1, 2, 3, 4]))
        );
        assert_eq!(chunks.len() * 5, paid);
        // Task-major: each task's six declarations, in declaration order.
        let labels: Vec<_> = chunks[..6].iter().map(|c| c["agent"].clone()).collect();
        assert_eq!(
            labels,
            [
                "claude-opus",
                "claude-sonnet",
                "codex-sol",
                "codex-luna",
                "glm-dashscope",
                "glm-spark"
            ]
        );
        assert!(chunks[..6].iter().all(|c| c["task"] == chunks[0]["task"]));
        assert_eq!(
            plan["estimate"]["nominal_minutes"],
            (controls + paid) as u64 * 10
        );
        assert!(
            plan["estimate"]["calculation"]
                .as_str()
                .unwrap()
                .contains(&format!("({controls} control + {paid} paid slots)"))
        );
        let rules: Vec<_> = plan["freshness"]["rules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["credential"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(rules, ["claude", "codex", "dsh:dashscope", "dsh:spark-glm"]);
        assert_eq!(plan["agents"].as_array().unwrap().len(), 6);
        assert_eq!(plan["concurrency"], 1);
    }
    assert_eq!(del["chunks"][4]["credential"], "dsh:dashscope");
    assert_eq!(del["chunks"][0]["model"], "claude-opus-5-5");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn missing_and_wrong_authorization_refuse_by_name_after_the_plan() {
    let p = Paid::new();
    let (code, out, err) = run(&mut p.command(&[]));
    assert_eq!(code, Some(2));
    assert!(
        err.contains("baseline refused: authorization_missing"),
        "{err}"
    );
    assert_eq!(
        serde_json::from_str::<Value>(&out).unwrap()["product_id"],
        p.plan().product_id
    );
    let wrong = baseline::Plan::new(&p.output).product_id;
    for extra in [&[][..], &["--resume"][..]] {
        let mut c = p.command(&["--authorized-product", &wrong]);
        let (code, _, err) = run(c.args(extra));
        assert_eq!(code, Some(2));
        assert!(
            err.contains("baseline refused: authorization_mismatch"),
            "{err}"
        );
        let mut c = p.command(&["--resume"]);
        let (code, _, err) = run(&mut c);
        assert_eq!(code, Some(2));
        assert!(err.contains("authorization_missing"), "{err}");
    }
    assert!(!p.output.exists());
    assert!(p.launches().is_empty());
}

#[test]
fn every_contract_refusal_names_its_family_and_cause_before_any_spend() {
    let p = Paid::new();
    let refuse = |expected: &str| {
        let (code, _, err) = run(&mut p.authorized(&[]));
        assert_eq!(code, Some(2), "{expected}: {err}");
        assert!(err.contains(expected), "{expected}: {err}");
        assert!(!contains(err.as_bytes(), CANARY), "{err}");
        assert!(!p.output.exists(), "{expected}");
        assert!(p.launches().is_empty(), "{expected}");
    };
    // Only the last family fails: nothing spends for the others either.
    p.select(|v| {
        v["families"].as_object_mut().unwrap().remove("dsh");
    });
    refuse("contract dsh: contract_missing");
    p.select(|v| v["families"]["codex"]["cli_version"] = json!("0.165.0"));
    refuse("contract codex: version_unsupported");
    p.select(|v| v["families"]["dsh"]["cli_sha256"] = json!("0".repeat(64)));
    refuse("contract dsh: digest_mismatch");
    p.select(|v| v["families"]["claude"]["auth_shape"] = json!("codex_chatgpt_v1"));
    refuse("contract claude: auth_shape_unsupported");
    p.select(|v| v["families"]["claude"]["profile"] = json!("claude_2_1_310_via_brokkr"));
    refuse("contract claude: profile_mismatch");
    // An opaque forwarding path cannot establish identity.
    let link = p.dir.path().join("codex-link");
    std::os::unix::fs::symlink(p.bin("codex"), &link).unwrap();
    p.select(|v| v["families"]["codex"]["executable"] = json!(link));
    refuse("contract codex: executable_unsafe");
    // The probe's banner must be the profile's exact version; the banner
    // itself (here carrying a canary) is never echoed.
    p.write_cli("dsh", &format!("0.1.6 {CANARY}"));
    p.select(|_| {});
    refuse("contract dsh: version_mismatch");
    p.write_cli("dsh", banner("dsh"));
    // Credential shapes with unknown validity refuse by family and route.
    p.select(|_| {});
    private(
        &p.cred("codex"),
        json!({"auth_mode":"chatgpt","OPENAI_API_KEY":null,"tokens":{"id_token":CANARY,"access_token":CANARY,"refresh_token":CANARY,"account_id":"a"},"last_refresh":"2026-11-22T00:00:00Z"})
            .to_string()
            .as_bytes(),
    );
    refuse("credential codex: credential_expiry_unknown");
    p.codex(now() + 24 * HOUR);
    // The old two-key dsh file carries no validity evidence.
    private(
        &p.cred("dsh"),
        json!({"DASHSCOPE_API_KEY":CANARY,"SPARK_API_KEY":CANARY})
            .to_string()
            .as_bytes(),
    );
    refuse("credential dsh:dashscope: private_input_invalid");
    // The version-1 selection has no contract at all.
    private(
        &p.config,
        json!({"version":1,"families":{"claude":{"executable":p.bin("claude"),"credential_file":p.cred("claude")},"codex":{"executable":p.bin("codex"),"credential_file":p.cred("codex")},"dsh":{"executable":p.bin("dsh"),"credential_file":p.cred("dsh")}}})
            .to_string()
            .as_bytes(),
    );
    refuse("contract claude: private_input_invalid");
}

#[test]
fn freshness_is_inclusive_and_never_overflows() {
    assert!(baseline::fresh_enough(1000 + 70 * 60, 1000, 70));
    assert!(!baseline::fresh_enough(1000 + 70 * 60 - 1, 1000, 70));
    assert!(!baseline::fresh_enough(u64::MAX, u64::MAX, 1));
    assert!(!baseline::fresh_enough(u64::MAX, 0, u64::MAX));
    let plan = baseline::Plan::for_tier(Some(tasks::Tier::Deliverability), Path::new("/x"));
    let pick = |blocked: &[&str]| {
        baseline::next_chunk(&plan.chunks, |k| !blocked.contains(&k)).map(|c| c.agent.clone())
    };
    assert_eq!(pick(&[]).as_deref(), Some("claude-opus"));
    assert_eq!(pick(&["claude"]).as_deref(), Some("codex-sol"));
    assert_eq!(
        pick(&["claude", "codex", "dsh:dashscope"]).as_deref(),
        Some("glm-spark")
    );
    assert_eq!(
        pick(&["claude", "codex", "dsh:dashscope", "dsh:spark-glm"]),
        None
    );
}

fn attempt_id(lines: &[String]) -> String {
    let line = lines
        .iter()
        .find(|l| l.contains("--confirm-cleanup "))
        .unwrap_or_else(|| panic!("{lines:?}"));
    line.rsplit("--confirm-cleanup ").next().unwrap()[..32].to_string()
}
fn paid_rows(p: &Paid) -> Vec<Value> {
    std::fs::read_to_string(p.output.join("paid.jsonl"))
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}
/// Every retained paid result.json, by trial id: committed score bytes.
fn retained_results(p: &Paid) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut out = std::collections::BTreeMap::new();
    let paid = p.output.join("paid");
    for attempt in std::fs::read_dir(&paid).into_iter().flatten() {
        let trials = attempt.unwrap().path().join("trials");
        for t in std::fs::read_dir(&trials).into_iter().flatten() {
            let t = t.unwrap().path();
            if let Ok(bytes) = std::fs::read(t.join("result.json")) {
                out.insert(t.file_name().unwrap().to_string_lossy().into(), bytes);
            }
        }
    }
    out
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
fn one_command_gates_on_controls_defers_stale_logins_and_resumes_by_slot() {
    let _scope = trial_scope();
    let p = Paid::new();
    // Claude's login is valid but inside the 70-minute margin: its chunks
    // defer while the other families run.
    p.claude(now() + 30 * 60);
    std::fs::write(&p.hold, b"codex gpt-6.1-sol 3").unwrap();
    let first = Running::start(p.authorized(&[]));
    // Mid-chunk: the third codex-sol trial has launched and is holding.
    wait("three codex-sol launches", || {
        p.count("codex gpt-6.1-sol") == 3
    });
    let launches = p.launches();
    assert_eq!(launches.len(), 3, "controls launch no CLI: {launches:?}");
    // One writer owns the run: a competing resume refuses and starts nothing.
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("run_writer_busy"), "{err}");
    first.signal(libc::SIGKILL);
    let (_, lines) = first.finish();
    assert!(
        lines.iter().any(|l| l.contains("deferring claude")),
        "{lines:?}"
    );
    let scored_before = retained_results(&p);
    assert_eq!(scored_before.len(), 2, "two codex-sol scores committed");
    // The coordinator died: cleanup of its attempt is unproved.
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    let attempt = attempt_id(&err.lines().map(str::to_string).collect::<Vec<_>>());
    // The orphaned trial still holds admission: confirmation cannot override it.
    let (code, _, err) = run(&mut p.authorized(&["--resume", "--confirm-cleanup", &attempt]));
    assert_eq!(code, Some(2));
    assert!(err.contains("trial admission is still held"), "{err}");
    std::fs::remove_file(&p.hold).unwrap();
    wait("the orphaned trial to end", swarmail_evals::admission::idle);
    assert_eq!(p.launches().len(), 3, "refusals launched nothing");
    // The orphaned worker may still have committed its own hash-bound score
    // after its owned cleanup; such a slot is scored and never re-run.
    let scored = |p: &Paid| {
        retained_results(p)
            .values()
            .filter(|b| !contains(b, "\"outcome\":\"harness_error\""))
            .count()
    };
    let orphan_scored = scored(&p);
    assert!((2..=3).contains(&orphan_scored));
    // Resume with the confirmation; interrupt between chunks, after codex-luna.
    let second = Running::start(p.authorized(&["--resume", "--confirm-cleanup", &attempt]));
    second.until("codex-luna closed", |l| {
        l.contains("chunk closed") && l.contains("agent=codex-luna")
    });
    second.signal(libc::SIGTERM);
    let (code, lines) = second.finish();
    assert_eq!(code, Some(2), "{lines:?}");
    assert!(
        lines
            .iter()
            .any(|l| l.contains("operator-confirmed cleanup"))
    );
    // codex-sol: one launch per committed slot plus the killed one only if
    // its score was lost; codex-luna: five.
    let codex_sol = 3 + 5 - orphan_scored;
    assert_eq!(p.count("codex gpt-6.1-sol"), codex_sol);
    assert_eq!(p.count("codex gpt-6-luna"), 5);
    let mid = retained_results(&p);
    for (id, bytes) in &scored_before {
        assert_eq!(mid.get(id), Some(bytes), "committed score bytes unchanged");
    }
    // Resume: dsh runs, then every remaining chunk is Claude's and the run
    // waits with its reason until the operator renews that login.
    let third = Running::start(p.authorized(&["--resume"]));
    third.until("the renewal wait", |l| {
        l.contains("baseline waiting") && l.contains("blocked: claude")
    });
    assert_eq!(p.count("claude claude-opus-5-5"), 0);
    p.claude(now() + 24 * HOUR);
    let (code, lines) = third.finish();
    assert_eq!(code, Some(1), "fake CLIs are measured failures: {lines:?}");
    let starts: Vec<_> = lines
        .iter()
        .filter_map(|l| l.split("agent=").nth(1)?.split(' ').next())
        .filter(|_| true)
        .collect();
    assert!(starts.iter().any(|a| *a == "claude-opus"), "{lines:?}");
    let launches = p.launches();
    assert!(!launches.contains(&"LEAK".to_string()));
    for (line, n) in [
        ("claude claude-opus-5-5", 5),
        ("claude claude-sonnet-5-5", 5),
        ("codex gpt-6.1-sol", codex_sol),
        ("codex gpt-6-luna", 5),
        ("dsh spark-glm", 5),
    ] {
        assert_eq!(p.count(line), n, "{line}: {launches:?}");
    }
    // An interrupt that reached a started dashscope trial leaves one unscored
    // launch; otherwise there are exactly five.
    let dashscope = p.count("dsh dashscope");
    assert!((5..=6).contains(&dashscope), "{launches:?}");
    // Claude ran last: every claude launch follows every other family's.
    let first_claude = launches
        .iter()
        .position(|l| l.starts_with("claude"))
        .unwrap();
    assert!(
        launches[first_claude..]
            .iter()
            .all(|l| l.starts_with("claude"))
    );
    let rows = paid_rows(&p);
    assert_eq!(rows.len(), 30);
    let plan = p.plan();
    for (i, row) in rows.iter().enumerate() {
        let spec = &plan.agents[i / 5];
        assert_eq!(row["agent"], spec.label);
        assert_eq!(row["outcome"], "fail");
        let family = spec.wrapper_profile.unwrap().family();
        assert_eq!(
            row["provenance"]["agent"]["cli"],
            json!({
                "profile": profile(family),
                "version": profile(family).cli_version(),
                "sha256": swarmail_evals::identity::file_hash(&p.bin(family)).unwrap(),
            })
        );
    }
    let controls = std::fs::read_to_string(p.output.join("controls.jsonl")).unwrap();
    let controls: Vec<Value> = controls
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(
        controls
            .iter()
            .map(|r| (r["agent"].clone(), r["outcome"].clone()))
            .collect::<Vec<_>>(),
        [
            (json!("oracle"), json!("pass")),
            (json!("null"), json!("fail"))
        ]
    );
    let report: Value =
        serde_json::from_slice(&std::fs::read(p.output.join("report.json")).unwrap()).unwrap();
    let attempts: u64 = report["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["attempts"].as_u64().unwrap())
        .sum();
    assert_eq!(attempts, 30);
    let end = retained_results(&p);
    for (id, bytes) in &scored_before {
        assert_eq!(end.get(id), Some(bytes));
    }
    // No credential, claim or banner byte reaches any retained or public file.
    let everything = all_bytes(&p.output);
    assert!(!contains(&everything, CANARY));
    assert!(!contains(&everything, SPARK_CANARY));
    assert!(!lines.iter().any(|l| l.contains(CANARY)));
    // Reopening a complete product launches nothing and reproduces its bytes.
    let before = (
        std::fs::read(p.output.join("paid.jsonl")).unwrap(),
        std::fs::read(p.output.join("report.json")).unwrap(),
    );
    std::fs::remove_file(p.output.join("report.json")).unwrap();
    let n = p.launches().len();
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(1), "{err}");
    assert_eq!(p.launches().len(), n);
    assert_eq!(
        std::fs::read(p.output.join("paid.jsonl")).unwrap(),
        before.0
    );
    assert_eq!(
        std::fs::read(p.output.join("report.json")).unwrap(),
        before.1
    );
    // Conflicting derived output refuses rather than being overwritten.
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(p.output.join("paid.jsonl"))
        .unwrap();
    f.write_all(b"{}\n").unwrap();
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("output_conflict paid.jsonl"), "{err}");
    // A replaced CLI (re-pinned or not) is identity drift for this run.
    p.write_cli("codex", &format!("{} ", banner("codex")));
    p.write_cli("codex", banner("codex"));
    std::fs::OpenOptions::new()
        .append(true)
        .open(p.bin("codex"))
        .unwrap()
        .write_all(b"# replaced\n")
        .unwrap();
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("contract codex: digest_mismatch"), "{err}");
    p.select(|_| {});
    let (code, _, err) = run(&mut p.authorized(&["--resume"]));
    assert_eq!(code, Some(2));
    assert!(err.contains("manifest_drift"), "{err}");
    assert_eq!(p.launches().len(), n);
}
