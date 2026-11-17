//! Protocol 4's API section (accepted decision 0132 §6, story B): fill, the
//! authored fixture ledger, the closed-loop cell scheduler and the exact
//! checks, for any surface whose operations are given as closures. The
//! native surface drives the shared clients (`clients/`); the eval crate's
//! `api-mcp` drives the same scheduler over MCP calls.
//!
//! What a row means (spec AC5, AC9): a planned operation is `started` once
//! a worker took its index before the cell's absolute deadline, and every
//! started operation ends in exactly one terminal row: `exact`,
//! `inexact_fidelity`, `wrong_result`, `failed` or `timeout`. Untaken slots
//! are `unstarted`. Planned = started + unstarted; started = exact +
//! failures (inexact counted with the failures, its latency kept apart).
//! Quantiles are nearest rank over exact successes only.

use crate::clients::{
    self, Capability, ClearScope, Family, Fetched, Method, OpError, Operation, Scope, Spend,
    Summary, Surface,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::time::Instant;

/// The store fill levels (0132 §6.4).
pub const FILLS: [u32; 4] = [0, 1_000, 10_000, 100_000];
/// Client concurrency (0132 §6.5, acceptance answer 1).
pub const CONCURRENCY: [u32; 3] = [1, 8, 32];
/// Warmup operations per cell, a cell total, discarded (0132 §6.5).
pub const WARMUP: u32 = 20;
/// Measured operations per cell (0132 §6.5).
pub const MEASURED: u32 = 100;
/// A cell's measured window ends at this absolute deadline or at its last
/// operation, whichever comes first; its warmup has its own of the same.
pub const CELL_DEADLINE: Duration = Duration::from_secs(5);
/// A fill level not confirmed retained by then is `fill_not_reached`.
pub const FILL_DEADLINE: Duration = Duration::from_secs(300);
/// Reused SMTP sessions that send the fill (0132 §6.4).
pub const FILL_SESSIONS: usize = 8;
/// Background recipients the fill goes to, round robin (0132 §6.4).
pub const BACKGROUND: u32 = 100;
/// Mail of the measured recipient (0132 §6.1's K).
pub const K: u32 = 10;
/// Survivors beside each delete cell's victims, and the size of each
/// recipient-clear scope and of the sentinel scope.
pub const SURVIVORS: u32 = 10;
/// The label of the fixed link (the eval fixture's own template).
pub const LINK_LABEL: &str = "Confirm your account:";
/// The fixtures' sender.
pub const SENDER: &str = "sender@api.mail-compare.invalid";
/// The ceiling every launch keeps below (0132 §2.4).
pub const CAP: u64 = 150_000;

/// What a fixture is for (spec "Data model").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// The stable ten-message set every read cell reads.
    Read,
    /// A delete cell's warmup or measured victim.
    Victim,
    /// Mail beside the victims that must survive.
    Survivor,
    /// A recipient-clear cell's own ten-message scope.
    ClearScope,
    /// The ten-message scope no clear may touch.
    Sentinel,
    /// A future wait's own message, authored at its attempt (MCP).
    Future,
}

/// One authored fixture: everything an exact check compares, written before
/// the server sees it, and the id the server gave it once resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Authored {
    pub token: String,
    pub role: Role,
    pub scope: Scope,
    pub subject: String,
    pub from: String,
    pub text: String,
    pub html: String,
    pub code: String,
    pub link: String,
    pub id: Option<String>,
}

/// SplitMix64, as the wire corpus uses it.
fn mix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The `code_link` corpus kind (0132 §6.5): a multipart message built from
/// the eval fixture's templates, with one six-digit code and one link under
/// [`LINK_LABEL`]. The token carries five digits, so the code is the only
/// six-digit run of the text. Deterministic in `(seed, role, n)`.
pub fn author(seed: u64, role: Role, n: u32, scope: &Scope) -> Authored {
    let letter = match role {
        Role::Read => 'r',
        Role::Victim => 'v',
        Role::Survivor => 's',
        Role::ClearScope => 'c',
        Role::Sentinel => 'n',
        Role::Future => 'f',
    };
    let token = format!("apx{letter}{n:05}q");
    let mut state = seed ^ u64::from(n).wrapping_mul(0xA24B_AED4_963E_E407) ^ (letter as u64);
    let code = format!("{}", 100_000 + mix(&mut state) % 900_000);
    let link = format!("http://app.api.mail-compare.invalid/confirm/{token}");
    let subject = format!("api-fixture {token} verify your account");
    let text = format!(
        "Hello {local},\n\nYour verification code is {code}.\n\n{LINK_LABEL} {link}\n\nReference {token}.\n",
        local = scope.local()
    );
    let html = format!(
        "<p>Hello {local},</p><p>Your verification code is <b>{code}</b>.</p><p>{LINK_LABEL} <a href=\"{link}\">{link}</a></p><p>Reference {token}.</p>",
        local = scope.local()
    );
    Authored {
        token,
        role,
        scope: scope.clone(),
        subject,
        from: SENDER.to_owned(),
        text,
        html,
        code,
        link,
        id: None,
    }
}

/// The RFC 5322 bytes of an authored fixture, CRLF throughout.
pub fn fixture_bytes(a: &Authored) -> Vec<u8> {
    let b = format!("b-{}", a.token);
    let crlf = |s: &str| s.replace('\n', "\r\n");
    format!(
        "From: <{from}>\r\nTo: <{to}>\r\nSubject: {subject}\r\nMessage-ID: <{token}@api.mail-compare.invalid>\r\nDate: Wed, 28 Oct 2026 00:00:00 +0000\r\nMIME-Version: 1.0\r\nContent-Type: multipart/alternative; boundary=\"{b}\"\r\n\r\n--{b}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{text}\r\n--{b}\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{html}\r\n--{b}--\r\n",
        from = a.from,
        to = a.scope.address,
        subject = a.subject,
        token = a.token,
        text = crlf(&a.text),
        html = a.html,
    )
    .into_bytes()
}

/// The measured recipient's scope.
pub fn read_scope() -> Scope {
    Scope::new("apiread00x")
}

/// Background recipient `n` (0..100), never a measured one.
pub fn background_scope(n: u32) -> Scope {
    Scope::new(&format!("apibg{n:05}x"))
}

/// The sentinel scope no recipient clear may touch.
pub fn sentinel_scope() -> Scope {
    Scope::new("apisent00x")
}

/// Delete cell `c`'s own scope (victims and survivors).
pub fn delete_scope(c: u32) -> Scope {
    Scope::new(&format!("apidel{c:04}x"))
}

/// Recipient-clear cell `c`'s scope `s`.
pub fn clear_scope(c: u32, s: u32) -> Scope {
    Scope::new(&format!("apiclr{c:01}{s:04}x"))
}

