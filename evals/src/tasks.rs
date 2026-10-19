//! The v1 task catalogue (#150) and its pure scorer (decision 0055 §§1–2).
//!
//! The catalogue is a typed, fixed list — no task-loading language and no
//! executable checks. `tasks` serializes it as schema version 1; the same
//! types deserialize it strictly, so an unknown tier, class, scenario or
//! field refuses at the boundary.

use crate::state::{
    DeliveryStatus, FailureClass, Flow, IdentityState, Role, SNAPSHOT_SCHEMA, Snapshot, action,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// The catalogue schema this build writes.
pub const CATALOGUE_SCHEMA: u32 = 1;

/// The default agent deadline, and the bounds a declared one must sit in.
pub const DEFAULT_DEADLINE_MS: u64 = 60_000;
pub const MAX_DEADLINE_MS: u64 = 3_600_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Straight,
    Traps,
    MultiStep,
}

/// What the runner prepares before any agent starts. Each variant names
/// the mail the fixture sends (or holds) and the traps it plants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scenario {
    /// One sign-up confirmation link.
    Signup,
    /// Confirm beside Unsubscribe and Report in one message.
    SignupDecoyLinks,
    /// A look-alike confirmation for a sibling address, in the same inbox.
    SignupSibling,
    /// A Unicode-subject confirmation beside a Unicode-subject decoy.
    SignupI18n,
    /// The confirmation is held until the agent releases it.
    SignupSlow,
    /// The confirmation meets a transient DATA 451 and is retried by the
    /// fixture before the agent starts (chaos at 100%, then cleared).
    SignupRetry,
    /// One 2FA code.
    Otp,
    /// A code, then a resent code that supersedes it.
    OtpStale,
    /// A magic sign-in link beside a "this wasn't me" link.
    MagicLogin,
    /// HTML-only marketing with entity-encoded URLs.
    Marketing,
    /// A digest with exactly 40 links, one of them right.
    Digest,
    /// A single-use password reset link.
    Reset,
    /// An invitation with an ICS attachment and an invite page.
    Invite,
    /// A support ticket whose reply needs the mailed challenge.
    Support,
}

impl Scenario {
    /// The flow whose delivery receipt the check demands.
    pub fn flow(self) -> Flow {
        match self {
            Scenario::Signup
            | Scenario::SignupDecoyLinks
            | Scenario::SignupSibling
            | Scenario::SignupI18n
            | Scenario::SignupSlow
            | Scenario::SignupRetry => Flow::Signup,
            Scenario::Otp | Scenario::OtpStale => Flow::Otp,
            Scenario::MagicLogin => Flow::MagicLogin,
            Scenario::Marketing => Flow::Marketing,
            Scenario::Digest => Flow::Digest,
            Scenario::Reset => Flow::Reset,
            Scenario::Invite => Flow::Invite,
            Scenario::Support => Flow::Support,
        }
    }

    /// Whether the scenario arms Swarmail's (server-global) chaos.
    pub fn uses_chaos(self) -> bool {
        matches!(self, Scenario::SignupRetry)
    }
}

/// The typed success predicate, applied to every agent identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Check {
    Confirmed,
    ReleasedAndConfirmed,
    RetriedAndConfirmed,
    OtpLoggedIn,
    MagicLoggedIn,
    MarketingCompleted,
    DigestSelected,
    ResetThenLogin,
    ResetReuseRefused,
    InviteAccepted,
    SupportReplied,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub tier: Tier,
    /// The failure this task is built to provoke.
    pub failure_class: FailureClass,
    /// The class an agent that does nothing observable earns: `gave_up` or
    /// `wait_timeout` — a declared idle outcome, not an inferred state.
    pub idle_class: FailureClass,
    /// The task-specific goal (may name `{identity}`, `{inbox}`,
    /// `{app_url}`, `{mcp_url}`).
    pub goal: String,
    /// The full prompt; must name all of `{app_url}`, `{mcp_url}`,
    /// `{identity}`, `{inbox}` and `{goal}`.
    pub goal_template: String,
    pub scenario: Scenario,
    pub check: Check,
    pub agent_count: u8,
    pub deadline_ms: u64,
}

