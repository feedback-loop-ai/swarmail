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
<<<<<<< HEAD
/// Declared recipe, never an observed tool inventory or binary attestation.
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
}
impl WrapperProfile {
    pub fn family(self) -> &'static str {
        match self {
            Self::Claude2_1_310 => "claude",
            Self::Codex0_164_0 => "codex",
            Self::Dsh0_1_5Rc1 => "dsh",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Claude2_1_310 => "claude_2_1_310",
            Self::Codex0_164_0 => "codex_0_164_0",
            Self::Dsh0_1_5Rc1 => "dsh_0_1_5_rc_1",
        }
    }
    pub fn validate_model(self, model: Option<&str>) -> Result<(), String> {
        crate::wrappers::validate_model(self.family(), model.ok_or("wrapper_model_missing")?)
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
impl ToolSurface {
    pub fn declared(profile: WrapperProfile) -> Self {
        let (restrictions, residuals): (&[&str], &[&str]) = match profile {
            WrapperProfile::Claude2_1_310 => (
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
            WrapperProfile::Codex0_164_0 => (
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
            WrapperProfile::Dsh0_1_5Rc1 => (
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
        Self::Wrapper {
            revision: 1,
            basis: SurfaceBasis::DeclaredConfiguration,
            profile,
            restrictions: restrictions.iter().map(|s| s.to_string()).collect(),
            residuals: residuals.iter().map(|s| s.to_string()).collect(),
        }
    }
    pub fn for_request(valid: bool, builtin: bool, profile: Option<WrapperProfile>) -> Self {
        if !valid {
            Self::InvalidRequest
        } else if builtin {
            Self::BuiltinNotApplicable
        } else {
            profile.map(Self::declared).unwrap_or(Self::ExternalUnknown)
        }
    }
    pub fn validate(&self, kind: AgentKind, model: &Availability<String>) -> Result<(), String> {
        match (self, kind) {
            (Self::BuiltinNotApplicable, AgentKind::Builtin)
            | (Self::ExternalUnknown, AgentKind::External)
            | (Self::InvalidRequest, AgentKind::Invalid) => Ok(()),
            (Self::Wrapper { profile, .. }, AgentKind::External)
                if *self == Self::declared(*profile) =>
            {
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
=======
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentIdentity {
>>>>>>> 0c71b8c (Bind eval evidence to reproducible provenance on current main)
    pub kind: AgentKind,
    pub executable_sha256: Availability<String>,
    pub command: Availability<CommandIdentity>,
    pub model: Availability<String>,
    pub config_id: Availability<String>,
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
<<<<<<< HEAD
            schema_version: crate::tasks::CATALOGUE_SCHEMA,
=======
            schema_version: 1,
>>>>>>> 0c71b8c (Bind eval evidence to reproducible provenance on current main)
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
<<<<<<< HEAD
        self.agent
            .tool_surface
            .validate(self.agent.kind, &self.agent.model)?;
        if let Some(m) = &self.matrix {
            seed_bytes(&m.master_seed)?;
        }
        if self.task.catalogue_schema != crate::tasks::CATALOGUE_SCHEMA
            || !is_hash(&self.task.catalogue_sha256)
            || self.task.scorer_id != "business-score-v2"
=======
        if let Some(m) = &self.matrix {
            seed_bytes(&m.master_seed)?;
        }
        if self.task.catalogue_schema != 1
            || !is_hash(&self.task.catalogue_sha256)
            || self.task.scorer_id != "business-score-v1"
>>>>>>> 0c71b8c (Bind eval evidence to reproducible provenance on current main)
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
