//! Route (b) at the wrapper boundary (decision 0119's 2026-11-23 addendum,
//! proposed 0138): the real wrapper speaks forge-driver/v1 to a stand-in
//! driver pinned like the release one, and reports its launch and the
//! session's usage on descriptor 3.
use super::*;
use swarmail_evals::runner::{self, LaunchReport, launch_report_of};
use swarmail_evals::tasks::Outcome;
use swarmail_evals::usage::{Fact, Reason};

/// The session-final checkpoint of each family, as the 2026-11-23 smoke
/// recorded it.
fn smoke_final(family: &str) -> Value {
    match family {
        "claude" => {
            json!({"step": "claude-code-session-finished", "input_tokens": 41871, "output_tokens": 180, "cache_read_tokens": 41867, "cache_write_tokens": 18455, "num_turns": 2, "total_cost_usd": 0.0798147, "exit_code": 0})
        }
        "codex" => {
            json!({"step": "codex-session-finished", "input_tokens": 45729, "output_tokens": 244, "cache_read_tokens": 41600, "num_turns": 1, "exit_code": 0})
        }
        _ => {
            json!({"step": "deepseek-harness-session-finished", "input_tokens": 11497, "output_tokens": 323, "cache_read_tokens": 4096, "num_turns": 2, "exit_code": 0})
        }
    }
}

pub(super) struct Nested {
    pub(super) c: Case,
    family: &'static str,
    model: &'static str,
    pub(super) driver: PathBuf,
    record: PathBuf,
    mode: PathBuf,
    pub(super) cli: PathBuf,
}