/// One planned cell: an operation at a concurrency, or the one global clear.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedCell {
    pub operation: Operation,
    /// `None` for the global clear: one invocation, no replication.
    pub concurrency: Option<u32>,
    pub warmup: u32,
    pub planned: u32,
    /// Clears only: what the documented clear removes.
    pub scope: Option<ClearScope>,
    /// Destructive cells: their index among their operation's cells, which
    /// names their own fixture scope.
    pub ordinal: u32,
}

impl PlannedCell {
    /// `list.c8`, `clear.all`.
    pub fn name(&self) -> String {
        match self.concurrency {
            Some(c) => format!("{}.c{c}", self.operation.name()),
            None => format!("{}.all", self.operation.name()),
        }
    }
}

/// A (target, surface, operation) cell that is published without a
/// measurement: its capability and why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityRow {
    pub operation: Operation,
    pub status: String,
    pub detail: String,
    pub cites: Vec<String>,
}

fn cite_list(c: &Capability) -> Vec<String> {
    c.cites()
        .iter()
        .map(|c| format!("{} :: {}", c.doc, c.needle))
        .collect()
}

/// The fixed order of a launch's cells (spec AC5, AC6): every read cell at
/// every concurrency, then the future wait (MCP only), then deletes, then
/// recipient clears, then the single global clear where that is the
/// narrowest documented clear. An operation that is not supported is a
/// published capability row, never a cell.
pub fn plan_cells(
    surface: Surface,
    capability: impl Fn(Operation) -> Capability,
) -> (Vec<PlannedCell>, Vec<CapabilityRow>) {
    let mut cells = Vec::new();
    let mut rows = Vec::new();
    let mut ops = vec![
        Operation::List,
        Operation::Search,
        Operation::Get,
        Operation::ExtractCode,
        Operation::ExtractLink,
    ];
    if surface == Surface::Mcp {
        ops.push(Operation::Wait);
    }
    ops.extend([Operation::Delete, Operation::Clear]);
    for op in ops {
        let cap = capability(op);
        if !cap.supported() {
            let detail = match &cap {
                Capability::NotSupported { reason, .. } => (*reason).to_owned(),
                Capability::NotInspected { why } => (*why).to_owned(),
                other => other.status().to_owned(),
            };
            rows.push(CapabilityRow {
                operation: op,
                status: cap.status().to_owned(),
                detail,
                cites: cite_list(&cap),
            });
            continue;
        }
        if op == Operation::Clear && cap.scope() == Some(ClearScope::All) {
            cells.push(PlannedCell {
                operation: op,
                concurrency: None,
                warmup: 0,
                planned: 1,
                scope: Some(ClearScope::All),
                ordinal: 0,
            });
            continue;
        }
        for (ordinal, c) in CONCURRENCY.into_iter().enumerate() {
            cells.push(PlannedCell {
                operation: op,
                concurrency: Some(c),
                warmup: WARMUP,
                planned: MEASURED,
                scope: cap.scope(),
                ordinal: ordinal as u32,
            });
        }
    }
    (cells, rows)
}

/// Every fixture the plan needs before its first cell, authored now and
/// independently of any response (spec AC3): the read set, each delete
/// cell's victims and survivors, each recipient-clear cell's scopes and the
/// sentinel. Future-wait fixtures are authored at their own attempts.
pub fn plan_fixtures(seed: u64, cells: &[PlannedCell]) -> Vec<Authored> {
    let mut out = Vec::new();
    let mut n = 0u32;
    let mut push = |out: &mut Vec<Authored>, role, scope: &Scope| {
        out.push(author(seed, role, n, scope));
        n += 1;
    };
    let reads = cells
        .iter()
        .any(|c| !c.operation.destructive() && c.operation != Operation::Wait);
    let destructive = cells.iter().any(|c| c.operation.destructive());
    if reads || destructive {
        for _ in 0..K {
            push(&mut out, Role::Read, &read_scope());
        }
    }
    for c in cells.iter().filter(|c| c.operation == Operation::Delete) {
        let scope = delete_scope(c.ordinal);
        for _ in 0..c.warmup + c.planned {
            push(&mut out, Role::Victim, &scope);
        }
        for _ in 0..SURVIVORS {
            push(&mut out, Role::Survivor, &scope);
        }
    }
    let recipient_clears: Vec<&PlannedCell> = cells
        .iter()
        .filter(|c| c.operation == Operation::Clear && c.scope == Some(ClearScope::Recipient))
        .collect();
    for c in &recipient_clears {
        for s in 0..c.warmup + c.planned {
            let scope = clear_scope(c.ordinal, s);
            for _ in 0..SURVIVORS {
                push(&mut out, Role::ClearScope, &scope);
            }
        }
    }
    if !recipient_clears.is_empty() {
        for _ in 0..SURVIVORS {
            push(&mut out, Role::Sentinel, &sentinel_scope());
        }
    }
    out
}

/// Every message the launch will hold beyond its fill: the fixtures, plus
/// one per future-wait attempt (warmup and measured). Computed before
/// launch, so fill plus this stays below the cap (spec AC6).
pub fn fixture_growth(seed: u64, cells: &[PlannedCell]) -> u64 {
    let waits: u64 = cells
        .iter()
        .filter(|c| c.operation == Operation::Wait)
        .map(|c| u64::from(c.warmup + c.planned))
        .sum();
    plan_fixtures(seed, cells).len() as u64 + waits
}

/// Refuse a fill whose total would reach the cap: pruning is never worked
/// around, it is kept from happening.
pub fn within_cap(fill: u32, growth: u64) -> Result<(), String> {
    let total = u64::from(fill) + growth;
    if total < CAP {
        Ok(())
    } else {
        Err(format!(
            "fill {fill} plus {growth} fixtures is {total}, not below the cap {CAP}"
        ))
    }
}

/// How one started operation ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
    Exact,
    /// A discrepancy v3's fidelity records already document: its latency is
    /// kept, unranked, and the cell is not at equal results.
    InexactFidelity {
        detail: String,
    },
    WrongResult {
        detail: String,
    },
    Failed {
        reason: String,
    },
    /// The operation's budget ran out with its request unresolved: the
    /// server may still have acted on it.
    Timeout,
}

impl Outcome {
    pub fn class(&self) -> &'static str {
        match self {
            Outcome::Exact => "exact",
            Outcome::InexactFidelity { .. } => "inexact_fidelity",
            Outcome::WrongResult { .. } => "wrong_result",
            Outcome::Failed { .. } => "failed",
            Outcome::Timeout => "timeout",
        }
    }
}

/// An operation error as an outcome.
pub fn from_error(e: OpError) -> Outcome {
    match e {
        OpError::Timeout => Outcome::Timeout,
        other => Outcome::Failed {
            reason: other.reason(),
        },
    }
}

