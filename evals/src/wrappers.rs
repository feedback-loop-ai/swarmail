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
    /// The trial's private observation socket when value capture is on
    /// (decision 0119 §4): a path inside the runner's private runtime, never
    /// a secret. Absent in every default trial.
    pub capture_socket: Option<PathBuf>,
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
        let capture_socket = match values.get(crate::toolvalues::ENV_CAPTURE_SOCKET) {
            None => None,
            Some(path) if Path::new(path).is_absolute() && !path.contains('\0') => {
                Some(PathBuf::from(path))
            }
            Some(_) => return Err("eval_capture_socket_invalid".into()),
        };
        Ok(Self {
            task,
            app,
            mcp,
            identity,
            inbox,
            goal,
            prompt_file,
            deadline_ms,
            capture_socket,
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
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let mut values = values;
        if let Ok(path) = std::env::var(crate::toolvalues::ENV_CAPTURE_SOCKET) {
            values.insert(crate::toolvalues::ENV_CAPTURE_SOCKET.to_string(), path);
        }
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

/// The server key an agent's MCP config names Swarmail by.
pub const SWARMAIL_SERVER: &str = "swarmail";
/// The optional inputs that name a competitor target's MCP surface
/// (proposed decision 0129 §6): the server key and the adapter's own
/// announced tool names, comma-separated. Neither set means Swarmail's.
pub const ENV_MCP_SERVER: &str = "EVAL_MCP_SERVER";
pub const ENV_MCP_TOOLS: &str = "EVAL_MCP_TOOLS";

/// The MCP surface an agent is configured with: a server key and the tool
/// names its allowlist admits. Both inputs or neither; every name is
/// validated, because each ends up in a CLI argument or a config key.
pub fn mcp_surface(
    server: Option<&str>,
    tools: Option<&str>,
) -> Result<(String, Vec<String>), String> {
    match (server, tools) {
        (None, None) => Ok((
            SWARMAIL_SERVER.into(),
            SWARMAIL_TOOLS.iter().map(|t| (*t).to_string()).collect(),
        )),
        (Some(server), Some(tools)) => {
            // `fixture` is the wrapper's own stdio server's key.
            if !crate::target::valid_server_key(server) || server == "fixture" {
                return Err("eval_mcp_server_invalid".into());
            }
            let names: Vec<String> = tools.split(',').map(str::to_string).collect();
            let unique: std::collections::BTreeSet<_> = names.iter().collect();
            if names.len() > 64
                || unique.len() != names.len()
                || !names.iter().all(|n| crate::target::valid_tool_name(n))
            {
                return Err("eval_mcp_tools_invalid".into());
            }
            Ok((server.into(), names))
        }
        _ => Err("eval_mcp_surface_incomplete".into()),
    }
}

/// No competitor cell runs on the dsh wrapper: spark is down and the
/// DashScope GLM quota is out (spec non-goals), so a competitor surface
/// there is refused by name before any credential is read.
pub fn refuse_dsh_competitor(harness: &str, server: &str) -> Result<(), String> {
    if harness == "dsh" && server != SWARMAIL_SERVER {
        return Err("dsh_competitor_unsupported".into());
    }
    Ok(())
}

/// Claude's `--allowedTools` value: the fixture tool, then the server's.
fn claude_allowed(server: &str, tools: &[String]) -> String {
    std::iter::once("mcp__fixture__eval_http".to_string())
        .chain(tools.iter().map(|n| format!("mcp__{server}__{n}")))
        .collect::<Vec<_>>()
        .join(",")
}

/// Codex's overrides for the isolated fixture helper.
fn fixture_overrides(
    executable: &Path,
    env: &BTreeMap<String, String>,
    cwd: &Path,
) -> Vec<(String, Value)> {
    vec![
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
    ]
}
/// Codex's `-c` overrides for the target's MCP server.
fn codex_server_overrides(server: &str, url: &str, headers: Value) -> Vec<(String, Value)> {
    vec![
        (format!("mcp_servers.{server}.url"), json!(url)),
        (format!("mcp_servers.{server}.http_headers"), headers),
        (
            format!("mcp_servers.{server}.startup_timeout_sec"),
            json!(20),
        ),
        (format!("mcp_servers.{server}.tool_timeout_sec"), json!(60)),
        (
            format!("mcp_servers.{server}.default_tools_approval_mode"),
            json!("approve"),
        ),
    ]
}

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

/// The paid-baseline (version 2) selection, proposed 0137: each family adds
/// its pinned CLI identity and auth-file shape to the version-1 locators. A
/// family the product does not name may be absent; nothing else may.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PaidSelection {
    executable: PathBuf,
    credential_file: PathBuf,
    profile: crate::provenance::WrapperProfile,
    cli_version: String,
    cli_sha256: String,
    auth_shape: String,
    /// dsh only: the interpreted pin of accepted 0137's 2026-11-23
    /// amendment. `executable` is then the pin's entry file and `cli_sha256`
    /// the pin's identity.
    #[serde(default)]
    interpreted: Option<crate::interpreted::Pin>,
    /// Route (b): the pinned brokkr driver this family may launch through,
    /// and the family's driver flags. Needed only for `--via-brokkr`.
    #[serde(default)]
    nested: Option<NestedSelection>,
}
/// The brokkr release whose driver route (b) runs (proposed 0138), pinned
/// apart from the delivery engine's `.brokkr/recipes/self/tooling.json`.
pub const DRIVER_VERSION: &str = "0.15.2";
/// The one platform the pinned release asset serves.
pub const DRIVER_PLATFORM: &str = "linux-x86_64";
/// The `SHA256SUMS` line of brokkr v0.15.2's `brokkr-linux-x86_64.tar.gz`:
/// the provenance a selection's driver pin names.
pub const DRIVER_ARCHIVE_SHA256: &str =
    "ecce9b5acea9e07eb5b9f978b87272aa87273add34f1f215cdc400aeb2f52eb3";
/// The `brokkr` executable extracted from that verified archive: the digest
/// an operator's selection pins (`driver_sha256`).
pub const DRIVER_RELEASE_SHA256: &str =
    "61cba4ee438eb0bd0dbc223f7d394219238aeecc061250e36281a4989e455062";
/// Whether a selection's driver digest is the published release executable.
/// A release build trusts exactly [`DRIVER_RELEASE_SHA256`], whatever the
/// environment says. A debug build (`debug_assertions`, the profile the
/// test suites build) also trusts the offline stand-in equipment beside its
/// own executable ([`standin_driver_admitted`]); such a build never spends a
/// credential that is not synthetic ([`seam_build_refuses_real_spend`]).
fn release_digest(sha256: &str) -> bool {
    sha256 == DRIVER_RELEASE_SHA256 || standin_driver_admitted(sha256)
}
/// Whether this build admits `sha256` as the nested driver: the published
/// release in every build, the adjacent stand-in in a debug build alone.
/// The release-profile proof (`release_seam.rs`) reads it.
pub fn driver_admitted(sha256: &str) -> bool {
    release_digest(sha256)
}
/// The fixed name of the offline stand-in executable the test suites build
/// beside this one (`tests/standin/paid_cli.rs`).
pub const STANDIN_DRIVER_NAME: &str = "paid-cli-standin";
/// Test equipment, compiled only into debug builds: the digest of the
/// stand-in built beside this executable. No environment value, flag or
/// selection field can widen the trusted set; a release build has no such
/// arm at all.
#[cfg(debug_assertions)]
fn standin_driver_admitted(sha256: &str) -> bool {
    // Beside this executable, or beside the profile directory when this is
    // a test harness under `deps/`.
    let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|me| me.parent().map(Path::to_path_buf))
    else {
        return false;
    };
    let dir = match dir.file_name() {
        Some(name) if name == "deps" => dir.parent().map_or(dir.clone(), Path::to_path_buf),
        _ => dir,
    };
    crate::identity::file_hash(&dir.join(STANDIN_DRIVER_NAME)).is_ok_and(|s| s == sha256)
}
#[cfg(not(debug_assertions))]
fn standin_driver_admitted(_sha256: &str) -> bool {
    false
}
/// Whether this build carries the stand-in seam above.
pub const SEAM_BUILD: bool = cfg!(debug_assertions);
/// The marker every offline credential fixture's authorizing value carries:
/// the value begins `SYNTHETIC-`, or it is a JWT whose signature segment
/// decodes to bytes that begin so (the codex fixture). A real login (an
/// issuer-signed JWT, an `sk-` key, an OAuth token) never does.
pub const SYNTHETIC_CREDENTIAL: &[u8] = b"SYNTHETIC-";
/// Whether one authorizing value is offline equipment.
fn synthetic_value(value: &str) -> bool {
    use base64::Engine as _;
    if value.as_bytes().starts_with(SYNTHETIC_CREDENTIAL) {
        return true;
    }
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 3
        && base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[2])
            .is_ok_and(|sig| sig.starts_with(SYNTHETIC_CREDENTIAL))
}
/// The JSON pointers of the values that authorize a spend, by
/// [`credential_key`]: the bearer each CLI presents, any key that could
/// stand in for it, and for dsh the one route's attested key.
fn authorizing_fields(key: &str) -> &'static [&'static str] {
    match key {
        "claude" => &["/claudeAiOauth/accessToken"],
        "codex" => &["/tokens/access_token", "/OPENAI_API_KEY"],
        "dsh:dashscope" => &["/DASHSCOPE_API_KEY/key"],
        "dsh:spark-glm" => &["/SPARK_API_KEY/key"],
        _ => &[],
    }
}
/// A build that carries the stand-in seam refuses a paid launch whose
/// credential is not offline equipment, before anything is spawned, on
/// every route and with the genuine release pin too. Only the family's
/// authorizing fields decide (security low 2 on c93e3931): the marker in
/// any other field (an id token, an account id, an extra key) admits
/// nothing. Every authorizing field present must be synthetic, a `null`
/// API key aside, and at least one must be.
fn seam_build_refuses_real_spend(credential: &Path, key: &str, seam: bool) -> Result<(), String> {
    if !seam {
        return Ok(());
    }
    let bytes = private_bytes(credential)?;
    let refused = || "paid_seam_build_refused".to_string();
    let doc: Value = serde_json::from_slice(&bytes).map_err(|_| refused())?;
    let mut seen = 0;
    for field in authorizing_fields(key) {
        match doc.pointer(field) {
            None | Some(Value::Null) => {}
            Some(Value::String(v)) if synthetic_value(v) => seen += 1,
            Some(_) => return Err(refused()),
        }
    }
    if seen == 0 {
        return Err(refused());
    }
    Ok(())
}
/// The effort words a nested claude or codex selection may name.
pub const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// Codex's inner sandbox class under the nested driver: a typed selection.
/// Only `read-only` and `workspace-write` are admitted (the operator's
/// 2026-11-24 ruling on #404): no enclosing boundary is verified that could
/// stand in for codex's own, so `danger-full-access` parses only to be
/// refused with its own reason before anything is read or spawned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CodexSandbox {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}
impl CodexSandbox {
    pub fn name(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::WorkspaceWrite => "workspace-write",
            Self::DangerFullAccess => "danger-full-access",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NestedSelection {
    driver: PathBuf,
    driver_sha256: String,
    driver_version: String,
    platform: String,
    archive_sha256: String,
    #[serde(default)]
    effort: Option<String>,
    #[serde(default)]
    codex_sandbox: Option<CodexSandbox>,
}
impl NestedSelection {
    /// The static shape, checked with the rest of the selection.
    fn validate(&self, family: &str) -> Result<(), String> {
        if !self.driver.is_absolute() {
            return Err("driver_selection_invalid".into());
        }
        if self.driver_version != DRIVER_VERSION {
            return Err("driver_version_unsupported".into());
        }
        if self.platform != DRIVER_PLATFORM {
            return Err("driver_platform_unsupported".into());
        }
        if self.archive_sha256 != DRIVER_ARCHIVE_SHA256 {
            return Err("driver_archive_unrecognized".into());
        }
        if !crate::provenance::is_hash(&self.driver_sha256) {
            return Err("driver_digest_invalid".into());
        }
        let effort_ok = match (family, self.effort.as_deref()) {
            ("dsh", None) => true,
            ("dsh", Some(_)) | (_, None) => false,
            (_, Some(e)) => EFFORTS.contains(&e),
        };
        if !effort_ok {
            return Err("driver_effort_invalid".into());
        }
        if (family == "codex") != self.codex_sandbox.is_some() {
            return Err("driver_sandbox_invalid".into());
        }
        if self.codex_sandbox == Some(CodexSandbox::DangerFullAccess) {
            return Err("driver_sandbox_unconfined".into());
        }
        Ok(())
    }
    /// The coordinator's admission (host baseline and `matrix --brokkr`
    /// alike): the pin must be the published release executable, whose
    /// archive line [`DRIVER_ARCHIVE_SHA256`] `validate` already required.
    fn release(&self) -> Result<(), String> {
        if release_digest(&self.driver_sha256) {
            Ok(())
        } else {
            Err("driver_release_unrecognized".into())
        }
    }
    /// The launch-time proof: the flag names the pinned driver, this host is
    /// the pinned platform, and the bytes and banner are the release's.
    fn check(&self, via: &Path) -> Result<(), String> {
        if via != self.driver {
            return Err("driver_selection_mismatch".into());
        }
        if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            return Err("driver_platform_unsupported".into());
        }
        check_pinned(&self.driver, &self.driver_sha256).map_err(|e| format!("driver_{e}"))?;
        probe_version(&self.driver, None, &format!("brokkr {DRIVER_VERSION}"))
            .map_err(|e| format!("driver_{e}"))
    }
}
impl PaidSelection {
    /// The identity a paid run binds: the CLI's pin for a direct launch; for
    /// a nested one, a digest over the CLI's pin, the driver's pin and the
    /// family's driver flags, so a changed driver, effort or codex sandbox
    /// class drifts on resume and at launch like a changed CLI.
    fn identity(&self, nested: bool) -> Result<String, String> {
        if !nested {
            return Ok(self.cli_sha256.clone());
        }
        let n = self.nested.as_ref().ok_or("driver_pin_missing")?;
        Ok(crate::provenance::sha256(
            format!(
                "swarmail-nested-pin/1\ncli {}\ndriver {} {} {}\neffort {}\nsandbox {}\n",
                self.cli_sha256,
                n.driver_sha256,
                n.driver_version,
                n.platform,
                n.effort.as_deref().unwrap_or("-"),
                n.codex_sandbox.map_or("-", CodexSandbox::name),
            )
            .as_bytes(),
        ))
    }
    /// The native program a launch or probe runs, and the argument that
    /// precedes the CLI's own: the pinned runtime on the pinned entry, or
    /// the pinned binary alone.
    fn program(&self) -> (&Path, Option<PathBuf>) {
        match &self.interpreted {
            Some(pin) => (&pin.runtime, Some(pin.entry_path())),
            None => (&self.executable, None),
        }
    }
    /// Verify the pinned bytes: the native binary, or the runtime and then
    /// every package entry. Nothing of the package runs before this passes.
    fn check(&self) -> Result<(), String> {
        match &self.interpreted {
            None => check_pinned(&self.executable, &self.cli_sha256),
            Some(pin) => {
                check_pinned(&pin.runtime, &pin.runtime_sha256)
                    .map_err(|e| format!("runtime_{e}"))?;
                crate::interpreted::compare(
                    &pin.files,
                    &crate::interpreted::measure(&pin.package_root)?,
                )
            }
        }
    }
    /// The bounded, credential-free banners: the runtime's own first, then
    /// the CLI's through the pinned runtime.
    fn probe(&self) -> Result<(), String> {
        if let Some(pin) = &self.interpreted {
            probe_version(&pin.runtime, None, &pin.runtime_version)
                .map_err(|e| format!("runtime_{e}"))?;
        }
        let (program, entry) = self.program();
        probe_version(program, entry.as_deref(), &version_banner(self.profile))
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PaidFamilies {
    #[serde(default)]
    claude: Option<PaidSelection>,
    #[serde(default)]
    codex: Option<PaidSelection>,
    #[serde(default)]
    dsh: Option<PaidSelection>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PaidManifest {
    version: u32,
    families: PaidFamilies,
}
/// The dsh paid shape: each route's key with the operator-attested UTC
/// valid-through instant (Unix seconds) in the same record.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DshAttested {
    #[serde(rename = "DASHSCOPE_API_KEY")]
    dashscope: AttestedKey,
    #[serde(rename = "SPARK_API_KEY")]
    spark: AttestedKey,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AttestedKey {
    key: String,
    valid_through: u64,
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
    crate::provenance::decode(&private_bytes(path)?).map_err(|_| "private_input_invalid".into())
}
fn private_bytes(path: &Path) -> Result<Vec<u8>, String> {
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
    Ok(b)
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
    if let Some(socket) = inputs.capture_socket.as_ref().and_then(|p| p.to_str()) {
        out.insert(
            crate::toolvalues::ENV_CAPTURE_SOCKET.into(),
            socket.to_string(),
        );
    }
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

/// One family's selection from either closed version: v1 locators alone, or
/// v2 locators with the paid contract that binds them.
struct Selected {
    executable: PathBuf,
    credential_file: PathBuf,
    paid: Option<PaidSelection>,
}
/// Whether an agent argv runs a paid wrapper selection, decided from the
/// selection the wrapper itself would read, never from profile metadata
/// (hold 1 on a73ac17a, part of #404). An argv naming a wrapper subcommand
/// is paid when its `--config` holds a version-2 selection for that
/// family, and also, failing closed, when its selection cannot be read or
/// it names none. Anything else is no wrapper of this executable.
pub fn argv_selects_paid<S: AsRef<std::ffi::OsStr>>(argv: &[S]) -> bool {
    match argv_selection(argv) {
        None => false,
        Some((family, config)) => config
            .is_none_or(|config| selection(&config, family).map_or(true, |s| s.paid.is_some())),
    }
}
/// The Codex sandbox class an agent argv's nested selection names, read
/// from the selection itself (hold 4 on a73ac17a): what the driver is told,
/// what the row's descriptor records. `None` when it cannot be read.
pub fn argv_codex_class<S: AsRef<std::ffi::OsStr>>(argv: &[S]) -> Option<&'static str> {
    let (family, config) = argv_selection(argv)?;
    selection(&config?, family)
        .ok()?
        .paid?
        .nested?
        .codex_sandbox
        .filter(|class| *class != CodexSandbox::DangerFullAccess)
        .map(CodexSandbox::name)
}
/// The wrapper family and `--config` an agent argv names, if it runs a
/// wrapper subcommand of this executable.
fn argv_selection<S: AsRef<std::ffi::OsStr>>(
    argv: &[S],
) -> Option<(&'static str, Option<PathBuf>)> {
    use std::os::unix::ffi::OsStrExt as _;
    let words: Vec<&std::ffi::OsStr> = argv.iter().map(AsRef::as_ref).collect();
    let (at, family) = words
        .iter()
        .enumerate()
        .find_map(|(i, w)| match w.to_str() {
            Some("agent-claude") => Some((i, "claude")),
            Some("agent-codex") => Some((i, "codex")),
            Some("agent-dsh") => Some((i, "dsh")),
            _ => None,
        })?;
    let mut config = None;
    let mut rest = words[at + 1..].iter();
    while let Some(word) = rest.next() {
        if word.as_bytes() == b"--config" {
            config = rest.next().map(PathBuf::from);
        } else if let Some(value) = word.as_bytes().strip_prefix(b"--config=") {
            config = Some(PathBuf::from(std::ffi::OsStr::from_bytes(value)));
        }
    }
    Some((family, config))
}
fn selection(config: &Path, family: &str) -> Result<Selected, String> {
    if !config.is_absolute() {
        return Err("wrapper_config_locator_invalid".into());
    }
    let bytes = private_bytes(config)?;
    if let Ok(manifest) = crate::provenance::decode::<Manifest>(&bytes) {
        validate_v1(&manifest)?;
        let s = match family {
            "claude" => manifest.families.claude,
            "codex" => manifest.families.codex,
            _ => manifest.families.dsh,
        };
        return Ok(Selected {
            executable: s.executable,
            credential_file: s.credential_file,
            paid: None,
        });
    }
    let paid = paid_selection(&bytes, family)?;
    Ok(Selected {
        executable: paid.executable.clone(),
        credential_file: paid.credential_file.clone(),
        paid: Some(paid),
    })
}
fn paid_selection(bytes: &[u8], family: &str) -> Result<PaidSelection, String> {
    let manifest: PaidManifest =
        crate::provenance::decode(bytes).map_err(|_| "private_input_invalid")?;
    if manifest.version != 2 {
        return Err("wrapper_config_version".into());
    }
    let selected = match family {
        "claude" => manifest.families.claude,
        "codex" => manifest.families.codex,
        _ => manifest.families.dsh,
    }
    .ok_or("contract_missing")?;
    if !selected.executable.is_absolute() || !selected.credential_file.is_absolute() {
        return Err("wrapper_selection_invalid".into());
    }
    if selected.profile.family() != family || selected.profile.via_brokkr() {
        return Err("profile_mismatch".into());
    }
    if !selected.profile.approved() {
        return Err("profile_superseded".into());
    }
    if selected.cli_version != selected.profile.cli_version() {
        return Err("version_unsupported".into());
    }
    if !crate::provenance::is_hash(&selected.cli_sha256) {
        return Err("digest_invalid".into());
    }
    // dsh is admitted only as an interpreted package under its full pin; a
    // native family never carries one.
    match (family, &selected.interpreted) {
        ("dsh", None) => return Err("interpreted_pin_missing".into()),
        ("dsh", Some(pin)) => {
            pin.validate()?;
            if selected.executable != pin.entry_path() {
                return Err("executable_not_entry".into());
            }
            if selected.cli_sha256 != pin.identity() {
                return Err("digest_mismatch".into());
            }
        }
        (_, Some(_)) => return Err("interpreted_pin_unexpected".into()),
        _ => {}
    }
    if selected.auth_shape != auth_shape(family) {
        return Err("auth_shape_unsupported".into());
    }
    if let Some(nested) = &selected.nested {
        nested.validate(family)?;
    }
    Ok(selected)
}
/// The one auth-file shape each family's paid contract may name.
pub fn auth_shape(family: &str) -> &'static str {
    match family {
        "claude" => "claude_oauth_v1",
        "codex" => "codex_chatgpt_v1",
        _ => "dsh_routes_attested_v1",
    }
}
/// The exact `--version` line each profile's CLI answers.
fn version_banner(profile: crate::provenance::WrapperProfile) -> String {
    match profile.family() {
        "claude" => format!("{} (Claude Code)", profile.cli_version()),
        "codex" => format!("codex-cli {}", profile.cli_version()),
        _ => profile.cli_version().to_string(),
    }
}
/// A pinned executable is a regular file itself (no symlink or forwarding
/// path), executable, owned by the operator or root and writable by no one
/// else, under safe ancestors, and its bytes hash to the pin.
fn check_pinned(executable: &Path, sha256: &str) -> Result<(), String> {
    directory(executable.parent().ok_or("executable_unsafe")?).map_err(|_| "executable_unsafe")?;
    let m = std::fs::symlink_metadata(executable).map_err(|_| "executable_unavailable")?;
    let uid = unsafe { libc::geteuid() };
    if !m.file_type().is_file()
        || m.mode() & 0o111 == 0
        || m.mode() & 0o022 != 0
        || (m.uid() != uid && m.uid() != 0)
    {
        return Err("executable_unsafe".into());
    }
    let mut head = [0u8; 4];
    let n = File::open(executable)
        .and_then(|mut f| f.read(&mut head))
        .map_err(|_| "executable_unavailable")?;
    if !native_image(&head[..n]) {
        return Err("executable_not_native".into());
    }
    if crate::identity::file_hash(executable).map_err(|_| "executable_unavailable")? != sha256 {
        return Err("digest_mismatch".into());
    }
    Ok(())
}
/// Whether `head` opens a native executable image: ELF, or Mach-O thin or
/// universal. An interpreter script (`#!`) or any other launcher forwards to
/// bytes its digest cannot cover, so it can never establish CLI identity.
fn native_image(head: &[u8]) -> bool {
    const MAGIC: [[u8; 4]; 7] = [
        *b"\x7fELF",
        [0xfe, 0xed, 0xfa, 0xce],
        [0xfe, 0xed, 0xfa, 0xcf],
        [0xce, 0xfa, 0xed, 0xfe],
        [0xcf, 0xfa, 0xed, 0xfe],
        [0xca, 0xfe, 0xba, 0xbe],
        [0xbe, 0xba, 0xfe, 0xca],
    ];
    MAGIC.iter().any(|m| head == m)
}
const PROBE_LIMIT: usize = 4096;
const PROBE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);
/// Run `EXECUTABLE --version` with a closed environment, an empty private
/// HOME, no stdin and a bounded stdout, and kill its whole group afterwards.
/// The banner never leaves this function; only the verdict does.
fn probe_version(executable: &Path, entry: Option<&Path>, expected: &str) -> Result<(), String> {
    use std::os::unix::fs::DirBuilderExt as _;
    use std::os::unix::process::CommandExt as _;
    let home = crate::runner::private_tempdir("cli-probe-").map_err(|_| "version_probe_failed")?;
    let mut probe = Command::new(executable);
    probe
        .args(entry)
        .arg("--version")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .env("HOME", home.path());
    // Every child gets the private runtime map (decision 0075 §1): a private
    // TMPDIR and the four XDG directories, never the operator's defaults.
    for (key, name) in [
        ("TMPDIR", "tmp"),
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_CACHE_HOME", "cache"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_STATE_HOME", "state"),
    ] {
        let dir = home.path().join(name);
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&dir)
            .map_err(|_| "version_probe_failed")?;
        probe.env(key, dir);
    }
    let mut child = probe
        .current_dir(home.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|_| "version_probe_failed")?;
    let group = child.id() as i32;
    let stdout = child.stdout.take().expect("piped stdout");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = stdout.take(PROBE_LIMIT as u64 + 1).read_to_end(&mut out);
        let _ = tx.send(out);
    });
    let started = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if started.elapsed() < PROBE_DEADLINE => {
                std::thread::sleep(std::time::Duration::from_millis(10))
            }
            _ => break None,
        }
    };
    // The group is the probe's alone: no descendant outlives the verdict.
    unsafe { libc::kill(-group, libc::SIGKILL) };
    let _ = child.wait();
    let out = rx
        .recv_timeout(std::time::Duration::from_secs(2))
        .map_err(|_| "version_probe_failed")?;
    let status = status.ok_or("version_probe_timeout")?;
    if !status.success() || out.len() > PROBE_LIMIT {
        return Err("version_probe_failed".into());
    }
    let line = out.strip_suffix(b"\n").unwrap_or(&out);
    if line != expected.as_bytes() {
        return Err("version_mismatch".into());
    }
    Ok(())
}

