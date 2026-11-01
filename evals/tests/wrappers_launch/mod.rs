//! Actual exec into a local stand-in, with independent Python TOML decoding.
use super::*;
use std::{
    fs::OpenOptions,
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::PathBuf,
};
fn private(path: &Path, bytes: &[u8]) {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    f.write_all(bytes).unwrap();
}
fn dir(path: &Path) {
    std::fs::DirBuilder::new().mode(0o700).create(path).unwrap();
}
use std::os::unix::fs::DirBuilderExt;
const CANARY: &str = "SYNTHETIC-CREDENTIAL-CANARY-DO-NOT-RETAIN";
struct Case {
    root: tempfile::TempDir,
    work: PathBuf,
    home: PathBuf,
    manifest: PathBuf,
    capture: PathBuf,
    values: BTreeMap<String, String>,
    runtime: BTreeMap<String, String>,
}
impl Case {
    fn new(family: &str) -> Self {
        let root = runner::private_tempdir("wrapper-proof-").unwrap();
        let base = root.path();
        let work = base.join("work \" ;$(literal) 🦀");
        dir(&work);
        let values = environment(&work, "http://127.0.0.1:12346/");
        let mut runtime = BTreeMap::from([
            ("PATH".into(), "/usr/bin:/bin".into()),
            ("LANG".into(), "C".into()),
            ("LC_ALL".into(), "C".into()),
            ("PWD".into(), work.to_str().unwrap().into()),
        ]);
        for key in [
            "HOME",
            "TMPDIR",
            "XDG_CONFIG_HOME",
            "XDG_CACHE_HOME",
            "XDG_DATA_HOME",
            "XDG_STATE_HOME",
        ] {
            let path = base.join(key);
            dir(&path);
            runtime.insert(key.into(), path.to_str().unwrap().into());
        }
        let home = PathBuf::from(&runtime["HOME"]);
        let auth = match family {
            "claude" => {
                json!({"claudeAiOauth":{"accessToken":CANARY,"expiresAt":1999999999999u64,"scopes":["user:inference"]}})
            }
            "codex" => {
                json!({"auth_mode":"chatgpt","OPENAI_API_KEY":null,"tokens":{"id_token":CANARY,"access_token":CANARY,"refresh_token":"SOURCE-REFRESH-NEVER-FORWARDED","account_id":"synthetic-account"},"last_refresh":"2026-10-26T00:00:00Z"})
            }
            _ => json!({"DASHSCOPE_API_KEY":CANARY,"SPARK_API_KEY":"SPARK-CANARY"}),
        };
        let credential = base.join("selected private.json");
        private(&credential, auth.to_string().as_bytes());
        let capture = base.join("capture.json");
        let script = base.join("standin");
        let body = format!(
            "#!/usr/bin/python3\nFAMILY={}\nCAPTURE={}\nCANARY={}\n{}",
            json!(family),
            json!(capture),
            json!(CANARY),
            include_str!("standin.py")
        );
        private(&script, body.as_bytes());
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut families = json!({});
        for f in ["claude", "codex", "dsh"] {
            families[f] = json!({"executable":script,"credential_file":if f == family {credential.clone()} else {base.join("unselected-must-not-be-opened")}});
        }
        let manifest = base.join("private selection.json");
        private(
            &manifest,
            json!({"version":1,"families":families})
                .to_string()
                .as_bytes(),
        );
        Self {
            root,
            work,
            home,
            manifest,
            capture,
            values,
            runtime,
        }
    }
    fn command(&self, family: &str, model: &str) -> std::process::Command {
        let mut c = std::process::Command::new(common::exe());
        c.env_clear()
            .envs(&self.runtime)
            .envs(&self.values)
            .env("ANTHROPIC_API_KEY", CANARY)
            .env("OPENAI_API_KEY", CANARY)
            .env("CLAUDE_CODE_OAUTH_TOKEN", CANARY)
            .env("HTTP_PROXY", CANARY)
            .env("BASH_ENV", CANARY)
            .env("DSH_HOME", "/ambient/refused")
            .current_dir(&self.work)
            .args([&format!("agent-{family}"), "--model", model, "--config"])
            .arg(&self.manifest);
        c
    }
}
#[test]
fn all_six_models_exec_literal_recipes_private_auth_and_no_raw_output() {
    for (family, models) in [
        ("claude", ["claude-opus-5-5", "claude-sonnet-5-5"]),
        ("codex", ["gpt-6.1-sol", "gpt-6-luna"]),
        ("dsh", ["dashscope/glm-5.3", "spark-glm/GLM-5.3-Flash-EXL3"]),
    ] {
        for model in models {
            let c = Case::new(family);
            let source = std::fs::read(c.root.path().join("selected private.json")).unwrap();
            let out = c.command(family, model).output().unwrap();
            assert!(out.status.success(), "{family} {model}: {out:?}");
            assert!(out.stdout.is_empty());
            assert!(out.stderr.is_empty());
            let row: Value = serde_json::from_slice(&std::fs::read(&c.capture).unwrap()).unwrap();
            assert_eq!(row["cwd"], json!(c.work));
            assert!(!row.to_string().contains(CANARY));
            let mut expected_env = c.runtime.clone();
            match family {
                "claude" => {
                    expected_env.insert("DISABLE_AUTOUPDATER".into(), "1".into());
                }
                "codex" => {
                    expected_env.insert(
                        "CODEX_HOME".into(),
                        c.home.join(".codex").to_str().unwrap().into(),
                    );
                }
                _ => {
                    expected_env.insert(
                        "DSH_HOME".into(),
                        c.home.join(".dsh").to_str().unwrap().into(),
                    );
                    expected_env.insert("DSH_TELEMETRY_DISABLED".into(), "1".into());
                }
            }
            assert_eq!(row["env"], json!(expected_env));
            let expected_helper_env: BTreeMap<_, _> = c
                .runtime
                .clone()
                .into_iter()
                .chain(c.values.clone())
                .collect();
            let fixture;
            let argv: Vec<String> = serde_json::from_value(row["argv"].clone()).unwrap();
            if family == "claude" {
                let allowed = std::iter::once("mcp__fixture__eval_http".to_string())
                    .chain(
                        wrappers::SWARMAIL_TOOLS
                            .iter()
                            .map(|n| format!("mcp__swarmail__{n}")),
                    )
                    .collect::<Vec<_>>()
                    .join(",");
                let mcp_path = c.home.join("mcp.json");
                let expected = vec![
                    "-p",
                    "--output-format",
                    "stream-json",
                    "--verbose",
                    "--model",
                    model,
                    "--no-session-persistence",
                    "--permission-mode",
                    "dontAsk",
                    "--permission-prompts",
                    "none",
                    "--tools",
                    "",
                    "--strict-mcp-config",
                    "--mcp-config",
                    mcp_path.to_str().unwrap(),
                    "--allowedTools",
                    &allowed,
                    "--restricted",
                    "--disable-slash-commands",
                ]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>();
                assert_eq!(argv, expected);
                assert_eq!(row["stdin"], c.values["EVAL_GOAL"]);
                let servers = &row["config"]["mcpServers"];
                assert_eq!(servers.as_object().unwrap().len(), 2);
                assert_eq!(
                    servers["swarmail"],
                    json!({"type":"http","url":c.values["EVAL_MCP_URL"],"headers":{"X-Eval-Identity":c.values["EVAL_IDENTITY"]}})
                );
                fixture = servers["fixture"].clone();
                assert_eq!(fixture["type"], "stdio");
            } else if family == "codex" {
                assert_eq!(
                    &argv[..12],
                    [
                        "exec",
                        "--json",
                        "--model",
                        model,
                        "-C",
                        c.work.to_str().unwrap(),
                        "--skip-git-repo-check",
                        "--ephemeral",
                        "--ignore-user-config",
                        "--ignore-rules",
                        "--sandbox",
                        "read-only"
                    ]
                );
                assert_eq!(argv.last().unwrap(), "-");
                assert_eq!(argv.len(), 37);
                assert!(argv[12..36].chunks(2).all(|a| a[0] == "-c"));
                assert_eq!(row["stdin"], c.values["EVAL_GOAL"]);
                let config = &row["config"];
                assert_eq!(config["web_search"], "disabled");
                assert_eq!(
                    config["projects"][c.work.to_str().unwrap()]["trust_level"],
                    "untrusted"
                );
                let servers = &config["mcp_servers"];
                assert_eq!(servers.as_object().unwrap().len(), 2);
                assert_eq!(
                    servers["swarmail"],
                    json!({"url":c.values["EVAL_MCP_URL"],"http_headers":{"X-Eval-Identity":c.values["EVAL_IDENTITY"]},"startup_timeout_sec":20,"tool_timeout_sec":60,"default_tools_approval_mode":"approve"})
                );
                fixture = servers["fixture"].clone();
                assert_eq!(fixture["cwd"], json!(c.work));
                assert_eq!(fixture["default_tools_approval_mode"], "approve");
            } else {
                assert_eq!(
                    argv,
                    vec![
                        "--profile".into(),
                        "headless".into(),
                        "--patch".into(),
                        c.home.join("seat.yml").to_str().unwrap().into(),
                        c.values["EVAL_GOAL"].clone()
                    ]
                );
                assert_eq!(row["stdin"], "");
                let rows = row["config"].as_array().unwrap();
                let (provider, model_id) = model.split_once('/').unwrap();
                assert_eq!(rows.len(), if provider == "spark-glm" { 5 } else { 4 });
                assert_eq!(
                    rows[1],
                    json!({"id":"agent-default-model","config":{"provider":provider,"model":model_id}})
                );
                let providers = &rows[0]["config"]["providers"];
                assert_eq!(providers.as_object().unwrap().len(), 1);
                let route = &providers[provider];
                assert_eq!(route["api"], "openai-completions");
                if provider == "dashscope" {
                    assert_eq!(
                        *route,
                        json!({"displayName":"Model Studio (Token Plan)","apiKeyEnv":"DASHSCOPE_API_KEY","api":"openai-completions","baseURL":"https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1","compat":{"thinkingFormat":"deepseek","supportsDeveloperRole":false},"models":[{"id":"glm-5.3"}]})
                    );
                } else {
                    assert_eq!(
                        *route,
                        json!({"displayName":"DGX Spark vLLM (GLM)","apiKeyEnv":"SPARK_API_KEY","api":"openai-completions","baseURL":"http://spark:8888/v1","models":[{"id":"GLM-5.3-Flash-EXL3","contextWindow":200000,"maxTokens":32768}]})
                    );
                    assert_eq!(
                        rows[4],
                        json!({"id":"compaction-basic","config":{"modelPolicies":[{"provider":provider,"model":model_id,"thresholdRatio":0.8,"retainRatio":0.16}]}})
                    );
                }
                assert_eq!(
                    rows[2],
                    json!({"id":"session-persistence-jsonl","config":{"root":c.home.join("sessions"),"compression":"none","packChunks":false}})
                );
                let servers = rows[3]["insert"].as_array().unwrap();
                assert_eq!(servers.len(), 2);
                assert_eq!(
                    servers[0],
                    json!({"id":"mcp-swarmail","name":"@deepseek-ai/dsh-mcp-client","config":{"serverName":"swarmail","transport":"streamable-http","url":c.values["EVAL_MCP_URL"],"headers":{"X-Eval-Identity":c.values["EVAL_IDENTITY"]},"failOnStartupError":true,"toolCallTimeoutMs":60000}})
                );
                assert_eq!(servers[1]["name"], "@deepseek-ai/dsh-mcp-client");
                fixture = servers[1]["config"].clone();
                assert_eq!(fixture["cwd"], json!(c.work));
                assert_eq!(fixture["transport"], "stdio");
                assert_eq!(fixture["serverName"], "fixture");
                assert_eq!(fixture["failOnStartupError"], true);
            }
            assert_eq!(fixture["command"], common::exe());
            assert_eq!(fixture["args"], json!(["fixture-http-isolated"]));
            assert_eq!(fixture["env"], json!(expected_helper_env));
            assert_eq!(
                source,
                std::fs::read(c.root.path().join("selected private.json")).unwrap()
            );
            assert!(!c.work.join("NEVER").exists());
        }
    }
}
#[test]
fn entrypoint_input_refusals_precede_any_cli_launch() {
    for (family, model) in [
        ("claude", "claude-opus-5-5"),
        ("codex", "gpt-6.1-sol"),
        ("dsh", "dashscope/glm-5.3"),
    ] {
        for key in INPUTS {
            let c = Case::new(family);
            let out = c.command(family, model).env_remove(key).output().unwrap();
            assert_eq!(out.status.code(), Some(2));
            assert!(out.stdout.is_empty());
            assert!(!c.capture.exists());
            assert!(!String::from_utf8_lossy(&out.stderr).contains(CANARY));
        }
        let c = Case::new(family);
        let out = c
            .command(family, model)
            .env("EVAL_GOAL", "prompt mismatch")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(!c.capture.exists());
    }
}