/// What an operation closure hands back.
#[derive(Debug, Clone)]
pub struct Done {
    pub outcome: Outcome,
    pub spend: Spend,
    /// A future wait's instants (MCP only).
    pub wait: Option<WaitTimes>,
}

impl Done {
    pub fn of(outcome: Outcome, spend: Spend) -> Done {
        Done {
            outcome,
            spend,
            wait: None,
        }
    }
}

/// A future wait's instants, all on the run's one monotonic clock (spec
/// AC7): the wait call's start and return, the SMTP send's start and its
/// `250`. `ack_to_return_ns` is signed: a return before the acknowledgement
/// is kept as the negative offset it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitTimes {
    pub call_ns: u64,
    pub send_ns: u64,
    pub ack_ns: Option<u64>,
    pub return_ns: u64,
    pub ack_to_return_ns: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Warmup,
    Measured,
}

/// One terminal row: one started operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpRow {
    pub phase: Phase,
    pub index: u32,
    pub worker: u32,
    pub start_ns: u64,
    pub end_ns: u64,
    #[serde(flatten)]
    pub outcome: Outcome,
    pub requests: u32,
    pub bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait: Option<WaitTimes>,
}

/// One phase of one cell, as the scheduler ran it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Ran {
    pub rows: Vec<OpRow>,
    pub planned: u32,
    pub started: u32,
    pub unstarted: u32,
    /// The client's own interval: the instant before the first index was
    /// offered and the instant the last worker returned.
    pub start_ns: u64,
    pub end_ns: u64,
    /// Some operation ran out of budget with its request in flight.
    pub unresolved: bool,
}

/// A boxed operation future.
pub type OpFuture = Pin<Box<dyn Future<Output = Done> + Send>>;

/// The run's monotonic clock (the wire driver's own `Clock` shape, kept
/// here so the eval crate compiles this module without the driver's).
#[derive(Debug, Clone, Copy)]
pub struct Clock(pub std::time::Instant);

impl Clock {
    pub fn now_ns(&self) -> u64 {
        self.0.elapsed().as_nanos() as u64
    }
}

/// Run `planned` operations on `workers` closed-loop workers, each taking
/// the next index from one shared queue, until the queue is empty or the
/// absolute deadline `budget` from now passes. A worker never starts an
/// index at or after the deadline, and every operation gets only what
/// remains of it. Every taken index ends in exactly one row.
pub async fn run_cell<F>(
    clock: Clock,
    phase: Phase,
    workers: u32,
    planned: u32,
    budget: Duration,
    op: Arc<F>,
) -> Ran
where
    F: Fn(u32, u32, Instant) -> OpFuture + Send + Sync + 'static,
{
    let start_ns = clock.now_ns();
    let cutoff = Instant::now() + budget;
    let next = Arc::new(AtomicU32::new(0));
    let mut set = tokio::task::JoinSet::new();
    for worker in 0..workers.max(1) {
        let (op, next) = (op.clone(), next.clone());
        set.spawn(async move {
            let mut rows = Vec::new();
            loop {
                if Instant::now() >= cutoff {
                    break;
                }
                let index = next.fetch_add(1, Ordering::SeqCst);
                if index >= planned {
                    break;
                }
                let start = clock.now_ns();
                let done = match tokio::time::timeout_at(cutoff, op(worker, index, cutoff)).await {
                    Ok(d) => d,
                    Err(_) => Done::of(Outcome::Timeout, Spend::default()),
                };
                rows.push(OpRow {
                    phase,
                    index,
                    worker,
                    start_ns: start,
                    end_ns: clock.now_ns(),
                    outcome: done.outcome,
                    requests: done.spend.requests,
                    bytes: done.spend.bytes,
                    wait: done.wait,
                });
            }
            rows
        });
    }
    let mut rows = Vec::new();
    while let Some(r) = set.join_next().await {
        // A worker that panicked loses no accounting silently: the panic is
        // the driver's bug, and it propagates.
        rows.extend(r.expect("a cell worker never panics"));
    }
    rows.sort_by_key(|r| r.index);
    let started = rows.len() as u32;
    Ran {
        unresolved: rows.iter().any(|r| r.outcome == Outcome::Timeout),
        rows,
        planned,
        started,
        unstarted: planned - started,
        start_ns,
        end_ns: clock.now_ns(),
    }
}

/// Nearest-rank quantile over sorted values (0091 §5's method).
pub fn nearest_rank(sorted: &[u64], p: u32) -> Option<u64> {
    if sorted.is_empty() {
        return None;
    }
    let n = sorted.len() as u64;
    let rank = (u64::from(p) * n).div_ceil(100).max(1);
    sorted.get((rank - 1) as usize).copied()
}

/// One cell's published figures (spec AC9), derived from its rows and its
/// resource window, so admission can recompute every one of them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellStats {
    pub planned: u32,
    pub started: u32,
    pub unstarted: u32,
    pub exact: u32,
    pub inexact_fidelity: u32,
    pub wrong_result: u32,
    pub failed: u32,
    pub timeout: u32,
    pub p50_ns: Option<u64>,
    pub p95_ns: Option<u64>,
    pub p99_ns: Option<u64>,
    /// Kept apart from the quantiles: the latencies of inexact answers.
    pub inexact_ns: Vec<u64>,
    /// Exact successes per second of the measured client window.
    pub exact_per_second: Option<f64>,
    /// The window's raw target CPU delta over started operations, labelled
    /// per started operation; `None` for no window or nothing started.
    pub cpu_us_per_started: Option<f64>,
    pub client_window_ns: u64,
}

/// The figures of one measured phase. `cpu_usec` is the resource window's
/// raw `usage_usec` delta, when the window was opened.
pub fn stats(ran: &Ran, cpu_usec: Option<u64>) -> CellStats {
    let mut exact: Vec<u64> = Vec::new();
    let mut inexact = Vec::new();
    let mut count = BTreeMap::<&str, u32>::new();
    for r in &ran.rows {
        let d = r.end_ns.saturating_sub(r.start_ns);
        *count.entry(r.outcome.class()).or_default() += 1;
        match r.outcome {
            Outcome::Exact => exact.push(d),
            Outcome::InexactFidelity { .. } => inexact.push(d),
            _ => {}
        }
    }
    exact.sort_unstable();
    let window = ran.end_ns.saturating_sub(ran.start_ns);
    let c = |k: &str| count.get(k).copied().unwrap_or(0);
    CellStats {
        planned: ran.planned,
        started: ran.started,
        unstarted: ran.unstarted,
        exact: c("exact"),
        inexact_fidelity: c("inexact_fidelity"),
        wrong_result: c("wrong_result"),
        failed: c("failed"),
        timeout: c("timeout"),
        p50_ns: nearest_rank(&exact, 50),
        p95_ns: nearest_rank(&exact, 95),
        p99_ns: nearest_rank(&exact, 99),
        inexact_ns: inexact,
        exact_per_second: (window > 0).then(|| exact.len() as f64 / (window as f64 / 1e9)),
        cpu_us_per_started: cpu_usec
            .filter(|_| ran.started > 0)
            .map(|u| u as f64 / f64::from(ran.started)),
        client_window_ns: window,
    }
}