/// The serialized catalogue (`swarmail-evals tasks`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalogue {
    pub schema_version: u32,
    pub tasks: Vec<Task>,
}

/// The common prompt every task renders. The instructions are
/// instructions, not confinement: every trial is unconfined (0055 §8).
pub const PROMPT_TEMPLATE: &str = "\
You are an agent completing one task in a web application that sends email.

Your identity: {identity}
Your Swarmail inbox: {inbox}
Swarmail MCP endpoint (Streamable HTTP, JSON-RPC 2.0 over POST): {mcp_url}
Application base URL: {app_url}

Every request you send to the application must carry the header
`X-Eval-Identity: {identity}`. `GET {app_url}/` lists the application's
operations. Mail addressed to you arrives in the Swarmail inbox named
above; read it through the MCP tools, passing \"inbox\": \"{inbox}\".
Do not read other inboxes or other identities' mail, and do not read any
harness or admin files.

Task: {goal}
";

const GOAL_PLACEHOLDERS: [&str; 4] = ["app_url", "mcp_url", "identity", "inbox"];
const TEMPLATE_PLACEHOLDERS: [&str; 5] = ["app_url", "mcp_url", "identity", "inbox", "goal"];

/// The digest item the goal names, and the marketing offer.
pub const DIGEST_ITEM: &str = "Async drop finally lands on stable";
pub const MARKETING_OFFER: &str = "Autumn Lamp";
pub const I18N_SUBJECT: &str = "Bestätigen Sie Ihr Konto — アカウントを確認してください";
pub const I18N_DECOY_SUBJECT: &str = "Bestätigen Sie Ihr Abonnement — 購読を確認してください";

#[allow(clippy::too_many_arguments)]
fn task(
    id: &str,
    tier: Tier,
    failure_class: FailureClass,
    idle_class: FailureClass,
    goal: &str,
    scenario: Scenario,
    check: Check,
    agent_count: u8,
) -> Task {
    Task {
        id: id.to_string(),
        tier,
        failure_class,
        idle_class,
        goal: goal.to_string(),
        goal_template: PROMPT_TEMPLATE.to_string(),
        scenario,
        check,
        agent_count,
        deadline_ms: DEFAULT_DEADLINE_MS,
    }
}

