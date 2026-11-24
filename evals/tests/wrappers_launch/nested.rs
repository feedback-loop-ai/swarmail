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
                json!({"DASHSCOPE_API_KEY":{"key":CANARY,"valid_through":future},"SPARK_API_KEY":{"key":"SPARK-CANARY","valid_through":future}})
                    .to_string()
                    .as_bytes(),
            );
        }
        if self.family == "codex" {
            let token = common::paid::jwt(future);
            replace_private(
                &credential,
                json!({"auth_mode":"chatgpt","OPENAI_API_KEY":null,"tokens":{"id_token":token,"access_token":token,"refresh_token":"SOURCE-REFRESH-NEVER-FORWARDED","account_id":"synthetic-account"},"last_refresh":"2026-10-26T00:00:00Z"})
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
            .arg(self.binding());
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
    let (exit, err, report) = n.launch(json!({"mode": "deaf", "finals": finals}));
    assert_eq!(exit, Some(0), "{err}");
    assert!(began.elapsed() < std::time::Duration::from_secs(60));
    assert!(began.elapsed() >= wrappers::DRIVER_EXIT_GRACE);
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
        let out = cmd.arg("--paid-launch").arg(n.binding()).output().unwrap();
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
    let n = Nested::new("claude", "claude-sonnet-5-5");
    let standin = swarmail_evals::identity::file_hash(&n.driver).unwrap();
    for (admit, code) in [
        (false, "driver_release_unrecognized"),
        (true, "nested_paid_unbound: recipe_bind_"),
    ] {
        n.clear_home();
        let _ = std::fs::remove_file(&n.mode);
        private(&n.mode, json!({"mode": "complete"}).to_string().as_bytes());
        let mut cmd = n.c.command("claude", "claude-sonnet-5-5");
        cmd.arg("--via-brokkr").arg(&n.driver);
        if admit {
            cmd.env(wrappers::STANDIN_DRIVER_ENV, &standin);
        }
        let out = cmd.output().unwrap();
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{code}: {err}");
        assert!(err.contains(code), "{code}: {err}");
        assert!(!n.record.exists(), "{code}: the driver ran");
        assert!(std::fs::read_dir(&n.c.home).unwrap().next().is_none());
    }
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
    let out = command.output().unwrap();
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
        // The coordinator admits the release executable alone; this proof
        // pins the stand-in, admitted by the test-equipment digest. Only
        // this test reads the variable in-process; every launch below runs
        // with a cleared environment.
        let standin = swarmail_evals::identity::file_hash(&n.driver).unwrap();
        unsafe { std::env::remove_var(wrappers::STANDIN_DRIVER_ENV) };
        let refused = wrappers::paid_contracts(&n.c.manifest, &[family], true).unwrap_err();
        assert!(refused.contains("driver_release_unrecognized"), "{refused}");
        unsafe { std::env::set_var(wrappers::STANDIN_DRIVER_ENV, &standin) };
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
            n.c.command(family, model)
                .arg("--via-brokkr")
                .arg(&n.driver)
                .arg("--paid-launch")
                .arg(format!("{sha}:{future}"))
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
        let out = bound(&cli_sha);
        assert_eq!(out.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&out.stderr).contains("wrapper_contract_drift"));
        assert!(!n.record.exists(), "{family}: the driver ran after drift");
    }
}