/// Body comparison that ignores only the transport's line-ending and
/// trailing-whitespace choices (the wire driver's `same_body`).
pub fn same_body(authored: Option<&str>, observed: Option<&str>) -> bool {
    let norm = |s: &str| s.replace("\r\n", "\n").trim_end().to_owned();
    match (authored, observed) {
        (None, None) => true,
        (Some(a), Some(o)) => norm(a) == norm(o),
        (None, Some(o)) => o.trim().is_empty(),
        (Some(_), None) => false,
    }
}

/// `list`: exactly the expected ids, no duplicate, none missing or extra.
pub fn check_list(got: Result<Vec<Summary>, OpError>, expected: &BTreeSet<String>) -> Outcome {
    let got = match got {
        Ok(g) => g,
        Err(e) => return from_error(e),
    };
    let mut ids = BTreeSet::new();
    for s in &got {
        if !ids.insert(s.id.clone()) {
            return Outcome::WrongResult {
                detail: format!("duplicate id {}", s.id),
            };
        }
    }
    if &ids == expected {
        Outcome::Exact
    } else {
        Outcome::WrongResult {
            detail: format!(
                "{} ids, {} missing, {} extra",
                ids.len(),
                expected.difference(&ids).count(),
                ids.difference(expected).count()
            ),
        }
    }
}

/// `search`: exactly the one expected id.
pub fn check_search(got: Result<Vec<Summary>, OpError>, expected: &str) -> Outcome {
    match got {
        Err(e) => from_error(e),
        Ok(g) if g.len() == 1 && g[0].id == expected => Outcome::Exact,
        Ok(g) => Outcome::WrongResult {
            detail: format!("{} hits", g.len()),
        },
    }
}

/// The discrepancies v3's fidelity records document, per family: only
/// these may be `inexact_fidelity`. MailSlurper keeps one decoded body per
/// message and no raw source, so a part it does not keep is documented.
pub fn documented_discrepancy(family: Family, field: &str) -> bool {
    family == Family::Mailslurper && matches!(field, "text" | "html")
}

/// `get`: subject, from, to, text and HTML as authored (0132 §6.1).
pub fn check_get(family: Family, got: Result<Fetched, OpError>, a: &Authored) -> Outcome {
    let f = match got {
        Ok(f) => f,
        Err(e) => return from_error(e),
    };
    let mut wrong = Vec::new();
    if f.subject.as_deref() != Some(a.subject.as_str()) {
        wrong.push("subject");
    }
    if !f
        .from
        .as_deref()
        .is_some_and(|x| x.eq_ignore_ascii_case(&a.from))
    {
        wrong.push("from");
    }
    if f.to.len() != 1 || !f.to[0].eq_ignore_ascii_case(&a.scope.address) {
        wrong.push("to");
    }
    if !same_body(Some(&a.text), f.text.as_deref()) {
        wrong.push("text");
    }
    if !same_body(Some(&a.html), f.html.as_deref()) {
        wrong.push("html");
    }
    if wrong.is_empty() {
        Outcome::Exact
    } else if wrong.iter().all(|w| documented_discrepancy(family, w)) {
        Outcome::InexactFidelity {
            detail: wrong.join(","),
        }
    } else {
        Outcome::WrongResult {
            detail: wrong.join(","),
        }
    }
}

/// `extract_code` and `extract_link`: exactly the authored value.
pub fn check_value(got: Result<Option<String>, OpError>, want: &str) -> Outcome {
    match got {
        Err(e) => from_error(e),
        Ok(Some(v)) if v == want => Outcome::Exact,
        Ok(Some(_)) => Outcome::WrongResult {
            detail: "a different value".into(),
        },
        Ok(None) => Outcome::WrongResult {
            detail: "no value".into(),
        },
    }
}

/// Invalidate every exact row of `rows` that `bad` names (by index), or
/// every one when `bad` is `None`: an uncertain postcheck never leaves an
/// exact success standing.
fn invalidate(rows: &mut [OpRow], bad: Option<&BTreeSet<u32>>, detail: &str) {
    for r in rows.iter_mut() {
        if r.outcome == Outcome::Exact && bad.is_none_or(|b| b.contains(&r.index)) {
            r.outcome = Outcome::WrongResult {
                detail: detail.to_owned(),
            };
        }
    }
}

/// The untimed batch check after a delete cell (spec AC6). `victims[i]` is
/// the id row `i` deleted, in one index space across warmup and measured
/// rows (warmup first). A successful delete whose victim is still retained
/// is wrong; a missing survivor or unstarted victim, or any extra id,
/// invalidates every success of the cell: the scope was changed by
/// something other than the cell's own deletes. A failed delete's victim
/// may be either way. Returns the problems found.
pub fn postcheck_delete(
    retained: &BTreeSet<String>,
    survivors: &BTreeSet<String>,
    victims: &[String],
    rows: &mut [OpRow],
    warmup: u32,
) -> Vec<String> {
    let mut problems = Vec::new();
    let slot = |r: &OpRow| match r.phase {
        Phase::Warmup => r.index as usize,
        Phase::Measured => (warmup + r.index) as usize,
    };
    let started: BTreeMap<usize, bool> = rows
        .iter()
        .map(|r| (slot(r), r.outcome == Outcome::Exact))
        .collect();
    let mut still = BTreeSet::new();
    for r in rows.iter() {
        if r.outcome == Outcome::Exact && retained.contains(&victims[slot(r)]) {
            still.insert(slot(r));
        }
    }
    if !still.is_empty() {
        problems.push(format!("{} deleted victims still retained", still.len()));
    }
    for r in rows.iter_mut() {
        if still.contains(&slot(r)) {
            r.outcome = Outcome::WrongResult {
                detail: "victim still retained".into(),
            };
        }
    }
    let known: BTreeSet<&String> = survivors.iter().chain(victims.iter()).collect();
    let missing_survivor = survivors.iter().any(|s| !retained.contains(s));
    let missing_unstarted = victims
        .iter()
        .enumerate()
        .any(|(i, v)| !started.contains_key(&i) && !retained.contains(v));
    let extra = retained.iter().any(|id| !known.contains(id));
    if missing_survivor || missing_unstarted || extra {
        problems.push(format!(
            "scope changed outside the cell's deletes (survivor missing {missing_survivor}, unstarted victim missing {missing_unstarted}, extra id {extra})"
        ));
        invalidate(rows, None, "scope changed outside the cell's own deletes");
    }
    problems
}