impl Nested {
    pub(super) fn new(family: &'static str, model: &'static str) -> Self {
        let c = Case::new(family);
        let base = c.root.path().to_path_buf();
        let mode = base.join("driver-mode.json");
        let record = base.join("driver-record.json");
        let body = format!(
            "#!{}\nFAMILY={}\nMODE_FILE={}\nRECORD={}\n{}",
            standin_python(),
            json!(family),
            json!(mode),
            json!(record),
            include_str!("fake_driver.py")
        );
        let driver = base.join("brokkr-0.15.2").join("brokkr");
        dir(driver.parent().unwrap());
        common::paid::install_native(&driver, body.as_bytes());
        // The pinned CLI, named as the driver looks it up on PATH.
        let cli_dir = base.join("cli");
        dir(&cli_dir);
        let cli = cli_dir.join(if family == "dsh" { "node" } else { family });
        common::paid::install_native(&cli, b"#!/bin/sh\nexit 0\n");
        let n = Self {
            c,
            family,
            model,
            driver,
            record,
            mode,
            cli,
        };
        n.select(json!({}));
        n
    }
    /// A version-2 selection with the CLI pin and the driver pin; `edit`
    /// overrides members of the nested pin (null removes it).
    fn select(&self, edit: Value) {
        let c = &self.c;
        let future = 4_102_444_800u64;
        let credential = c.root.path().join("selected private.json");
        if self.family == "dsh" {
            replace_private(
                &credential,
                json!({"DASHSCOPE_API_KEY":{"key":CANARY,"valid_through":future},"SPARK_API_KEY":{"key":"SYNTHETIC-SPARK-CANARY","valid_through":future}})
                    .to_string()
                    .as_bytes(),
            );
        }
        if self.family == "codex" {
            let token = common::paid::jwt(future);
            replace_private(
                &credential,
                json!({"auth_mode":"chatgpt","OPENAI_API_KEY":null,"tokens":{"id_token":CANARY,"access_token":token,"refresh_token":"SOURCE-REFRESH-NEVER-FORWARDED","account_id":"synthetic-account"},"last_refresh":"2026-10-26T00:00:00Z"})
                    .to_string()
                    .as_bytes(),
            );
        }
        let profile = match self.family {
            "claude" => swarmail_evals::provenance::WrapperProfile::Claude2_1_329,
            "codex" => swarmail_evals::provenance::WrapperProfile::Codex0_166_0,
            _ => swarmail_evals::provenance::WrapperProfile::Dsh0_2_2Rc1,
        };
        let mut nested = json!({
            "driver": self.driver,
            "driver_sha256": swarmail_evals::identity::file_hash(&self.driver).unwrap(),
            "driver_version": wrappers::DRIVER_VERSION,
            "platform": wrappers::DRIVER_PLATFORM,
            "archive_sha256": wrappers::DRIVER_ARCHIVE_SHA256,
        });
        match self.family {
            "claude" => nested["effort"] = json!("medium"),
            "codex" => {
                nested["effort"] = json!("low");
                nested["codex_sandbox"] = json!("workspace-write");
            }
            _ => {}
        }
        if let Some(edit) = edit.as_object() {
            for (k, v) in edit {
                if v.is_null() {
                    nested.as_object_mut().unwrap().remove(k);
                } else {
                    nested[k] = v.clone();
                }
            }
        }
        let mut family = json!({
            "executable": self.cli,
            "credential_file": credential,
            "profile": profile,
            "cli_version": profile.cli_version(),
            "cli_sha256": swarmail_evals::identity::file_hash(&self.cli).unwrap(),
            "auth_shape": wrappers::auth_shape(self.family),
            "nested": nested,
        });
        if edit.is_null() {
            family.as_object_mut().unwrap().remove("nested");
        }
        if self.family == "dsh" {
            let pin = self.dsh_pin();
            family["executable"] = json!(pin.entry_path());
            family["cli_sha256"] = json!(pin.identity());
            family["interpreted"] = json!(pin);
        }
        replace_private(
            &c.manifest,
            json!({"version": 2, "families": {self.family: family}})
                .to_string()
                .as_bytes(),
        );
    }
    /// Where the debug admission looks for the stand-in it trusts: beside
    /// the profile directory this harness runs under `deps/` in.
    fn admitted_image() -> PathBuf {
        let me = std::env::current_exe().unwrap();
        let dir = me.parent().unwrap();
        let dir = if dir.file_name().is_some_and(|n| n == "deps") {
            dir.parent().unwrap()
        } else {
            dir
        };
        dir.join(wrappers::STANDIN_DRIVER_NAME)
    }
    /// At the point of use, after a restore and reselect: the selected
    /// digest, this test's private driver bytes and the image the debug
    /// admission hashes agree. A read failure and a digest mismatch name
    /// themselves, so a refusal that follows is the verifier's own.
    fn restored_inputs_agree(&self, restored: &[u8]) {
        let selection: Value =
            serde_json::from_slice(&std::fs::read(&self.c.manifest).unwrap()).unwrap();
        let selected = selection["families"][self.family]["nested"]["driver_sha256"]
            .as_str()
            .map(str::to_owned);
        let private = swarmail_evals::identity::file_hash(&self.driver);
        let image = Self::admitted_image();
        let adjacent = swarmail_evals::identity::file_hash(&image);
        let bytes = std::fs::read(&self.driver).map(|b| b == restored);
        assert_eq!(bytes.as_ref().ok(), Some(&true), "private bytes restored");
        assert_eq!(
            (selected.as_ref(), private.as_ref().ok()),
            (private.as_ref().ok(), adjacent.as_ref().ok()),
            "selected / private / adjacent {} ({private:?}, {adjacent:?})",
            image.display()
        );
    }
    /// Re-pin after the driver or CLI bytes changed.
    pub(super) fn reselect(&self) {
        let _ = std::fs::remove_dir_all(self.c.root.path().join("dsh-nested"));
        self.select(json!({}));
    }
    /// dsh's package: an executable entry whose `#!` names the pinned
    /// runtime itself.
    pub(super) fn dsh_pin(&self) -> swarmail_evals::interpreted::Pin {
        let root = self.c.root.path().join("dsh-nested");
        if !root.exists() {
            std::fs::create_dir_all(root.join("bin")).unwrap();
            for d in [root.clone(), root.join("bin")] {
                std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
            let entry = root.join("bin/dsh.js");
            std::fs::write(&entry, format!("#!{}\n// dsh\n", self.cli.display())).unwrap();
            std::fs::set_permissions(&entry, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        swarmail_evals::interpreted::pin(&self.cli, "v22.11.0", &root, "bin/dsh.js").unwrap()
    }
    fn clear_home(&self) {
        for entry in std::fs::read_dir(&self.c.home).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                std::fs::remove_dir_all(&path).unwrap();
            } else {
                std::fs::remove_file(&path).unwrap();
            }
        }
        let _ = std::fs::remove_file(&self.record);
    }
    /// One launch with a fresh HOME and descriptor 3 as the launch pipe:
    /// the exit code, stderr and what the pipe carried.
    pub(super) fn launch(&self, mode: Value) -> (Option<i32>, String, Vec<u8>) {
        self.launch_env(mode, &[])
    }
    /// [`Nested::launch`] with extra wrapper environment (test equipment).
    pub(super) fn launch_env(
        &self,
        mode: Value,
        env: &[(&str, &str)],
    ) -> (Option<i32>, String, Vec<u8>) {
        use std::os::fd::IntoRawFd;
        use std::os::unix::process::CommandExt;
        self.clear_home();
        let _ = std::fs::remove_file(&self.mode);
        private(&self.mode, mode.to_string().as_bytes());
        let (mut reader, writer) = std::io::pipe().unwrap();
        let raw = writer.into_raw_fd();
        let mut cmd = self.c.command(self.family, self.model);
        cmd.arg("--via-brokkr")
            .arg(&self.driver)
            .arg("--paid-launch")
            .arg(self.binding())
            .envs(env.iter().copied());
        let mut cmd = granted(&cmd, 1, &self.binding());
        unsafe {
            cmd.pre_exec(move || {
                let ok = if raw == 3 {
                    libc::fcntl(raw, libc::F_SETFD, 0) == 0
                } else {
                    libc::dup2(raw, 3) == 3
                };
                if ok {
                    Ok(())
                } else {
                    Err(std::io::Error::last_os_error())
                }
            });
        }
        let out = cmd.output().unwrap();
        // The parent's write end closes, so the pipe reads to its end.
        unsafe { libc::close(raw) };
        drop(cmd);
        let mut report = Vec::new();
        std::io::Read::read_to_end(&mut reader, &mut report).unwrap();
        (
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
            report,
        )
    }
    /// The host route's paid-launch binding a coordinator would pass: the
    /// current selection's nested identity, valid inside every stand-in
    /// login (claude's ends in 2033). A selection without a nested pin
    /// gets a well-formed digest that cannot match.
    pub(super) fn binding(&self) -> String {
        let sha = wrappers::contract_identity(&self.c.manifest, self.family, true)
            .unwrap_or_else(|_| "0".repeat(64));
        format!("{sha}:1900000000")
    }
    fn record(&self) -> Value {
        serde_json::from_slice(&std::fs::read(&self.record).unwrap()).unwrap()
    }
}

/// `cmd` run as the paid coordinator's child: the debug-only grant
/// stand-in hands it `endpoints` grant endpoints and answers each once for
/// `grant`, through the coordinator's own endpoint code. Environment and
/// working directory are `cmd`'s.
pub(super) fn granted(
    cmd: &std::process::Command,
    endpoints: usize,
    grant: &str,
) -> std::process::Command {
    faulty(cmd, endpoints, grant, &[])
}

/// [`granted`], with the stand-in coordinator's `fault` arguments.
pub(super) fn faulty(
    cmd: &std::process::Command,
    endpoints: usize,
    grant: &str,
    fault: &[&str],
) -> std::process::Command {
    let mut c = std::process::Command::new(common::exe());
    c.env_clear()
        .args(["grant-standin", "--endpoints", &endpoints.to_string()])
        .args(["--grant", grant])
        .args(fault)
        .arg("--")
        .arg(cmd.get_program())
        .args(cmd.get_args());
    for (k, v) in cmd.get_envs() {
        if let Some(v) = v {
            c.env(k, v);
        }
    }
    if let Some(dir) = cmd.get_current_dir() {
        c.current_dir(dir);
    }
    c
}

pub(super) const CASES: [(&str, &str); 3] = [
    ("claude", "claude-sonnet-5-5"),
    ("codex", "gpt-6-luna"),
    ("dsh", "spark-glm/GLM-5.3-Flash-EXL3"),
];

fn flags(family: &str, model: &str) -> Vec<String> {
    let f: Vec<&str> = match family {
        "claude" => vec![
            "--permission-mode",
            "acceptEdits",
            "--model",
            model,
            "--effort",
            "medium",
        ],
        "codex" => vec![
            "--model",
            model,
            "--effort",
            "low",
            "--sandbox",
            "workspace-write",
        ],
        _ => vec!["--model", model],
    };
    ["driver", family, "--"]
        .into_iter()
        .chain(f)
        .map(String::from)
        .collect()
}

#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
#[test]
fn a_nested_session_reports_ready_then_its_session_final_usage() {
    for (family, model) in CASES {
        let n = Nested::new(family, model);
        let finals = [smoke_final(family), smoke_final(family)];
        let (code, err, report) = n.launch(json!({"mode": "complete", "finals": finals}));
        assert_eq!(code, Some(0), "{family}: {err}");
        assert!(!err.contains(CANARY));
        let LaunchReport::Nested(receipt) = launch_report_of(&report) else {
            panic!("{family}: {}", String::from_utf8_lossy(&report));
        };
        assert_eq!(
            (receipt.disposition.as_str(), receipt.family.as_str()),
            ("completed", family)
        );
        // The session-final totals, counted once; never the per-turn row
        // or the totals the result object repeats.
        let (f, want) = (receipt.facts, smoke_final(family));
        let count = |k: &str| Fact::Present {
            value: want[k].as_u64().unwrap(),
        };
        assert_eq!(f.input_tokens, count("input_tokens"), "{family}");
        assert_eq!(f.output_tokens, count("output_tokens"));
        assert_eq!(f.cache_read_tokens, count("cache_read_tokens"));
        assert_eq!(f.num_turns, count("num_turns"));
        if family == "claude" {
            assert_eq!(f.cache_write_tokens, count("cache_write_tokens"));
            assert_eq!(f.total_cost_usd, Fact::Present { value: 0.0798147 });
        } else {
            let absent = Reason::FieldAbsent;
            assert_eq!(f.total_cost_usd, Fact::Unavailable { reason: absent });
            assert_eq!(f.cache_write_tokens, Fact::Unavailable { reason: absent });
        }
        let rec = n.record();
        // Only the observed family flags follow the separator.
        assert_eq!(rec["argv"], json!(flags(family, model)), "{family}");
        // hello, start with exactly the eight keys, shutdown, then EOF.
        let engine = rec["hello"]["engine_version"].as_str().unwrap();
        assert!(engine.starts_with("swarmail-evals "));
        let start = &rec["start"];
        assert_eq!(start["seat"], "trial");
        let mut keys: Vec<_> = start["input"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        keys.sort();
        let want_keys = [
            "allowed_results",
            "context",
            "feature",
            "phase",
            "result_path",
            "role_path",
            "seat",
            "workdir",
        ];
        assert_eq!(keys, want_keys);
        assert_eq!(start["input"]["feature"], json!(n.c.values["EVAL_GOAL"]));
        assert_eq!(start["input"]["allowed_results"], json!(["done", "stuck"]));
        assert_eq!(start["input"]["workdir"], json!(n.c.work));
        assert!(
            rec["charter"]
                .as_str()
                .unwrap()
                .contains("Swarmail evaluation trial")
        );
        assert_eq!(rec["result_written"], true);
        assert_eq!(rec["shutdown"]["type"], "shutdown");
        assert_eq!(rec["eof_after_shutdown"], true);
        assert_eq!(receipt.attempt_id, start["attempt_id"].as_str().unwrap());
        // The driver resolves the pinned CLI first on PATH, never inherits
        // the launch pipe, and sees only the selected route's key.
        let env = &rec["env"];
        let cli_dir = n.cli.parent().unwrap().display().to_string();
        assert_eq!(env["PATH"], json!(format!("{cli_dir}:/usr/bin:/bin")));
        assert_eq!(rec["fd3"], Value::Null, "{family}: the launch pipe leaked");
        assert_eq!(
            rec["fd4"],
            Value::Null,
            "{family}: the grant endpoint leaked"
        );
        assert!(!env.to_string().contains(CANARY), "{family}");
        assert!(env.get(SEAM_SENTINEL).is_none());
        assert_eq!(env["HOME"], json!(n.c.home));
        let home: Vec<(String, String)> = serde_json::from_value(rec["home"].clone()).unwrap();
        for (path, mode) in &home {
            let want = if n.c.home.join(path).is_dir() {
                "0o700"
            } else {
                "0o600"
            };
            assert_eq!(mode, want, "{family}: {path}");
        }
        let names: Vec<&str> = home.iter().map(|(p, _)| p.as_str()).collect();
        let expected: &[&str] = match family {
            "claude" => &[
                ".claude/.credentials.json",
                ".claude/settings.json",
                ".claude.json",
            ],
            "codex" => &[".codex/auth.json", ".codex/config.toml"],
            _ => &[".dsh/profiles/headless/cordis.patch.yml"],
        };
        for p in expected {
            assert!(names.contains(p), "{family}: {p} in {names:?}");
        }
        match family {
            "claude" => assert_eq!(env["BROKKR_CLAUDE_BIN"], json!(n.cli)),
            "codex" => assert_eq!(env["CODEX_HOME"], json!(n.c.home.join(".codex"))),
            _ => {
                assert_eq!(env["BROKKR_DSH_BIN"], json!(n.dsh_pin().entry_path()));
                assert_eq!(env["SPARK_API_KEY"], "<present>");
                assert!(env.get("DASHSCOPE_API_KEY").is_none());
            }
        }
    }
    // dsh's other route: the DashScope key alone, by name.
    let n = Nested::new("dsh", "dashscope/glm-5.3");
    let (code, err, _) = n.launch(json!({"mode": "complete", "finals": [smoke_final("dsh")]}));
    assert_eq!(code, Some(0), "{err}");
    let env = &n.record()["env"];
    assert_eq!(env["DASHSCOPE_API_KEY"], "<present>");
    assert!(env.get("SPARK_API_KEY").is_none());
}

#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
#[test]
fn every_nested_ending_without_a_completed_result_is_a_typed_harness_error() {
    let n = Nested::new("claude", "claude-sonnet-5-5");
    let finals = json!([smoke_final("claude")]);
    // Nothing launched: a driver that exits 0 silently, or refuses before
    // work, is unstarted, never the agent giving up.
    for (mode, code) in [
        ("silent", "driver_launched_nothing"),
        ("refuse", "driver_refused_start"),
    ] {
        let (exit, err, report) = n.launch(json!({"mode": mode}));
        assert_eq!(exit, Some(2), "{mode}");
        assert!(err.contains(code), "{mode}: {err}");
        assert_eq!(report, b"unstarted\n", "{mode}");
        assert_eq!(launch_report_of(&report), LaunchReport::Unstarted);
    }
    // Launched, then no completed result: ready, the receipt naming why,
    // and the usage the session-final checkpoint carried.
    for (mode, result, code) in [
        ("accepted_only", "done", "driver_result_missing"),
        ("failed", "done", "driver_session_failed"),
        ("complete", "gave_up", "driver_result_invalid"),
    ] {
        let (exit, err, report) =
            n.launch(json!({"mode": mode, "finals": finals, "result": result}));
        assert_eq!(exit, Some(2), "{mode}");
        assert!(err.contains(code), "{mode}: {err}");
        let LaunchReport::Nested(receipt) = launch_report_of(&report) else {
            panic!("{mode}: {}", String::from_utf8_lossy(&report));
        };
        assert_eq!(receipt.disposition, code);
        assert_eq!(receipt.facts.input_tokens, Fact::Present { value: 41871 });
    }
    // Bounded shutdown: a protocol error past acceptance closes the
    // driver's stdin at once (the stand-in exits on that EOF), and a driver
    // that ignores shutdown while holding its stdout open is killed after
    // the grace. Both keep their typed receipt.
    let (exit, err, report) = n.launch(json!({"mode": "garbage"}));
    assert_eq!(exit, Some(2), "{err}");
    let LaunchReport::Nested(receipt) = launch_report_of(&report) else {
        panic!("{}", String::from_utf8_lossy(&report));
    };
    assert!(err.contains(&receipt.disposition), "{err}");
    assert_ne!(receipt.disposition, "completed");
    assert_eq!(n.record()["stdin_closed"], true);
    let began = std::time::Instant::now();
    // An injected short grace (debug builds only): the kill and reap
    // follow it, not the production 3 s.
    let (exit, err, report) = n.launch_env(
        json!({"mode": "deaf", "finals": finals}),
        &[(wrappers::EXIT_GRACE_TEST_ENV, "200")],
    );
    assert_eq!(exit, Some(0), "{err}");
    assert!(began.elapsed() < std::time::Duration::from_secs(60));
    assert!(began.elapsed() >= std::time::Duration::from_millis(200));
    let LaunchReport::Nested(receipt) = launch_report_of(&report) else {
        panic!("{}", String::from_utf8_lossy(&report));
    };
    assert_eq!(receipt.disposition, "completed");
    assert_eq!(receipt.facts.input_tokens, Fact::Present { value: 41871 });
    // No session-final checkpoint: every fact unavailable, never zero.
    let (exit, _, report) = n.launch(json!({"mode": "complete", "finals": []}));
    assert_eq!(exit, Some(0));
    let LaunchReport::Nested(receipt) = launch_report_of(&report) else {
        panic!("{}", String::from_utf8_lossy(&report));
    };
    let none = swarmail_evals::usage::Facts::unavailable(Reason::NoSessionFinal);
    assert_eq!(receipt.facts, none);
}

#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
#[test]
fn the_driver_pin_refuses_before_any_private_write_or_spawn() {
    let n = Nested::new("codex", "gpt-6-luna");
    let other = n.c.root.path().join("other-brokkr");
    common::paid::install_native(&other, b"#!/bin/sh\necho 'brokkr 0.12.0'\n");
    let other_sha = swarmail_evals::identity::file_hash(&other).unwrap();
    let refuse = |edit: Value, via: &Path, code: &str| {
        n.select(edit);
        n.clear_home();
        let mut cmd = n.c.command("codex", "gpt-6-luna");
        cmd.arg("--via-brokkr").arg(via);
        cmd.arg("--paid-launch").arg(n.binding());
        let out = granted(&cmd, 1, &n.binding()).output().unwrap();
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{code}: {err}");
        assert!(err.contains(code), "{code}: {err}");
        assert!(!n.record.exists(), "{code}: the driver ran");
        let home_empty = std::fs::read_dir(&n.c.home).unwrap().next().is_none();
        assert!(home_empty, "{code}: HOME written");
    };
    refuse(json!(null), &n.driver, "driver_pin_missing");
    refuse(json!({}), &other, "driver_selection_mismatch");
    refuse(
        json!({"driver_sha256": "0".repeat(64)}),
        &n.driver,
        "driver_release_unrecognized",
    );
    // An admitted pin over other bytes: measured at launch.
    let altered = n.c.root.path().join("altered-brokkr");
    let mut bytes = std::fs::read(&n.driver).unwrap();
    bytes.push(0);
    std::fs::write(&altered, bytes).unwrap();
    std::fs::set_permissions(&altered, std::fs::Permissions::from_mode(0o700)).unwrap();
    refuse(
        json!({"driver": altered, "driver_sha256": other_sha}),
        &altered,
        "driver_digest_mismatch",
    );
    refuse(
        json!({"driver_version": "0.12.0"}),
        &n.driver,
        "driver_version_unsupported",
    );
    refuse(
        json!({"archive_sha256": "1".repeat(64)}),
        &n.driver,
        "driver_archive_unrecognized",
    );
    refuse(
        json!({"codex_sandbox": null}),
        &n.driver,
        "driver_sandbox_invalid",
    );
    refuse(
        json!({"codex_sandbox": "danger-full-access"}),
        &n.driver,
        "driver_sandbox_unconfined",
    );
    // A driver whose banner is not the release's.
    let swapped = json!({"driver": other, "driver_sha256": other_sha});
    refuse(swapped, &other, "driver_version_mismatch");
}

/// Fail closed (the operator's 2026-11-24 ruling): a valid nested selection
/// with neither the host's paid-launch binding nor the recipe's read-only
/// binds refuses before any credential read, HOME write or driver spawn.
/// Outside a recipe seat the release pin refuses the stand-in first; with
/// the stand-in admitted, this host's writable mounts refuse it.
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
#[test]
fn an_unbound_nested_selection_refuses_before_any_spend() {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::process::CommandExt;
    let n = Nested::new("claude", "claude-sonnet-5-5");
    let fresh = |n: &Nested| {
        n.clear_home();
        let _ = std::fs::remove_file(&n.mode);
        private(&n.mode, json!({"mode": "complete"}).to_string().as_bytes());
        let mut cmd = n.c.command("claude", "claude-sonnet-5-5");
        cmd.arg("--via-brokkr").arg(&n.driver);
        cmd
    };
    let refuses = |n: &Nested, mut cmd: std::process::Command, code: &str| {
        let out = cmd.output().unwrap();
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{code}: {err}");
        assert!(err.contains(code), "{code}: {err}");
        assert!(!n.record.exists(), "{code}: the driver ran");
        assert!(std::fs::read_dir(&n.c.home).unwrap().next().is_none());
    };
    // No binding: the recipe route. Claude's read-only auth there awaits
    // the operator's ruling; dsh's read-only binds this host's mount table
    // does not hold.
    refuses(&n, fresh(&n), "paid_unbound: recipe_auth_unruled");
    let d = Nested::new("dsh", "spark-glm/GLM-5.3-Flash-EXL3");
    d.clear_home();
    let mut cmd = d.c.command("dsh", "spark-glm/GLM-5.3-Flash-EXL3");
    cmd.arg("--via-brokkr").arg(&d.driver);
    refuses(&d, cmd, "paid_unbound: recipe_bind_");
    // A self-computed binding with no coordinator (the panel's hold 2): the
    // identity is a pure function of the selection, so it admits nothing.
    let mut cmd = fresh(&n);
    cmd.arg("--paid-launch").arg(n.binding());
    refuses(&n, cmd, "paid_grant_channel_missing");
    // A caller-made endpoint, its reply queued by a process that is not the
    // harness: the kernel names the pair's creator, and it is not an
    // ancestor running this executable. The pair is made close-on-exec
    // atomically, so no concurrent spawn inherits it.
    let fds = cloexec_pair();
    let (ours, theirs) = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
    assert_eq!(
        unsafe { libc::send(ours.as_raw_fd(), b"granted\n".as_ptr().cast(), 8, 0) },
        8
    );
    let mut cmd = fresh(&n);
    cmd.arg("--paid-launch").arg(n.binding());
    let raw = theirs.as_raw_fd();
    unsafe {
        cmd.pre_exec(move || {
            let placed = if raw == 4 {
                libc::fcntl(raw, libc::F_SETFD, 0) == 0
            } else {
                libc::dup2(raw, 4) == 4
            };
            if placed {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        });
    }
    refuses(&n, cmd, "paid_grant_unauthenticated");
    drop((ours, theirs));
    // The genuine coordinator code refuses a request it was not asked to
    // grant, and answers an endpoint once: a second launch on it is refused.
    let mut cmd = fresh(&n);
    cmd.arg("--paid-launch").arg(n.binding());
    refuses(&n, granted(&cmd, 1, "0:0"), "paid_grant_refused");
    // The security hold on 6074b83e: the registered coordinator, alive,
    // with a well-formed grant queued before the wrapper asked; a reply
    // with another nonce; one for another contract; and the right reply
    // sent by another process. Each refuses before any spend, and the
    // queued one without waiting for the coordinator's bound.
    for (fault, code) in [
        ("prequeue", "paid_grant_reply_stale"),
        ("nonce", "paid_grant_reply_unbound"),
        ("contract", "paid_grant_reply_unbound"),
        ("sender", "paid_grant_unauthenticated"),
    ] {
        let mut cmd = fresh(&n);
        cmd.arg("--paid-launch").arg(n.binding());
        let started = std::time::Instant::now();
        refuses(&n, faulty(&cmd, 1, &n.binding(), &["--fault", fault]), code);
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "{fault}"
        );
    }
    let mut twice = std::process::Command::new("/bin/sh");
    let mut once = fresh(&n);
    once.arg("--paid-launch").arg(n.binding());
    let words: Vec<String> = std::iter::once(once.get_program())
        .chain(once.get_args())
        .map(|a| format!("'{}'", a.to_str().unwrap()))
        .collect();
    twice
        .arg("-c")
        .arg(format!("{0} 3>/dev/null; {0} 3>/dev/null", words.join(" ")))
        .env_clear()
        .envs(once.get_envs().filter_map(|(k, v)| v.map(|v| (k, v))))
        .current_dir(once.get_current_dir().unwrap());
    let out = granted(&twice, 1, &n.binding()).output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("paid_grant_channel_closed"), "{err}");
    // A driver that is neither the release nor the stand-in refuses first.
    let driver = std::fs::read(&n.driver).unwrap();
    let mut altered = driver.clone();
    altered.push(0);
    std::fs::write(&n.driver, &altered).unwrap();
    n.reselect();
    let _ = std::fs::remove_file(&n.record);
    let mut cmd = fresh(&n);
    // The retired test-equipment variable, naming the altered bytes, in
    // this child's environment alone: it admits nothing.
    cmd.arg("--paid-launch").arg(n.binding()).env(
        "EVAL_TEST_STANDIN_DRIVER_SHA256",
        swarmail_evals::identity::file_hash(&n.driver).unwrap(),
    );
    refuses(
        &n,
        granted(&cmd, 1, &n.binding()),
        "driver_release_unrecognized",
    );
    std::fs::write(&n.driver, &driver).unwrap();
    n.reselect();
}

/// A local sequenced-packet pair made close-on-exec atomically, so no
/// concurrent spawn inherits it.
#[cfg(target_os = "linux")]
fn cloexec_pair() -> [libc::c_int; 2] {
    let mut fds = [0; 2];
    let r = unsafe {
        libc::socketpair(
            libc::AF_UNIX,
            libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
            0,
            fds.as_mut_ptr(),
        )
    };
    assert_eq!(r, 0);
    fds
}
/// Off Linux its only caller is ignored; the stub keeps it compiling.
#[cfg(not(target_os = "linux"))]
fn cloexec_pair() -> [libc::c_int; 2] {
    unreachable!("the grant is Linux's")
}

/// Through the real `trial` command with no coordinator: no paid-launch
/// binding and no recipe binds, so the nested selection fails closed. The
/// driver never runs, and the trial is a harness error, never a scored
/// attempt (the operator's 2026-11-24 ruling).
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
#[test]
fn a_nested_trial_without_a_binding_never_starts_the_driver() {
    let _scope = common::trial_scope();
    let n = Nested::new("claude", "claude-sonnet-5-5");
    private(
        &n.mode,
        json!({"mode": "complete", "finals": [smoke_final("claude")]})
            .to_string()
            .as_bytes(),
    );
    let mut command = common::cli();
    command
        .args(["trial", "--task", "signup", "--agent", "claude-nested"])
        .args(["--agent-model", "claude-sonnet-5-5"])
        .args(["--wrapper-profile", "claude_2_1_329_via_brokkr"])
        .args(["--deadline-ms", "60000", "--"])
        .arg(common::exe())
        .args(["agent-claude", "--model", "claude-sonnet-5-5", "--config"])
        .arg(&n.c.manifest)
        .arg("--via-brokkr")
        .arg(&n.driver);
    let out = command.output().unwrap();
    let row = common::one_row(&out);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!err.contains(CANARY));
    assert_eq!(row.outcome, Outcome::HarnessError, "{err}");
    assert!(err.contains("harness error: agent_unstarted"), "{err}");
    assert!(!n.record.exists(), "the unbound driver ran");
}

