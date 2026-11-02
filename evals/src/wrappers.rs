//! Fixed CLI recipes from the versioned evidence; no provider discovery or fallback.
use crate::tasks;
use reqwest::{Url, header::HeaderValue};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub const INPUTS: [&str; 8] = [
    "EVAL_TASK_ID",
    "EVAL_APP_URL",
    "EVAL_MCP_URL",
    "EVAL_IDENTITY",
    "EVAL_INBOX",
    "EVAL_GOAL",
    "EVAL_PROMPT_FILE",
    "EVAL_DEADLINE_MS",
];
#[derive(Clone)]
pub struct Inputs {
    pub task: String,
    pub app: Url,
    pub mcp: Url,
    pub identity: HeaderValue,
    pub inbox: String,
    pub goal: String,
    pub prompt_file: PathBuf,
    pub deadline_ms: u64,
}

pub fn endpoint(value: &str, path: &str) -> Result<Url, String> {
    let url = Url::parse(value).map_err(|_| "eval_endpoint_invalid")?;
    // Numeric loopback only: no DNS, credentials, queries or fragment. The
    // harness announces IPv4 loopback endpoints; IPv6 loopback is also safe.
    let loopback = url
        .host_str()
        .map(|h| h.trim_matches(['[', ']']))
        .and_then(|h| h.parse::<std::net::IpAddr>().ok())
        .is_some_and(|ip| ip.is_loopback());
    if !loopback
        || url.scheme() != "http"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != path
        || url.port_or_known_default() == Some(0)
    {
        return Err("eval_endpoint_invalid".into());
    }
    Ok(url)
}
impl Inputs {
    pub fn read(values: &BTreeMap<String, String>) -> Result<Self, String> {
        let get = |k: &str| {
            values
                .get(k)
                .filter(|v| !v.is_empty())
                .cloned()
                .ok_or_else(|| "eval_input_missing".to_string())
        };
        let task = get(INPUTS[0])?;
        if tasks::find(&task).is_none() {
            return Err("eval_task_invalid".into());
        }
        let app = endpoint(&get(INPUTS[1])?, "/")?;
        let mcp = endpoint(&get(INPUTS[2])?, "/mcp")?;
        let identity =
            HeaderValue::from_str(&get(INPUTS[3])?).map_err(|_| "eval_identity_invalid")?;
        identity.to_str().map_err(|_| "eval_identity_invalid")?;
        let inbox = get(INPUTS[4])?;
        if inbox.chars().any(char::is_control) {
            return Err("eval_inbox_invalid".into());
        }
        let goal = get(INPUTS[5])?;
        let prompt_file = PathBuf::from(get(INPUTS[6])?);
        let deadline = get(INPUTS[7])?;
        if !deadline.bytes().all(|b| b.is_ascii_digit()) {
            return Err("eval_deadline_invalid".into());
        }
        let deadline_ms: u64 = deadline.parse().map_err(|_| "eval_deadline_invalid")?;
        if deadline_ms == 0 || deadline_ms > tasks::MAX_DEADLINE_MS {
            return Err("eval_deadline_invalid".into());
        }
        if !prompt_file.is_absolute() || read_prompt(&prompt_file)? != goal.as_bytes() {
            return Err("eval_prompt_mismatch".into());
        }
        Ok(Self {
            task,
            app,
            mcp,
            identity,
            inbox,
            goal,
            prompt_file,
            deadline_ms,
        })
    }
    pub fn environment() -> Result<Self, String> {
        let values = INPUTS
            .into_iter()
            .map(|key| {
                std::env::var(key)
                    .map(|v| (key.to_string(), v))
                    .map_err(|_| "eval_input_missing".to_string())
            })
            .collect::<Result<_, _>>()?;
        Self::read(&values)
    }
}

pub fn validate_model(harness: &str, model: &str) -> Result<(), String> {
    let supported = match harness {
        "claude" => ["claude-opus-5-5", "claude-sonnet-5-5"].contains(&model),
        "codex" => ["gpt-6.1-sol", "gpt-6-luna"].contains(&model),
        "dsh" => ["dashscope/glm-5.3", "spark-glm/GLM-5.3-Flash-EXL3"].contains(&model),
        _ => false,
    };
    if supported {
        Ok(())
    } else {
        Err("wrapper_model_unsupported".into())
    }
}

fn read_prompt(path: &Path) -> Result<Vec<u8>, String> {
    use std::{io::Read, os::unix::fs::OpenOptionsExt};
    let f = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "eval_prompt_unavailable")?;
    let meta = f.metadata().map_err(|_| "eval_prompt_unavailable")?;
    if !meta.is_file() || meta.len() > 1024 * 1024 {
        return Err("eval_prompt_invalid".into());
    }
    let mut bytes = Vec::new();
    f.take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "eval_prompt_unavailable")?;
    if bytes.len() > 1024 * 1024 {
        return Err("eval_prompt_invalid".into());
    }
    Ok(bytes)
}

