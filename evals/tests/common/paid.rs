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
        "claude" => "2.1.329 (Claude Code)",
        "codex" => "codex-cli 0.166.0",
        _ => "0.2.2-rc.1",
    }
}
/// The fake node runtime's own banner and the package's entry file.
pub const NODE_BANNER: &str = "v22.11.0";
pub const DSH_ENTRY: &str = "bin/dsh.js";
pub fn profile(family: &str) -> WrapperProfile {
    match family {
        "claude" => WrapperProfile::Claude2_1_329,
        "codex" => WrapperProfile::Codex0_166_0,
        _ => WrapperProfile::Dsh0_2_2Rc1,
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

/// Install the native stand-in at `path` in forwarding mode, with
/// `behaviour` as the script it replaces itself with. A forwarder's pin
/// covers only the forwarder, so only launch-boundary proofs use it, never
/// a CLI-identity proof.
pub fn install_native(path: &Path, behaviour: &[u8]) {
    let mut hop = path.as_os_str().to_owned();
    hop.push(".hop");
    let hop = PathBuf::from(hop);
    let tmp = path.with_extension("next");
    std::fs::write(&tmp, behaviour).unwrap();
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::rename(&tmp, &hop).unwrap();
    std::fs::copy(env!("CARGO_BIN_EXE_paid-cli-standin"), &tmp).unwrap();
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::rename(&tmp, path).unwrap();
}

/// Install the native stand-in at `path` as a self-contained fake CLI: the
/// image plus its marker and `config`, one file with no forwarding.
pub fn install_configured(path: &Path, config: &Value) {
    let tmp = path.with_extension("next");
    let mut image = std::fs::read(env!("CARGO_BIN_EXE_paid-cli-standin")).unwrap();
    image.extend_from_slice(b"\0PAID-CLI-STANDIN-CONFIG\0");
    image.extend_from_slice(config.to_string().as_bytes());
    std::fs::write(&tmp, image).unwrap();
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::rename(&tmp, path).unwrap();
}

/// The stand-in decodes TOML independently with the standard library's
/// tomllib (Python 3.11+). /usr/bin/python3 is new enough on Linux runners,
/// but macOS ships 3.9 there, so take the first python3 that has tomllib,
/// trying /usr/bin/python3 and then the test's PATH.
pub fn standin_python() -> &'static str {
    static PYTHON: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PYTHON.get_or_init(|| {
        let path = std::env::var_os("PATH").unwrap_or_default();
        std::iter::once(PathBuf::from("/usr/bin/python3"))
            .chain(std::env::split_paths(&path).map(|dir| dir.join("python3")))
            .filter(|candidate| candidate.is_absolute())
            .find(|candidate| {
                std::process::Command::new(candidate)
                    .args(["-c", "import tomllib"])
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .status()
                    .is_ok_and(|status| status.success())
            })
            .expect("the wrapper stand-in needs a python3 with tomllib (3.11+)")
            .to_str()
            .expect("a UTF-8 interpreter path")
            .to_owned()
    })
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
    /// Route (b): the one pinned stand-in driver every family's selection
    /// names, and its mode file (see [`Paid::nested`]).
    pub driver: Option<PathBuf>,
}
impl Paid {
    pub fn new() -> Self {
        Self::for_tier(tasks::Tier::Deliverability)
    }
    pub fn for_tier(tier: tasks::Tier) -> Self {
        Self::build(tier, false)
    }
    /// The same private run routed through one offline forge-driver/v1
    /// stand-in (`tests/wrappers_launch/fake_driver.py`, admitted only by a
    /// debug build): the CLIs stand under the names the driver looks up,
    /// dsh's entry starts the pinned runtime, and each family's selection
    /// carries its nested pin (codex at `workspace-write`).
    pub fn nested() -> Self {
        Self::build(tasks::Tier::Deliverability, true)
    }
    fn build(tier: tasks::Tier, nested: bool) -> Self {
        let dir = runner::private_tempdir("paid-baseline-").unwrap();
        let driver = nested.then(|| dir.path().join("brokkr-0.15.2").join("brokkr"));
        let p = Self {
            tier,
            driver,
            config: dir.path().join("selection.json"),
            log: dir.path().join("launches.log"),
            hold: dir.path().join("hold"),
            output: dir.path().join("run"),
            dir,
        };
        if let Some(driver) = &p.driver {
            for d in [driver.parent().unwrap(), &p.dir.path().join("cli")] {
                std::fs::create_dir(d).unwrap();
                std::fs::set_permissions(d, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
            p.driver_mode(json!({"mode": "ok"}));
            let body = format!(
                "#!{}\nFAMILY=None\nMODE_FILE={}\nRECORD={}\n{}",
                standin_python(),
                json!(p.dir.path().join("driver-mode.json")),
                json!(p.dir.path().join("driver-record.json")),
                include_str!("../wrappers_launch/fake_driver.py")
            );
            install_native(driver, body.as_bytes());
        }
        for family in FAMILIES {
            p.write_cli(family, banner(family));
        }
        p.claude(now() + 24 * HOUR);
        p.codex(now() + 24 * HOUR);
        p.dsh(now() + 24 * HOUR, now() + 24 * HOUR);
        p.select(|_| {});
        p
    }
    /// Each family's pinned native binary: for dsh, the node runtime. The
    /// nested route names them as the driver looks them up on PATH.
    pub fn bin(&self, family: &str) -> PathBuf {
        match (&self.driver, family) {
            (None, _) => self.dir.path().join(format!("{family}-cli")),
            (Some(_), "dsh") => self.dir.path().join("cli").join("node"),
            (Some(_), _) => self.dir.path().join("cli").join(family),
        }
    }
    /// Replace the stand-in driver's mode (its session finals by family).
    pub fn driver_mode(&self, mode: Value) {
        let path = self.dir.path().join("driver-mode.json");
        let _ = std::fs::remove_file(&path);
        private(&path, mode.to_string().as_bytes());
    }
    /// The dsh package root: an entry, a manifest and an in-root link.
    pub fn package(&self) -> PathBuf {
        let root = self.dir.path().join("dsh-package");
        if !root.exists() {
            use std::os::unix::fs::PermissionsExt as _;
            // The nested route starts the entry itself: it names the runtime.
            let entry = match self.driver {
                Some(_) => format!("#!{}\n// dsh entry\n", self.bin("dsh").display()),
                None => "// dsh entry\n".into(),
            };
            let mode = if self.driver.is_some() { 0o755 } else { 0o644 };
            for (rel, body, mode) in [
                (DSH_ENTRY, entry.as_str(), mode),
                ("package.json", "{\"name\":\"dsh\"}", 0o644),
            ] {
                let path = root.join(rel);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, body).unwrap();
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
            }
            for dir in [root.clone(), root.join("bin")] {
                std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
        root
    }
    /// The interpreted pin the dsh selection names, measured now.
    pub fn dsh_pin(&self) -> swarmail_evals::interpreted::Pin {
        swarmail_evals::interpreted::pin(&self.bin("dsh"), NODE_BANNER, &self.package(), DSH_ENTRY)
            .unwrap()
    }
    pub fn cred(&self, family: &str) -> PathBuf {
        self.dir.path().join(format!("{family}-credential.json"))
    }
    /// The stand-in answers `--version` with `version`; any other launch logs
    /// its family and model (dsh: its route), holds (bounded) while the hold
    /// file names this launch as `<family> <model> <n>`, and exits: a
    /// measured failure. Its whole behaviour rides in the pinned file, so
    /// the digest a row records covers every byte that ran.
    /// dsh's fake is the node runtime: it answers its own banner, and
    /// `version` only when given the package's entry first.
    pub fn write_cli(&self, family: &str, version: &str) {
        let mut config = json!({
            "family": family,
            "version": version,
            "log": self.log,
            "hold": self.hold,
            "canaries": [CANARY, SPARK_CANARY],
        });
        if family == "dsh" {
            config["version"] = json!(NODE_BANNER);
            config["entry"] = json!(self.package().join(DSH_ENTRY));
            config["entry_version"] = json!(version);
        }
        install_configured(&self.bin(family), &config);
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
        let pin = self.dsh_pin();
        if let Some(driver) = &self.driver {
            for family in FAMILIES {
                families[family]["nested"] = json!({
                    "driver": driver,
                    "driver_sha256": swarmail_evals::identity::file_hash(driver).unwrap(),
                    "driver_version": swarmail_evals::wrappers::DRIVER_VERSION,
                    "platform": swarmail_evals::wrappers::DRIVER_PLATFORM,
                    "archive_sha256": swarmail_evals::wrappers::DRIVER_ARCHIVE_SHA256,
                });
            }
            families["claude"]["nested"]["effort"] = json!("medium");
            families["codex"]["nested"]["effort"] = json!("low");
            families["codex"]["nested"]["codex_sandbox"] = json!("workspace-write");
        }
        families["dsh"]["executable"] = json!(pin.entry_path());
        families["dsh"]["cli_sha256"] = json!(pin.identity());
        families["dsh"]["interpreted"] = json!(pin);
        let mut v = json!({"version":2,"families":families});
        edit(&mut v);
        private(&self.config, v.to_string().as_bytes());
    }
    pub fn plan(&self) -> baseline::Plan {
        baseline::Plan::with_route(
            Some(self.tier),
            &self.output,
            baseline::FRESHNESS_MINUTES,
            self.driver.as_deref(),
        )
    }
    pub fn command(&self, extra: &[&str]) -> Command {
        let mut c = cli();
        let tier = serde_json::to_value(self.tier).unwrap();
        c.args(["baseline", "--tier", tier.as_str().unwrap(), "--config"])
            .arg(&self.config);
        if let Some(driver) = &self.driver {
            c.arg("--via-brokkr").arg(driver);
        }
        c.arg("--output")
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