/// One planned family's verified local contract: public identity only.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contract {
    pub family: String,
    pub cli: crate::provenance::CliIdentity,
    pub auth_shape: String,
}
/// Verify every named family's v2 contract before anything spends: profile,
/// exact supported version, pinned and measured digest, bounded version
/// probe and auth-file shape. The refusal is `<family>: <finite cause>`.
pub fn paid_contracts(
    config: &Path,
    families: &[&str],
    nested: bool,
) -> Result<Vec<Contract>, String> {
    if !config.is_absolute() {
        return Err("selection: wrapper_config_locator_invalid".into());
    }
    let bytes = private_bytes(config).map_err(|e| format!("selection: {e}"))?;
    families
        .iter()
        .map(|family| {
            let s = paid_selection(&bytes, family).map_err(|e| format!("{family}: {e}"))?;
            s.check().map_err(|e| format!("{family}: {e}"))?;
            s.probe().map_err(|e| format!("{family}: {e}"))?;
            let sha256 = s.identity(nested).map_err(|e| format!("{family}: {e}"))?;
            if let (true, Some(n)) = (nested, &s.nested) {
                n.release().map_err(|e| format!("{family}: {e}"))?;
                n.check(&n.driver).map_err(|e| format!("{family}: {e}"))?;
            }
            Ok(Contract {
                family: family.to_string(),
                cli: crate::provenance::CliIdentity {
                    profile: if nested { s.profile.via() } else { s.profile },
                    version: s.cli_version,
                    sha256,
                },
                auth_shape: s.auth_shape,
            })
        })
        .collect()
}