#[test]
fn non_serializable_identity_and_runtime_paths_refuse_before_launch() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let c = Case::new("claude");
    let out = c
        .command("claude", "claude-opus-5-5")
        .env("EVAL_IDENTITY", "non-ascii-🦀")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(!c.capture.exists());
    let native = c
        .root
        .path()
        .join(OsString::from_vec(b"native-home-\xff".to_vec()));
    dir(&native);
    let out = c
        .command("claude", "claude-opus-5-5")
        .env("HOME", native)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(!c.capture.exists());
    assert!(!String::from_utf8_lossy(&out.stderr).contains(CANARY));
}

#[test]
fn unsupported_goals_config_and_executables_refuse_without_launch() {
    for goal in ["-option".into(), "a".repeat(65536)] {
        let mut c = Case::new("dsh");
        std::fs::write(&c.values["EVAL_PROMPT_FILE"], &goal).unwrap();
        c.values.insert("EVAL_GOAL".into(), goal);
        assert_eq!(
            c.command("dsh", "dashscope/glm-5.3")
                .output()
                .unwrap()
                .status
                .code(),
            Some(2)
        );
        assert!(!c.capture.exists());
    }
    let mut c = Case::new("dsh");
    let goal = "a".repeat(65535);
    std::fs::write(&c.values["EVAL_PROMPT_FILE"], &goal).unwrap();
    c.values.insert("EVAL_GOAL".into(), goal);
    assert!(
        c.command("dsh", "dashscope/glm-5.3")
            .output()
            .unwrap()
            .status
            .success()
    );
    for family in ["claude", "codex", "dsh"] {
        let model = match family {
            "claude" => "claude-opus-5-5",
            "codex" => "gpt-6.1-sol",
            _ => "dashscope/glm-5.3",
        };
        for defect in [
            "mode",
            "symlink",
            "malformed",
            "unknown",
            "duplicate",
            "oversized",
            "exec",
            "empty-home",
        ] {
            let c = Case::new(family);
            let auth = c.root.path().join("selected private.json");
            match defect {
                "mode" => {
                    std::fs::set_permissions(&auth, std::fs::Permissions::from_mode(0o644)).unwrap()
                }
                "symlink" => {
                    std::fs::remove_file(&auth).unwrap();
                    std::os::unix::fs::symlink("absent", &auth).unwrap();
                }
                "malformed" => std::fs::write(&auth, b"not-json").unwrap(),
                "unknown" => {
                    let mut v: Value =
                        serde_json::from_slice(&std::fs::read(&auth).unwrap()).unwrap();
                    v["unexpected"] = json!(CANARY);
                    std::fs::write(&auth, v.to_string()).unwrap();
                }
                "duplicate" => {
                    let mut b = std::fs::read(&c.manifest).unwrap();
                    b.splice(1..1, b"\"version\":1,".iter().copied());
                    std::fs::write(&c.manifest, b).unwrap();
                }
                "oversized" => std::fs::write(&auth, vec![b' '; 65537]).unwrap(),
                "exec" => {
                    std::fs::write(c.root.path().join("standin"), b"#!/absent/interpreter\n")
                        .unwrap();
                }
                _ => private(&c.home.join(".env"), b"refuse"),
            }
            let out = c.command(family, model).output().unwrap();
            assert_eq!(out.status.code(), Some(2), "{family} {defect}");
            assert!(out.stdout.is_empty());
            assert!(!String::from_utf8_lossy(&out.stderr).contains(CANARY));
            assert!(!c.capture.exists());
        }
    }
}