/// The worker context a paid coordinator hands one trial (main.rs's
/// `WorkerContext`), built from the library's own identities: the slot, the
/// candidate Swarmail, the agent as the trial will declare it with its
/// nested CLI identity, and the paid-launch binding.
fn paid_context(
    n: &Nested,
    trial_id: &str,
    task: &str,
    argv: &[std::ffi::OsString],
    coordinates: &swarmail_evals::provenance::MatrixCoordinates,
) -> String {
    use swarmail_evals::provenance::{CliIdentity, FixtureInputs, WrapperProfile};
    let seed = coordinates.cell_seed(task, "claude-nested").unwrap();
    let fixture = FixtureInputs::resolve(Some(&seed), None).unwrap();
    let profile = WrapperProfile::Claude2_1_329ViaBrokkr;
    let mut req = runner::TrialRequest {
        task: Some(task.into()),
        agent: Some("claude-nested".into()),
        argv: argv.to_vec(),
        deadline_ms: Some(60000),
        agent_model: Some("claude-sonnet-5-5".into()),
        wrapper_profile: Some(profile),
        fixture: fixture.clone(),
        ..Default::default()
    };
    req.matrix = Some(coordinates.clone());
    req.trial_id = Some(trial_id.into());
    let mut agent = runner::make_provenance(&req, None, None).agent;
    let sha = wrappers::contract_identity(&n.c.manifest, "claude", true).unwrap();
    agent.cli = Some(CliIdentity {
        profile,
        version: profile.cli_version().into(),
        sha256: sha.clone(),
    });
    let target = swarmail_evals::identity::TargetBuild::external(common::swarmail_bin().into());
    json!({
        "cell": {
            "trial_id": trial_id, "task": task, "agent": "claude-nested",
            "fixture": fixture, "coordinates": coordinates, "deadline_ms": 60000,
        },
        "target": target.identity,
        "agent": agent,
        "paid": {"cli_sha256": sha, "valid_through": 1_900_000_000u64},
    })
    .to_string()
}