/// The untimed batch check after a recipient-clear cell: each successfully
/// cleared scope is empty; every unstarted scope still holds its ten; and
/// `outside` (the sentinel, the read set, the background) holds exactly what
/// it held. `counts[s]` is scope `s`'s count after the cell, one index space
/// across warmup and measured rows.
pub fn postcheck_clear(
    counts: &[u64],
    rows: &mut [OpRow],
    warmup: u32,
    outside_intact: bool,
) -> Vec<String> {
    let mut problems = Vec::new();
    let slot = |r: &OpRow| match r.phase {
        Phase::Warmup => r.index as usize,
        Phase::Measured => (warmup + r.index) as usize,
    };
    let started: BTreeSet<usize> = rows.iter().map(slot).collect();
    let bad: BTreeSet<u32> = rows
        .iter()
        .filter(|r| r.outcome == Outcome::Exact && counts[slot(r)] != 0)
        .map(|r| r.index)
        .collect();
    if !bad.is_empty() {
        problems.push(format!("{} cleared scopes still hold mail", bad.len()));
        for r in rows.iter_mut() {
            if r.outcome == Outcome::Exact && counts[slot(r)] != 0 {
                r.outcome = Outcome::WrongResult {
                    detail: "scope still holds mail".into(),
                };
            }
        }
    }
    let untouched_ok = counts
        .iter()
        .enumerate()
        .all(|(s, n)| started.contains(&s) || *n == u64::from(SURVIVORS));
    if !untouched_ok || !outside_intact {
        problems.push(format!(
            "mail outside the cleared scopes changed (unstarted scopes intact {untouched_ok}, outside intact {outside_intact})"
        ));
        invalidate(rows, None, "mail outside the cleared scope changed");
    }
    problems
}

// ============================================================================
// Fill and fixtures (spec AC4)
// ============================================================================

const WORDS: [&str; 16] = [
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india", "juliet",
    "kilo", "lima", "mike", "november", "oscar", "papa",
];

fn sentence(state: &mut u64, words: usize) -> String {
    (0..words)
        .map(|_| WORDS[(mix(state) % 16) as usize])
        .collect::<Vec<_>>()
        .join(" ")
}

/// Fill message `index`: the v3 corpus (`wire::authored` with kinds plain,
/// html, multipart cycled, byte for byte, which a test proves) addressed to
/// `to` instead of the corpus's one recipient.
pub fn fill_bytes(seed: u64, run_tag: &str, index: u32, to: &str) -> Vec<u8> {
    let token = format!("mc-{run_tag}-{index:06}");
    let mut state = seed ^ u64::from(index).wrapping_mul(0xA24B_AED4_963E_E407);
    let subject = format!("mail-compare {token} {}", sentence(&mut state, 3));
    let words = sentence(&mut state, 24);
    let text = format!("{token} {words}");
    let html = format!("<p>{token}</p><p>{words}</p>");
    let mut head = format!(
        "From: <sender@mail-compare.invalid>\r\nTo: <{to}>\r\nSubject: {subject}\r\nMessage-ID: <{token}@mail-compare.invalid>\r\nDate: Wed, 28 Oct 2026 00:00:00 +0000\r\nMIME-Version: 1.0\r\n"
    );
    let body = match index % 3 {
        0 => {
            head.push_str("Content-Type: text/plain; charset=utf-8\r\n");
            format!("{text}\r\n")
        }
        1 => {
            head.push_str("Content-Type: text/html; charset=utf-8\r\n");
            format!("{html}\r\n")
        }
        _ => {
            let b = format!("b-{token}");
            head.push_str(&format!(
                "Content-Type: multipart/alternative; boundary=\"{b}\"\r\n"
            ));
            format!(
                "--{b}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{text}\r\n--{b}\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{html}\r\n--{b}--\r\n"
            )
        }
    };
    format!("{head}\r\n{body}").into_bytes()
}

/// How a fill level ended (0132 §6.4).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FillRecord {
    pub fill: u32,
    pub accepted: u64,
    pub refused: u64,
    pub failed: u64,
    pub ambiguous: u64,
    /// The retained count the store confirmed, untimed; `None` when it
    /// could not be read completely within the bound.
    pub retained: Option<u64>,
    pub reached: bool,
    pub elapsed_ns: u64,
    pub detail: Option<String>,
}

/// One SMTP exchange's bound while filling or preparing fixtures.
pub const SMTP_STEP: Duration = Duration::from_secs(10);
/// An untimed verification listing's bound.
pub const VERIFY_DEADLINE: Duration = Duration::from_secs(120);

/// Every message `client`'s store holds outside the API fixtures: the fill.
/// Swarmail's unauthenticated sessions deliver to its `default` inbox (0004);
/// a per-mailbox store sums the background mailboxes; a shared store is
/// scanned whole.
pub async fn background_count(client: &clients::Client, deadline: Instant) -> Result<u64, OpError> {
    let mut spend = Spend::default();
    match client.family {
        Family::Swarmail => {
            let default = Scope {
                address: String::new(),
                inbox: "default".into(),
            };
            Ok(
                clients::swarmail::list(&client.conn, &default, None, deadline, &mut spend)
                    .await?
                    .len() as u64,
            )
        }
        Family::Inbucket | Family::Greenmail => {
            let mut n = 0;
            for b in 0..BACKGROUND {
                n += client
                    .list(&background_scope(b), deadline, &mut spend)
                    .await?
                    .len() as u64;
            }
            Ok(n)
        }
        _ => Ok(client
            .all_ids(deadline)
            .await?
            .map(|ids| ids.len() as u64)
            .unwrap_or(0)),
    }
}