use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    ffi::{CString, OsString},
    fs::File,
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{
            ffi::OsStrExt,
            fs::{MetadataExt, OpenOptionsExt},
            process::CommandExt,
        },
    },
    process::{Command, Stdio},
};
const PRIVATE_LIMIT: u64 = 65536;
const AMBIENT: &[&str] = &[
    ".claude",
    ".codex",
    ".agents",
    ".dsh",
    ".mcp.json",
    "AGENTS.md",
    "AGENTS.local.md",
    "CLAUDE.md",
    "CLAUDE.local.md",
    ".env",
];
pub const SWARMAIL_TOOLS: &[&str] = &[
    "swarmail_search_emails",
    "swarmail_get_email",
    "swarmail_get_latest_email",
    "swarmail_get_deliverability_report",
    "swarmail_delete_email",
    "swarmail_clear_inbox",
    "swarmail_clear_all",
    "swarmail_wait_for_email",
    "swarmail_extract_links",
    "swarmail_extract_codes",
    "swarmail_seed_email",
    "swarmail_set_chaos",
    "swarmail_clear_chaos",
    "swarmail_wait_for_code",
    "swarmail_wait_for_link",
    "swarmail_create_inbox",
    "swarmail_list_attachments",
    "swarmail_get_attachment",
];