/// Through the real `trial` command under a paid coordinator's binding:
/// `inbox-isolation`'s three agents each run a nested session, and the
/// retained leaf commits one usage companion listing every session by
/// assignment, its figure the checked per-field sum (the operator's
/// 2026-11-24 ruling). The row itself carries no usage; a report reads the
/// companion, and refuses without it rather than taking journal tokens.
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
#[test]
fn a_three_agent_nested_trial_commits_every_sessions_usage() {
    let _scope = common::trial_scope();
    let n = Nested::new("claude", "claude-sonnet-5-5");
    private(
        &n.mode,
        json!({"mode": "complete", "per_agent": true, "finals": [smoke_final("claude")]})
            .to_string()
            .as_bytes(),
    );
    let evidence = n.c.root.path().join("retained");
    let trial_id = format!("{:032x}", 0x404u32);
    let coordinates = swarmail_evals::provenance::MatrixCoordinates {
        matrix_id: "f".repeat(32),
        master_seed: "e".repeat(64),
        task_index: 0,
        agent_index: 0,
        repetition_index: 0,
    };
    let seed = coordinates
        .cell_seed("inbox-isolation", "claude-nested")
        .unwrap();
    let fixture = swarmail_evals::provenance::FixtureInputs::resolve(Some(&seed), None).unwrap();
    let argv: Vec<std::ffi::OsString> = vec![
        common::exe().into(),
        "agent-claude".into(),
        "--model".into(),
        "claude-sonnet-5-5".into(),
        "--config".into(),
        n.c.manifest.clone().into(),
        "--via-brokkr".into(),
        n.driver.clone().into(),
    ];
    let context = paid_context(&n, &trial_id, "inbox-isolation", &argv, &coordinates);
    let mut command = common::cli();
    command
        .args([
            "trial",
            "--task",
            "inbox-isolation",
            "--agent",
            "claude-nested",
        ])
        .args(["--agent-model", "claude-sonnet-5-5"])
        .args(["--wrapper-profile", "claude_2_1_329_via_brokkr"])
        .args(["--deadline-ms", "60000", "--seed", &fixture.seed])
        .args(["--fixture-time", &fixture.fixture_time])
        .args(["--worker-context", &context, "--artifacts-dir"])
        .arg(&evidence)
        .arg("--")
        .args(&argv);
    // Without the coordinator's endpoints the worker refuses before any
    // server or agent starts, whatever its context says.
    let out = command.output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        common::one_row(&out).outcome,
        Outcome::HarnessError,
        "{err}"
    );
    assert!(err.contains("paid_grant_channel_missing"), "{err}");
    assert!(
        !n.record.exists(),
        "an unauthorized worker started the driver"
    );
    let _ = std::fs::remove_dir_all(&evidence);
    let grant = format!(
        "{}:1900000000",
        wrappers::contract_identity(&n.c.manifest, "claude", true).unwrap()
    );
    let out = granted(&command, 3, &grant).output().unwrap();
    let row = common::one_row(&out);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!err.contains(CANARY));
    // The stand-in sessions did nothing to the mail: a scored failure with
    // its application grade.
    assert_eq!(row.outcome, Outcome::Fail, "{err}");
    assert_eq!(row.trial_id, trial_id);
    let text = serde_json::to_string(&row).unwrap();
    assert!(!text.contains("usage"), "the row carries no usage");
    let leaf = evidence.join(&trial_id);
    let retained = swarmail_evals::artifacts::trial(&leaf).unwrap();
    assert_eq!(retained.row, row);
    let usage = retained.usage.expect("the committed companion");
    assert_eq!(usage.trial_id, trial_id);
    assert_eq!(usage.sessions.len(), 3);
    for (i, s) in usage.sessions.iter().enumerate() {
        assert_eq!((s.agent as usize, s.disposition.as_str()), (i, "completed"));
        assert_eq!(
            s.facts.input_tokens,
            Fact::Present {
                value: 41871 + 1000 * i as u64
            }
        );
    }
    let mut attempts: Vec<_> = usage.attempts().collect();
    attempts.dedup();
    assert_eq!(attempts.len(), 3, "distinct driver attempts");
    let totals = usage.totals();
    assert_eq!(
        totals.input_tokens,
        Fact::Present {
            value: 3 * 41871 + 3000
        }
    );
    assert_eq!(totals.output_tokens, Fact::Present { value: 3 * 180 });
    assert_eq!(totals.num_turns, Fact::Present { value: 6 });
    // The report reads the companion from the retained root, and refuses
    // without it; the journal is never asked.
    let rows = n.c.root.path().join("rows.jsonl");
    std::fs::write(&rows, format!("{text}\n")).unwrap();
    let report = |extra: &[&std::ffi::OsStr]| {
        common::cli()
            .arg("report")
            .arg("--input")
            .arg(&rows)
            .args(extra)
            .output()
            .unwrap()
    };
    let read = report(&["--usage-from".as_ref(), evidence.as_os_str()]);
    assert_eq!(read.status.code(), Some(0));
    let page: Value = serde_json::from_slice(&read.stdout).unwrap();
    let nested = &page["groups"][0]["nested_usage"];
    assert_eq!(nested["total_tokens"]["value"], 3 * (41871 + 180) + 3000);
    assert_eq!(nested["tokens_per_success"]["reason"], "no_passes");
    let refused = report(&[]);
    assert_eq!(refused.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&refused.stderr).contains("nested_usage_requires_artifacts"));
    // An orphan, a removed binding and a tampered companion all refuse.
    let meta_path = leaf.join("metadata.json");
    let meta = std::fs::read(&meta_path).unwrap();
    let companion = std::fs::read(leaf.join("usage.json")).unwrap();
    let mut unbound: Value = serde_json::from_slice(&meta).unwrap();
    unbound.as_object_mut().unwrap().remove("usage");
    std::fs::write(&meta_path, unbound.to_string()).unwrap();
    let e = swarmail_evals::artifacts::trial(&leaf).unwrap_err();
    assert_eq!(e, "orphan usage companion");
    std::fs::remove_file(leaf.join("usage.json")).unwrap();
    let e = swarmail_evals::artifacts::trial(&leaf).unwrap_err();
    assert_eq!(e, "usage companion missing");
    let refused = report(&["--usage-from".as_ref(), evidence.as_os_str()]);
    assert_eq!(refused.status.code(), Some(2));
    std::fs::write(&meta_path, &meta).unwrap();
    let mut tampered = companion.clone();
    let at = tampered.iter().position(|b| *b == b'4').unwrap();
    tampered[at] = b'5';
    std::fs::write(leaf.join("usage.json"), &tampered).unwrap();
    let e = swarmail_evals::artifacts::trial(&leaf).unwrap_err();
    assert_eq!(e, "byte binding mismatch");
    std::fs::write(leaf.join("usage.json"), &companion).unwrap();
    assert!(swarmail_evals::artifacts::trial(&leaf).is_ok());
    duplicate_owners_refuse_over_a_baseline_root(&n, &leaf, &row);
}