/// Send `n` fill messages over [`FILL_SESSIONS`] reused sessions, round robin
/// over the [`BACKGROUND`] recipients, then confirm the retained count, all
/// within [`FILL_DEADLINE`]. A refused, failed or ambiguous send is never
/// retried and the fill is never reduced: the level is then not reached.
/// `fixtures` is how many API fixtures the store already holds (none, when
/// the fill comes first).
pub async fn fill(
    smtp: std::net::SocketAddr,
    client: &clients::Client,
    (seed, run_tag): (u64, &str),
    n: u32,
    clock: Clock,
) -> FillRecord {
    let start = clock.now_ns();
    let deadline = Instant::now() + FILL_DEADLINE;
    let mut rec = FillRecord {
        fill: n,
        ..FillRecord::default()
    };
    let mut set = tokio::task::JoinSet::new();
    for s in 0..FILL_SESSIONS as u32 {
        let tag = run_tag.to_owned();
        set.spawn(async move {
            let mut tally = [0u64; 4];
            let mut session = None;
            let mut i = s;
            while i < n {
                if Instant::now() >= deadline {
                    break;
                }
                if session.is_none() {
                    match clients::smtp::Session::open(smtp, SMTP_STEP).await {
                        Ok(x) => session = Some(x),
                        Err(_) => {
                            tally[2] += 1;
                            i += FILL_SESSIONS as u32;
                            continue;
                        }
                    }
                }
                let sess = session.as_mut().expect("opened above");
                let to = background_scope(i % BACKGROUND).address;
                let bytes = fill_bytes(seed, &tag, i, &to);
                let left = deadline.saturating_duration_since(Instant::now());
                let sent = tokio::time::timeout(left, sess.send(SENDER, &to, &bytes))
                    .await
                    .unwrap_or(clients::smtp::Sent::Ambiguous);
                match sent {
                    clients::smtp::Sent::Accepted => tally[0] += 1,
                    clients::smtp::Sent::Refused(_) => tally[1] += 1,
                    clients::smtp::Sent::Failed => {
                        tally[2] += 1;
                        session = None;
                    }
                    clients::smtp::Sent::Ambiguous => {
                        tally[3] += 1;
                        session = None;
                    }
                }
                i += FILL_SESSIONS as u32;
            }
            if let Some(s) = session {
                s.quit().await;
            }
            tally
        });
    }
    while let Some(t) = set.join_next().await {
        let t = t.expect("a fill session never panics");
        rec.accepted += t[0];
        rec.refused += t[1];
        rec.failed += t[2];
        rec.ambiguous += t[3];
    }
    match background_count(client, deadline).await {
        Ok(c) => rec.retained = Some(c),
        Err(e) => rec.detail = Some(format!("retained count unreadable: {}", e.reason())),
    }
    rec.reached = rec.accepted == u64::from(n) && rec.retained == Some(u64::from(n));
    if !rec.reached && rec.detail.is_none() {
        rec.detail = Some(format!(
            "accepted {} retained {:?} of {n} within {} s",
            rec.accepted,
            rec.retained,
            FILL_DEADLINE.as_secs()
        ));
    }
    rec.elapsed_ns = clock.now_ns() - start;
    rec
}

/// Send every fixture and resolve its id. Swarmail's fixtures go one AUTH
/// session per scope, into that scope's inbox (0004); everyone else's over
/// [`FILL_SESSIONS`] plain sessions. Ids are resolved by listing each scope
/// untimed and matching the authored token in the subject: the server tells
/// the client which id holds which token, never what the message contains.
/// Every token must appear exactly once and nothing else may be there.
pub async fn prepare(
    smtp: std::net::SocketAddr,
    client: &clients::Client,
    fixtures: &mut [Authored],
) -> Result<(), String> {
    let mut by_scope: BTreeMap<Scope, Vec<usize>> = BTreeMap::new();
    for (i, f) in fixtures.iter().enumerate() {
        by_scope.entry(f.scope.clone()).or_default().push(i);
    }
    let swarmail = client.family == Family::Swarmail;
    let mut plain: Vec<Option<clients::smtp::Session>> = (0..FILL_SESSIONS).map(|_| None).collect();
    let mut k = 0usize;
    for (scope, idx) in &by_scope {
        let mut authed = None;
        for &i in idx {
            let sess = if swarmail {
                if authed.is_none() {
                    let mut s = clients::smtp::Session::open(smtp, SMTP_STEP).await?;
                    s.auth(&scope.inbox).await?;
                    authed = Some(s);
                }
                authed.as_mut().expect("opened above")
            } else {
                let slot = &mut plain[k % FILL_SESSIONS];
                k += 1;
                if slot.is_none() {
                    *slot = Some(clients::smtp::Session::open(smtp, SMTP_STEP).await?);
                }
                slot.as_mut().expect("opened above")
            };
            match sess
                .send(SENDER, &scope.address, &fixture_bytes(&fixtures[i]))
                .await
            {
                clients::smtp::Sent::Accepted => {}
                other => {
                    return Err(format!(
                        "fixture {} not accepted: {other:?}",
                        fixtures[i].token
                    ));
                }
            }
        }
        if let Some(s) = authed {
            s.quit().await;
        }
    }
    for s in plain.into_iter().flatten() {
        s.quit().await;
    }
    for (scope, idx) in &by_scope {
        let deadline = Instant::now() + VERIFY_DEADLINE;
        let mut spend = Spend::default();
        let listed = client
            .list(scope, deadline, &mut spend)
            .await
            .map_err(|e| format!("resolving {}: {}", scope.address, e.reason()))?;
        if listed.len() != idx.len() {
            return Err(format!(
                "{} holds {} messages, {} authored",
                scope.address,
                listed.len(),
                idx.len()
            ));
        }
        for &i in idx {
            let token = &fixtures[i].token;
            let hits: Vec<&Summary> = listed
                .iter()
                .filter(|s| s.subject.contains(token.as_str()))
                .collect();
            match hits.as_slice() {
                [one] => fixtures[i].id = Some(one.id.clone()),
                _ => return Err(format!("{token}: {} matches in its scope", hits.len())),
            }
        }
    }
    Ok(())
}

// ============================================================================
// Records (spec AC9)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CellStatus {
    Measured,
    /// Not measured: its warmup failed, its fixtures could not be prepared,
    /// its fill was not reached, an earlier cell left a call unresolved, or
    /// the chunk's deadline came first. Every planned operation is unstarted.
    Blocked,
}

/// One cell as published.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellRecord {
    pub cell: String,
    pub operation: Operation,
    pub concurrency: Option<u32>,
    pub status: CellStatus,
    pub blocked: Option<String>,
    pub method: Option<Method>,
    pub scope: Option<ClearScope>,
    pub routes: Vec<String>,
    pub cites: Vec<String>,
    pub warmup: Option<Ran>,
    pub measured: Ran,
    pub postcheck: Vec<String>,
}

impl CellRecord {
    /// A cell that never ran: every planned operation unstarted.
    pub fn blocked(cell: &PlannedCell, cap: &Capability, why: &str, at_ns: u64) -> CellRecord {
        let mut r = CellRecord::new(cell, cap);
        r.status = CellStatus::Blocked;
        r.blocked = Some(why.to_owned());
        r.measured = Ran {
            planned: cell.planned,
            unstarted: cell.planned,
            start_ns: at_ns,
            end_ns: at_ns,
            ..Ran::default()
        };
        r
    }