// Secret-bearing inputs deliberately have neither Debug nor Serialize.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    executable: PathBuf,
    credential_file: PathBuf,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Families {
    claude: Selection,
    codex: Selection,
    dsh: Selection,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    families: Families,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClaudeAuth {
    #[serde(rename = "claudeAiOauth")]
    oauth: OAuth,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OAuth {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "expiresAt")]
    expires_at: u64,
    scopes: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexAuth {
    auth_mode: String,
    #[serde(
        rename = "OPENAI_API_KEY",
        deserialize_with = "crate::provenance::nullable"
    )]
    api_key: Option<String>,
    tokens: Tokens,
    last_refresh: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tokens {
    id_token: String,
    access_token: String,
    refresh_token: String,
    account_id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DshAuth {
    #[serde(rename = "DASHSCOPE_API_KEY")]
    dashscope: String,
    #[serde(rename = "SPARK_API_KEY")]
    spark: String,
}

fn cstring(s: &std::ffi::OsStr) -> Result<CString, String> {
    CString::new(s.as_bytes()).map_err(|_| "private_path_invalid".into())
}
fn fd_open(parent: &File, name: &std::ffi::OsStr, flags: i32, mode: u32) -> Result<File, String> {
    let name = cstring(name)?;
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            mode,
        )
    };
    if fd < 0 {
        return Err("private_path_unavailable".into());
    }
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn safe_dir(file: &File, path: &Path) -> Result<(), String> {
    let m = file.metadata().map_err(|_| "private_directory")?;
    let uid = unsafe { libc::geteuid() };
    // The sticky bit, spelled out: libc's S_ISVTX is u32 on Linux but u16 on macOS.
    let sticky_tmp =
        path == Path::new(crate::runner::SHARED_TMP) && m.uid() == 0 && m.mode() & 0o1000 != 0;
    if !m.is_dir() || (m.uid() != 0 && m.uid() != uid) || (!sticky_tmp && m.mode() & 0o022 != 0) {
        return Err("private_directory_unsafe".into());
    }
    Ok(())
}
/// Walk descriptors rather than checking a path and reopening it for payload.
fn directory(path: &Path) -> Result<File, String> {
    if !path.is_absolute()
        || path.components().any(|c| {
            !matches!(
                c,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        })
    {
        return Err("private_path_invalid".into());
    }
    let mut f = File::open("/").map_err(|_| "private_directory")?;
    let mut at = PathBuf::from("/");
    safe_dir(&f, &at)?;
    for c in path.components().skip(1) {
        f = fd_open(&f, c.as_os_str(), libc::O_RDONLY | libc::O_DIRECTORY, 0)?;
        at.push(c);
        safe_dir(&f, &at)?;
    }
    Ok(f)
}
fn private_metadata(m: &std::fs::Metadata, uid: u32) -> bool {
    m.is_file()
        && m.uid() == uid
        && m.mode() & 0o077 == 0
        && m.nlink() == 1
        && m.len() <= PRIVATE_LIMIT
}
fn private_read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let parent = directory(path.parent().ok_or("private_path_invalid")?)?;
    let f = fd_open(
        &parent,
        path.file_name().ok_or("private_path_invalid")?,
        libc::O_RDONLY,
        0,
    )?;
    let m = f.metadata().map_err(|_| "private_input_unavailable")?;
    if !private_metadata(&m, unsafe { libc::geteuid() }) {
        return Err("private_input_unsafe".into());
    }
    let mut b = Vec::new();
    f.take(PRIVATE_LIMIT + 1)
        .read_to_end(&mut b)
        .map_err(|_| "private_input_unavailable")?;
    if b.len() as u64 > PRIVATE_LIMIT {
        return Err("private_input_oversized".into());
    }
    crate::provenance::decode(&b).map_err(|_| "private_input_invalid".into())
}
fn make_dir(parent: &File, name: &str) -> Result<File, String> {
    let c = CString::new(name).expect("fixed directory name");
    if unsafe { libc::mkdirat(parent.as_raw_fd(), c.as_ptr(), 0o700) } != 0 {
        return Err("private_directory_collision".into());
    }
    fd_open(parent, name.as_ref(), libc::O_RDONLY | libc::O_DIRECTORY, 0)
}
fn write_private(parent: &File, name: &str, bytes: &[u8]) -> Result<(), String> {
    let mut f = fd_open(
        parent,
        name.as_ref(),
        libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL,
        0o600,
    )?;
    f.write_all(bytes)
        .map_err(|_| "private_payload_write".into())
}
fn json_bytes(value: Value) -> Vec<u8> {
    serde_json::to_vec(&value).expect("typed configuration")
}
fn exists_or_unknown(path: &Path) -> bool {
    !matches!(std::fs::symlink_metadata(path), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
}
fn preflight(family: &str, cwd: &Path, system: &Path) -> Result<(), String> {
    let managed = match family {
        "claude" => Some("claude-code"),
        "codex" => Some("codex"),
        _ => None,
    };
    if managed.is_some_and(|name| exists_or_unknown(&system.join(name))) {
        return Err("wrapper_managed_config_present".into());
    }
    for ancestor in cwd.ancestors() {
        if AMBIENT
            .iter()
            .any(|name| exists_or_unknown(&ancestor.join(name)))
        {
            return Err("wrapper_ambient_config_present".into());
        }
    }
    Ok(())
}
fn runtime(cwd: &Path) -> Result<BTreeMap<OsString, OsString>, String> {
    cwd.to_str().ok_or("wrapper_runtime_invalid")?;
    let cwd_fd = directory(cwd)?;
    let m = cwd_fd.metadata().map_err(|_| "wrapper_runtime_invalid")?;
    if m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o777 != 0o700 {
        return Err("wrapper_runtime_invalid".into());
    }
    let mut map: BTreeMap<OsString, OsString> =
        [("PATH", "/usr/bin:/bin"), ("LANG", "C"), ("LC_ALL", "C")]
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
    for key in [
        "HOME",
        "TMPDIR",
        "XDG_CONFIG_HOME",
        "XDG_CACHE_HOME",
        "XDG_DATA_HOME",
        "XDG_STATE_HOME",
    ] {
        let value = std::env::var_os(key).ok_or("wrapper_runtime_missing")?;
        value.to_str().ok_or("wrapper_runtime_invalid")?;
        let f = directory(Path::new(&value))?;
        let m = f.metadata().map_err(|_| "wrapper_runtime_invalid")?;
        if m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o777 != 0o700 {
            return Err("wrapper_runtime_invalid".into());
        }
        map.insert(key.into(), value);
    }
    map.insert("PWD".into(), cwd.as_os_str().into());
    Ok(map)
}
fn nonempty(s: &str) -> Result<(), String> {
    if s.is_empty() || s.contains('\0') {
        Err("private_credential_invalid".into())
    } else {
        Ok(())
    }
}
fn helper_env(inputs: &Inputs, map: &BTreeMap<OsString, OsString>) -> BTreeMap<String, String> {
    let mut out: BTreeMap<_, _> = map
        .iter()
        .map(|(k, v)| {
            (
                k.to_str().expect("runtime key").into(),
                v.to_str().expect("validated runtime value").to_string(),
            )
        })
        .collect();
    out.extend([
        (INPUTS[0].into(), inputs.task.clone()),
        (INPUTS[1].into(), inputs.app.to_string()),
        (INPUTS[2].into(), inputs.mcp.to_string()),
        (
            INPUTS[3].into(),
            inputs.identity.to_str().expect("validated identity").into(),
        ),
        (INPUTS[4].into(), inputs.inbox.clone()),
        (INPUTS[5].into(), inputs.goal.clone()),
        (
            INPUTS[6].into(),
            inputs
                .prompt_file
                .to_str()
                .expect("validated prompt path")
                .to_string(),
        ),
        (INPUTS[7].into(), inputs.deadline_ms.to_string()),
    ]);
    out
}
/// A finite TOML value encoder: JSON strings need TOML's \uXXXX control escapes.
fn toml(value: &Value) -> String {
    match value {
        Value::String(s) => {
            let mut out = String::from("\"");
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c if c.is_control() => out.push_str(&format!("\\u{:04X}", c as u32)),
                    c => out.push(c),
                }
            }
            out.push('"');
            out
        }
        Value::Array(a) => format!("[{}]", a.iter().map(toml).collect::<Vec<_>>().join(", ")),
        Value::Object(o) => format!(
            "{{{}}}",
            o.iter()
                .map(|(k, v)| format!("{} = {}", toml(&json!(k)), toml(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Bool(_) | Value::Number(_) => value.to_string(),
        Value::Null => unreachable!("no null in fixed TOML recipes"),
    }
}

// JSON flow syntax is YAML 1.2, but YAML also forbids raw DEL/C1 controls.
fn yaml_flow(value: &Value) -> String {
    serde_json::to_string(value)
        .expect("typed row")
        .chars()
        .map(|c| {
            if c.is_control() {
                format!("\\u{:04X}", c as u32)
            } else {
                c.to_string()
            }
        })
        .collect()
}

fn manifest(config: &Path) -> Result<Manifest, String> {
    if !config.is_absolute() {
        return Err("wrapper_config_locator_invalid".into());
    }
    let manifest: Manifest = private_read(config)?;
    if manifest.version != 1 {
        return Err("wrapper_config_version".into());
    }
    for selection in [
        &manifest.families.claude,
        &manifest.families.codex,
        &manifest.families.dsh,
    ] {
        if !selection.executable.is_absolute() || !selection.credential_file.is_absolute() {
            return Err("wrapper_selection_invalid".into());
        }
    }
    Ok(manifest)
}
/// Validate the credential-free selection after controls, before admitting a
/// potentially large paid product. This never opens a credential file.
pub fn validate_config(config: &Path) -> Result<(), String> {
    manifest(config).map(|_| ())
}

fn validate_goal(family: &str, goal: &str) -> Result<(), String> {
    if family == "dsh" && (goal.starts_with('-') || goal.contains('\0') || goal.len() > 65535) {
        Err("wrapper_goal_unsupported".into())
    } else {
        Ok(())
    }
}
/// The brokkr driver a wrapper may launch the family CLI through (decision
/// 0119): an absolute path to a regular executable, checked before any
/// private file is read.
fn validate_driver(driver: &Path) -> Result<(), String> {
    if !driver.is_absolute() {
        return Err("driver_invalid".into());
    }
    match std::fs::metadata(driver) {
        Ok(m) if m.is_file() && m.mode() & 0o111 != 0 => Ok(()),
        _ => Err("driver_invalid".into()),
    }
}

/// `DRIVER <family> -- <program> <args...>` with the direct launch's
/// environment, working directory and output routing: the driver is only a
/// different launcher for the same argv. Test-only (decision 0119, revision 3):
/// no production path reaches it while `driver_contract_unknown` is refused,
/// so the compiler, not a comment, keeps it out of the shipped binary. The
/// follow-up that implements the observed F2 invocation lifts the refusal and
/// restores the arm in `run()` that launches through the driver; this
/// assembly and its inline proof are its starting point.
#[cfg(test)]
fn through_driver(direct: &Command, family: &str, driver: &Path) -> Command {
    let mut cmd = Command::new(driver);
    cmd.arg(family)
        .arg("--")
        .arg(direct.get_program())
        .args(direct.get_args())
        .env_clear();
    for (key, value) in direct.get_envs() {
        if let Some(value) = value {
            cmd.env(key, value);
        }
    }
    if let Some(dir) = direct.get_current_dir() {
        cmd.current_dir(dir);
    }
    cmd.stdout(Stdio::null()).stderr(Stdio::null());
    cmd
}

pub fn run(
    harness: &str,
    model: &str,
    config: &Path,
    via_brokkr: Option<&Path>,
) -> Result<(), String> {
    validate_model(harness, model)?;
    if !config.is_absolute() {
        return Err("wrapper_config_locator_invalid".into());
    }
    if let Some(driver) = via_brokkr {
        validate_driver(driver)?;
        // Fail closed (decision 0119, revision 2): the invocation brokkr's
        // driver accepts (F2) has never been observed, and running a guessed
        // argv would turn a driver error into the agent's exit, scored as the
        // task's idle class. This refusal precedes every environment read,
        // credential read and spawn, and has no switch: `through_driver` below
        // is the test-only assembly the follow-up will lift this refusal onto.
        return Err("driver_contract_unknown".into());
    }
    let inputs = Inputs::environment()?;
    validate_goal(harness, &inputs.goal)?;
    let manifest = manifest(config)?;
    let selected = match harness {
        "claude" => manifest.families.claude,
        "codex" => manifest.families.codex,
        _ => manifest.families.dsh,
    };
    let m =
        std::fs::metadata(&selected.executable).map_err(|_| "wrapper_executable_unavailable")?;
    if !m.is_file() || m.mode() & 0o111 == 0 {
        return Err("wrapper_executable_invalid".into());
    }
    let cwd = std::env::current_dir().map_err(|_| "wrapper_workdir")?;
    preflight(harness, &cwd, Path::new("/etc"))?;
    let map = runtime(&cwd)?;
    let home = PathBuf::from(map.get(std::ffi::OsStr::new("HOME")).expect("home"));
    if std::fs::read_dir(&home)
        .map_err(|_| "wrapper_home")?
        .next()
        .is_some()
    {
        return Err("wrapper_home_not_fresh".into());
    }
    let home_fd = directory(&home)?;
    let executable = std::env::current_exe().map_err(|_| "wrapper_helper_executable")?;
    executable
        .to_str()
        .ok_or("wrapper_helper_executable_invalid")?;
    let env = helper_env(&inputs, &map);
    let headers = json!({"X-Eval-Identity":inputs.identity.to_str().expect("identity")});
    let mut cmd = Command::new(&selected.executable);
    cmd.env_clear()
        .envs(&map)
        .current_dir(&cwd)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match harness {
        "claude" => {
            let auth: ClaudeAuth = private_read(&selected.credential_file)?;
            nonempty(&auth.oauth.access_token)?;
            if auth.oauth.expires_at > 9_007_199_254_740_991
                || auth.oauth.scopes != ["user:inference"]
            {
                return Err("private_credential_invalid".into());
            }
            let dir = make_dir(&home_fd, ".claude")?;
            write_private(
                &dir,
                ".credentials.json",
                &json_bytes(
                    json!({"claudeAiOauth":{"accessToken":auth.oauth.access_token,"expiresAt":auth.oauth.expires_at,"scopes":auth.oauth.scopes}}),
                ),
            )?;
            write_private(
                &home_fd,
                "mcp.json",
                &json_bytes(json!({"mcpServers":{
                    "swarmail":{"type":"http","url":inputs.mcp.as_str(),"headers":headers},
                    "fixture":{"type":"stdio","command":executable,"args":["fixture-http-isolated"],"env":env}
                }})),
            )?;
            let allowed = std::iter::once("mcp__fixture__eval_http".to_string())
                .chain(SWARMAIL_TOOLS.iter().map(|n| format!("mcp__swarmail__{n}")))
                .collect::<Vec<_>>()
                .join(",");
            cmd.args([
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
            ])
            .arg(home.join("mcp.json"))
            .args([
                "--allowedTools",
                &allowed,
                "--restricted",
                "--disable-slash-commands",
            ])
            .env("DISABLE_AUTOUPDATER", "1");
        }
        "codex" => {
            let auth: CodexAuth = private_read(&selected.credential_file)?;
            for token in [
                &auth.tokens.id_token,
                &auth.tokens.access_token,
                &auth.tokens.account_id,
                &auth.tokens.refresh_token,
            ] {
                nonempty(token)?;
            }
            if auth.auth_mode != "chatgpt"
                || auth.api_key.is_some()
                || chrono::DateTime::parse_from_rfc3339(&auth.last_refresh).is_err()
            {
                return Err("private_credential_invalid".into());
            }
            let dir = make_dir(&home_fd, ".codex")?;
            write_private(
                &dir,
                "auth.json",
                &json_bytes(
                    json!({"auth_mode":"chatgpt","OPENAI_API_KEY":null,"tokens":{"id_token":auth.tokens.id_token,"access_token":auth.tokens.access_token,"account_id":auth.tokens.account_id,"refresh_token":"unavailable"},"last_refresh":chrono::Utc::now().to_rfc3339()}),
                ),
            )?;
            cmd.env("CODEX_HOME", home.join(".codex"))
                .args(["exec", "--json", "--model", model, "-C"])
                .arg(&cwd)
                .args([
                    "--skip-git-repo-check",
                    "--ephemeral",
                    "--ignore-user-config",
                    "--ignore-rules",
                    "--sandbox",
                    "read-only",
                ]);
            let overrides = vec![
                ("web_search".to_string(), json!("disabled")),
                (
                    format!("projects.{}.trust_level", toml(&json!(cwd))),
                    json!("untrusted"),
                ),
                (
                    "mcp_servers.swarmail.url".into(),
                    json!(inputs.mcp.as_str()),
                ),
                ("mcp_servers.swarmail.http_headers".into(), headers),
                ("mcp_servers.swarmail.startup_timeout_sec".into(), json!(20)),
                ("mcp_servers.swarmail.tool_timeout_sec".into(), json!(60)),
                (
                    "mcp_servers.swarmail.default_tools_approval_mode".into(),
                    json!("approve"),
                ),
                ("mcp_servers.fixture.command".into(), json!(executable)),
                (
                    "mcp_servers.fixture.args".into(),
                    json!(["fixture-http-isolated"]),
                ),
                ("mcp_servers.fixture.env".into(), json!(env)),
                ("mcp_servers.fixture.cwd".into(), json!(cwd)),
                (
                    "mcp_servers.fixture.default_tools_approval_mode".into(),
                    json!("approve"),
                ),
            ];
            for (k, v) in overrides {
                cmd.arg("-c").arg(format!("{k}={}", toml(&v)));
            }
            cmd.arg("-");
        }
        _ => {
            let auth: DshAuth = private_read(&selected.credential_file)?;
            nonempty(&auth.dashscope)?;
            nonempty(&auth.spark)?;
            let (provider, model_id) = model.split_once('/').expect("validated model");
            let (key, token, route) = if provider == "dashscope" {
                (
                    "DASHSCOPE_API_KEY",
                    auth.dashscope,
                    json!({"displayName":"Model Studio (Token Plan)","apiKeyEnv":"DASHSCOPE_API_KEY","api":"openai-completions","baseURL":"https://token-plan.ap-southeast-1.maas.aliyuncs.com/compatible-mode/v1","compat":{"thinkingFormat":"deepseek","supportsDeveloperRole":false},"models":[{"id":model_id}]}),
                )
            } else {
                (
                    "SPARK_API_KEY",
                    auth.spark,
                    json!({"displayName":"DGX Spark vLLM (GLM)","apiKeyEnv":"SPARK_API_KEY","api":"openai-completions","baseURL":"http://spark:8888/v1","models":[{"id":model_id,"contextWindow":200000,"maxTokens":32768}]}),
                )
            };
            make_dir(&home_fd, ".dsh")?;
            make_dir(&home_fd, "sessions")?;
            let mut rows = vec![
                json!({"id":"llm-pi-ai","config":{"providers":{provider:route}}}),
                json!({"id":"agent-default-model","config":{"provider":provider,"model":model_id}}),
                json!({"id":"session-persistence-jsonl","config":{"root":home.join("sessions"),"compression":"none","packChunks":false}}),
                json!({"insert":[
                    {"id":"mcp-swarmail","name":"@deepseek-ai/dsh-mcp-client","config":{"serverName":"swarmail","transport":"streamable-http","url":inputs.mcp.as_str(),"headers":headers,"failOnStartupError":true,"toolCallTimeoutMs":60000}},
                    {"id":"mcp-fixture","name":"@deepseek-ai/dsh-mcp-client","config":{"serverName":"fixture","transport":"stdio","command":executable,"args":["fixture-http-isolated"],"env":env,"cwd":cwd,"failOnStartupError":true}}
                ]}),
            ];
            if provider == "spark-glm" {
                rows.push(json!({"id":"compaction-basic","config":{"modelPolicies":[{"provider":provider,"model":model_id,"thresholdRatio":0.8,"retainRatio":0.16}]}}));
            }
            // JSON flow nodes are YAML 1.2 nodes; list markers retain the measured patch-list grammar.
            let overlay = rows
                .iter()
                .map(|r| format!("- {}\n", yaml_flow(r)))
                .collect::<String>();
            write_private(&home_fd, "seat.yml", overlay.as_bytes())?;
            cmd.env("DSH_HOME", home.join(".dsh"))
                .env("DSH_TELEMETRY_DISABLED", "1")
                .env(key, token)
                .args(["--profile", "headless", "--patch"])
                .arg(home.join("seat.yml"))
                .arg(&inputs.goal);
        }
    }
    if harness == "dsh" {
        cmd.stdin(Stdio::null());
    } else {
        // Recheck the opened descriptor, then pass that same file as stdin.
        let mut f = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&inputs.prompt_file)
            .map_err(|_| "eval_prompt_unavailable")?;
        let mut bytes = Vec::new();
        (&mut f)
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "eval_prompt_unavailable")?;
        if bytes != inputs.goal.as_bytes() {
            return Err("eval_prompt_mismatch".into());
        }
        use std::io::{Seek, SeekFrom};
        f.seek(SeekFrom::Start(0))
            .map_err(|_| "eval_prompt_unavailable")?;
        cmd.stdin(f);
    }
    let _error = cmd.exec();
    Err("wrapper_exec_failed".into())
}

/// Prevent credentials inherited by MCP launchers from entering the helper.
/// This exec boundary creates neither a group nor a cleanup owner.
pub fn isolated_helper() -> Result<(), String> {
    let inputs = Inputs::environment()?;
    let cwd = std::env::current_dir().map_err(|_| "helper_workdir")?;
    let map = runtime(&cwd)?;
    let mut cmd = Command::new(std::env::current_exe().map_err(|_| "helper_executable")?);
    cmd.arg("fixture-http")
        .env_clear()
        .envs(helper_env(&inputs, &map));
    let _error = cmd.exec();
    Err("helper_exec_failed".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};
    #[test]
    fn managed_and_ambient_sources_fail_closed_without_a_production_override() {
        let root = crate::runner::private_tempdir("wrapper-proof-").unwrap();
        let cwd = root.path().join("work");
        let etc = root.path().join("system");
        std::fs::create_dir(&cwd).unwrap();
        std::fs::create_dir(&etc).unwrap();
        for (family, name) in [("claude", "claude-code"), ("codex", "codex")] {
            assert!(preflight(family, &cwd, &etc).is_ok());
            let target = etc.join(name);
            std::fs::create_dir(&target).unwrap();
            assert!(preflight(family, &cwd, &etc).is_err());
            std::fs::remove_dir(&target).unwrap();
            for bytes in [b"".as_slice(), b"managed"] {
                std::fs::write(&target, bytes).unwrap();
                assert!(preflight(family, &cwd, &etc).is_err());
                std::fs::remove_file(&target).unwrap();
            }
            symlink("absent", &target).unwrap();
            assert!(preflight(family, &cwd, &etc).is_err());
            std::fs::remove_file(target).unwrap();
        }
        // ENOTDIR is not evidence of absence.
        let file = etc.join("not-a-directory");
        std::fs::write(&file, b"").unwrap();
        assert!(preflight("claude", &cwd, &file).is_err());
        for name in AMBIENT {
            let path = root.path().join(name);
            symlink("absent", &path).unwrap();
            for family in ["claude", "codex", "dsh"] {
                assert!(preflight(family, &cwd, &etc).is_err());
            }
            std::fs::remove_file(path).unwrap();
        }
    }
    #[test]
    fn through_driver_assembles_the_direct_argv_byte_for_byte() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let driver = Path::new("/opt/brokkr/brokkr-driver");
        // The shapes of the three families' direct launches: the goal on stdin
        // for claude and codex, as the last literal argument for dsh.
        let non_utf8 = OsStr::from_bytes(b"model-\xff-bytes");
        for (family, program, args) in [
            (
                "claude",
                "/opt/claude/bin/claude",
                vec![
                    OsStr::new("-p"),
                    OsStr::new("--mcp-config"),
                    OsStr::new("/home/x/mcp.json"),
                ],
            ),
            (
                "codex",
                "/opt/codex/bin/codex",
                vec![OsStr::new("exec"), OsStr::new("-m"), non_utf8],
            ),
            (
                "dsh",
                "/opt/dsh/bin/dsh",
                vec![
                    OsStr::new("--profile"),
                    OsStr::new("headless"),
                    OsStr::new("a goal; with $(shell) \" metacharacters"),
                ],
            ),
        ] {
            let mut direct = Command::new(program);
            direct
                .args(&args)
                .env_clear()
                .env("HOME", "/run/home")
                .env("PATH", "/usr/bin:/bin")
                .env("EMPTY", "")
                .env_remove("REMOVED")
                .current_dir("/run/work");
            let driven = through_driver(&direct, family, driver);
            assert_eq!(driven.get_program(), driver.as_os_str(), "{family}");
            let argv: Vec<&OsStr> = driven.get_args().collect();
            assert_eq!(argv[0], OsStr::new(family));
            assert_eq!(argv[1], OsStr::new("--"));
            assert_eq!(argv[2], OsStr::new(program), "{family}: the program");
            assert_eq!(
                &argv[3..],
                &args[..],
                "{family}: the argv after the program, byte for byte"
            );
            let direct_env: Vec<_> = direct.get_envs().filter(|(_, v)| v.is_some()).collect();
            let driven_env: Vec<_> = driven.get_envs().collect();
            assert_eq!(
                driven_env, direct_env,
                "{family}: the same keys and values, nothing added, nothing inherited"
            );
            assert_eq!(driven.get_current_dir(), direct.get_current_dir());
        }
        // A direct launch with no working directory stays without one.
        let bare = Command::new("/opt/claude/bin/claude");
        assert_eq!(
            through_driver(&bare, "claude", driver).get_current_dir(),
            None
        );
    }
    #[test]
    fn the_via_brokkr_refusal_comes_after_the_driver_check_and_before_every_read() {
        let root = crate::runner::private_tempdir("wrapper-proof-").unwrap();
        let plain = root.path().join("not-executable");
        std::fs::write(&plain, b"#!/bin/sh\n").unwrap();
        let exe = root.path().join("driver");
        std::fs::write(&exe, b"#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o700)).unwrap();
        let config = Path::new("/nonexistent/selection.json");
        let run_with = |driver: &Path| run("claude", "claude-sonnet-5-5", config, Some(driver));
        assert_eq!(run_with(Path::new("rel")), Err("driver_invalid".into()));
        assert_eq!(run_with(&plain), Err("driver_invalid".into()));
        assert_eq!(run_with(&exe), Err("driver_contract_unknown".into()));
        // The same call without the flag goes on to read its inputs and refuses
        // for a different reason, which shows the refusal above preceded them.
        let direct = run("claude", "claude-sonnet-5-5", config, None).unwrap_err();
        assert_ne!(direct, "driver_contract_unknown");
    }
    #[test]
    fn descriptor_inputs_refuse_unsafe_paths_types_schemas_and_bounds() {
        let root = crate::runner::private_tempdir("wrapper-proof-").unwrap();
        let parent = directory(root.path()).unwrap();
        let path = root.path().join("input");
        write_private(
            &parent,
            "input",
            br#"{"DASHSCOPE_API_KEY":"one","SPARK_API_KEY":"two"}"#,
        )
        .unwrap();
        assert!(private_read::<DshAuth>(&path).is_ok());
        assert!(write_private(&parent, "input", b"replace").is_err());
        let before = std::fs::read(&path).unwrap();
        for mode in [0o644, 0o620, 0o601] {
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
            assert!(private_read::<DshAuth>(&path).is_err());
        }
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let link = root.path().join("link");
        symlink(&path, &link).unwrap();
        assert!(private_read::<DshAuth>(&link).is_err());
        assert!(private_read::<DshAuth>(root.path()).is_err());
        let fifo = root.path().join("fifo");
        let c = cstring(fifo.as_os_str()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
        assert!(private_read::<DshAuth>(&fifo).is_err());
        let hard = root.path().join("hard");
        std::fs::hard_link(&path, &hard).unwrap();
        assert!(private_read::<DshAuth>(&path).is_err());
        std::fs::remove_file(hard).unwrap();
        let alias = root.path().join("alias");
        symlink(root.path(), &alias).unwrap();
        assert!(private_read::<DshAuth>(&alias.join("input")).is_err());
        assert!(directory(Path::new("relative")).is_err());
        assert!(directory(&root.path().join("../escape")).is_err());
        let uid = unsafe { libc::geteuid() };
        let metadata = std::fs::metadata(&path).unwrap();
        assert_eq!(metadata.uid(), uid);
        assert!(!private_metadata(&metadata, uid + 1));
        for bytes in [
            b"{}".as_slice(),
            br#"{"DASHSCOPE_API_KEY":"x","DASHSCOPE_API_KEY":"y","SPARK_API_KEY":"z"}"#,
            br#"{"DASHSCOPE_API_KEY":"x","SPARK_API_KEY":"z","extra":1}"#,
            b"not-json",
        ] {
            std::fs::write(&path, bytes).unwrap();
            assert!(private_read::<DshAuth>(&path).is_err());
        }
        let mut bytes = before;
        bytes.resize(PRIVATE_LIMIT as usize, b' ');
        std::fs::write(&path, &bytes).unwrap();
        assert!(private_read::<DshAuth>(&path).is_ok());
        bytes.push(b' ');
        std::fs::write(&path, bytes).unwrap();
        assert!(private_read::<DshAuth>(&path).is_err());
        let unsafe_parent = root.path().join("unsafe");
        std::fs::create_dir(&unsafe_parent).unwrap();
        std::fs::set_permissions(&unsafe_parent, std::fs::Permissions::from_mode(0o777)).unwrap();
        assert!(directory(&unsafe_parent).is_err());
        for bytes in [br#"{"claudeAiOauth":{"accessToken":"x","expiresAt":1,"scopes":["user:inference"],"extra":1}}"#.as_slice(),br#"{"claudeAiOauth":{"accessToken":"x","accessToken":"y","expiresAt":1,"scopes":["user:inference"]}}"#] {
            std::fs::write(&path,bytes).unwrap();
            assert!(private_read::<ClaudeAuth>(&path).is_err());
        }
        assert!(validate_goal("dsh", "nul\0goal").is_err());
        assert!(validate_goal("dsh", &"x".repeat(65535)).is_ok());
        assert!(validate_goal("dsh", &"x".repeat(65536)).is_err());
        assert_eq!(
            toml(&json!("\"\\\n\r\t\u{0008}\u{007f}🦀")),
            "\"\\\"\\\\\\n\\r\\t\\u0008\\u007F🦀\""
        );
    }
}