/// Over a baseline-shaped root (`paid/<attempt>/trials/<id>`, with an
/// interrupted attempt and a capture tree beside it): one valid leaf reads;
/// the same leaf under a second attempt, a second trial whose companion
/// claims the same driver attempts, and a row that is not the committed one
/// all refuse (the panel's tests 1 and independent 5 on a73ac17a).
fn duplicate_owners_refuse_over_a_baseline_root(
    n: &Nested,
    leaf: &Path,
    row: &runner::TrialResult,
) {
    use swarmail_evals::artifacts;
    let dir = |p: &Path| {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(p)
            .unwrap()
    };
    let copy = |from: &Path, to: &Path| {
        dir(to);
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    };
    let root = n.c.root.path().join("product");
    let paid = root.join("paid");
    let first = paid.join("a".repeat(32)).join("trials");
    dir(&first);
    copy(leaf, &first.join(&row.trial_id));
    // Siblings that confer no ownership: an interrupted attempt with no
    // trials and a capture tree.
    dir(&paid.join("b".repeat(32)));
    dir(&root.join("capture").join("values"));
    let read = |rows: &[runner::TrialResult]| artifacts::companions(&paid, rows);
    assert_eq!(read(std::slice::from_ref(row)).unwrap().len(), 1);
    // A row that is not the committed one.
    let mut other = row.clone();
    other.wall_time_ms += 1;
    assert_eq!(
        read(std::slice::from_ref(&other)).unwrap_err(),
        "usage row mismatch"
    );
    // The same leaf under a second attempt: two directories own one trial.
    let second = paid.join("c".repeat(32)).join("trials");
    dir(&second);
    copy(leaf, &second.join(&row.trial_id));
    assert_eq!(
        read(std::slice::from_ref(row)).unwrap_err(),
        "duplicate usage owner"
    );
    std::fs::remove_dir_all(&second).unwrap();
    // A second, individually valid trial whose companion names the same
    // driver attempts: the bill has one owner.
    let retained = artifacts::trial(leaf).unwrap();
    let mut twin = retained.row.clone();
    twin.trial_id = format!("{:032x}", 0x405u32);
    let mut usage = retained.usage.clone().unwrap();
    usage.trial_id = twin.trial_id.clone();
    let twins = paid.join("d".repeat(32)).join("trials");
    dir(&twins);
    let state = retained.state.clone().map(|mut s| {
        s.trial_id = twin.trial_id.clone();
        s
    });
    artifacts::retain_trial(&twins, &twin, state.as_ref(), Some(&usage)).unwrap();
    assert!(artifacts::trial(&twins.join(&twin.trial_id)).is_ok());
    assert_eq!(read(std::slice::from_ref(&twin)).unwrap().len(), 1);
    assert_eq!(
        read(&[row.clone(), twin]).unwrap_err(),
        "duplicate usage owner"
    );
}