    pub fn new(cell: &PlannedCell, cap: &Capability) -> CellRecord {
        let (method, routes) = match cap {
            Capability::Supported { method, routes, .. } => (
                Some(*method),
                routes.iter().map(|r| (*r).to_owned()).collect(),
            ),
            _ => (None, Vec::new()),
        };
        CellRecord {
            cell: cell.name(),
            operation: cell.operation,
            concurrency: cell.concurrency,
            status: CellStatus::Measured,
            blocked: None,
            method,
            scope: cell.scope,
            routes,
            cites: cite_list(cap),
            warmup: None,
            measured: Ran::default(),
            postcheck: Vec::new(),
        }
    }
}

/// One launch's API section: the `api.json` artifact of a chunk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiRecord {
    pub surface: Surface,
    pub target: String,
    pub repetition: u32,
    pub fill: FillRecord,
    pub fixtures: u64,
    pub growth: u64,
    pub capabilities: Vec<CapabilityRow>,
    pub cells: Vec<CellRecord>,
    /// Why the launch was abandoned after a call it could not resolve.
    pub abandoned: Option<String>,
    /// The store layout the numbers come from (0132 §6.4): whether the fill
    /// shares the measured recipient's listing.
    pub store: String,
}

/// How a family's store relates the fill to the measured recipient.
pub fn store_label(family: Family) -> &'static str {
    match family {
        Family::Swarmail => "per-test inbox: the fill is outside the measured inbox",
        Family::Inbucket | Family::Greenmail => {
            "per-mailbox store: the fill is in other recipients' mailboxes"
        }
        _ => "shared store: listings and scans see the whole fill",
    }
}

// ============================================================================
// One launch's cells, any surface (spec AC5, AC6)
// ============================================================================

/// The resource bounds of each measured window: the native driver reads the
/// target leaf; a test without a leaf records nothing. Setup, warmup and
/// postchecks are outside every window.
pub trait Meter: Send + Sync {
    fn open<'a>(&'a self, cell: &'a str) -> MeterFuture<'a>;
    fn close<'a>(&'a self, cell: &'a str) -> MeterFuture<'a>;
}

/// A window bound's future: the leaf's read, or a control round trip.
pub type MeterFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

/// No resource bounds (deterministic tests and unenforced real proofs).
pub struct NoMeter;

impl Meter for NoMeter {
    fn open<'a>(&'a self, _: &'a str) -> MeterFuture<'a> {
        Box::pin(async { Ok(()) })
    }
    fn close<'a>(&'a self, _: &'a str) -> MeterFuture<'a> {
        Box::pin(async { Ok(()) })
    }
}

/// What a surface supplies for one cell: an operation closure per phase
/// (warmup indices first, measured after, in one fixture index space) and
/// the untimed postcheck of a destructive cell.
pub trait Surfacer: Send + Sync {
    /// The closure for `cell`'s `phase`; `offset` is where the phase's
    /// indices start in the cell's fixture index space.
    fn op(
        &self,
        cell: &PlannedCell,
        offset: u32,
    ) -> Arc<dyn Fn(u32, u32, Instant) -> OpFuture + Send + Sync>;
    /// After a destructive cell: check the batch ledger and rewrite rows.
    fn postcheck<'a>(
        &'a self,
        cell: &'a PlannedCell,
        rows: &'a mut Vec<OpRow>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, String>> + Send + 'a>>;
}

/// Run `cells` in order (spec AC5): each cell's warmup on its own five-second
/// bound, then its measured operations inside the meter's window. A failed
/// warmup blocks that cell; a call left unresolved blocks every later cell
/// and abandons the launch (its caller tears it down). `deadline` is the
/// chunk's: a cell not started by then is blocked.
pub async fn run_cells(
    cells: &[PlannedCell],
    capability: &dyn Fn(Operation) -> Capability,
    surface: &dyn Surfacer,
    meter: &dyn Meter,
    clock: Clock,
    deadline: Option<Instant>,
) -> Result<(Vec<CellRecord>, Option<String>), String> {
    let mut out = Vec::new();
    let mut abandoned: Option<String> = None;
    for cell in cells {
        let cap = capability(cell.operation);
        if let Some(why) = &abandoned {
            out.push(CellRecord::blocked(cell, &cap, why, clock.now_ns()));
            continue;
        }
        if deadline.is_some_and(|d| Instant::now() >= d) {
            out.push(CellRecord::blocked(
                cell,
                &cap,
                "chunk_deadline",
                clock.now_ns(),
            ));
            continue;
        }
        let workers = cell.concurrency.unwrap_or(1);
        let mut rec = CellRecord::new(cell, &cap);
        if cell.warmup > 0 {
            let warm = run_cell(
                clock,
                Phase::Warmup,
                workers,
                cell.warmup,
                CELL_DEADLINE,
                Arc::new(wrap(surface.op(cell, 0))),
            )
            .await;
            let failed = warm.unstarted > 0
                || warm.rows.iter().any(|r| {
                    matches!(
                        r.outcome,
                        Outcome::Failed { .. } | Outcome::Timeout | Outcome::WrongResult { .. }
                    )
                });
            let unresolved = warm.unresolved;
            rec.warmup = Some(warm);
            if unresolved {
                abandoned = Some(format!(
                    "{}: a warmup call was left unresolved",
                    cell.name()
                ));
            }
            if failed {
                let mut b = CellRecord::blocked(cell, &cap, "warmup_failed", clock.now_ns());
                b.warmup = rec.warmup;
                out.push(b);
                continue;
            }
        }
        let name = cell.name();
        meter.open(&name).await?;
        let measured = run_cell(
            clock,
            Phase::Measured,
            workers,
            cell.planned,
            CELL_DEADLINE,
            Arc::new(wrap(surface.op(cell, cell.warmup))),
        )
        .await;
        meter.close(&name).await?;
        if measured.unresolved {
            abandoned = Some(format!(
                "{}: a measured call was left unresolved",
                cell.name()
            ));
        }
        rec.measured = measured;
        if cell.operation.destructive() {
            let mut rows: Vec<OpRow> = rec
                .warmup
                .iter()
                .flat_map(|w| w.rows.clone())
                .chain(rec.measured.rows.clone())
                .collect();
            rec.postcheck = surface.postcheck(cell, &mut rows).await?;
            let (warm, meas): (Vec<OpRow>, Vec<OpRow>) =
                rows.into_iter().partition(|r| r.phase == Phase::Warmup);
            if let Some(w) = rec.warmup.as_mut() {
                w.rows = warm;
            }
            rec.measured.rows = meas;
        }
        out.push(rec);
    }
    Ok((out, abandoned))
}

fn wrap(
    f: Arc<dyn Fn(u32, u32, Instant) -> OpFuture + Send + Sync>,
) -> impl Fn(u32, u32, Instant) -> OpFuture + Send + Sync + 'static {
    move |w, i, d| f(w, i, d)
}