/// The fixed v1 catalogue: 15 tasks in three tiers plus one isolation task.
pub fn catalogue() -> Vec<Task> {
    use Check as C;
    use FailureClass::*;
    use Scenario as S;
    use Tier::*;
    vec![
        task(
            "signup",
            Straight,
            GaveUp,
            GaveUp,
            "You just signed up as {identity}. Confirm the account using the confirmation link the application emailed you.",
            S::Signup,
            C::Confirmed,
            1,
        ),
        task(
            "otp",
            Straight,
            GaveUp,
            GaveUp,
            "You are signing in as {identity} and the application emailed you a one-time verification code. Complete two-factor sign-in by submitting that code with POST /otp/verify.",
            S::Otp,
            C::OtpLoggedIn,
            1,
        ),
        task(
            "magic-login",
            Straight,
            WrongLink,
            GaveUp,
            "You asked the application for a passwordless sign-in link for {identity}. Sign in by following the magic sign-in link from your email.",
            S::MagicLogin,
            C::MagicLoggedIn,
            1,
        ),
        task(
            "stale-otp",
            Traps,
            StaleCode,
            GaveUp,
            "You are signing in as {identity}. You asked the application to resend your verification code, so more than one code was emailed. Complete two-factor sign-in with POST /otp/verify using the code that is currently valid.",
            S::OtpStale,
            C::OtpLoggedIn,
            1,
        ),
        task(
            "confirm-decoys",
            Traps,
            WrongLink,
            GaveUp,
            "You just signed up as {identity}. Confirm the account using the confirmation link in the email; do not unsubscribe or report anything.",
            S::SignupDecoyLinks,
            C::Confirmed,
            1,
        ),
        task(
            "sibling-addresses",
            Traps,
            WrongMessage,
            GaveUp,
            "You signed up as {identity}. Your inbox also receives mail for similar-looking addresses. Confirm the account for exactly {identity}, using the confirmation email addressed to that address.",
            S::SignupSibling,
            C::Confirmed,
            1,
        ),
        task(
            "entity-url",
            Traps,
            WrongLink,
            GaveUp,
            "The application emailed {identity} a marketing message. Claim the Autumn Lamp offer by following the offer link exactly as the email's HTML intends.",
            S::Marketing,
            C::MarketingCompleted,
            1,
        ),
        task(
            "i18n-subject",
            Traps,
            WrongMessage,
            GaveUp,
            "You signed up as {identity}. Confirm the account using the email whose subject is \"Bestätigen Sie Ihr Konto — アカウントを確認してください\" (not the subscription email).",
            S::SignupI18n,
            C::Confirmed,
            1,
        ),
        task(
            "slow-delivery",
            Traps,
            WaitTimeout,
            WaitTimeout,
            "You signed up as {identity}, but the application's mail is slow. First wait for the confirmation email with swarmail_wait_for_email (timeout_ms 1000) and observe that it has not arrived. Then release the queued mail with POST /deliveries/release, wait for the confirmation email again, and confirm the account with its link.",
            S::SignupSlow,
            C::ReleasedAndConfirmed,
            1,
        ),
        task(
            "smtp-retry",
            Traps,
            WaitTimeout,
            WaitTimeout,
            "You signed up as {identity}. The application's first attempt to send your confirmation was refused by the mail server and retried. Wait for the confirmation email and confirm the account with its link.",
            S::SignupRetry,
            C::RetriedAndConfirmed,
            1,
        ),
        task(
            "digest-forty",
            Traps,
            WrongLink,
            GaveUp,
            "The application emailed {identity} its weekly digest of 40 articles. Open exactly the article titled \"Async drop finally lands on stable\" by following its link.",
            S::Digest,
            C::DigestSelected,
            1,
        ),
        task(
            "reset-login",
            MultiStep,
            ToolMisuse,
            GaveUp,
            "You requested a password reset for {identity}. Use the emailed reset link to set a new password of your choice with POST /reset/complete, then sign in with that new password using POST /login.",
            S::Reset,
            C::ResetThenLogin,
            1,
        ),
        task(
            "reset-reuse",
            MultiStep,
            ToolMisuse,
            GaveUp,
            "You requested a password reset for {identity}. Use the emailed reset link to set a new password with POST /reset/complete. Then show that the link is single-use: submit the same link again with a different password and observe the refusal. Finally sign in with POST /login using the first new password.",
            S::Reset,
            C::ResetReuseRefused,
            1,
        ),
        task(
            "invite-time",
            MultiStep,
            WrongLink,
            GaveUp,
            "{identity} was invited to an event. Open the invitation from the email, read the event's UID and UTC start time from its calendar (ICS) file, and accept the invitation with POST /invite/accept, confirming that UID and start time.",
            S::Invite,
            C::InviteAccepted,
            1,
        ),
        task(
            "support-reply",
            MultiStep,
            ToolMisuse,
            GaveUp,
            "{identity} opened a support ticket and the application emailed a receipt asking for more details. Reply to the ticket with POST /support/reply, quoting the ticket number and the reply challenge from that email, and describe the problem in one sentence.",
            S::Support,
            C::SupportReplied,
            1,
        ),
        task(
            "inbox-isolation",
            MultiStep,
            WrongMessage,
            GaveUp,
            "You just signed up as {identity}. Other agents share this mail server, each with its own inbox. Confirm your own account using the confirmation link from your own inbox.",
            S::Signup,
            C::Confirmed,
            3,
        ),
    ]
}

pub fn find(id: &str) -> Option<Task> {
    catalogue().into_iter().find(|t| t.id == id)
}