/// A nested paid contract binds the CLI's pin with the driver's pin and the
/// family's driver flags: resume compares it live, and the launch binding
/// refuses the direct identity.
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
#[test]
fn a_nested_contract_binds_the_driver_pin_and_its_flags() {
    for (family, model) in [("claude", "claude-sonnet-5-5"), ("codex", "gpt-6-luna")] {
        let n = Nested::new(family, model);
        let hop = PathBuf::from(format!("{}.hop", n.cli.display()));
        std::fs::remove_file(&n.cli).unwrap();
        std::fs::remove_file(&hop).unwrap();
        let banner = if family == "claude" {
            "2.1.329 (Claude Code)"
        } else {
            "codex-cli 0.166.0"
        };
        let log = n.c.root.path().join("cli.log");
        common::paid::install_configured(
            &n.cli,
            &json!({"family": family, "version": banner, "log": log, "hold": n.c.root.path().join("hold"), "canaries": [CANARY]}),
        );
        n.reselect();
        // The coordinator admits the release executable and, in a debug
        // build alone, the stand-in built beside it: any other driver bytes
        // refuse, whatever the environment says.
        let driver = std::fs::read(&n.driver).unwrap();
        let mut altered = driver.clone();
        altered.push(0);
        std::fs::write(&n.driver, &altered).unwrap();
        n.reselect();
        // No process-global environment write here: the retired variable
        // admits nothing, and the child-environment proof in
        // `an_unbound_nested_selection_refuses_before_any_spend` carries it.
        let refused = wrappers::paid_contracts(&n.c.manifest, &[family], true).unwrap_err();
        assert!(refused.contains("driver_release_unrecognized"), "{refused}");
        std::fs::write(&n.driver, &driver).unwrap();
        n.reselect();
        n.restored_inputs_agree(&driver);
        let contract = |nested: bool| wrappers::paid_contracts(&n.c.manifest, &[family], nested);
        let direct = contract(false).unwrap().remove(0);
        let cli_sha = swarmail_evals::identity::file_hash(&n.cli).unwrap();
        assert_eq!(
            (direct.cli.profile.via_brokkr(), direct.cli.sha256.as_str()),
            (false, cli_sha.as_str())
        );
        let nested = contract(true).unwrap().remove(0);
        assert_eq!(nested.cli.profile, direct.cli.profile.via());
        assert_ne!(nested.cli.sha256, cli_sha);
        let surface = swarmail_evals::provenance::ToolSurface::declared(nested.cli.profile);
        assert_eq!(nested.cli.validate(&surface), Ok(()));
        // Each pinned part moves the identity, so resume refuses the change.
        let edits = if family == "codex" {
            vec![
                json!({"codex_sandbox": "read-only"}),
                json!({"effort": "high"}),
            ]
        } else {
            vec![json!({"effort": "high"})]
        };
        for edit in edits {
            n.select(edit.clone());
            assert_ne!(
                contract(true).unwrap()[0].cli.sha256,
                nested.cli.sha256,
                "{edit}"
            );
        }
        n.select(json!(null));
        let err = contract(true).unwrap_err();
        assert!(err.contains("driver_pin_missing"), "{err}");
        // At launch: bound to the nested identity it runs; bound to the
        // CLI's alone it drifts before anything spawns.
        n.select(json!({}));
        // Inside both stand-in logins' validity (claude's ends in 2033).
        let future = 1_900_000_000u64;
        let bound = |sha: &str| {
            n.clear_home();
            let _ = std::fs::remove_file(&n.mode);
            private(
                &n.mode,
                json!({"mode": "complete", "finals": [smoke_final(family)]})
                    .to_string()
                    .as_bytes(),
            );
            let mut cmd = n.c.command(family, model);
            cmd.arg("--via-brokkr")
                .arg(&n.driver)
                .arg("--paid-launch")
                .arg(format!("{sha}:{future}"));
            granted(&cmd, 1, &format!("{sha}:{future}"))
                .output()
                .unwrap()
        };
        let out = bound(&nested.cli.sha256);
        assert_eq!(
            out.status.code(),
            Some(0),
            "{family}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        // Hold 4 on a73ac17a: each admitted codex class reaches the driver's
        // argv and the row's descriptor from the one selection.
        if family == "codex" {
            for class in swarmail_evals::provenance::CODEX_CLASSES {
                n.select(json!({"codex_sandbox": class}));
                let sha = wrappers::contract_identity(&n.c.manifest, family, true).unwrap();
                let out = bound(&sha);
                assert_eq!(out.status.code(), Some(0), "{class}: {out:?}");
                let argv: Vec<String> = serde_json::from_value(n.record()["argv"].clone()).unwrap();
                assert!(
                    argv.windows(2).any(|w| w == ["--sandbox", class]),
                    "{argv:?}"
                );
                let req = runner::TrialRequest {
                    task: Some("signup".into()),
                    agent: Some("codex-nested".into()),
                    argv: [
                        common::exe().into(),
                        "agent-codex".into(),
                        "--model".into(),
                        model.into(),
                        "--config".into(),
                        n.c.manifest.clone().into_os_string(),
                        "--via-brokkr".into(),
                        n.driver.clone().into_os_string(),
                    ]
                    .to_vec(),
                    agent_model: Some(model.into()),
                    wrapper_profile: Some(nested.cli.profile),
                    ..Default::default()
                };
                let agent = runner::make_provenance(&req, None, None).agent;
                let swarmail_evals::provenance::ToolSurface::Wrapper { restrictions, .. } =
                    &agent.tool_surface
                else {
                    panic!("a wrapper surface");
                };
                assert_eq!(
                    restrictions[0],
                    format!("{class} sandbox (driver-selected)")
                );
            }
            n.select(json!({}));
        }
        let out = bound(&cli_sha);
        assert_eq!(out.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&out.stderr).contains("wrapper_contract_drift"));
        assert!(!n.record.exists(), "{family}: the driver ran after drift");
    }
}