/// The native surface over the shared clients: one keep-alive client per
/// worker, the authored ledger and the cell's own fixtures.
pub struct NativeSurface {
    pub clients: Arc<Vec<clients::Client>>,
    pub fixtures: Arc<Vec<Authored>>,
}

impl NativeSurface {
    fn of(&self, role: Role, scope: Option<&Scope>) -> Vec<Authored> {
        self.fixtures
            .iter()
            .filter(|f| f.role == role && scope.is_none_or(|s| &f.scope == s))
            .cloned()
            .collect()
    }

    fn reads(&self) -> Arc<Vec<Authored>> {
        Arc::new(self.of(Role::Read, None))
    }
}

fn id_of(a: &Authored) -> String {
    a.id.clone().unwrap_or_default()
}

impl Surfacer for NativeSurface {
    fn op(
        &self,
        cell: &PlannedCell,
        offset: u32,
    ) -> Arc<dyn Fn(u32, u32, Instant) -> OpFuture + Send + Sync> {
        let clients = self.clients.clone();
        let reads = self.reads();
        let label = LINK_LABEL;
        match cell.operation {
            Operation::List => {
                let expected: BTreeSet<String> = reads.iter().map(id_of).collect();
                Arc::new(move |w, _i, d| {
                    let (clients, expected) = (clients.clone(), expected.clone());
                    Box::pin(async move {
                        let mut spend = Spend::default();
                        let got = clients[w as usize].list(&read_scope(), d, &mut spend).await;
                        Done::of(check_list(got, &expected), spend)
                    })
                })
            }
            Operation::Search
            | Operation::Get
            | Operation::ExtractCode
            | Operation::ExtractLink => {
                let op = cell.operation;
                Arc::new(move |w, i, d| {
                    let (clients, reads) = (clients.clone(), reads.clone());
                    Box::pin(async move {
                        let a = &reads[((i + offset) % K) as usize];
                        let c = &clients[w as usize];
                        let mut spend = Spend::default();
                        let id = id_of(a);
                        let outcome = match op {
                            Operation::Search => {
                                check_search(c.search(&a.scope, &a.token, d, &mut spend).await, &id)
                            }
                            Operation::Get => {
                                check_get(c.family, c.get(&a.scope, &id, d, &mut spend).await, a)
                            }
                            Operation::ExtractCode => check_value(
                                c.extract_code(&a.scope, &id, d, &mut spend).await,
                                &a.code,
                            ),
                            _ => check_value(
                                c.extract_link(&a.scope, &id, label, d, &mut spend).await,
                                &a.link,
                            ),
                        };
                        Done::of(outcome, spend)
                    })
                })
            }
            Operation::Delete => {
                let victims = Arc::new(self.of(Role::Victim, Some(&delete_scope(cell.ordinal))));
                Arc::new(move |w, i, d| {
                    let (clients, victims) = (clients.clone(), victims.clone());
                    Box::pin(async move {
                        let a = &victims[(i + offset) as usize];
                        let mut spend = Spend::default();
                        let r = clients[w as usize]
                            .delete(&a.scope, &id_of(a), d, &mut spend)
                            .await;
                        Done::of(r.map_or_else(from_error, |()| Outcome::Exact), spend)
                    })
                })
            }
            Operation::Clear | Operation::Wait => {
                let ordinal = cell.ordinal;
                let all = cell.scope == Some(ClearScope::All);
                Arc::new(move |w, i, d| {
                    let clients = clients.clone();
                    Box::pin(async move {
                        let scope = if all {
                            read_scope()
                        } else {
                            clear_scope(ordinal, i + offset)
                        };
                        let mut spend = Spend::default();
                        let r = clients[w as usize].clear(&scope, d, &mut spend).await;
                        Done::of(r.map_or_else(from_error, |()| Outcome::Exact), spend)
                    })
                })
            }
        }
    }

    fn postcheck<'a>(
        &'a self,
        cell: &'a PlannedCell,
        rows: &'a mut Vec<OpRow>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, String>> + Send + 'a>> {
        Box::pin(async move {
            let c = &self.clients[0];
            let deadline = Instant::now() + VERIFY_DEADLINE;
            let mut spend = Spend::default();
            let ids = |v: Vec<Summary>| v.into_iter().map(|s| s.id).collect::<BTreeSet<_>>();
            let reason = |e: OpError| format!("postcheck listing: {}", e.reason());
            match (cell.operation, cell.scope) {
                (Operation::Delete, _) => {
                    let scope = delete_scope(cell.ordinal);
                    let retained =
                        ids(c.list(&scope, deadline, &mut spend).await.map_err(reason)?);
                    let survivors: BTreeSet<String> = self
                        .of(Role::Survivor, Some(&scope))
                        .iter()
                        .map(id_of)
                        .collect();
                    let victims: Vec<String> = self
                        .of(Role::Victim, Some(&scope))
                        .iter()
                        .map(id_of)
                        .collect();
                    Ok(postcheck_delete(
                        &retained,
                        &survivors,
                        &victims,
                        rows,
                        cell.warmup,
                    ))
                }
                (Operation::Clear, Some(ClearScope::Recipient)) => {
                    let mut counts = Vec::new();
                    for s in 0..cell.warmup + cell.planned {
                        let scope = clear_scope(cell.ordinal, s);
                        counts.push(
                            c.list(&scope, deadline, &mut spend)
                                .await
                                .map_err(reason)?
                                .len() as u64,
                        );
                    }
                    let sentinel = c
                        .list(&sentinel_scope(), deadline, &mut spend)
                        .await
                        .map_err(reason)?;
                    let read = ids(c
                        .list(&read_scope(), deadline, &mut spend)
                        .await
                        .map_err(reason)?);
                    let want: BTreeSet<String> = self.reads().iter().map(id_of).collect();
                    let intact = sentinel.len() as u32 == SURVIVORS && read == want;
                    Ok(postcheck_clear(&counts, rows, cell.warmup, intact))
                }
                (Operation::Clear, _) => {
                    // The global clear: afterwards the store is empty.
                    let left = match c.all_ids(deadline).await.map_err(reason)? {
                        Some(ids) => ids.len() as u64,
                        None => {
                            let mut n = background_count(c, deadline).await.map_err(reason)?;
                            let scopes: BTreeSet<Scope> =
                                self.fixtures.iter().map(|f| f.scope.clone()).collect();
                            for s in scopes {
                                n += c
                                    .list(&s, deadline, &mut spend)
                                    .await
                                    .map_err(reason)?
                                    .len() as u64;
                            }
                            n
                        }
                    };
                    if left == 0 {
                        Ok(Vec::new())
                    } else {
                        invalidate(rows, None, "the store is not empty after the global clear");
                        Ok(vec![format!(
                            "{left} messages remain after the global clear"
                        )])
                    }
                }
                _ => Ok(Vec::new()),
            }
        })
    }
}
