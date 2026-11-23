//! Route (b) at the wrapper boundary (decision 0119's 2026-11-23 addendum,
//! proposed 0138): the real wrapper speaks forge-driver/v1 to a stand-in
//! driver pinned like the release one, and reports its launch and the
//! session's usage on descriptor 3.
use super::*;
use swarmail_evals::runner::{LaunchReport, launch_report_of};
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
                nested["codex_sandbox"] = json!("danger-full-access");
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
            .arg("--launch-report");
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
            "danger-full-access",
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
        let out = cmd.arg("--via-brokkr").arg(via).output().unwrap();
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
    // A driver whose banner is not the release's.
    let swapped = json!({"driver": other, "driver_sha256": other_sha});
    refuse(swapped, &other, "driver_version_mismatch");
}