/// The placeholders a template names, in order; `Err` names a malformed
/// brace.
fn placeholders(template: &str) -> Result<Vec<&str>, String> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let close = after
            .find('}')
            .ok_or_else(|| "an unclosed `{` in a template".to_string())?;
        let name = &after[..close];
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            return Err(format!("a malformed placeholder `{{{name}}}`"));
        }
        out.push(name);
        rest = &after[close + 1..];
    }
    if rest.contains('}') {
        return Err("an unopened `}` in a template".to_string());
    }
    Ok(out)
}

/// Validate one task; the catalogue validator adds uniqueness.
pub fn validate_task(task: &Task) -> Result<(), String> {
    let id = &task.id;
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(format!("task id {id:?} must be lowercase kebab-case"));
    }
    if !matches!(
        task.idle_class,
        FailureClass::GaveUp | FailureClass::WaitTimeout
    ) {
        return Err(format!("{id}: idle_class must be gave_up or wait_timeout"));
    }
    if matches!(
        task.failure_class,
        FailureClass::HarnessError | FailureClass::Budget
    ) {
        return Err(format!(
            "{id}: failure_class names what the task provokes, never budget or harness_error"
        ));
    }
    if task.deadline_ms == 0 || task.deadline_ms > MAX_DEADLINE_MS {
        return Err(format!(
            "{id}: deadline_ms must be between 1 and {MAX_DEADLINE_MS}"
        ));
    }
    if !(1..=3).contains(&task.agent_count) {
        return Err(format!("{id}: agent_count must be between 1 and 3"));
    }
    if task.scenario.uses_chaos() && task.agent_count > 1 {
        return Err(format!(
            "{id}: chaos is server-global and never mixes with a shared server"
        ));
    }
    for name in placeholders(&task.goal)? {
        if !GOAL_PLACEHOLDERS.contains(&name) {
            return Err(format!("{id}: unknown goal placeholder {{{name}}}"));
        }
    }
    let names = placeholders(&task.goal_template)?;
    for name in &names {
        if !TEMPLATE_PLACEHOLDERS.contains(name) {
            return Err(format!("{id}: unknown template placeholder {{{name}}}"));
        }
    }
    for required in TEMPLATE_PLACEHOLDERS {
        if !names.contains(&required) {
            return Err(format!("{id}: the template must name {{{required}}}"));
        }
    }
    Ok(())
}

pub fn validate_catalogue(tasks: &[Task]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for task in tasks {
        validate_task(task)?;
        if !seen.insert(task.id.as_str()) {
            return Err(format!("duplicate task id {}", task.id));
        }
    }
    Ok(())
}

/// The values a prompt is rendered with.
#[derive(Debug, Clone)]
pub struct PromptVars<'a> {
    pub app_url: &'a str,
    pub mcp_url: &'a str,
    pub identity: &'a str,
    pub inbox: &'a str,
}

fn substitute(template: &str, lookup: impl Fn(&str) -> Option<String>) -> Result<String, String> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let close = after
            .find('}')
            .ok_or_else(|| "an unclosed `{` in a template".to_string())?;
        let name = &after[..close];
        out.push_str(&lookup(name).ok_or_else(|| format!("unknown placeholder {{{name}}}"))?);
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Render the task's prompt: the goal first, then the template around it.
/// Values are inserted verbatim and never re-scanned.
pub fn render_prompt(task: &Task, vars: &PromptVars<'_>) -> Result<String, String> {
    let base = |name: &str| match name {
        "app_url" => Some(vars.app_url.to_string()),
        "mcp_url" => Some(vars.mcp_url.to_string()),
        "identity" => Some(vars.identity.to_string()),
        "inbox" => Some(vars.inbox.to_string()),
        _ => None,
    };
    let goal = substitute(&task.goal, base)?;
    substitute(&task.goal_template, |name| {
        if name == "goal" {
            Some(goal.clone())
        } else {
            base(name)
        }
    })
}

/// A trial's outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Pass,
    Fail,
    HarnessError,
}

/// The business verdict over a snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    pub outcome: Outcome,
    pub failure_class: Option<FailureClass>,
}