// Linux only: the effective helper environment is read from /proc/<pid>/environ.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn isolated_helper_exec_has_a_credential_free_effective_environment() {
    let c = Case::new("claude");
    let mut command = tokio::process::Command::new(common::exe());
    command
        .arg("fixture-http-isolated")
        .current_dir(&c.work)
        .env_clear()
        .envs(&c.runtime)
        .envs(&c.values)
        .env("CLAUDE_CODE_OAUTH_TOKEN", CANARY)
        .env("DASHSCOPE_API_KEY", CANARY)
        .env("CODEX_HOME", "/ambient")
        .env("ARBITRARY_SECRET", CANARY)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-03-26\"}}\n").await.unwrap();
    let mut stdout = tokio::io::BufReader::new(child.stdout.take().unwrap()).lines();
    let reply = tokio::time::timeout(std::time::Duration::from_secs(10), stdout.next_line())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(serde_json::from_str::<Value>(&reply).unwrap()["id"], 1);
    // A protocol reply acknowledges the second exec, so this is the effective
    // helper map, not merely a proposed per-server environment.
    let bytes = std::fs::read(format!("/proc/{}/environ", child.id().unwrap())).unwrap();
    let env: BTreeMap<String, String> = bytes
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| {
            let s = std::str::from_utf8(s).unwrap();
            let (k, v) = s.split_once('=').unwrap();
            (k.into(), v.into())
        })
        .collect();
    let expected: BTreeMap<_, _> = c.runtime.into_iter().chain(c.values).collect();
    assert_eq!(env, expected);
    drop(stdin);
    assert!(child.wait().await.unwrap().success());
}

