//! Versioned public fixture inputs. Private entropy never enters this module.
use chrono::{DateTime, SecondsFormat, Utc};
use ring::digest::{Context, SHA256};
use serde::{Deserialize, Serialize};

pub const ALGORITHM: &str = "fixture-sha256-v1";
pub const RESULT_LIMIT: usize = 64 * 1024;

pub fn sha256(bytes: &[u8]) -> String {
    hex(ring::digest::digest(&SHA256, bytes).as_ref())
}
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn is_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn framed(parts: &[&[u8]]) -> [u8; 32] {
    let mut hash = Context::new(&SHA256);
    for p in parts {
        hash.update(&(p.len() as u64).to_be_bytes());
        hash.update(p);
    }
    hash.finish().as_ref().try_into().expect("SHA-256 length")
}
pub fn seed_bytes(seed: &str) -> Result<[u8; 32], String> {
    if !is_hash(seed) {
        return Err("seed must be 64 lowercase hexadecimal digits".into());
    }
    let mut out = [0; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&seed[i * 2..i * 2 + 2], 16).expect("validated hex");
    }
    Ok(out)
}
pub fn fixture_time(s: &str) -> Result<DateTime<Utc>, String> {
    if s.len() != 20 || !s.ends_with('Z') {
        return Err("fixture time must be whole-second UTC RFC3339".into());
    }
    let t = DateTime::parse_from_rfc3339(s)
        .map_err(|_| "invalid fixture time")?
        .with_timezone(&Utc);
    if t.to_rfc3339_opts(SecondsFormat::Secs, true) != s
        || !("1970-01-01"..="9998-12-01").contains(&&s[..10])
    {
        return Err("fixture time outside supported range".into());
    }
    Ok(t)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureInputs {
    pub seed: String,
    pub fixture_time: String,
    pub seed_algorithm: String,
    pub projection_version: u32,
}
impl Default for FixtureInputs {
    fn default() -> Self {
        Self::resolve(None, None).expect("generated inputs")
    }
}
impl FixtureInputs {
    pub fn resolve(seed: Option<&str>, time: Option<&str>) -> Result<Self, String> {
        use rand::Rng;
        let seed = seed
            .map(str::to_string)
            .unwrap_or_else(|| hex(&rand::rng().random::<[u8; 32]>()));
        let fixture_time = time
            .map(str::to_string)
            .unwrap_or_else(|| Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true));
        let inputs = Self {
            seed,
            fixture_time,
            seed_algorithm: ALGORITHM.into(),
            projection_version: 1,
        };
        inputs.validate()?;
        Ok(inputs)
    }
    pub fn validate(&self) -> Result<(), String> {
        seed_bytes(&self.seed)?;
        fixture_time(&self.fixture_time)?;
        if self.seed_algorithm != ALGORITHM || self.projection_version != 1 {
            return Err("unsupported fixture algorithm/projection".into());
        }
        Ok(())
    }
    pub fn choice(
        &self,
        task: &str,
        slot: u64,
        flow: &str,
        generation: u64,
        field: &str,
    ) -> [u8; 32] {
        framed(&[
            b"swarmail-eval-choice-v1",
            &seed_bytes(&self.seed).expect("validated seed"),
            task.as_bytes(),
            &slot.to_be_bytes(),
            flow.as_bytes(),
            &generation.to_be_bytes(),
            field.as_bytes(),
        ])
    }
    pub fn select(
        &self,
        task: &str,
        slot: u64,
        flow: &str,
        generation: u64,
        field: &str,
        n: u64,
    ) -> u64 {
        u64::from_be_bytes(
            self.choice(task, slot, flow, generation, field)[..8]
                .try_into()
                .expect("eight bytes"),
        ) % n
    }
    pub fn time(&self) -> DateTime<Utc> {
        fixture_time(&self.fixture_time).expect("validated time")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatrixCoordinates {
    pub matrix_id: String,
    pub master_seed: String,
    pub task_index: u64,
    pub agent_index: u64,
    pub repetition_index: u64,
}
impl MatrixCoordinates {
    pub fn cell_seed(&self, task: &str, label: &str) -> Result<String, String> {
        Ok(hex(&framed(&[
            b"swarmail-eval-cell-v1",
            &seed_bytes(&self.master_seed)?,
            &self.task_index.to_be_bytes(),
            task.as_bytes(),
            &self.agent_index.to_be_bytes(),
            label.as_bytes(),
            &self.repetition_index.to_be_bytes(),
        ])))
    }
}
pub fn public_argv(argv: &[String]) -> String {
    let argc = (argv.len() as u64).to_be_bytes();
    let mut parts: Vec<&[u8]> = vec![b"swarmail-eval-public-argv-v1", &argc];
    parts.extend(argv.iter().map(|s| s.as_bytes()));
    hex(&framed(&parts))
}

/// Unlike serde's default Option handling, every nullable field is required.
pub fn nullable<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(d)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    MeasuredBytes,
    BuildGit,
    OwnedBuildContext,
    IdenticalOwnedArtifact,
    DeclaredPublicArgv,
    OperatorDeclared,
    Builtin,
    ResolvedInput,
    ObservedFixture,
    CatalogueDefinition,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    NotResolved,
    NotReadable,
    NotBuilt,
    NoGit,
    BuildContextChanged,
    ExternalSourceUnknown,
    NotDeclared,
    PrivateArgv,
    RequestInvalid,
    Builtin,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Availability<T> {
    Present { value: T, source: Source },
    Unavailable { reason: Reason },
    NotApplicable { reason: Reason },
}
impl<T> Availability<T> {
    pub fn present(value: T, source: Source) -> Self {
        Self::Present { value, source }
    }
    pub fn unavailable(reason: Reason) -> Self {
        Self::Unavailable { reason }
    }
    pub fn check(&self, sources: &[Source], builtin_model: bool) -> Result<(), String> {
        match self {
            Self::Present { source, .. } if sources.contains(source) => Ok(()),
            Self::Unavailable { reason } if *reason != Reason::Builtin => Ok(()),
            Self::NotApplicable {
                reason: Reason::Builtin,
            } if builtin_model => Ok(()),
            _ => Err("invalid availability/source combination".into()),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutableIdentity {
    pub executable_sha256: Availability<String>,
    pub build_commit: Availability<String>,
    pub source_dirty: Availability<bool>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HarnessIdentity {
    pub package: String,
    pub version: String,
    pub executable_sha256: Availability<String>,
    pub build_commit: Availability<String>,
    pub source_dirty: Availability<bool>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetIdentity {
    pub executable_sha256: Availability<String>,
    pub build_commit: Availability<String>,
    pub source_dirty: Availability<bool>,
    pub settings: Availability<ServerSettings>,
    pub chaos: Availability<ChaosObservation>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerSettings {
    pub max_per_inbox: u64,
    pub in_memory: bool,
    pub loopback: bool,
    pub port_zero: bool,
    pub inherited_settings_removed: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChaosObservation {
    pub probability: u8,
    pub refusal_observed: bool,
    pub cleared: bool,
    pub retry_accepted: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskIdentity {
    pub catalogue_schema: u32,
    pub catalogue_sha256: String,
    pub definition_sha256: Availability<String>,
    pub scorer_id: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Builtin,
    External,
    Invalid,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandIdentity {
    pub algorithm: String,
    pub sha256: String,
}
/// Declared recipe, never an observed tool inventory or binary attestation.
///
/// The approved set is accepted 0137's 2026-11-23 amendment: Claude Code
/// 2.1.329, codex-cli 0.166.0 and dsh 0.2.2-rc.1, each direct or through a
/// brokkr driver. The superseded selectors (2.1.310, 0.164.0, 0.1.5-rc.1)
/// stay decodable so published rows keep their exact bytes, and refuse
/// everywhere a run selects a CLI ([`WrapperProfile::approved`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
#[value(rename_all = "snake_case")]
pub enum WrapperProfile {
    #[serde(rename = "claude_2_1_310")]
    #[value(name = "claude_2_1_310")]
    Claude2_1_310,
    #[serde(rename = "codex_0_164_0")]
    #[value(name = "codex_0_164_0")]
    Codex0_164_0,
    #[serde(rename = "dsh_0_1_5_rc_1")]
    #[value(name = "dsh_0_1_5_rc_1")]
    Dsh0_1_5Rc1,
    /// The same declared configuration launched through a brokkr driver
    /// (decision 0119): the driver adds flags this profile does not
    /// enumerate, so such a row never claims the direct-launch surface.
    #[serde(rename = "claude_2_1_310_via_brokkr")]
    #[value(name = "claude_2_1_310_via_brokkr")]
    Claude2_1_310ViaBrokkr,
    #[serde(rename = "codex_0_164_0_via_brokkr")]
    #[value(name = "codex_0_164_0_via_brokkr")]
    Codex0_164_0ViaBrokkr,
    #[serde(rename = "dsh_0_1_5_rc_1_via_brokkr")]
    #[value(name = "dsh_0_1_5_rc_1_via_brokkr")]
    Dsh0_1_5Rc1ViaBrokkr,
    #[serde(rename = "claude_2_1_329")]
    #[value(name = "claude_2_1_329")]
    Claude2_1_329,
    #[serde(rename = "codex_0_166_0")]
    #[value(name = "codex_0_166_0")]
    Codex0_166_0,
    #[serde(rename = "dsh_0_2_2_rc_1")]
    #[value(name = "dsh_0_2_2_rc_1")]
    Dsh0_2_2Rc1,
    #[serde(rename = "claude_2_1_329_via_brokkr")]
    #[value(name = "claude_2_1_329_via_brokkr")]
    Claude2_1_329ViaBrokkr,
    #[serde(rename = "codex_0_166_0_via_brokkr")]
    #[value(name = "codex_0_166_0_via_brokkr")]
    Codex0_166_0ViaBrokkr,
    #[serde(rename = "dsh_0_2_2_rc_1_via_brokkr")]
    #[value(name = "dsh_0_2_2_rc_1_via_brokkr")]
    Dsh0_2_2Rc1ViaBrokkr,
}
impl WrapperProfile {
    /// The approved direct selectors (accepted 0137, 2026-11-23 amendment).
    pub const APPROVED: [Self; 3] = [Self::Claude2_1_329, Self::Codex0_166_0, Self::Dsh0_2_2Rc1];
    pub fn family(self) -> &'static str {
        match self.direct() {
            Self::Claude2_1_310 | Self::Claude2_1_329 => "claude",
            Self::Codex0_164_0 | Self::Codex0_166_0 => "codex",
            _ => "dsh",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Claude2_1_310 => "claude_2_1_310",
            Self::Codex0_164_0 => "codex_0_164_0",
            Self::Dsh0_1_5Rc1 => "dsh_0_1_5_rc_1",
            Self::Claude2_1_310ViaBrokkr => "claude_2_1_310_via_brokkr",
            Self::Codex0_164_0ViaBrokkr => "codex_0_164_0_via_brokkr",
            Self::Dsh0_1_5Rc1ViaBrokkr => "dsh_0_1_5_rc_1_via_brokkr",
            Self::Claude2_1_329 => "claude_2_1_329",
            Self::Codex0_166_0 => "codex_0_166_0",
            Self::Dsh0_2_2Rc1 => "dsh_0_2_2_rc_1",
            Self::Claude2_1_329ViaBrokkr => "claude_2_1_329_via_brokkr",
            Self::Codex0_166_0ViaBrokkr => "codex_0_166_0_via_brokkr",
            Self::Dsh0_2_2Rc1ViaBrokkr => "dsh_0_2_2_rc_1_via_brokkr",
        }
    }
    /// True for the selectors that declare a launch through a brokkr driver.
    pub fn via_brokkr(self) -> bool {
        self.direct() != self
    }
    /// The selector that launches this one's configuration through a brokkr
    /// driver (itself when it already does).
    pub fn via(self) -> Self {
        match self {
            Self::Claude2_1_310 => Self::Claude2_1_310ViaBrokkr,
            Self::Codex0_164_0 => Self::Codex0_164_0ViaBrokkr,
            Self::Dsh0_1_5Rc1 => Self::Dsh0_1_5Rc1ViaBrokkr,
            Self::Claude2_1_329 => Self::Claude2_1_329ViaBrokkr,
            Self::Codex0_166_0 => Self::Codex0_166_0ViaBrokkr,
            Self::Dsh0_2_2Rc1 => Self::Dsh0_2_2Rc1ViaBrokkr,
            other => other,
        }
    }
    /// The direct-launch selector whose declared configuration this one shares.
    pub fn direct(self) -> Self {
        match self {
            Self::Claude2_1_310ViaBrokkr => Self::Claude2_1_310,
            Self::Codex0_164_0ViaBrokkr => Self::Codex0_164_0,
            Self::Dsh0_1_5Rc1ViaBrokkr => Self::Dsh0_1_5Rc1,
            Self::Claude2_1_329ViaBrokkr => Self::Claude2_1_329,
            Self::Codex0_166_0ViaBrokkr => Self::Codex0_166_0,
            Self::Dsh0_2_2Rc1ViaBrokkr => Self::Dsh0_2_2Rc1,
            other => other,
        }
    }
    /// Whether a run may still select this profile's CLI: only the three
    /// amended versions, direct or through the driver. A superseded selector
    /// decodes (published rows) and refuses as `profile_superseded`.
    pub fn approved(self) -> bool {
        Self::APPROVED.contains(&self.direct())
    }
    pub fn validate_model(self, model: Option<&str>) -> Result<(), String> {
        crate::wrappers::validate_model(self.family(), model.ok_or("wrapper_model_missing")?)
    }
    /// The exact CLI version this profile's recipe was written against.
    pub fn cli_version(self) -> &'static str {
        match self.direct() {
            Self::Claude2_1_310 => "2.1.310",
            Self::Codex0_164_0 => "0.164.0",
            Self::Dsh0_1_5Rc1 => "0.1.5-rc.1",
            Self::Claude2_1_329 => "2.1.329",
            Self::Codex0_166_0 => "0.166.0",
            _ => "0.2.2-rc.1",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceBasis {
    DeclaredConfiguration,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ToolSurface {
    BuiltinNotApplicable,
    ExternalUnknown,
    InvalidRequest,
    Wrapper {
        revision: u8,
        basis: SurfaceBasis,
        profile: WrapperProfile,
        restrictions: Vec<String>,
        residuals: Vec<String>,
    },
}
/// The Codex sandbox classes a nested selection may run (the operator's
/// 2026-11-24 ruling on #404); `danger-full-access` is refused before any
/// row exists.
pub const CODEX_CLASSES: [&str; 2] = ["read-only", "workspace-write"];
impl ToolSurface {
    pub fn declared(profile: WrapperProfile) -> Self {
        Self::declared_class(profile, None)
    }
    /// The declared surface, with the Codex sandbox class the selection
    /// actually names under the driver (hold 4 on a73ac17a). `None` is a
    /// class that could not be read: such a selection never launches.
    pub fn declared_class(profile: WrapperProfile, codex_class: Option<&str>) -> Self {
        let (restrictions, residuals): (&[&str], &[&str]) = match profile.family() {
            "claude" => (
                &[
                    "empty builtins",
                    "strict two-server MCP",
                    "explicit MCP tool permissions",
                    "restricted settings",
                    "disabled skills",
                ],
                &[
                    "nearby-version measurement",
                    "server-delivered policy unmeasured",
                    "exact-version live inventory unmeasured",
                    "unconfined; same-uid access",
                ],
            ),
            "codex" => (
                &[
                    "read-only sandbox",
                    "web search disabled",
                    "ephemeral",
                    "user config and rules ignored",
                    "two declared MCP servers",
                ],
                &[
                    "native shell, reading and edit tools",
                    "code-mode MCP access",
                    "ChatGPT account connectors; removal unproved",
                    "project trust override effect unmeasured",
                    "unconfined; same-uid access",
                ],
            ),
            _ => (
                &[
                    "fresh DSH_HOME",
                    "overlay-only MCP servers",
                    "telemetry disabled",
                ],
                &[
                    "native shell, file, web and subagent tools; removal unmeasured",
                    "plugin composition unmeasured",
                    "live HTTP MCP unmeasured",
                    "unconfined; same-uid access",
                ],
            ),
        };
        let mut residuals: Vec<String> = residuals.iter().map(|s| s.to_string()).collect();
        let mut restrictions: Vec<String> = restrictions.iter().map(|s| s.to_string()).collect();
        if profile.via_brokkr() {
            // The nested route applies its own launch, not the direct one's
            // flags: each amended family's descriptor names what it passes.
            // A superseded via profile never launched the driver
            // (`driver_contract_unknown`); its published rows keep the
            // descriptor they were written with.
            match profile.family() {
                _ if !profile.approved() => {}
                "claude" => {
                    restrictions = vec![
                        "permission mode acceptEdits".into(),
                        "two declared MCP servers (user scope)".into(),
                        "explicit MCP tool permissions (settings)".into(),
                        "model and effort flags".into(),
                    ];
                    residuals.push("builtin tools, settings and skills not restricted".into());
                }
                "codex" => {
                    restrictions = vec![
                        match codex_class {
                            Some(class) => format!("{class} sandbox (driver-selected)"),
                            None => "driver sandbox class unread".into(),
                        },
                        "web search disabled".into(),
                        "fresh CODEX_HOME configuration".into(),
                        "two declared MCP servers".into(),
                        "model and effort flags".into(),
                    ];
                    residuals.push("ephemeral session and rule exclusion not applied".into());
                }
                _ => {}
            }
            residuals.push("brokkr driver flags unenumerated".to_string());
        }
        Self::Wrapper {
            revision: 1,
            basis: SurfaceBasis::DeclaredConfiguration,
            profile,
            restrictions,
            residuals,
        }
    }
    pub fn for_request(
        valid: bool,
        builtin: bool,
        profile: Option<WrapperProfile>,
        codex_class: Option<&str>,
    ) -> Self {
        if !valid {
            Self::InvalidRequest
        } else if builtin {
            Self::BuiltinNotApplicable
        } else {
            profile
                .map(|p| Self::declared_class(p, codex_class))
                .unwrap_or(Self::ExternalUnknown)
        }
    }
    /// Whether this is a surface the profile can declare: under the driver,
    /// Codex's with one of the admitted classes or an unread one.
    fn admitted(&self, profile: WrapperProfile) -> bool {
        if profile.via_brokkr() && profile.approved() && profile.family() == "codex" {
            CODEX_CLASSES
                .iter()
                .map(|c| Some(*c))
                .chain([None])
                .any(|c| *self == Self::declared_class(profile, c))
        } else {
            *self == Self::declared(profile)
        }
    }
    pub fn validate(&self, kind: AgentKind, model: &Availability<String>) -> Result<(), String> {
        match (self, kind) {
            (Self::BuiltinNotApplicable, AgentKind::Builtin)
            | (Self::ExternalUnknown, AgentKind::External)
            | (Self::InvalidRequest, AgentKind::Invalid) => Ok(()),
            (Self::Wrapper { profile, .. }, AgentKind::External) if self.admitted(*profile) => {
                profile.validate_model(match model {
                    Availability::Present { value, .. } => Some(value),
                    _ => None,
                })
            }
            _ => Err("invalid declared tool surface".into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentIdentity {
    pub tool_surface: ToolSurface,
    pub kind: AgentKind,
    pub executable_sha256: Availability<String>,
    pub command: Availability<CommandIdentity>,
    pub model: Availability<String>,
    pub config_id: Availability<String>,
    /// The verified CLI binary a paid-baseline row's wrapper launched
    /// (proposed 0137): its profile, the canonical version the bounded probe
    /// returned and the pinned, measured SHA-256. Absent on every other row,
    /// so historically accepted records keep their exact bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli: Option<CliIdentity>,
}
/// Public CLI identity: never the private path, banner or credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliIdentity {
    pub profile: WrapperProfile,
    pub version: String,
    pub sha256: String,
}
impl CliIdentity {
    /// A binding agrees with the row's declared wrapper surface, names that
    /// profile's exact version and carries a lowercase SHA-256. Under an
    /// amended `*_via_brokkr` profile the digest is the nested identity: the
    /// CLI's pin with the driver's (route (b), proposed 0138).
    pub fn validate(&self, surface: &ToolSurface) -> Result<(), String> {
        match surface {
            ToolSurface::Wrapper { profile, .. }
                if *profile == self.profile
                    && profile.approved()
                    && self.version == profile.cli_version()
                    && is_hash(&self.sha256) =>
            {
                Ok(())
            }
            _ => Err("invalid cli identity".into()),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub agent_deadline_ms: Availability<u64>,
    pub startup_ms: u64,
    pub io_ms: u64,
    pub shutdown_ms: u64,
    #[serde(deserialize_with = "nullable")]
    pub matrix_child_envelope_ms: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub fixture: FixtureInputs,
    #[serde(deserialize_with = "nullable")]
    pub matrix: Option<MatrixCoordinates>,
    pub task: TaskIdentity,
    pub harness: HarnessIdentity,
    pub target: TargetIdentity,
    pub agent: AgentIdentity,
    pub limits: Limits,
}
fn check_hash(a: &Availability<String>, sources: &[Source]) -> Result<(), String> {
    a.check(sources, false)?;
    if let Availability::Present { value, .. } = a
        && !is_hash(value)
    {
        return Err("invalid digest".into());
    }
    Ok(())
}
fn check_revision(a: &Availability<String>, sources: &[Source]) -> Result<(), String> {
    a.check(sources, false)?;
    if let Availability::Present { value, .. } = a
        && !((value.len() == 40 || value.len() == 64)
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
    {
        return Err("invalid revision".into());
    }
    Ok(())
}
pub fn catalogue_hash() -> String {
    sha256(
        &serde_json::to_vec(&crate::tasks::Catalogue {
            schema_version: crate::tasks::CATALOGUE_SCHEMA,
            tasks: crate::tasks::catalogue(),
        })
        .expect("typed catalogue"),
    )
}
pub fn task_hash(task: &str) -> Option<String> {
    crate::tasks::find(task).map(|t| sha256(&serde_json::to_vec(&t).expect("typed task")))
}
impl Provenance {
    pub fn validate(&self) -> Result<(), String> {
        self.fixture.validate()?;
        self.agent
            .tool_surface
            .validate(self.agent.kind, &self.agent.model)?;
        if let Some(cli) = &self.agent.cli {
            cli.validate(&self.agent.tool_surface)?;
        }
        if let Some(m) = &self.matrix {
            seed_bytes(&m.master_seed)?;
        }
        if self.task.catalogue_schema != crate::tasks::CATALOGUE_SCHEMA
            || !is_hash(&self.task.catalogue_sha256)
            || self.task.scorer_id != "business-score-v2"
        {
            return Err("unsupported catalogue/scorer".into());
        }
        self.task
            .definition_sha256
            .check(&[Source::CatalogueDefinition], false)?;
        check_hash(&self.task.definition_sha256, &[Source::CatalogueDefinition])?;
        check_hash(&self.harness.executable_sha256, &[Source::MeasuredBytes])?;
        check_hash(&self.target.executable_sha256, &[Source::MeasuredBytes])?;
        check_hash(&self.agent.executable_sha256, &[Source::MeasuredBytes])?;
        check_revision(&self.harness.build_commit, &[Source::BuildGit])?;
        check_revision(
            &self.target.build_commit,
            &[Source::OwnedBuildContext, Source::IdenticalOwnedArtifact],
        )?;
        self.harness
            .source_dirty
            .check(&[Source::BuildGit], false)?;
        self.target.source_dirty.check(
            &[Source::OwnedBuildContext, Source::IdenticalOwnedArtifact],
            false,
        )?;
        self.target
            .settings
            .check(&[Source::ResolvedInput], false)?;
        self.target.chaos.check(&[Source::ObservedFixture], false)?;
        let command_sources = if self.agent.kind == AgentKind::Builtin {
            &[Source::Builtin][..]
        } else {
            &[Source::DeclaredPublicArgv][..]
        };
        let config_sources = if self.agent.kind == AgentKind::Builtin {
            &[Source::Builtin][..]
        } else {
            &[Source::OperatorDeclared][..]
        };
        self.agent.command.check(command_sources, false)?;
        if let Availability::Present { value, .. } = &self.agent.command
            && (value.algorithm != "public-argv-sha256-v1" || !is_hash(&value.sha256))
        {
            return Err("invalid command identity".into());
        }
        self.agent.config_id.check(config_sources, false)?;
        self.agent.model.check(
            &[Source::OperatorDeclared],
            self.agent.kind == AgentKind::Builtin,
        )?;
        if self.agent.kind == AgentKind::Builtin
            && self.agent.model
                != (Availability::NotApplicable {
                    reason: Reason::Builtin,
                })
        {
            return Err("builtin model is not applicable".into());
        }
        if self.agent.kind == AgentKind::Invalid {
            let invalid = |reason: &Reason| *reason == Reason::RequestInvalid;
            if !matches!(&self.agent.executable_sha256,Availability::Unavailable{reason} if invalid(reason))
                || !matches!(&self.agent.command,Availability::Unavailable{reason} if invalid(reason))
                || !matches!(&self.agent.model,Availability::Unavailable{reason} if invalid(reason))
                || !matches!(&self.agent.config_id,Availability::Unavailable{reason} if invalid(reason))
            {
                return Err("invalid request carries agent identity".into());
            }
        }
        for a in [&self.agent.model, &self.agent.config_id] {
            if let Availability::Present { value, .. } = a
                && (value.is_empty() || value.len() > 1024)
            {
                return Err("invalid public declaration".into());
            }
        }
        self.limits
            .agent_deadline_ms
            .check(&[Source::ResolvedInput], false)?;
        if let Availability::Present { value, .. } = &self.limits.agent_deadline_ms
            && (*value == 0 || *value > crate::tasks::MAX_DEADLINE_MS)
        {
            return Err("deadline outside supported bounds".into());
        }

        if self.limits.startup_ms != 30_000
            || self.limits.io_ms != 10_000
            || self.limits.shutdown_ms != 5_000
        {
            return Err("unsupported lifecycle bounds".into());
        }
        match (
            &self.matrix,
            &self.limits.agent_deadline_ms,
            self.limits.matrix_child_envelope_ms,
        ) {
            (None, _, None) => {}
            (Some(_), Availability::Present { value, .. }, Some(envelope))
                if envelope == value + 300_000 => {}
            _ => return Err("invalid child envelope".into()),
        }
        Ok(())
    }
}

/// Serde's default recursion limit is 128; retained contracts allow 64.
pub fn depth(bytes: &[u8]) -> Result<(), String> {
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escape = false;
    for &b in bytes {
        if quoted {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                quoted = false;
            }
        } else {
            match b {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 64 {
                        return Err("JSON nesting exceeds 64".into());
                    }
                }
                b'}' | b']' => {
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
        }
    }
    Ok(())
}
pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    depth(bytes)?;
    serde_json::from_slice(bytes).map_err(|_| "invalid typed JSON".into())
}

/// Retained dimension arrays refuse their 4097th element during decoding.
pub fn cells<'de, D, T>(d: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Bounded<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Bounded<T> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("at most 4096 elements")
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<T>, A::Error> {
            let mut out = Vec::new();
            while let Some(value) = seq.next_element()? {
                if out.len() == 4096 {
                    return Err(serde::de::Error::custom("retained dimension exceeds 4096"));
                }
                out.push(value);
            }
            Ok(out)
        }
    }
    d.deserialize_seq(Bounded(std::marker::PhantomData))
}

#[cfg(test)]
mod via_brokkr_tests {
    use super::*;

    const DIRECT: [WrapperProfile; 6] = [
        WrapperProfile::Claude2_1_310,
        WrapperProfile::Codex0_164_0,
        WrapperProfile::Dsh0_1_5Rc1,
        WrapperProfile::Claude2_1_329,
        WrapperProfile::Codex0_166_0,
        WrapperProfile::Dsh0_2_2Rc1,
    ];
    const VIA: [WrapperProfile; 6] = [
        WrapperProfile::Claude2_1_310ViaBrokkr,
        WrapperProfile::Codex0_164_0ViaBrokkr,
        WrapperProfile::Dsh0_1_5Rc1ViaBrokkr,
        WrapperProfile::Claude2_1_329ViaBrokkr,
        WrapperProfile::Codex0_166_0ViaBrokkr,
        WrapperProfile::Dsh0_2_2Rc1ViaBrokkr,
    ];

    #[test]
    fn the_approved_set_is_exactly_the_three_amended_profiles() {
        let approved: Vec<_> = DIRECT.into_iter().filter(|p| p.approved()).collect();
        assert_eq!(approved, WrapperProfile::APPROVED);
        for (profile, version) in
            WrapperProfile::APPROVED
                .into_iter()
                .zip(["2.1.329", "0.166.0", "0.2.2-rc.1"])
        {
            assert_eq!(profile.cli_version(), version);
        }
        // A superseded selector still decodes (published rows keep their
        // bytes) and names its own version, but is never approved, direct or
        // through the driver.
        for (profile, version) in DIRECT[..3].iter().zip(["2.1.310", "0.164.0", "0.1.5-rc.1"]) {
            assert_eq!(profile.cli_version(), version);
            assert!(!profile.approved());
        }
        assert!(VIA[..3].iter().all(|p| !p.approved()));
        assert!(VIA[3..].iter().all(|p| p.approved()));
        let families: Vec<_> = DIRECT.iter().map(|p| p.family()).collect();
        assert_eq!(
            families,
            ["claude", "codex", "dsh", "claude", "codex", "dsh"]
        );
    }

    #[test]
    fn each_via_brokkr_selector_round_trips_by_its_finite_name() {
        for (direct, via) in DIRECT.into_iter().zip(VIA) {
            assert_eq!(via.name(), format!("{}_via_brokkr", direct.name()));
            assert_eq!(via.family(), direct.family());
            assert_eq!(via.direct(), direct);
            assert_eq!(direct.direct(), direct);
            assert!(via.via_brokkr() && !direct.via_brokkr());
            let json = serde_json::to_string(&via).unwrap();
            assert_eq!(json, format!("\"{}\"", via.name()));
            assert_eq!(serde_json::from_str::<WrapperProfile>(&json).unwrap(), via);
            // The command line accepts exactly the finite names.
            let parsed = <WrapperProfile as clap::ValueEnum>::from_str(via.name(), false).unwrap();
            assert_eq!(parsed, via);
        }
        assert!(serde_json::from_str::<WrapperProfile>("\"claude_2_1_310_via_other\"").is_err());
    }

    #[test]
    fn a_via_brokkr_selector_validates_only_its_own_families_models() {
        for (via, model, foreign) in [
            (
                WrapperProfile::Claude2_1_310ViaBrokkr,
                "claude-sonnet-5-5",
                "gpt-6-luna",
            ),
            (
                WrapperProfile::Codex0_164_0ViaBrokkr,
                "gpt-6-luna",
                "claude-opus-5-5",
            ),
            (
                WrapperProfile::Dsh0_1_5Rc1ViaBrokkr,
                "dashscope/glm-5.3",
                "gpt-6.1-sol",
            ),
        ] {
            assert!(via.validate_model(Some(model)).is_ok());
            assert!(via.validate_model(Some(foreign)).is_err());
            assert!(via.validate_model(None).is_err());
        }
    }

    /// Each route's descriptor names what that route applies (the panel's
    /// correctness 5 on a73ac17a): the nested claude launch passes only
    /// acceptEdits, model and effort, so none of the direct launch's flag
    /// restrictions is claimed for it, and the nested codex descriptor names
    /// the class the selection runs (hold 4).
    #[test]
    fn a_via_brokkr_surface_names_what_its_route_applies() {
        for (direct, via) in DIRECT.into_iter().zip(VIA) {
            let (
                ToolSurface::Wrapper {
                    restrictions: direct_restrictions,
                    residuals: direct_residuals,
                    revision: direct_revision,
                    basis: direct_basis,
                    ..
                },
                ToolSurface::Wrapper {
                    restrictions,
                    residuals,
                    revision,
                    basis,
                    profile,
                },
            ) = (ToolSurface::declared(direct), ToolSurface::declared(via))
            else {
                panic!("a declared surface is a wrapper surface");
            };
            assert_eq!(profile, via);
            assert_eq!((revision, basis), (direct_revision, direct_basis));
            assert_eq!(residuals[..direct_residuals.len()], direct_residuals[..]);
            assert_eq!(
                residuals.last().map(String::as_str),
                Some("brokkr driver flags unenumerated")
            );
            match via.family() {
                // A superseded via profile keeps its published descriptor.
                _ if !via.approved() => assert_eq!(restrictions, direct_restrictions),
                "claude" => {
                    for direct_only in [
                        "empty builtins",
                        "strict two-server MCP",
                        "restricted settings",
                        "disabled skills",
                    ] {
                        assert!(direct_restrictions.iter().any(|r| r == direct_only));
                        assert!(
                            !restrictions.iter().any(|r| r == direct_only),
                            "{direct_only}"
                        );
                    }
                    assert_eq!(restrictions[0], "permission mode acceptEdits");
                }
                "codex" => {
                    assert_eq!(direct_restrictions[0], "read-only sandbox");
                    assert_eq!(restrictions[0], "driver sandbox class unread");
                    for class in CODEX_CLASSES {
                        let surface = ToolSurface::declared_class(via, Some(class));
                        let ToolSurface::Wrapper { restrictions, .. } = &surface else {
                            panic!("a wrapper surface");
                        };
                        assert_eq!(
                            restrictions[0],
                            format!("{class} sandbox (driver-selected)")
                        );
                        assert_eq!(
                            ToolSurface::for_request(true, false, Some(via), Some(class)),
                            surface
                        );
                        let model = Availability::present(
                            "gpt-6-luna".to_string(),
                            Source::OperatorDeclared,
                        );
                        assert!(surface.validate(AgentKind::External, &model).is_ok());
                    }
                    assert_ne!(
                        ToolSurface::declared_class(via, Some("read-only")),
                        ToolSurface::declared_class(via, Some("workspace-write"))
                    );
                    // A class outside the admitted two is no declared surface.
                    let forged = ToolSurface::declared_class(via, Some("danger-full-access"));
                    let model =
                        Availability::present("gpt-6-luna".to_string(), Source::OperatorDeclared);
                    assert!(forged.validate(AgentKind::External, &model).is_err());
                }
                _ => assert_eq!(restrictions, direct_restrictions),
            }
            assert_ne!(ToolSurface::declared(via), ToolSurface::declared(direct));
            // The request-level constructor accepts a via-brokkr profile like any other.
            assert_eq!(
                ToolSurface::for_request(true, false, Some(via), None),
                ToolSurface::declared(via)
            );
        }
    }

    #[test]
    fn a_surface_validates_against_its_own_model_and_no_other_profile() {
        let model =
            Availability::present("claude-sonnet-5-5".to_string(), Source::OperatorDeclared);
        let via = ToolSurface::declared(WrapperProfile::Claude2_1_310ViaBrokkr);
        assert!(via.validate(AgentKind::External, &model).is_ok());
        // A via-brokkr surface forged with the direct profile's name is refused.
        let mut forged = ToolSurface::declared(WrapperProfile::Claude2_1_310);
        if let ToolSurface::Wrapper { profile, .. } = &mut forged {
            *profile = WrapperProfile::Claude2_1_310ViaBrokkr;
        }
        assert!(forged.validate(AgentKind::External, &model).is_err());
    }
}

#[cfg(test)]
mod cli_identity_tests {
    use super::*;

    #[test]
    fn a_cli_binding_must_match_its_direct_surface_version_and_digest() {
        let surface = ToolSurface::declared(WrapperProfile::Codex0_166_0);
        let good = CliIdentity {
            profile: WrapperProfile::Codex0_166_0,
            version: "0.166.0".into(),
            sha256: "a".repeat(64),
        };
        assert!(good.validate(&surface).is_ok());
        let mut other = good.clone();
        other.profile = WrapperProfile::Claude2_1_329;
        assert!(other.validate(&surface).is_err());
        // A superseded profile's binding refuses even against its own surface.
        let old = CliIdentity {
            profile: WrapperProfile::Codex0_164_0,
            version: "0.164.0".into(),
            sha256: "a".repeat(64),
        };
        assert!(
            old.validate(&ToolSurface::declared(WrapperProfile::Codex0_164_0))
                .is_err()
        );
        let mut version = good.clone();
        version.version = "0.165.0".into();
        assert!(version.validate(&surface).is_err());
        let mut digest = good.clone();
        digest.sha256 = "A".repeat(64);
        assert!(digest.validate(&surface).is_err());
        // An amended via-brokkr binding holds against its own surface only.
        let mut via = good.clone();
        via.profile = WrapperProfile::Codex0_166_0ViaBrokkr;
        let via_surface = ToolSurface::declared(WrapperProfile::Codex0_166_0ViaBrokkr);
        assert!(via.validate(&via_surface).is_ok());
        // The nested descriptor never claims the direct launch's class.
        let ToolSurface::Wrapper { restrictions, .. } = &via_surface else {
            panic!("a wrapper surface");
        };
        assert!(!restrictions.iter().any(|r| r == "read-only sandbox"));
        assert_eq!(restrictions[0], "driver sandbox class unread");
        assert!(via.validate(&surface).is_err());
        assert!(good.validate(&via_surface).is_err());
        let mut old_via = old.clone();
        old_via.profile = WrapperProfile::Codex0_164_0ViaBrokkr;
        assert!(
            old_via
                .validate(&ToolSurface::declared(
                    WrapperProfile::Codex0_164_0ViaBrokkr
                ))
                .is_err()
        );
        for p in [
            WrapperProfile::Claude2_1_310,
            WrapperProfile::Codex0_164_0,
            WrapperProfile::Dsh0_1_5Rc1,
            WrapperProfile::Claude2_1_329,
            WrapperProfile::Codex0_166_0,
            WrapperProfile::Dsh0_2_2Rc1,
        ] {
            assert!(p.via().via_brokkr() && p.via().direct() == p);
            assert_eq!(p.via().via(), p.via());
        }
        assert!(good.validate(&ToolSurface::ExternalUnknown).is_err());
        // Absent on every other row: the serialized agent keeps its bytes.
        let json = serde_json::to_value(&good).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"profile":"codex_0_166_0","version":"0.166.0","sha256":"a".repeat(64)})
        );
    }
}