impl Verdict {
    pub fn pass() -> Verdict {
        Verdict {
            outcome: Outcome::Pass,
            failure_class: None,
        }
    }
    pub fn fail(class: FailureClass) -> Verdict {
        Verdict {
            outcome: Outcome::Fail,
            failure_class: Some(class),
        }
    }
    pub fn harness_error() -> Verdict {
        Verdict {
            outcome: Outcome::HarnessError,
            failure_class: Some(FailureClass::HarnessError),
        }
    }
}

fn accepted_count(snap: &Snapshot, identity: &str, actions: &[&str]) -> usize {
    snap.events
        .iter()
        .filter(|e| {
            e.accepted
                && e.claimed.as_deref() == Some(identity)
                && actions.contains(&e.action.as_str())
        })
        .count()
}

/// Every true fact must be backed by the public-plane event that sets it:
/// a snapshot claiming a success no request produced is malformed.
fn validate_facts(snap: &Snapshot, who: &IdentityState) -> Result<(), String> {
    let f = &who.facts;
    let id = who.identity.as_str();
    if who.role == Role::Decoy && f != &Default::default() {
        return Err(format!("decoy identity {id} carries facts"));
    }
    let backed = |fact: bool, name: &str, actions: &[&str]| -> Result<(), String> {
        if fact && accepted_count(snap, id, actions) == 0 {
            Err(format!("{id}: fact {name} has no accepted event"))
        } else {
            Ok(())
        }
    };
    backed(f.confirmed, "confirmed", &[action::CONFIRM])?;
    backed(f.otp_verified, "otp_verified", &[action::OTP_VERIFY])?;
    backed(
        f.logged_in,
        "logged_in",
        &[action::OTP_VERIFY, action::MAGIC_LOGIN, action::LOGIN],
    )?;
    backed(f.magic_login, "magic_login", &[action::MAGIC_LOGIN])?;
    backed(f.post_reset_login, "post_reset_login", &[action::LOGIN])?;
    backed(
        f.invite_accepted,
        "invite_accepted",
        &[action::INVITE_ACCEPT],
    )?;
    backed(
        f.invite_time_matched,
        "invite_time_matched",
        &[action::INVITE_ACCEPT],
    )?;
    backed(f.reply_received, "reply_received", &[action::SUPPORT_REPLY])?;
    backed(
        f.digest_selected,
        "digest_selected",
        &[action::DIGEST_SELECT],
    )?;
    backed(
        f.marketing_completed,
        "marketing_completed",
        &[action::MARKETING],
    )?;
    backed(f.slow_released, "slow_released", &[action::RELEASE])?;
    if accepted_count(snap, id, &[action::RESET]) != f.reset_count as usize {
        return Err(format!("{id}: reset_count disagrees with its events"));
    }
    if f.reset_replay_rejected
        && !snap.events.iter().any(|e| {
            !e.accepted && e.claimed.as_deref() == Some(id) && e.action == action::RESET_REPLAY
        })
    {
        return Err(format!(
            "{id}: fact reset_replay_rejected has no refused replay"
        ));
    }
    Ok(())
}

/// Whether a snapshot is eligible for business scoring against the task it
/// claims to score: structurally valid, and with no fixture delivery whose
/// final status is `failed` or `parked` (decision 0055 §2, precedence step
/// 1). `Err` is unscorable state — a harness error, never a verdict. A
/// `held` delivery, or one delivered after a transient refusal, is not an
/// infrastructure failure.
pub fn validate_snapshot(task: &Task, snap: &Snapshot) -> Result<(), String> {
    if snap.schema_version != SNAPSHOT_SCHEMA {
        return Err(format!(
            "snapshot schema {} is not {SNAPSHOT_SCHEMA}",
            snap.schema_version
        ));
    }
    if snap.task != task.id {
        return Err(format!(
            "snapshot is for task {}, not {}",
            snap.task, task.id
        ));
    }
    if !snap.frozen {
        return Err("snapshot was not frozen".to_string());
    }
    let agents = snap
        .identities
        .iter()
        .filter(|i| i.role == Role::Agent)
        .count();
    if agents != task.agent_count as usize {
        return Err(format!(
            "snapshot has {agents} agent identities, the task declares {}",
            task.agent_count
        ));
    }
    let mut names = HashSet::new();
    for who in &snap.identities {
        if !names.insert(who.identity.as_str()) {
            return Err(format!("duplicate identity {}", who.identity));
        }
        validate_facts(snap, who)?;
    }
    if !snap.events.windows(2).all(|w| w[0].seq < w[1].seq) {
        return Err("event sequence numbers are not increasing".to_string());
    }
    if !snap.deliveries.windows(2).all(|w| w[0].seq < w[1].seq) {
        return Err("delivery sequence numbers are not increasing".to_string());
    }
    if let Some(d) = snap
        .deliveries
        .iter()
        .find(|d| matches!(d.status, DeliveryStatus::Failed | DeliveryStatus::Parked))
    {
        let last = d.attempts.last().map_or("no attempt".to_string(), |a| {
            format!("{} {}", a.stage, a.code)
        });
        return Err(format!(
            "fixture delivery {} ({} to {}, {:?}) ended {:?} after {last}",
            d.seq, d.message, d.identity, d.flow, d.status
        ));
    }
    Ok(())
}