// Linux only: the descendants guard and the test below prove ownership by
// PID/start-time identities read from /proc.
#[cfg(target_os = "linux")]
struct Descendants(Vec<common::ProcessIdentity>);
#[cfg(target_os = "linux")]
impl Drop for Descendants {
    fn drop(&mut self) {
        for p in &self.0 {
            if common::identity_alive(p) {
                common::send_signal(p.pid, libc::SIGKILL);
            }
        }
    }
}

#[cfg(target_os = "linux")]
#[test]
fn wrapper_exec_descendants_and_private_state_remain_owned_through_every_ending() {
    use std::{
        io::{BufRead, BufReader},
        os::unix::net::UnixListener,
    };
    let _scope = common::trial_scope();
    for family in ["claude", "codex", "dsh"] {
        let (model, profile) = match family {
            "claude" => ("claude-opus-5-5", "claude_2_1_310"),
            "codex" => ("gpt-6.1-sol", "codex_0_164_0"),
            _ => ("dashscope/glm-5.3", "dsh_0_1_5_rc_1"),
        };
        for ending in ["normal", "nonzero", "deadline", "term", "int"] {
            let c = Case::new(family);
            let socket = c.root.path().join("ready.sock");
            let listener = UnixListener::bind(&socket).unwrap();
            listener.set_nonblocking(true).unwrap();
            let body = format!(
                "#!/usr/bin/python3\nSOCKET={}\nENDING={}\nCANARY={}\n{}",
                json!(socket),
                json!(ending),
                json!(CANARY),
                include_str!("descendants.py")
            );
            std::fs::write(c.root.path().join("standin"), body).unwrap();
            let evidence = c.root.path().join("retained");
            let mut command = common::cli();
            command
                .args([
                    "trial",
                    "--task",
                    "signup",
                    "--agent",
                    "selected",
                    "--agent-model",
                    model,
                    "--wrapper-profile",
                    profile,
                    "--deadline-ms",
                    if ending == "deadline" {
                        "5000"
                    } else {
                        "60000"
                    },
                    "--artifacts-dir",
                ])
                .arg(&evidence)
                .args([
                    "--",
                    common::exe(),
                    &format!("agent-{family}"),
                    "--model",
                    model,
                    "--config",
                ])
                .arg(&c.manifest)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            let mut trial = common::ChildGuard {
                child: Some(command.spawn().unwrap()),
            };
            let mut stream = None;
            common::eventually("stand-in readiness connection", || {
                match listener.accept() {
                    Ok((s, _)) => {
                        stream = Some(s);
                        true
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => false,
                    Err(e) => panic!("{e}"),
                }
            });
            let mut stream = stream.unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .unwrap();
            let mut line = String::new();
            BufReader::new(stream.try_clone().unwrap())
                .read_line(&mut line)
                .unwrap();
            let ready: Value = serde_json::from_str(&line).unwrap();
            let ids = ready["pids"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| common::process_identity(p.as_i64().unwrap() as i32).unwrap())
                .collect::<Vec<_>>();
            let _all_trial_children = Descendants(common::process_descendants(trial.pid()));
            let owned = Descendants(ids);
            assert_eq!(owned.0.len(), 3);
            assert!(owned.0.iter().all(common::identity_alive));
            let leader = owned.0[0].pid;
            assert!(owned.0.iter().all(|p| p.pgrp == leader));
            let home = PathBuf::from(ready["home"].as_str().unwrap());
            assert!(home.exists());
            if family == "claude" {
                assert!(home.join(".claude/.credentials.json").is_file());
            }
            if family == "codex" {
                assert!(home.join(".codex/auth.json").is_file());
            }
            match ending {
                "normal" | "nonzero" => stream.write_all(b"release\n").unwrap(),
                "term" => common::send_signal(trial.pid(), libc::SIGTERM),
                "int" => common::send_signal(trial.pid(), libc::SIGINT),
                _ => {}
            }
            // Use completion as the fence: eventual post-return death would
            // conceal a missing cleanup await. The guard cleans a failed proof.
            common::eventually("trial cleanup completion", || {
                trial.child.as_mut().unwrap().try_wait().unwrap().is_some()
            });
            assert!(
                owned.0.iter().all(|p| !common::identity_alive(p)),
                "descendants alive at publication"
            );
            assert!(
                common::process_identity(leader).is_none(),
                "direct child must be reaped"
            );
            assert!(
                !home.exists(),
                "private state remains until cleanup, then is disposed"
            );
            let out = trial.wait_with_output();
            let row = common::one_row(&out);
            let surface = serde_json::to_value(&row.provenance.agent.tool_surface).unwrap();
            assert_eq!(surface["profile"], profile);
            assert_eq!(surface["basis"], "declared_configuration");
            if ending == "normal" {
                assert_eq!(row.agent_exit_status, Some(0));
            }
            if ending == "nonzero" {
                assert_eq!(row.agent_exit_status, Some(7));
            }
            let retained = swarmail_evals::artifacts::trial(&evidence.join(&row.trial_id)).unwrap();
            assert_eq!(retained.row, row);
            let report = swarmail_evals::report::report(std::slice::from_ref(&row));
            assert_eq!(report["groups"][0]["tool_surfaces"][0], surface);
            for bytes in [&out.stdout, &out.stderr] {
                assert!(!String::from_utf8_lossy(bytes).contains(CANARY));
            }
            for name in ["result.json", "metadata.json", "diagnostics.log"] {
                let b = std::fs::read(evidence.join(&row.trial_id).join(name)).unwrap();
                assert!(!String::from_utf8_lossy(&b).contains(CANARY));
                assert!(!String::from_utf8_lossy(&b).contains(c.manifest.to_str().unwrap()));
            }
            let state_path = evidence.join(&row.trial_id).join("state.json");
            if retained.state.is_some() {
                let bytes = std::fs::read(state_path).unwrap();
                assert!(!String::from_utf8_lossy(&bytes).contains(CANARY));
            } else {
                assert!(!state_path.exists());
            }
            assert!(!report.to_string().contains(CANARY));
        }
    }
}

#[test]
fn declared_surfaces_are_required_canonical_model_bound_and_retained() {
    use swarmail_evals::{artifacts, provenance::*, report, results};
    let mut rows = Vec::new();
    for (profile, model) in [
        (WrapperProfile::Claude2_1_310, "claude-opus-5-5"),
        (WrapperProfile::Codex0_164_0, "gpt-6.1-sol"),
        (WrapperProfile::Dsh0_1_5Rc1, "dashscope/glm-5.3"),
    ] {
        let req = runner::TrialRequest {
            task: Some("signup".into()),
            agent: Some("same-label".into()),
            argv: vec!["/bin/true".into()],
            agent_model: Some(model.into()),
            wrapper_profile: Some(profile),
            ..Default::default()
        };
        let row = runner::refused_row(&req, &runner::new_trial_id());
        let bytes = serde_json::to_vec(&row).unwrap();
        assert_eq!(results::read(&bytes).unwrap(), row);
        let v = serde_json::to_value(&row).unwrap();
        for defect in [
            "missing",
            "unknown",
            "revision",
            "basis",
            "restriction",
            "residual",
            "duplicate-item",
            "wrong-model",
            "wrong-profile",
        ] {
            let mut bad = v.clone();
            let s = &mut bad["provenance"]["agent"]["tool_surface"];
            match defect {
                "missing" => {
                    bad["provenance"]["agent"]
                        .as_object_mut()
                        .unwrap()
                        .remove("tool_surface");
                }
                "unknown" => s["unknown"] = json!(true),
                "revision" => s["revision"] = json!(2),
                "basis" => s["basis"] = json!("observed"),
                "restriction" => s["restrictions"][0] = json!("made up"),
                "residual" => s["residuals"] = json!([]),
                "duplicate-item" => {
                    let a = s["restrictions"].as_array_mut().unwrap();
                    a.push(a[0].clone());
                }
                "wrong-model" => {
                    bad["provenance"]["agent"]["model"]["value"] = json!("unsupported")
                }
                _ => s["profile"] = json!("unknown"),
            }
            assert!(
                results::read(&serde_json::to_vec(&bad).unwrap()).is_err(),
                "{defect}"
            );
        }
        let mut duplicate = String::from_utf8(bytes.clone()).unwrap();
        duplicate = duplicate.replacen("\"revision\":1", "\"revision\":1,\"revision\":1", 1);
        assert!(results::read(duplicate.as_bytes()).is_err());
        let root = runner::private_tempdir("surface-proof-").unwrap();
        artifacts::retain_trial(root.path(), &row, None).unwrap();
        assert_eq!(
            artifacts::trial(&root.path().join(&row.trial_id))
                .unwrap()
                .row,
            row
        );
        rows.push(row);
    }
    let report = report::report(&rows);
    assert_eq!(report["groups"].as_array().unwrap().len(), 1);
    assert_eq!(
        report["groups"][0]["tool_surfaces"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(report["groups"][0]["attempts"], 3);
    assert_eq!(report["groups"][0]["harness_errors"], 3);
    assert_eq!(report["groups"][0]["success_rate"], Value::Null);
    assert!(
        report["tool_surface_note"]
            .as_str()
            .unwrap()
            .contains("asymmetric")
    );
    for (req, expected) in [
        (
            runner::TrialRequest {
                task: Some("signup".into()),
                agent: Some("oracle".into()),
                ..Default::default()
            },
            ToolSurface::BuiltinNotApplicable,
        ),
        (
            runner::TrialRequest {
                task: Some("signup".into()),
                agent: Some("claude-opus".into()),
                argv: vec!["/bin/true".into()],
                ..Default::default()
            },
            ToolSurface::ExternalUnknown,
        ),
        (runner::TrialRequest::default(), ToolSurface::InvalidRequest),
    ] {
        let row = runner::refused_row(&req, &runner::new_trial_id());
        assert_eq!(row.provenance.agent.tool_surface, expected);
        assert!(results::read(&serde_json::to_vec(&row).unwrap()).is_ok());
    }
    for declaration in [
        r#"[{"label":"oracle","argv":[],"wrapper_profile":"claude_2_1_310"}]"#,
        r#"[{"label":"external","argv":["/bin/true"],"model":"gpt-6.1-sol","wrapper_profile":"claude_2_1_310"}]"#,
    ] {
        assert!(report::parse_agents(declaration).is_err());
    }
}

#[test]
fn claude_explicit_allowance_matches_real_swarmail_tools_list() {
    let _scope = common::trial_scope();
    let c = Case::new("claude");
    let script = c.root.path().join("standin");
    let mut body = std::fs::read_to_string(&script).unwrap();
    body.push_str(r#"
import urllib.request
url = config['mcpServers']['swarmail']['url']
def rpc(method, params):
    request=urllib.request.Request(url,data=json.dumps({'jsonrpc':'2.0','id':1,'method':method,'params':params}).encode(),headers={'Content-Type':'application/json'})
    with urllib.request.build_opener(urllib.request.ProxyHandler({})).open(request) as response: return json.load(response)['result']
rpc('initialize',{'protocolVersion':'2025-03-26','capabilities':{},'clientInfo':{'name':'offline-parity','version':'1'}})
names=sorted(t['name'] for t in rpc('tools/list',{})['tools'])
result['server_tools']=names
pathlib.Path(CAPTURE).write_text(json.dumps(result))
"#);
    std::fs::write(script, body).unwrap();
    let out = common::cli()
        .args([
            "trial",
            "--task",
            "signup",
            "--agent",
            "parity",
            "--agent-model",
            "claude-opus-5-5",
            "--wrapper-profile",
            "claude_2_1_310",
            "--",
            common::exe(),
            "agent-claude",
            "--model",
            "claude-opus-5-5",
            "--config",
        ])
        .arg(&c.manifest)
        .output()
        .unwrap();
    let row = common::one_row(&out);
    assert_eq!(row.agent_exit_status, Some(0));
    let captured: Value = serde_json::from_slice(&std::fs::read(c.capture).unwrap()).unwrap();
    let mut expected = wrappers::SWARMAIL_TOOLS.to_vec();
    expected.sort();
    assert_eq!(captured["server_tools"], json!(expected));
}

#[test]
fn fresh_controls_authorize_the_six_selected_standin_cells_and_a_failure_report() {
    use swarmail_evals::{artifacts, baseline, provenance::*, report, tasks::Outcome};
    let _scope = common::trial_scope();
    let owner = runner::private_tempdir("baseline-launch-proof-").unwrap();
    let claude = Case::new("claude");
    let codex = Case::new("codex");
    let dsh = Case::new("dsh");
    let mut families = json!({});
    for (family, case) in [("claude", &claude), ("codex", &codex), ("dsh", &dsh)] {
        let manifest: Value =
            serde_json::from_slice(&std::fs::read(&case.manifest).unwrap()).unwrap();
        families[family] = manifest["families"][family].clone();
    }
    let manifest = owner.path().join("selected.json");
    private(
        &manifest,
        json!({"version":1,"families":families})
            .to_string()
            .as_bytes(),
    );
    let mut plan = baseline::Plan::new(owner.path());
    // Exercise the same owners on one catalogue task, not a different paid
    // plan in the product. The actual dry-run independently pins 32/480.
    plan.tasks = vec!["signup".into()];
    plan.control_rows = 2;
    plan.paid_repetitions = 1;
    plan.paid_rows = 6;
    let agents = owner.path().join("controls-agents.json");
    private(&agents, &serde_json::to_vec(&plan.control_agents).unwrap());
    let retained = owner.path().join("controls");
    let out = common::cli()
        .args([
            "matrix",
            "--tasks",
            "signup",
            "--trials",
            "1",
            "--deadline-ms",
            "600000",
            "--agents",
        ])
        .arg(&agents)
        .arg("--artifacts-dir")
        .arg(&retained)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let controls = baseline::rows(&out.stdout).unwrap();
    let matrix_root = std::fs::read_dir(&retained)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let matrix: artifacts::MatrixPlan = decode(
        &artifacts::read_file(&matrix_root, "matrix-plan.json", artifacts::PLAN_LIMIT).unwrap(),
    )
    .unwrap();
    let h = controls[0].provenance.harness.clone();
    let p = &controls[0].provenance.target;
    let target = ExecutableIdentity {
        executable_sha256: p.executable_sha256.clone(),
        build_commit: p.build_commit.clone(),
        source_dirty: p.source_dirty.clone(),
    };
    baseline::validate_controls(&controls, &matrix, &plan, 1, &h, &target).unwrap();
    assert_eq!(artifacts::matrix(&matrix_root).unwrap().len(), 2);
    assert!(!claude.capture.exists() && !codex.capture.exists() && !dsh.capture.exists());
    let effective = plan
        .effective_agents(Path::new(common::exe()), &manifest)
        .unwrap();
    let selected = owner.path().join("selected-agents.json");
    private(&selected, &serde_json::to_vec(&effective).unwrap());
    let retained = owner.path().join("paid");
    let out = common::cli()
        .args([
            "matrix",
            "--tasks",
            "signup",
            "--trials",
            "1",
            "--deadline-ms",
            "600000",
            "--agents",
        ])
        .arg(&selected)
        .arg("--artifacts-dir")
        .arg(&retained)
        .output()
        .unwrap();
    let paid = baseline::rows(&out.stdout).unwrap();
    assert_eq!(paid.len(), 6);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        paid.iter()
            .all(|r| r.outcome == Outcome::Fail && r.agent_exit_status == Some(0))
    );
    let root = std::fs::read_dir(retained)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let matrix: artifacts::MatrixPlan =
        decode(&artifacts::read_file(&root, "matrix-plan.json", artifacts::PLAN_LIMIT).unwrap())
            .unwrap();
    baseline::validate_accounting(&paid, &matrix, &effective, &plan.tasks, 1, &h, &target).unwrap();
    let replay = artifacts::matrix(&root).unwrap();
    assert!(
        replay
            .iter()
            .zip(&paid)
            .all(|(a, b)| a.original_result == *b)
    );
    let report = report::report(&paid);
    assert_eq!(report["groups"].as_array().unwrap().len(), 6);
    assert!(
        report["tool_surface_note"]
            .as_str()
            .unwrap()
            .contains("asymmetric")
    );
    assert!(claude.capture.exists() && codex.capture.exists() && dsh.capture.exists());
    assert!(!String::from_utf8_lossy(&out.stdout).contains(CANARY));
    assert!(!report.to_string().contains(CANARY));
}

#[test]
fn closed_private_records_refuse_semantic_credential_and_selection_errors_before_staging() {
    for (family, model, defects) in [
        (
            "claude",
            "claude-opus-5-5",
            vec![
                "empty-token",
                "nul-token",
                "scopes",
                "expiry",
                "negative-expiry",
                "extra-refresh",
            ],
        ),
        (
            "codex",
            "gpt-6.1-sol",
            vec![
                "mode",
                "api-key",
                "timestamp",
                "id-token",
                "access-token",
                "refresh-token",
                "account-id",
                "extra-token",
            ],
        ),
        (
            "dsh",
            "dashscope/glm-5.3",
            vec!["empty-key", "empty-other-key", "nul-key", "wrong-type"],
        ),
    ] {
        for defect in defects {
            let c = Case::new(family);
            let path = c.root.path().join("selected private.json");
            let mut record: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            match (family, defect) {
                ("claude", "empty-token") => record["claudeAiOauth"]["accessToken"] = json!(""),
                ("claude", "nul-token") => {
                    record["claudeAiOauth"]["accessToken"] = json!("invalid\0token")
                }
                ("claude", "scopes") => {
                    record["claudeAiOauth"]["scopes"] = json!(["user:inference", "user:profile"])
                }
                ("claude", "expiry") => {
                    record["claudeAiOauth"]["expiresAt"] = json!(9007199254740992u64)
                }
                ("claude", "negative-expiry") => record["claudeAiOauth"]["expiresAt"] = json!(-1),
                ("claude", _) => record["claudeAiOauth"]["refreshToken"] = json!(CANARY),
                ("codex", "mode") => record["auth_mode"] = json!("apikey"),
                ("codex", "api-key") => record["OPENAI_API_KEY"] = json!(CANARY),
                ("codex", "timestamp") => record["last_refresh"] = json!("not-a-time"),
                ("codex", "id-token") => record["tokens"]["id_token"] = json!(""),
                ("codex", "access-token") => record["tokens"]["access_token"] = json!(""),
                ("codex", "refresh-token") => record["tokens"]["refresh_token"] = json!(""),
                ("codex", "account-id") => record["tokens"]["account_id"] = json!(""),
                ("codex", _) => record["tokens"]["unknown"] = json!(CANARY),
                (_, "empty-key") => record["DASHSCOPE_API_KEY"] = json!(""),
                (_, "empty-other-key") => record["SPARK_API_KEY"] = json!(""),
                (_, "nul-key") => record["DASHSCOPE_API_KEY"] = json!("invalid\0key"),
                _ => record["DASHSCOPE_API_KEY"] = json!(false),
            }
            let before = record.to_string();
            std::fs::write(&path, &before).unwrap();
            let out = c.command(family, model).output().unwrap();
            assert_eq!(out.status.code(), Some(2), "{family} {defect}");
            assert!(out.stdout.is_empty());
            assert!(!c.capture.exists());
            assert!(std::fs::read_dir(&c.home).unwrap().next().is_none());
            assert_eq!(std::fs::read(&path).unwrap(), before.as_bytes());
            assert!(!String::from_utf8_lossy(&out.stderr).contains(CANARY));
        }
    }
    for defect in [
        "version",
        "family",
        "extra",
        "relative-executable",
        "relative-credential",
    ] {
        let c = Case::new("claude");
        let mut record: Value =
            serde_json::from_slice(&std::fs::read(&c.manifest).unwrap()).unwrap();
        match defect {
            "version" => record["version"] = json!(2),
            "family" => {
                record["families"].as_object_mut().unwrap().remove("dsh");
            }
            "extra" => record["families"]["claude"]["env"] = json!({"secret":CANARY}),
            "relative-executable" => record["families"]["claude"]["executable"] = json!("relative"),
            _ => record["families"]["claude"]["credential_file"] = json!("relative"),
        }
        std::fs::write(&c.manifest, record.to_string()).unwrap();
        let out = c.command("claude", "claude-opus-5-5").output().unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(!c.capture.exists());
        assert!(std::fs::read_dir(c.home).unwrap().next().is_none());
        assert!(!String::from_utf8_lossy(&out.stderr).contains(CANARY));
    }
}