/// A granted nested trial that would score, run with no retained leaf to
/// commit its usage companion in, is a harness error, never a scored row
/// without usage (the panel's tests 2 on a73ac17a).
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "bind ownership is read from /proc (Linux)"
)]
#[test]
fn a_scored_nested_trial_without_retention_is_a_harness_error() {
    let _scope = common::trial_scope();
    let n = Nested::new("claude", "claude-sonnet-5-5");
    private(
        &n.mode,
        json!({"mode": "complete", "finals": [smoke_final("claude")]})
            .to_string()
            .as_bytes(),
    );
    let trial_id = format!("{:032x}", 0x406u32);
    let coordinates = swarmail_evals::provenance::MatrixCoordinates {
        matrix_id: "f".repeat(32),
        master_seed: "e".repeat(64),
        task_index: 0,
        agent_index: 0,
        repetition_index: 0,
    };
    let seed = coordinates.cell_seed("signup", "claude-nested").unwrap();
    let fixture = swarmail_evals::provenance::FixtureInputs::resolve(Some(&seed), None).unwrap();
    let argv: Vec<std::ffi::OsString> = vec![
        common::exe().into(),
        "agent-claude".into(),
        "--model".into(),
        "claude-sonnet-5-5".into(),
        "--config".into(),
        n.c.manifest.clone().into(),
        "--via-brokkr".into(),
        n.driver.clone().into(),
    ];
    let context = paid_context(&n, &trial_id, "signup", &argv, &coordinates);
    let mut command = common::cli();
    command
        .args(["trial", "--task", "signup", "--agent", "claude-nested"])
        .args(["--agent-model", "claude-sonnet-5-5"])
        .args(["--wrapper-profile", "claude_2_1_329_via_brokkr"])
        .args(["--deadline-ms", "60000", "--seed", &fixture.seed])
        .args(["--fixture-time", &fixture.fixture_time])
        .args(["--worker-context", &context, "--"])
        .args(&argv);
    let grant = format!(
        "{}:1900000000",
        wrappers::contract_identity(&n.c.manifest, "claude", true).unwrap()
    );
    let out = granted(&command, 1, &grant).output().unwrap();
    let row = common::one_row(&out);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(n.record.exists(), "the granted session ran: {err}");
    assert_eq!(row.outcome, Outcome::HarnessError, "{err}");
    assert!(err.contains("usage_companion_unretained"), "{err}");
}