fn delivered(snap: &Snapshot, identity: &str, flow: Flow) -> bool {
    snap.deliveries
        .iter()
        .any(|d| d.identity == identity && d.flow == flow && d.status == DeliveryStatus::Delivered)
}

fn satisfied(task: &Task, snap: &Snapshot, who: &IdentityState) -> bool {
    let f = &who.facts;
    let id = who.identity.as_str();
    if !delivered(snap, id, task.scenario.flow()) {
        return false;
    }
    match task.check {
        Check::Confirmed => f.confirmed,
        Check::ReleasedAndConfirmed => f.slow_released && f.confirmed,
        Check::RetriedAndConfirmed => {
            f.confirmed
                && snap.deliveries.iter().any(|d| {
                    d.identity == id
                        && d.flow == Flow::Signup
                        && d.status == DeliveryStatus::Delivered
                        && d.attempts.iter().any(|a| (400..500).contains(&a.code))
                        && d.attempts.last().is_some_and(|a| a.code == 250)
                })
        }
        Check::OtpLoggedIn => f.otp_verified && f.logged_in,
        Check::MagicLoggedIn => f.magic_login && f.logged_in,
        Check::MarketingCompleted => f.marketing_completed,
        Check::DigestSelected => f.digest_selected,
        Check::ResetThenLogin => f.reset_count >= 1 && f.post_reset_login,
        Check::ResetReuseRefused => {
            f.reset_count == 1 && f.reset_replay_rejected && f.post_reset_login
        }
        Check::InviteAccepted => f.invite_accepted && f.invite_time_matched,
        Check::SupportReplied => f.reply_received,
    }
}

/// The pure business check (decision 0055 §2, precedence steps 3–5): a
/// satisfied predicate passes; otherwise the first relevant refused action
/// names the class; otherwise the task's declared idle class. Step 2 (the
/// deadline) belongs to the runner; step 1's snapshot part (a failed or
/// parked fixture delivery) is enforced here too, through
/// `validate_snapshot`, so `check` replay agrees with the live trial. `Err`
/// is a malformed, mismatched or unscorable snapshot — a harness error,
/// never a verdict.
pub fn score(task: &Task, snap: &Snapshot) -> Result<Verdict, String> {
    validate_snapshot(task, snap)?;
    let multi_agent = task.agent_count > 1;
    let foreign = snap
        .events
        .iter()
        .any(|e| !e.accepted && e.action == action::FOREIGN_TOKEN);
    let all = snap
        .identities
        .iter()
        .filter(|i| i.role == Role::Agent)
        .all(|who| satisfied(task, snap, who));
    if all && !(multi_agent && foreign) {
        return Ok(Verdict::pass());
    }
    let first = snap.events.iter().find(|e| {
        !e.accepted
            && e.category.is_some()
            && !(task.check == Check::ResetReuseRefused && e.action == action::RESET_REPLAY)
    });
    Ok(match first {
        Some(event) => Verdict::fail(event.category.expect("filtered on category")),
        None => Verdict::fail(task.idle_class),
    })
}
