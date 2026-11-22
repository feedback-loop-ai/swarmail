//! The paid baseline's offline fixture (proposed 0137): fake direct CLIs
//! that answer the bounded version probe and log each launch, their private
//! credentials, the version-2 selection and a stderr-watching runner.
use super::*;
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

pub const CANARY: &str = "SYNTHETIC-PAID-CANARY-NEVER-RETAINED";
pub const SPARK_CANARY: &str = "SYNTHETIC-SPARK-CANARY-NEVER-RETAINED";
pub const FAMILIES: [&str; 3] = ["claude", "codex", "dsh"];
pub const HOUR: u64 = 3600;

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
pub fn private(path: &Path, bytes: &[u8]) {
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
pub fn banner(family: &str) -> &'static str {
    match family {
        "claude" => "2.1.310 (Claude Code)",
        "codex" => "codex-cli 0.164.0",
        _ => "0.1.5-rc.1",
    }
}
pub fn profile(family: &str) -> WrapperProfile {
    match family {
        "claude" => WrapperProfile::Claude2_1_310,
        "codex" => WrapperProfile::Codex0_164_0,
        _ => WrapperProfile::Dsh0_1_5Rc1,
    }
}
pub fn jwt(exp: u64) -> String {
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
pub struct Paid {
    pub dir: tempfile::TempDir,
    pub config: PathBuf,
    pub log: PathBuf,
    pub hold: PathBuf,
    pub output: PathBuf,
    pub tier: tasks::Tier,
}
impl Paid {
    pub fn new() -> Self {
        Self::for_tier(tasks::Tier::Deliverability)
    }
    pub fn for_tier(tier: tasks::Tier) -> Self {
        let dir = runner::private_tempdir("paid-baseline-").unwrap();
        let p = Self {
            tier,
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
    pub fn bin(&self, family: &str) -> PathBuf {
        self.dir.path().join(format!("{family}-cli"))
    }
    pub fn cred(&self, family: &str) -> PathBuf {
        self.dir.path().join(format!("{family}-credential.json"))
    }
    /// The stand-in answers `--version` with `version`; any other launch logs
    /// its family and model (dsh: its route), holds (bounded) while the hold
    /// file names this launch as `<family> <model> <n>`, and exits: a
    /// measured failure.
    pub fn write_cli(&self, family: &str, version: &str) {
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
    pub fn claude(&self, until: u64) {
        private(
            &self.cred("claude"),
            json!({"claudeAiOauth":{"accessToken":CANARY,"expiresAt":until * 1000,"scopes":["user:inference"]}})
                .to_string()
                .as_bytes(),
        );
    }
    pub fn codex(&self, until: u64) {
        private(
            &self.cred("codex"),
            json!({"auth_mode":"chatgpt","OPENAI_API_KEY":null,"tokens":{"id_token":CANARY,"access_token":jwt(until),"refresh_token":CANARY,"account_id":"synthetic-account"},"last_refresh":"2026-11-22T00:00:00Z"})
                .to_string()
                .as_bytes(),
        );
    }
    pub fn dsh(&self, dashscope: u64, spark: u64) {
        private(
            &self.cred("dsh"),
            json!({"DASHSCOPE_API_KEY":{"key":CANARY,"valid_through":dashscope},"SPARK_API_KEY":{"key":SPARK_CANARY,"valid_through":spark}})
                .to_string()
                .as_bytes(),
        );
    }
    /// The v2 selection with each fake's measured digest; `edit` may break it.
    pub fn select(&self, edit: impl FnOnce(&mut Value)) {
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
    pub fn plan(&self) -> baseline::Plan {
        baseline::Plan::for_tier(Some(self.tier), &self.output)
    }
    pub fn command(&self, extra: &[&str]) -> Command {
        let mut c = cli();
        let tier = serde_json::to_value(self.tier).unwrap();
        c.args(["baseline", "--tier", tier.as_str().unwrap(), "--config"])
            .arg(&self.config)
            .arg("--output")
            .arg(&self.output)
            .args(["--renewal-poll-seconds", "1"])
            .args(extra);
        c
    }
    pub fn authorized(&self, extra: &[&str]) -> Command {
        let id = self.plan().product_id;
        let mut c = self.command(&["--authorized-product", &id]);
        c.args(extra);
        c
    }
    /// Every credential valid but inside the default margin: all chunks defer.
    pub fn stale(&self) {
        let soon = now() + 10 * 60;
        self.claude(soon);
        self.codex(soon);
        self.dsh(soon, soon);
    }
    /// Run controls for real, reach the all-deferred wait, then interrupt it:
    /// the wait states its reason and no paid slot starts.
    pub fn controls_then_interrupted_wait(&self) {
        self.stale();
        let r = Running::start(self.authorized(&[]));
        r.until("the renewal wait", |l| {
            l.contains("baseline waiting: no chunk can start; blocked: claude, codex, dsh:dashscope, dsh:spark-glm")
        });
        r.signal(libc::SIGINT);
        let (code, lines) = r.finish();
        assert_eq!(code, Some(2), "{lines:?}");
        assert!(
            lines.iter().any(|l| l.contains("baseline interrupted")),
            "{lines:?}"
        );
        assert!(self.launches().is_empty());
        assert!(!self.output.join("paid").exists());
        assert!(!self.output.join("paid.jsonl").exists());
        assert!(!self.output.join("report.json").exists());
    }
    pub fn launches(&self) -> Vec<String> {
        std::fs::read_to_string(&self.log)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }
    pub fn count(&self, line: &str) -> usize {
        self.launches().iter().filter(|l| *l == line).count()
    }
}

/// A running baseline whose stderr lines are collected as they arrive.
pub struct Running {
    pub child: Child,
    pub lines: Arc<Mutex<Vec<String>>>,
}
impl Running {
    pub fn start(mut c: Command) -> Self {
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
    pub fn until(&self, what: &str, f: impl Fn(&str) -> bool) {
        wait(what, || self.lines.lock().unwrap().iter().any(|l| f(l)));
    }
    pub fn signal(&self, sig: i32) {
        unsafe { libc::kill(self.child.id() as i32, sig) };
    }
    pub fn finish(mut self) -> (Option<i32>, Vec<String>) {
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
pub fn wait(what: &str, f: impl Fn() -> bool) {
    let start = Instant::now();
    while !f() {
        assert!(
            start.elapsed() < Duration::from_secs(180),
            "timed out waiting for {what}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
pub fn run(c: &mut Command) -> (Option<i32>, String, String) {
    let out = c.stdin(Stdio::null()).output().unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}
/// Every byte under `root`, for leak checks.
pub fn all_bytes(root: &Path) -> Vec<u8> {
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
pub fn contains(hay: &[u8], needle: &str) -> bool {
    hay.windows(needle.len()).any(|w| w == needle.as_bytes())
}