/// The identity a coordinator binds a family's launch to (`--paid-launch
/// SHA256:...`), read from the selection without probing anything: the
/// shape is checked, the bytes are not. Admission is [`paid_contracts`].
pub fn contract_identity(config: &Path, family: &str, nested: bool) -> Result<String, String> {
    paid_selection(&private_bytes(config)?, family)?.identity(nested)
}

/// The credential a declaration spends: one login per Claude or Codex
/// family, one attested key per dsh route.
pub fn credential_key(family: &str, model: &str) -> String {
    match family {
        "dsh" => format!("dsh:{}", model.split_once('/').map_or(model, |(p, _)| p)),
        other => other.to_string(),
    }
}
fn claude_validated(auth: &ClaudeAuth) -> Result<(), String> {
    nonempty(&auth.oauth.access_token)?;
    if auth.oauth.expires_at > 9_007_199_254_740_991 || auth.oauth.scopes != ["user:inference"] {
        return Err("private_credential_invalid".into());
    }
    Ok(())
}
fn codex_validated(auth: &CodexAuth) -> Result<(), String> {
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
    Ok(())
}
/// The NumericDate `exp` of a bounded base64url JWT payload: an integer, or
/// the expiry is unknown. The decoded claims never leave this function.
fn jwt_expiry(token: &str) -> Result<u64, String> {
    use base64::Engine as _;
    let unknown = || "credential_expiry_unknown".to_string();
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 || parts[1].len() > 8192 {
        return Err(unknown());
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1].trim_end_matches('='))
        .map_err(|_| unknown())?;
    let claims: Value = serde_json::from_slice(&payload).map_err(|_| unknown())?;
    claims
        .get("exp")
        .and_then(Value::as_u64)
        .filter(|exp| *exp <= i64::MAX as u64)
        .ok_or_else(unknown)
}
/// A Codex login lasts until its access token's `exp`. The id token is
/// identity evidence the CLI does not need to be current; a short-lived id
/// token must not keep the login inside the freshness margin forever.
fn codex_expiry(auth: &CodexAuth) -> Result<u64, String> {
    jwt_expiry(&auth.tokens.access_token)
}
/// A paid launch's credential must be unexpired and, when the coordinator
/// admitted its chunk with a margin, still valid through that instant.
fn still_valid(until: u64, through: u64) -> Result<(), String> {
    if until <= now_unix() {
        Err("private_credential_expired".into())
    } else if until < through {
        Err("private_credential_margin".into())
    } else {
        Ok(())
    }
}
/// The run binding a paid-baseline trial hands its wrapper in argv
/// (`--paid-launch SHA256:VALID_THROUGH`, never the environment, which stays
/// the accepted agent inputs of 0075 §1): the manifest's CLI digest and the
/// validity the chunk was admitted with. Anything malformed refuses.
fn paid_binding(launch: Option<&str>) -> Result<Option<(String, u64)>, String> {
    let Some(launch) = launch else {
        return Ok(None);
    };
    match launch.split_once(':') {
        Some((sha, through)) if crate::provenance::is_hash(sha) => through
            .parse()
            .map(|t| Some((sha.to_string(), t)))
            .map_err(|_| "wrapper_paid_binding_invalid".into()),
        _ => Err("wrapper_paid_binding_invalid".into()),
    }
}
/// Tell the runner, on the descriptor it handed down, whether the pinned CLI
/// is about to run (and, nested, what the session reported). The descriptor
/// is closed on exec first, so the CLI never inherits it and a model can
/// never write the report.
fn report(reporting: bool, word: &[u8]) {
    if !reporting {
        return;
    }
    let fd = crate::runner::LAUNCH_FD;
    unsafe {
        if libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) == 0 {
            let _ = libc::write(fd, word.as_ptr().cast(), word.len());
        }
    }
}
/// Mark the launch pipe close-on-exec before a child is spawned.
fn seal_launch_fd(reporting: bool) {
    if reporting {
        unsafe { libc::fcntl(crate::runner::LAUNCH_FD, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
}
fn dsh_route(auth: DshAttested, provider: &str) -> AttestedKey {
    if provider == "dashscope" {
        auth.dashscope
    } else {
        auth.spark
    }
}
/// When the credential behind `key` stops being valid, in Unix seconds, read
/// from the v2 selection's credential with its contract's shape: Claude's
/// `expiresAt` (milliseconds), the Codex access token's `exp` (its
/// `last_refresh` must parse; copy time is no validity evidence), or the dsh
/// route's attested `valid_through`. Malformed or absent evidence refuses.
pub fn valid_until(config: &Path, key: &str) -> Result<u64, String> {
    let (family, route) = key.split_once(':').unwrap_or((key, ""));
    let bytes = private_bytes(config)?;
    let selected = paid_selection(&bytes, family)?;
    match family {
        "claude" => {
            let auth: ClaudeAuth = private_read(&selected.credential_file)?;
            claude_validated(&auth)?;
            Ok(auth.oauth.expires_at / 1000)
        }
        "codex" => {
            let auth: CodexAuth = private_read(&selected.credential_file)?;
            codex_validated(&auth)?;
            codex_expiry(&auth)
        }
        _ => {
            let auth: DshAttested = private_read(&selected.credential_file)?;
            nonempty(&auth.dashscope.key)?;
            nonempty(&auth.spark.key)?;
            if !matches!(route, "dashscope" | "spark-glm") {
                return Err("private_credential_invalid".into());
            }
            Ok(dsh_route(auth, route).valid_through)
        }
    }
}
fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn validate_v1(manifest: &Manifest) -> Result<(), String> {
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
    Ok(())
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

/// Launch the family CLI, or say why not. Returning at all means the CLI
/// never ran: a paid coordinator learns that on the launch descriptor and
/// keeps the slot unscored (proposed 0137).
pub fn run(
    harness: &str,
    model: &str,
    config: &Path,
    via_brokkr: Option<&Path>,
    paid_launch: Option<&str>,
    launch_report: bool,
) -> Result<(), String> {
    // A nested wrapper reports on the pipe with or without a paid binding;
    // a direct one only under a paid binding, as before.
    let reporting = paid_launch.is_some() || (launch_report && via_brokkr.is_some());
    match launch(harness, model, config, via_brokkr, paid_launch, reporting) {
        // A nested session that began was reported `ready` and given its
        // receipt: never `unstarted` after work.
        Ok(Launched::Nested(end)) => end,
        Err(e) => {
            report(reporting, b"unstarted\n");
            Err(e)
        }
    }
}
/// How a launch that returned ended: only the nested route returns after
/// work began (a direct launch replaces this process).
enum Launched {
    Nested(Result<(), String>),
}
fn launch(
    harness: &str,
    model: &str,
    config: &Path,
    via_brokkr: Option<&Path>,
    paid_launch: Option<&str>,
    reporting: bool,
) -> Result<Launched, String> {
    validate_model(harness, model)?;
    if !config.is_absolute() {
        return Err("wrapper_config_locator_invalid".into());
    }
    if let Some(driver) = via_brokkr {
        // The path's shape first, before any environment or private read.
        validate_driver(driver)?;
    }
    let inputs = Inputs::environment()?;
    validate_goal(harness, &inputs.goal)?;
    let selected = selection(config, harness).map_err(|e| match e.as_str() {
        "contract_missing" => "wrapper_contract_missing".to_string(),
        _ => e,
    })?;
    // An interpreted pin runs its native runtime; the entry file it is given
    // need not be executable itself.
    let (program, entry) = match &selected.paid {
        Some(paid) => {
            let (program, entry) = paid.program();
            (program.to_path_buf(), entry)
        }
        None => (selected.executable.clone(), None),
    };
    let m = std::fs::metadata(&program).map_err(|_| "wrapper_executable_unavailable")?;
    if !m.is_file() || m.mode() & 0o111 == 0 {
        return Err("wrapper_executable_invalid".into());
    }
    // A paid contract binds the bytes: recheck the pin at launch, so a CLI
    // replaced since preflight never runs under the recorded identity. A
    // coordinator's run binding wins over the mutable selection: a selection
    // re-pinned mid-run refuses rather than running the new binary.
    let binding = paid_binding(paid_launch)?;
    let through = binding.as_ref().map_or(0, |(_, t)| *t);
    match (&selected.paid, &binding) {
        (Some(paid), Some((sha, _))) if *sha != paid.identity(via_brokkr.is_some())? => {
            return Err("wrapper_contract_drift".into());
        }
        (Some(paid), _) => paid.check().map_err(|e| format!("wrapper_cli_{e}"))?,
        (None, Some(_)) => return Err("wrapper_contract_missing".into()),
        (None, None) => {}
    }
    // Route (b): the selection pins the driver beside the CLI, and the
    // driver must be the published release on every route (the panel's
    // hold 3 on a73ac17a): a host grant never stands in for the pin.
    let nested = match via_brokkr {
        Some(driver) => {
            let pin = selected
                .paid
                .as_ref()
                .and_then(|p| p.nested.as_ref())
                .ok_or("driver_pin_missing")?;
            pin.release()?;
            Some((driver, pin))
        }
        None => None,
    };
    // Fail closed (the operator's 2026-11-24 ruling, holds 1 and 2 on
    // a73ac17a): a paid launch, direct or nested, runs only on the live
    // coordinator's authenticated grant (the host baseline) or inside the
    // recipe seat's verified read-only binds, proven in this process's own
    // mount table, before any credential is read.
    if let Some(paid) = &selected.paid {
        match &binding {
            Some((sha, t)) => crate::grant::request(&crate::grant::want(sha, *t))?,
            // The recipe route binds claude's and codex's auth read-only,
            // which no session has yet been shown to run under (the 2026-11-23
            // smoke needed them writable): refused until the operator rules.
            None if matches!(harness, "claude" | "codex") => {
                return Err("paid_unbound: recipe_auth_unruled".into());
            }
            None => {
                let mut sources = vec![paid.program().0, &selected.credential_file, config];
                if let Some((driver, _)) = nested {
                    sources.push(driver);
                }
                if let Some(pin) = &paid.interpreted {
                    sources.push(&pin.package_root);
                }
                // Every declared read-only source under the seat's HOME, the
                // profiles overlay and the selected paths (guarantees 1 on
                // 6074b83e): the whole inventory, not only what this
                // selection reads.
                let home = crate::binds::source_home(config)
                    .ok_or("paid_unbound: recipe_bind_home_unknown")?;
                crate::binds::verify_live(&home, &sources)
                    .map_err(|e| format!("paid_unbound: {e}"))?;
            }
        }
        seam_build_refuses_real_spend(
            &selected.credential_file,
            &credential_key(harness, model),
            SEAM_BUILD,
        )?;
    }
    if let Some((driver, pin)) = nested {
        pin.check(driver)?;
    }
    let paid = selected.paid.is_some();
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
    let (server, tools) = mcp_surface(
        std::env::var(ENV_MCP_SERVER).ok().as_deref(),
        std::env::var(ENV_MCP_TOOLS).ok().as_deref(),
    )?;
    refuse_dsh_competitor(harness, &server)?;
    let mut cmd = match nested {
        Some((driver, _)) => {
            let mut c = Command::new(driver);
            c.args(["driver", harness, "--"]);
            c
        }
        None => {
            let mut c = Command::new(&program);
            c.args(entry.clone());
            c
        }
    };
    cmd.env_clear()
        .envs(&map)
        .current_dir(&cwd)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // The driver finds the CLI by name on PATH: the pinned program's own
    // directory comes first, ahead of the fixed system path.
    if nested.is_some() {
        let dir = program.parent().ok_or("driver_cli_unresolvable")?;
        let mut path = OsString::from(dir.as_os_str());
        path.push(":/usr/bin:/bin");
        cmd.env("PATH", path);
    }
    match harness {
        "claude" => {
            let auth: ClaudeAuth = private_read(&selected.credential_file)?;
            claude_validated(&auth)?;
            if paid {
                still_valid(auth.oauth.expires_at / 1000, through)?;
            }
            let dir = make_dir(&home_fd, ".claude")?;
            write_private(
                &dir,
                ".credentials.json",
                &json_bytes(
                    json!({"claudeAiOauth":{"accessToken":auth.oauth.access_token,"expiresAt":auth.oauth.expires_at,"scopes":auth.oauth.scopes}}),
                ),
            )?;
            let servers = json!({"mcpServers":{
                server.as_str():{"type":"http","url":inputs.mcp.as_str(),"headers":headers},
                "fixture":{"type":"stdio","command":executable,"args":["fixture-http-isolated"],"env":env}
            }});
            let allowed = claude_allowed(&server, &tools);
            if let Some((_, pin)) = nested {
                // The driver owns the argv: the MCP servers are the user
                // scope's, the tools the settings' allowance.
                write_private(&home_fd, ".claude.json", &json_bytes(servers))?;
                write_private(
                    &dir,
                    "settings.json",
                    &json_bytes(
                        json!({"permissions":{"allow":allowed.split(',').collect::<Vec<_>>()}}),
                    ),
                )?;
                cmd.env("BROKKR_CLAUDE_BIN", &program)
                    .env("DISABLE_AUTOUPDATER", "1")
                    .args([
                        "--permission-mode",
                        "acceptEdits",
                        "--model",
                        model,
                        "--effort",
                    ])
                    .arg(pin.effort.as_deref().expect("validated effort"));
                return nested_session(harness, cmd, &inputs, &cwd, reporting)
                    .map(Launched::Nested);
            }
            write_private(&home_fd, "mcp.json", &json_bytes(servers))?;
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
            codex_validated(&auth)?;
            if paid {
                still_valid(codex_expiry(&auth)?, through)?;
            }
            let dir = make_dir(&home_fd, ".codex")?;
            write_private(
                &dir,
                "auth.json",
                &json_bytes(
                    json!({"auth_mode":"chatgpt","OPENAI_API_KEY":null,"tokens":{"id_token":auth.tokens.id_token,"access_token":auth.tokens.access_token,"account_id":auth.tokens.account_id,"refresh_token":"unavailable"},"last_refresh":chrono::Utc::now().to_rfc3339()}),
                ),
            )?;
            cmd.env("CODEX_HOME", home.join(".codex"));
            if let Some((_, pin)) = nested {
                if program.file_name() != Some(std::ffi::OsStr::new("codex")) {
                    return Err("driver_cli_unresolvable".into());
                }
                let mut overrides = vec![
                    ("web_search".to_string(), json!("disabled")),
                    (
                        format!("projects.{}.trust_level", toml(&json!(cwd))),
                        json!("untrusted"),
                    ),
                ];
                overrides.extend(codex_server_overrides(
                    &server,
                    inputs.mcp.as_str(),
                    headers,
                ));
                overrides.extend(fixture_overrides(&executable, &env, &cwd));
                let config = overrides
                    .iter()
                    .map(|(k, v)| format!("{k} = {}\n", toml(v)))
                    .collect::<String>();
                write_private(&dir, "config.toml", config.as_bytes())?;
                let sandbox = pin.codex_sandbox.expect("validated sandbox");
                cmd.args(["--model", model, "--effort"])
                    .arg(pin.effort.as_deref().expect("validated effort"))
                    .args(["--sandbox", sandbox.name()]);
                return nested_session(harness, cmd, &inputs, &cwd, reporting)
                    .map(Launched::Nested);
            }
            cmd.args(["exec", "--json", "--model", model, "-C"])
                .arg(&cwd)
                .args([
                    "--skip-git-repo-check",
                    "--ephemeral",
                    "--ignore-user-config",
                    "--ignore-rules",
                    "--sandbox",
                    "read-only",
                ]);
            let mut overrides = vec![
                ("web_search".to_string(), json!("disabled")),
                (
                    format!("projects.{}.trust_level", toml(&json!(cwd))),
                    json!("untrusted"),
                ),
            ];
            overrides.extend(codex_server_overrides(
                &server,
                inputs.mcp.as_str(),
                headers,
            ));
            overrides.extend(fixture_overrides(&executable, &env, &cwd));
            for (k, v) in overrides {
                cmd.arg("-c").arg(format!("{k}={}", toml(&v)));
            }
            cmd.arg("-");
        }
        _ => {
            let (provider, model_id) = model.split_once('/').expect("validated model");
            let auth = if paid {
                let attested: DshAttested = private_read(&selected.credential_file)?;
                nonempty(&attested.dashscope.key)?;
                nonempty(&attested.spark.key)?;
                let (dashscope, spark) = (attested.dashscope, attested.spark);
                let route_through = if provider == "dashscope" {
                    dashscope.valid_through
                } else {
                    spark.valid_through
                };
                still_valid(route_through, through)?;
                DshAuth {
                    dashscope: dashscope.key,
                    spark: spark.key,
                }
            } else {
                let auth: DshAuth = private_read(&selected.credential_file)?;
                nonempty(&auth.dashscope)?;
                nonempty(&auth.spark)?;
                auth
            };
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
            let dsh_home = make_dir(&home_fd, ".dsh")?;
            if nested.is_some() {
                // The driver composes the model and session rows; the route,
                // the MCP servers and compaction are the home profile's
                // overlay. Only the selected route's key reaches the child.
                let mut rows = vec![
                    json!({"id":"llm-pi-ai","config":{"providers":{provider:route}}}),
                    json!({"insert":[
                        {"id":"mcp-swarmail","name":"@deepseek-ai/dsh-mcp-client","config":{"serverName":"swarmail","transport":"streamable-http","url":inputs.mcp.as_str(),"headers":headers,"failOnStartupError":true,"toolCallTimeoutMs":60000}},
                        {"id":"mcp-fixture","name":"@deepseek-ai/dsh-mcp-client","config":{"serverName":"fixture","transport":"stdio","command":executable,"args":["fixture-http-isolated"],"env":env,"cwd":cwd,"failOnStartupError":true}}
                    ]}),
                ];
                if provider == "spark-glm" {
                    rows.push(json!({"id":"compaction-basic","config":{"modelPolicies":[{"provider":provider,"model":model_id,"thresholdRatio":0.8,"retainRatio":0.16}]}}));
                }
                let overlay = rows
                    .iter()
                    .map(|r| format!("- {}\n", yaml_flow(r)))
                    .collect::<String>();
                let profiles = make_dir(&dsh_home, "profiles")?;
                let headless = make_dir(&profiles, "headless")?;
                write_private(&headless, "cordis.patch.yml", overlay.as_bytes())?;
                let interpreted = selected
                    .paid
                    .as_ref()
                    .and_then(|p| p.interpreted.as_ref())
                    .expect("a paid dsh selection carries its interpreted pin");
                direct_entry(interpreted)?;
                cmd.env("BROKKR_DSH_BIN", interpreted.entry_path())
                    .env("DSH_HOME", home.join(".dsh"))
                    .env("DSH_TELEMETRY_DISABLED", "1")
                    .env(key, token)
                    .args(["--model", model]);
                return nested_session(harness, cmd, &inputs, &cwd, reporting)
                    .map(Launched::Nested);
            }
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
    report(paid_launch.is_some(), crate::runner::LAUNCH_READY);
    let _error = cmd.exec();
    Err("wrapper_exec_failed".into())
}

/// dsh's driver runs `BROKKR_DSH_BIN` as one program, so the pinned entry
/// must start the pinned runtime itself: an executable entry whose `#!` line
/// names the runtime's absolute path, or `/usr/bin/env node` with the
/// runtime's directory first on PATH (set above). No wrapper script of ours
/// stands between the runtime and the entry.
fn direct_entry(pin: &crate::interpreted::Pin) -> Result<(), String> {
    let entry = pin.entry_path();
    let m = std::fs::symlink_metadata(&entry).map_err(|_| "driver_entry_unavailable")?;
    if !m.file_type().is_file() || m.mode() & 0o111 == 0 {
        return Err("driver_entry_not_executable".into());
    }
    let mut head = [0u8; 256];
    let n = File::open(&entry)
        .and_then(|mut f| f.read(&mut head))
        .map_err(|_| "driver_entry_unavailable")?;
    let line = head[..n].split(|b| *b == b'\n').next().unwrap_or_default();
    let runtime = pin.runtime.as_os_str().as_bytes();
    let by_env = pin.runtime.file_name() == Some(std::ffi::OsStr::new("node"))
        && line == b"#!/usr/bin/env node";
    if line.strip_prefix(b"#!") == Some(runtime) || by_env {
        Ok(())
    } else {
        Err("driver_entry_shebang_unsupported".into())
    }
}

/// The fixed seat charter a nested session reads at `role_path`: the task
/// itself travels as the start's `feature`.
const NESTED_CHARTER: &str = "# Eval trial seat\n\nYou are the agent of one Swarmail evaluation trial. The Feature below is your task: complete it with the MCP tools you have, then write your result file. Write `done` when you finished the task and `stuck` when you could not.\n";
/// The results a nested trial's seat may write.
pub const NESTED_RESULTS: [&str; 2] = ["done", "stuck"];

/// Run one nested session: the charter and result target in a fresh seat
/// directory under the work directory, the driver spawned with its stdio
/// piped, the exchange to its end, and the receipt on the launch pipe.
/// `Err` only when nothing was launched; a launched session's own end is the
/// inner result.
fn nested_session(
    family: &str,
    mut cmd: Command,
    inputs: &Inputs,
    cwd: &Path,
    reporting: bool,
) -> Result<Result<(), String>, String> {
    use std::io::Write as _;
    let attempt_id = crate::runner::new_trial_id();
    let seat_name = format!(".swarmail-seat-{attempt_id}");
    let seat = make_dir(&directory(cwd)?, &seat_name)?;
    write_private(&seat, "charter.md", NESTED_CHARTER.as_bytes())?;
    let seat_dir = cwd.join(&seat_name);
    let text = |p: PathBuf| p.to_str().expect("a validated UTF-8 workdir").to_string();
    let start = crate::driver::StartInput {
        feature: inputs.goal.clone(),
        phase: "trial".into(),
        seat: "trial".into(),
        role_path: text(seat_dir.join("charter.md")),
        workdir: text(cwd.to_path_buf()),
        result_path: text(seat_dir.join("result.json")),
        allowed_results: NESTED_RESULTS.map(String::from).to_vec(),
        context: json!({"task": inputs.task}),
    };
    let id = crate::driver::Identity {
        effect_id: format!("trial-{attempt_id}"),
        attempt_id: attempt_id.clone(),
        seat: "trial".into(),
    };
    // The launch pipe stays the wrapper's: neither the driver nor anything
    // it starts inherits it.
    seal_launch_fd(reporting);
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "driver_spawn_failed")?;
    let mut stdin = child.stdin.take();
    let stdout = child.stdout.take().expect("piped stdout");
    let reaper = Reaper::new(child.id());
    let (exchange, end) = crate::driver::run(
        std::io::BufReader::new(stdout),
        id,
        &start,
        |bytes, last| {
            let sent = match stdin.as_mut() {
                Some(s) => s.write_all(&bytes).and_then(|()| s.flush()),
                None => Err(std::io::ErrorKind::BrokenPipe.into()),
            };
            if last {
                // Shutdown, then the driver's stdin closes, and it has a
                // bounded grace to exit and release its stdout.
                stdin = None;
                reaper.arm();
            }
            sent
        },
        || report(reporting, crate::runner::LAUNCH_READY),
    );
    // A protocol error ends the exchange without a shutdown: close stdin,
    // then the same bounded grace before the driver is killed. The typed
    // ending above is already decided either way.
    drop(stdin);
    reaper.arm();
    reaper.reap(child);
    let (launched, code) = match end {
        Ok(d) => (d.launched(), d.code()),
        Err(e) => (exchange.accepted(), e.code()),
    };
    if !launched {
        return Err(code.into());
    }
    let receipt = crate::usage::Receipt {
        disposition: code.into(),
        attempt_id,
        family: family.into(),
        facts: crate::usage::project(family, exchange.checkpoints()),
    };
    report(reporting, &crate::usage::receipt_line(&receipt));
    Ok(if code == "completed" {
        Ok(())
    } else {
        Err(code.into())
    })
}

/// How long a driver has, after its stdin closes, to exit before it is
/// killed: a driver that ignores `shutdown` or holds its stdout open must
/// not keep the wrapper (and the typed receipt) until the trial deadline.
pub const DRIVER_EXIT_GRACE: std::time::Duration = std::time::Duration::from_secs(3);
/// Test equipment, debug builds alone: a shorter grace for the offline
/// proofs (milliseconds, never longer than [`DRIVER_EXIT_GRACE`]). A release
/// build always waits the full grace.
pub const EXIT_GRACE_TEST_ENV: &str = "EVAL_TEST_DRIVER_EXIT_GRACE_MS";
fn exit_grace() -> std::time::Duration {
    #[cfg(debug_assertions)]
    if let Some(ms) = std::env::var(EXIT_GRACE_TEST_ENV)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
    {
        return std::time::Duration::from_millis(ms).min(DRIVER_EXIT_GRACE);
    }
    DRIVER_EXIT_GRACE
}

/// The bounded end of a nested driver. `arm` starts one watchdog that kills
/// the driver after [`DRIVER_EXIT_GRACE`] unless it was reaped first; the
/// reaper waits for exit without reaping (`WNOWAIT`), marks it under the
/// same lock, and only then reaps, so the kill can never reach a reused pid.
struct Reaper {
    pid: u32,
    reaped: std::sync::Arc<std::sync::Mutex<bool>>,
    armed: std::cell::Cell<bool>,
}
impl Reaper {
    fn new(pid: u32) -> Self {
        Self {
            pid,
            reaped: std::sync::Arc::default(),
            armed: std::cell::Cell::new(false),
        }
    }
    fn arm(&self) {
        if self.armed.replace(true) {
            return;
        }
        let (pid, reaped, grace) = (self.pid, self.reaped.clone(), exit_grace());
        std::thread::spawn(move || {
            std::thread::sleep(grace);
            let reaped = reaped.lock().unwrap_or_else(|e| e.into_inner());
            if !*reaped {
                unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
            }
        });
    }
    fn reap(&self, mut child: std::process::Child) {
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        while unsafe {
            libc::waitid(
                libc::P_PID,
                self.pid as libc::id_t,
                &mut info,
                libc::WEXITED | libc::WNOWAIT,
            )
        } != 0
            && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted
        {}
        *self.reaped.lock().unwrap_or_else(|e| e.into_inner()) = true;
        let _ = child.wait();
    }
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
    fn the_default_surface_is_swarmails_and_byte_identical_to_main() {
        let (server, tools) = mcp_surface(None, None).unwrap();
        assert_eq!(server, "swarmail");
        assert_eq!(tools.len(), SWARMAIL_TOOLS.len());
        // The expression the allowlist was built from before the surface
        // became selectable.
        let before = std::iter::once("mcp__fixture__eval_http".to_string())
            .chain(SWARMAIL_TOOLS.iter().map(|n| format!("mcp__swarmail__{n}")))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(claude_allowed(&server, &tools), before);
        let overrides = codex_server_overrides("swarmail", "http://127.0.0.1:1/mcp", json!({}));
        let keys: Vec<_> = overrides.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(
            keys,
            [
                "mcp_servers.swarmail.url",
                "mcp_servers.swarmail.http_headers",
                "mcp_servers.swarmail.startup_timeout_sec",
                "mcp_servers.swarmail.tool_timeout_sec",
                "mcp_servers.swarmail.default_tools_approval_mode",
            ]
        );
    }

    #[test]
    fn a_competitor_surface_is_its_own_server_key_and_announced_tools() {
        let (server, tools) =
            mcp_surface(Some("mail-target"), Some("mailhog_list,mailhog_wait")).unwrap();
        assert_eq!((server.as_str(), tools.len()), ("mail-target", 2));
        assert_eq!(
            claude_allowed(&server, &tools),
            "mcp__fixture__eval_http,mcp__mail-target__mailhog_list,mcp__mail-target__mailhog_wait"
        );
        assert!(!claude_allowed(&server, &tools).contains("swarmail"));
        let overrides = codex_server_overrides(
            &server,
            "http://127.0.0.1:9/mcp",
            json!({"X-Eval-Identity": "a"}),
        );
        assert_eq!(overrides[0].0, "mcp_servers.mail-target.url");
        assert_eq!(overrides[0].1, json!("http://127.0.0.1:9/mcp"));
        assert!(
            overrides
                .iter()
                .all(|(k, _)| k.starts_with("mcp_servers.mail-target."))
        );
    }

    #[test]
    fn the_dsh_wrapper_refuses_a_competitor_surface_by_name() {
        assert_eq!(
            refuse_dsh_competitor("dsh", "mail-target"),
            Err("dsh_competitor_unsupported".into())
        );
        assert_eq!(refuse_dsh_competitor("dsh", "swarmail"), Ok(()));
        assert_eq!(refuse_dsh_competitor("claude", "mail-target"), Ok(()));
        assert_eq!(refuse_dsh_competitor("codex", "mail-target"), Ok(()));
    }

    #[test]
    fn a_surface_is_both_inputs_or_neither_and_every_name_is_validated() {
        let refused = |s: Option<&str>, t: Option<&str>| mcp_surface(s, t).unwrap_err();
        assert_eq!(refused(Some("mail"), None), "eval_mcp_surface_incomplete");
        assert_eq!(
            refused(None, Some("mailhog_list")),
            "eval_mcp_surface_incomplete"
        );
        for server in ["", "Mail", "a_b", "fixture", "a b", &"a".repeat(33)] {
            assert_eq!(
                refused(Some(server), Some("t")),
                "eval_mcp_server_invalid",
                "{server}"
            );
        }
        let many = (0..65)
            .map(|i| format!("t{i}"))
            .collect::<Vec<_>>()
            .join(",");
        for tools in ["", "a,,b", "a,a", "A", "a-b", "a;b", "a b", &many] {
            assert_eq!(
                refused(Some("m"), Some(tools)),
                "eval_mcp_tools_invalid",
                "{tools}"
            );
        }
    }

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
    fn the_driver_path_is_checked_before_every_read() {
        let root = crate::runner::private_tempdir("wrapper-proof-").unwrap();
        let plain = root.path().join("not-executable");
        std::fs::write(&plain, b"#!/bin/sh\n").unwrap();
        let exe = root.path().join("driver");
        std::fs::write(&exe, b"#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o700)).unwrap();
        let config = Path::new("/nonexistent/selection.json");
        let run_with = |driver: &Path| {
            run(
                "claude",
                "claude-sonnet-5-5",
                config,
                Some(driver),
                None,
                true,
            )
        };
        assert_eq!(run_with(Path::new("rel")), Err("driver_invalid".into()));
        assert_eq!(run_with(&plain), Err("driver_invalid".into()));
        // A well-shaped driver goes on to the inputs, which this process
        // lacks: the same refusal as without the flag.
        let direct = run("claude", "claude-sonnet-5-5", config, None, None, false).unwrap_err();
        assert_eq!(run_with(&exe).unwrap_err(), direct);
        assert_ne!(direct, "driver_invalid");
    }
    fn nested(family: &str) -> NestedSelection {
        NestedSelection {
            driver: "/opt/brokkr/brokkr".into(),
            driver_sha256: DRIVER_RELEASE_SHA256.into(),
            driver_version: DRIVER_VERSION.into(),
            platform: DRIVER_PLATFORM.into(),
            archive_sha256: DRIVER_ARCHIVE_SHA256.into(),
            effort: (family != "dsh").then(|| "medium".into()),
            codex_sandbox: (family == "codex").then_some(CodexSandbox::WorkspaceWrite),
        }
    }
    #[test]
    fn a_nested_pin_names_the_release_and_each_familys_typed_flags() {
        for family in ["claude", "codex", "dsh"] {
            assert_eq!(nested(family).validate(family), Ok(()), "{family}");
        }
        type Edit = fn(&mut NestedSelection);
        let mut read_only = nested("codex");
        read_only.codex_sandbox = serde_json::from_value(json!("read-only")).unwrap();
        assert_eq!(read_only.codex_sandbox, Some(CodexSandbox::ReadOnly));
        assert_eq!(read_only.validate("codex"), Ok(()));
        let cases: [(&str, Edit, &str); 11] = [
            (
                "claude",
                |n| n.driver = "brokkr".into(),
                "driver_selection_invalid",
            ),
            (
                "claude",
                |n| n.driver_version = "0.12.0".into(),
                "driver_version_unsupported",
            ),
            (
                "claude",
                |n| n.platform = "linux-aarch64".into(),
                "driver_platform_unsupported",
            ),
            (
                "claude",
                |n| n.archive_sha256 = "0".repeat(64),
                "driver_archive_unrecognized",
            ),
            (
                "claude",
                |n| n.driver_sha256 = "x".into(),
                "driver_digest_invalid",
            ),
            ("claude", |n| n.effort = None, "driver_effort_invalid"),
            (
                "codex",
                |n| n.effort = Some("turbo".into()),
                "driver_effort_invalid",
            ),
            (
                "dsh",
                |n| n.effort = Some("high".into()),
                "driver_effort_invalid",
            ),
            (
                "codex",
                |n| n.codex_sandbox = None,
                "driver_sandbox_invalid",
            ),
            (
                "claude",
                |n| n.codex_sandbox = Some(CodexSandbox::ReadOnly),
                "driver_sandbox_invalid",
            ),
            (
                "codex",
                |n| n.codex_sandbox = Some(CodexSandbox::DangerFullAccess),
                "driver_sandbox_unconfined",
            ),
        ];
        for (family, edit, want) in cases {
            let mut n = nested(family);
            edit(&mut n);
            assert_eq!(n.validate(family), Err(want.into()), "{family} {want}");
        }
        assert_eq!(
            [
                CodexSandbox::ReadOnly,
                CodexSandbox::WorkspaceWrite,
                CodexSandbox::DangerFullAccess
            ]
            .map(CodexSandbox::name),
            ["read-only", "workspace-write", "danger-full-access"]
        );
        // Production admits the release executable's digest alone.
        assert_eq!(nested("claude").release(), Ok(()));
        let mut other = nested("claude");
        other.driver_sha256 = "a".repeat(64);
        assert_eq!(other.release(), Err("driver_release_unrecognized".into()));
        // No environment value widens the trusted set (the panel's hold 3 on
        // a73ac17a): the retired test-equipment variable admits nothing.
        // That proof runs with the variable in a child's environment
        // (`tests/wrappers_launch/nested.rs`), never by writing this
        // multi-threaded harness's own.
        // A debug build admits the stand-in built beside it, a release build
        // nothing but the published digest.
        assert!(!standin_driver_admitted(&"a".repeat(64)));
        assert_eq!(SEAM_BUILD, cfg!(debug_assertions));
        // A seam-bearing build spends synthetic equipment alone, whatever
        // the pin; a release build reads nothing here.
        let root = crate::runner::private_tempdir("seam-spend-").unwrap();
        use base64::Engine as _;
        let e = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let fixture_jwt = format!(
            "{}.{}.{}",
            e.encode("{}"),
            e.encode("{}"),
            e.encode("SYNTHETIC-C")
        );
        let real_jwt = format!(
            "{}.{}.{}",
            e.encode("{}"),
            e.encode("{}"),
            e.encode([0x9a, 0x01])
        );
        let codex = |access: &str, api: Value| json!({"auth_mode":"chatgpt","OPENAI_API_KEY":api,"tokens":{"id_token":"SYNTHETIC-ID","access_token":access,"refresh_token":"SYNTHETIC-R","account_id":"SYNTHETIC-A"},"last_refresh":"2026-10-26T00:00:00Z"});
        let dsh = |a: &str, b: &str| json!({"DASHSCOPE_API_KEY":{"key":a,"valid_through":1},"SPARK_API_KEY":{"key":b,"valid_through":1}});
        let cases: Vec<(&str, Value, bool)> = vec![
            (
                "claude",
                json!({"claudeAiOauth":{"accessToken":"SYNTHETIC-X"}}),
                true,
            ),
            (
                "claude",
                json!({"claudeAiOauth":{"accessToken":"sk-ant-oat01-x"}}),
                false,
            ),
            // The decoys: the marker beside, not in, the authorizing value.
            (
                "claude",
                json!({"claudeAiOauth":{"accessToken":"sk-ant-oat01-x","scopes":["SYNTHETIC-S"]}}),
                false,
            ),
            (
                "claude",
                json!({"claudeAiOauth":{"accessToken":"x SYNTHETIC-X"}}),
                false,
            ),
            ("claude", json!({"claudeAiOauth":{"accessToken":7}}), false),
            ("codex", codex("SYNTHETIC-C", Value::Null), true),
            ("codex", codex(&fixture_jwt, Value::Null), true),
            ("codex", codex(&real_jwt, Value::Null), false),
            ("codex", codex("a.b!.c", Value::Null), false),
            ("codex", codex("SYNTHETIC-C", json!("sk-real")), false),
            ("dsh:dashscope", dsh("SYNTHETIC-D", "sk-real"), true),
            ("dsh:spark-glm", dsh("SYNTHETIC-D", "sk-real"), false),
            ("dsh:spark-glm", dsh("sk-real", "SYNTHETIC-S"), true),
            ("dsh:dashscope", dsh("sk-real", "SYNTHETIC-S"), false),
            // No authorizing field at all, or a family with none.
            ("claude", json!({"note":"SYNTHETIC-X"}), false),
            (
                "other",
                json!({"claudeAiOauth":{"accessToken":"SYNTHETIC-X"}}),
                false,
            ),
        ];
        let path = root.path().join("credential.json");
        for (family, body, admitted) in &cases {
            std::fs::write(&path, body.to_string()).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
            let want = if *admitted {
                Ok(())
            } else {
                Err("paid_seam_build_refused".into())
            };
            assert_eq!(
                seam_build_refuses_real_spend(&path, family, true),
                want,
                "{family} {body}"
            );
            assert_eq!(seam_build_refuses_real_spend(&path, family, false), Ok(()));
        }
        std::fs::write(&path, b"not json").unwrap();
        assert_eq!(
            seam_build_refuses_real_spend(&path, "claude", true),
            Err("paid_seam_build_refused".into())
        );
        assert_eq!(
            seam_build_refuses_real_spend(&root.path().join("absent"), "claude", true),
            Err("private_path_unavailable".into())
        );
        // The launch check: the flag must name the pinned path, then the
        // bytes must hash to the pin, before any banner runs.
        let n = nested("claude");
        assert_eq!(
            n.check(Path::new("/elsewhere/brokkr")),
            Err("driver_selection_mismatch".into())
        );
        if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            assert_eq!(n.check(&n.driver), Err("driver_executable_unsafe".into()));
        } else {
            assert_eq!(
                n.check(&n.driver),
                Err("driver_platform_unsupported".into())
            );
        }
    }
    #[test]
    fn the_paid_decision_reads_the_selection_the_argv_names() {
        let root = crate::runner::private_tempdir("argv-paid-").unwrap();
        let write = |name: &str, body: serde_json::Value| {
            let path = root.path().join(name);
            std::fs::write(&path, body.to_string()).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
            path
        };
        let family = json!({"executable": "/bin/true", "credential_file": "/nonexistent"});
        let v1 = write(
            "v1.json",
            json!({"version": 1, "families": {"claude": family, "codex": family, "dsh": family}}),
        );
        let v2 = write("v2.json", json!({"version": 2, "families": {}}));
        // A valid paid selection is detected as one, not merely refused as
        // unreadable (panel low 6 on 6074b83e).
        let valid = write(
            "valid.json",
            json!({"version": 2, "families": {"claude": {
                "executable": "/bin/true",
                "credential_file": "/nonexistent",
                "profile": "claude_2_1_329",
                "cli_version": crate::provenance::WrapperProfile::Claude2_1_329.cli_version(),
                "cli_sha256": "0".repeat(64),
                "auth_shape": auth_shape("claude"),
            }}}),
        );
        assert!(selection(&valid, "claude").unwrap().paid.is_some());
        let argv = |words: &[&str]| words.iter().map(|w| w.to_string()).collect::<Vec<_>>();
        assert!(argv_selects_paid(&argv(&[
            "/x",
            "agent-claude",
            "--config",
            valid.to_str().unwrap()
        ])));
        let v1s = v1.to_str().unwrap();
        let v2s = v2.to_str().unwrap();
        // Not a wrapper of this executable: no paid decision to make.
        assert!(!argv_selects_paid(&argv(&["/bin/agent", "--config", v2s])));
        assert!(!argv_selects_paid(&argv(&[
            "/x",
            "agent-other",
            "--config",
            v2s
        ])));
        // A version-1 selection is the free direct contract, both spellings.
        assert!(!argv_selects_paid(&argv(&[
            "/x",
            "agent-claude",
            "--config",
            v1s
        ])));
        assert!(!argv_selects_paid(&argv(&[
            "/x",
            "agent-codex",
            &format!("--config={v1s}")
        ])));
        // A selection that is not version 1 is paid, and so, failing
        // closed, is one that cannot be read or is not named.
        assert!(argv_selects_paid(&argv(&[
            "/x",
            "agent-claude",
            "--config",
            v2s
        ])));
        assert!(argv_selects_paid(&argv(&[
            "/x",
            "agent-dsh",
            "--config",
            "/absent"
        ])));
        assert!(argv_selects_paid(&argv(&[
            "/x",
            "agent-claude",
            "--config"
        ])));
        assert!(argv_selects_paid(&argv(&["/x", "agent-claude"])));
    }
    #[test]
    fn a_dsh_entry_must_start_the_pinned_runtime_itself() {
        let root = crate::runner::private_tempdir("entry-proof-").unwrap();
        let pkg = root.path().join("pkg");
        std::fs::create_dir(&pkg).unwrap();
        let entry = pkg.join("cli.js");
        let pin = |runtime: &str| crate::interpreted::Pin {
            runtime: PathBuf::from(runtime),
            runtime_version: "v22.0.0".into(),
            runtime_sha256: "0".repeat(64),
            package_root: pkg.clone(),
            entry: "cli.js".into(),
            files: Default::default(),
        };
        let write = |body: &[u8], mode: u32| {
            std::fs::write(&entry, body).unwrap();
            std::fs::set_permissions(&entry, std::fs::Permissions::from_mode(mode)).unwrap();
        };
        assert_eq!(
            direct_entry(&pin("/opt/node/bin/node")),
            Err("driver_entry_unavailable".into())
        );
        write(b"#!/usr/bin/env node\nx", 0o755);
        assert_eq!(direct_entry(&pin("/opt/node/bin/node")), Ok(()));
        // The env form needs the runtime to be the `node` PATH resolves.
        assert_eq!(
            direct_entry(&pin("/opt/node/bin/node22")),
            Err("driver_entry_shebang_unsupported".into())
        );
        write(b"#!/opt/node/bin/node22\n", 0o755);
        assert_eq!(direct_entry(&pin("/opt/node/bin/node22")), Ok(()));
        write(b"#!/bin/sh\nexec node cli.js", 0o755);
        assert_eq!(
            direct_entry(&pin("/opt/node/bin/node")),
            Err("driver_entry_shebang_unsupported".into())
        );
        write(b"#!/usr/bin/env node\n", 0o644);
        assert_eq!(
            direct_entry(&pin("/opt/node/bin/node")),
            Err("driver_entry_not_executable".into())
        );
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

#[cfg(test)]
mod paid_tests {
    use super::*;
    use base64::Engine as _;

    fn token(payload: &str) -> String {
        let e = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        format!("{}.{}.sig", e.encode("{}"), e.encode(payload))
    }

    #[test]
    fn only_a_native_image_can_be_a_pinned_cli() {
        for magic in [
            &b"\x7fELF"[..],
            &[0xfe, 0xed, 0xfa, 0xce],
            &[0xfe, 0xed, 0xfa, 0xcf],
            &[0xce, 0xfa, 0xed, 0xfe],
            &[0xcf, 0xfa, 0xed, 0xfe],
            &[0xca, 0xfe, 0xba, 0xbe],
            &[0xbe, 0xba, 0xfe, 0xca],
        ] {
            assert!(native_image(magic), "{magic:?}");
        }
        for other in [&b"#!/b"[..], b"#!", b"", b"\x7fEL", b"MZ\x90\0"] {
            assert!(!native_image(other), "{other:?}");
        }
    }

    #[test]
    fn codex_validity_is_the_access_token_alone() {
        let auth = |id: String, access: String| CodexAuth {
            auth_mode: "chatgpt".into(),
            api_key: None,
            tokens: Tokens {
                id_token: id,
                access_token: access,
                refresh_token: "r".into(),
                account_id: "a".into(),
            },
            last_refresh: "2026-11-22T00:00:00Z".into(),
        };
        // An id token that expires first never shortens the login.
        let fresh = token(r#"{"exp":4102444800}"#);
        assert_eq!(
            codex_expiry(&auth(token(r#"{"exp":1}"#), fresh.clone())),
            Ok(4102444800)
        );
        assert_eq!(codex_expiry(&auth("opaque".into(), fresh)), Ok(4102444800));
        assert!(codex_expiry(&auth(token(r#"{"exp":1}"#), "opaque".into())).is_err());
    }

    #[test]
    fn access_token_expiry_is_a_bounded_integer_claim_or_unknown() {
        assert_eq!(jwt_expiry(&token(r#"{"exp":1767225600}"#)), Ok(1767225600));
        // Padded payloads decode the same.
        let padded = format!(
            "e30.{}.sig",
            base64::engine::general_purpose::URL_SAFE.encode(r#"{"exp":7}"#)
        );
        assert_eq!(jwt_expiry(&padded), Ok(7));
        for bad in [
            token(r#"{"exp":1.5}"#),
            token(r#"{"exp":-1}"#),
            token(r#"{"exp":"1767225600"}"#),
            token(r#"{"exp":18446744073709551615}"#),
            token(r#"{"iat":1}"#),
            token("not json"),
            "opaque-token".to_string(),
            "a.b".to_string(),
            "a.!!.c".to_string(),
            format!("a.{}.c", "A".repeat(8193)),
        ] {
            assert_eq!(
                jwt_expiry(&bad),
                Err("credential_expiry_unknown".to_string()),
                "{bad}"
            );
        }
    }

    #[test]
    fn each_profile_has_one_exact_banner_and_auth_shape() {
        use crate::provenance::WrapperProfile::*;
        assert_eq!(version_banner(Claude2_1_310), "2.1.310 (Claude Code)");
        assert_eq!(version_banner(Codex0_164_0), "codex-cli 0.164.0");
        assert_eq!(version_banner(Dsh0_1_5Rc1), "0.1.5-rc.1");
        assert_eq!(auth_shape("claude"), "claude_oauth_v1");
        assert_eq!(auth_shape("codex"), "codex_chatgpt_v1");
        assert_eq!(auth_shape("dsh"), "dsh_routes_attested_v1");
        assert_eq!(credential_key("dsh", "dashscope/glm-5.3"), "dsh:dashscope");
        assert_eq!(
            credential_key("dsh", "spark-glm/GLM-5.3-Flash-EXL3"),
            "dsh:spark-glm"
        );
        assert_eq!(credential_key("codex", "gpt-6-luna"), "codex");
    }
}
