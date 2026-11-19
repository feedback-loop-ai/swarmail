//! One API client per tool family (accepted decision 0132 §6 and §9.2): the
//! operations protocol 4's API section measures, with typed exact results,
//! one bounded keep-alive transport and every route cited.
//!
//! The same modules are compiled into the wire driver (`benches/`), its tests
//! (`tests/mail_compare.rs`) and the eval crate (`evals/`, by `#[path]`), so
//! each tool has exactly one client (0132 §9.2). They therefore use only
//! packages both manifests already name as direct dependencies: `reqwest`,
//! `serde`, `serde_json`, `tokio` and `base64`. Nothing here reads the
//! protocol or lock types; a target is named by its protocol name.
//!
//! Fairness (0132 §9.3): every client gets HTTP keep-alive, the largest
//! documented page size and a server-side filter where one is cited, and
//! nothing else: no cache, prefetch, index or expected-id hint. An answer is
//! complete only when paging reached the server's own count with every id
//! distinct; anything short of that is an error, never a shorter success.

#[path = "extract.rs"]
pub mod extract;
#[path = "greenmail.rs"]
pub mod greenmail;
#[path = "inbucket.rs"]
pub mod inbucket;
#[path = "mailhog.rs"]
pub mod mailhog;
#[path = "mailpit.rs"]
pub mod mailpit;
#[path = "mailslurper.rs"]
pub mod mailslurper;
#[path = "mime.rs"]
pub mod mime;
#[path = "smtp.rs"]
pub mod smtp;
#[path = "smtp4dev.rs"]
pub mod smtp4dev;
#[path = "swarmail.rs"]
pub mod swarmail;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::time::Instant;

/// The largest store protocol 4 ever holds (0132 §2.4's cap): no complete
/// scan may page further than this many messages.
pub const MAX_STORE: u64 = 150_000;
/// One response's byte bound. The largest legitimate page is a MailHog page
/// of 250 full corpus messages (about 1 MiB), well under this.
pub const MAX_RESPONSE_BYTES: u64 = 16 << 20;
/// One operation's aggregate byte bound over all its pages: a complete scan
/// of the largest store at the largest item size the corpus produces (a
/// MailHog item carries its whole raw message, under 4 KiB) fits under it.
pub const MAX_OPERATION_BYTES: u64 = 1 << 30;

/// The seven API families. Products that share an API share a family; their
/// citations stay per product, because each product's documents are its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Swarmail,
    Mailpit,
    Mailhog,
    Mailslurper,
    Smtp4dev,
    Inbucket,
    Greenmail,
}

impl Family {
    pub const ALL: [Family; 7] = [
        Family::Swarmail,
        Family::Mailpit,
        Family::Mailhog,
        Family::Mailslurper,
        Family::Smtp4dev,
        Family::Inbucket,
        Family::Greenmail,
    ];

    /// The family of a protocol target name. The three MailSlurper products
    /// (official, source build and Alkemio's image) speak one API.
    pub fn of(target: &str) -> Option<Family> {
        Some(match target {
            "swarmail" => Family::Swarmail,
            "mailpit" => Family::Mailpit,
            "mailhog" => Family::Mailhog,
            "mailslurper" | "mailslurper-source-build" | "alkemio-mailslurper" => {
                Family::Mailslurper
            }
            "smtp4dev" => Family::Smtp4dev,
            "inbucket" => Family::Inbucket,
            "greenmail" => Family::Greenmail,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Family::Swarmail => "swarmail",
            Family::Mailpit => "mailpit",
            Family::Mailhog => "mailhog",
            Family::Mailslurper => "mailslurper",
            Family::Smtp4dev => "smtp4dev",
            Family::Inbucket => "inbucket",
            Family::Greenmail => "greenmail",
        }
    }

    /// The module file that holds this family's client: what
    /// `clients_sha256` digests (0132 §9.2, §10).
    pub fn module(self) -> String {
        format!("{}.rs", self.name())
    }
}

/// The measured operations (0132 §6.1, §7.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    List,
    Search,
    Get,
    ExtractCode,
    ExtractLink,
    Delete,
    Clear,
    /// The MCP surface's future wait (0132 §7.4); no native cell has it.
    Wait,
}

impl Operation {
    pub const ALL: [Operation; 8] = [
        Operation::List,
        Operation::Search,
        Operation::Get,
        Operation::ExtractCode,
        Operation::ExtractLink,
        Operation::Delete,
        Operation::Clear,
        Operation::Wait,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Operation::List => "list",
            Operation::Search => "search",
            Operation::Get => "get",
            Operation::ExtractCode => "extract_code",
            Operation::ExtractLink => "extract_link",
            Operation::Delete => "delete",
            Operation::Clear => "clear",
            Operation::Wait => "wait",
        }
    }

    pub fn parse(s: &str) -> Option<Operation> {
        Operation::ALL.into_iter().find(|o| o.name() == s)
    }

    /// Whether the operation changes the store: its cells come after every
    /// read cell and own their fixtures (0132 §6.1).
    pub fn destructive(self) -> bool {
        matches!(self, Operation::Delete | Operation::Clear)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surface {
    Native,
    Mcp,
}

impl Surface {
    pub fn name(self) -> &'static str {
        match self {
            Surface::Native => "native",
            Surface::Mcp => "mcp",
        }
    }

    pub fn parse(s: &str) -> Option<Surface> {
        match s {
            "native" => Some(Surface::Native),
            "mcp" => Some(Surface::Mcp),
            _ => None,
        }
    }
}

/// What a clear removes (0132 §6.1): the narrowest documented scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClearScope {
    /// One recipient's mail (Swarmail's inbox, Inbucket's mailbox).
    Recipient,
    /// The whole store: measured once per fill and repetition, last.
    All,
}

/// How an operation reaches its answer, a label on every row (0132 §6.3):
/// a product feature is measured as such, and a missing one is labelled
/// with what the client does instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// A per-recipient store the tool itself keeps (Swarmail's per-test
    /// inboxes, Inbucket's and GreenMail's mailboxes).
    Inbox,
    /// A cited server-side filter, then an exact match on the client.
    ServerFilter,
    /// A complete scan of the store, filtered on the client.
    ClientScan,
    /// A cited route to one message (or one scope) by its id.
    ById,
    /// Extraction the server did at ingest (0005).
    Ingest,
    /// The client reads the message and runs the frozen oracle extractor.
    ClientExtract,
}

/// A captured byte that binds a route: a lock v5 document of the target
/// (`<file>` under `sources/<target>/`, or a `repo:` path for the
/// candidate) and a literal that must occur in it. The lock binds the
/// document's path, size and digest; the needle binds the claim to bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Cite {
    pub doc: &'static str,
    pub needle: &'static str,
}

/// One (target, surface, operation) cell of the capability table (0132
/// §6.3). Runtime errors never change it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Capability {
    Supported {
        method: Method,
        /// `METHOD /path` of every request the operation makes.
        routes: Vec<&'static str>,
        cites: Vec<Cite>,
        /// Clears only: the scope the documented clear removes.
        #[serde(skip_serializing_if = "Option::is_none")]
        scope: Option<ClearScope>,
    },
    NotSupported {
        reason: &'static str,
        cites: Vec<Cite>,
    },
    /// No captured document states the route either way: the cell is not
    /// inspected and blocks freezing (0132 §6.3). Missing evidence is never
    /// published as `not_supported`.
    NotInspected { why: &'static str },
    /// smtp4dev, Inbucket, GreenMail and the Alkemio image have no adapter
    /// (0132 §6.2, acceptance answer 12).
    NoMcpSurface,
    /// The official MailSlurper row does not start (0132 §2.2).
    DoesNotStart,
}

impl Capability {
    pub fn status(&self) -> &'static str {
        match self {
            Capability::Supported { .. } => "supported",
            Capability::NotSupported { .. } => "not_supported",
            Capability::NotInspected { .. } => "not_inspected",
            Capability::NoMcpSurface => "no_mcp_surface",
            Capability::DoesNotStart => "does_not_start",
        }
    }

    pub fn cites(&self) -> &[Cite] {
        match self {
            Capability::Supported { cites, .. } | Capability::NotSupported { cites, .. } => cites,
            _ => &[],
        }
    }

    pub fn supported(&self) -> bool {
        matches!(self, Capability::Supported { .. })
    }

    pub fn scope(&self) -> Option<ClearScope> {
        match self {
            Capability::Supported { scope, .. } => *scope,
            _ => None,
        }
    }
}

/// Shorthand for a family table's supported cell.
pub fn supported(method: Method, routes: &[&'static str], cites: &[Cite]) -> Capability {
    Capability::Supported {
        method,
        routes: routes.to_vec(),
        cites: cites.to_vec(),
        scope: None,
    }
}

/// The protocol targets that have an MCP surface (0132 §6.2): Swarmail's own
/// `POST /mcp`, and the 0129 adapters of Mailpit, MailHog and the
/// MailSlurper source build.
pub fn has_mcp_surface(target: &str) -> bool {
    matches!(
        target,
        "swarmail" | "mailpit" | "mailhog" | "mailslurper-source-build"
    )
}

/// The native capability of `op` on `target`, from its family's table.
pub fn native_capability(target: &str, op: Operation) -> Capability {
    if target == "mailslurper" {
        return Capability::DoesNotStart;
    }
    let Some(family) = Family::of(target) else {
        return Capability::NotInspected {
            why: "unknown target",
        };
    };
    if op == Operation::Wait {
        return Capability::NotSupported {
            reason: "the future wait is measured on the mcp surface only (0132 §7.4)",
            cites: Vec::new(),
        };
    }
    // Extraction reads the message: for a competitor it is supported
    // exactly where `get` is, with the same routes and citations, plus the
    // frozen extractor inside the measured time (0132 §6.1).
    if matches!(op, Operation::ExtractCode | Operation::ExtractLink) && family != Family::Swarmail {
        return match native_capability(target, Operation::Get) {
            Capability::Supported { routes, cites, .. } => Capability::Supported {
                method: Method::ClientExtract,
                routes,
                cites,
                scope: None,
            },
            other => other,
        };
    }
    match family {
        Family::Swarmail => swarmail::capability(op),
        Family::Mailpit => mailpit::capability(op),
        Family::Mailhog => mailhog::capability(op),
        Family::Mailslurper => mailslurper::capability(target, op),
        Family::Smtp4dev => smtp4dev::capability(op),
        Family::Inbucket => inbucket::capability(op),
        Family::Greenmail => greenmail::capability(op),
    }
}

/// A recipient scope: the address the mail was sent to and, for Swarmail,
/// the inbox its SMTP AUTH identity named (0004). Names are fixed-width, so
/// no scope's address is a substring of another's.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Scope {
    pub address: String,
    pub inbox: String,
}

/// The domain of every API fixture recipient.
pub const DOMAIN: &str = "api.mail-compare.invalid";

impl Scope {
    /// `<local>@api.mail-compare.invalid`, whose inbox is its local part.
    pub fn new(local: &str) -> Scope {
        Scope {
            address: format!("{local}@{DOMAIN}"),
            inbox: local.to_owned(),
        }
    }

    pub fn local(&self) -> &str {
        &self.inbox
    }
}

/// A message as a listing shows it: enough to match exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub id: String,
    pub to: Vec<String>,
    pub subject: String,
}

/// One message read by id.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Fetched {
    pub id: String,
    pub subject: Option<String>,
    pub from: Option<String>,
    pub to: Vec<String>,
    pub text: Option<String>,
    pub html: Option<String>,
    /// Swarmail only: the codes and links extracted at ingest (0005).
    pub codes: Option<Vec<String>>,
    pub links: Option<Vec<String>>,
}

/// Why an operation has no answer. None of these is a success.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpError {
    /// The operation's own budget ran out.
    Timeout,
    /// Connect, reset or another transport error.
    Transport,
    /// A status the route does not document for success.
    Status(u16),
    /// A body that is not the documented shape, or a required field absent.
    Malformed,
    /// Paging that could not establish completeness: a repeated id, a
    /// moving count, a page short before the count, an overshoot.
    Incomplete,
    /// A response or operation over its byte or page bound.
    Oversized,
    /// The capability table says no; never reached by a planned cell.
    NotSupported,
    /// A completed refusal on the measured surface, kept by its source so
    /// the published row names who refused (spec AC7, R8). Never retried.
    Refused(Refusal),
}

/// Who refused a measured MCP call, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    /// The eval runner's bridge: its in-flight bound was full.
    BusyBridge,
    /// The 0129 adapter: its admission queue was full.
    BusyAdapter,
    /// The target's own tool-call rate limit (Swarmail's 1,024 a second).
    RateLimited,
    /// The bridge refused an id already in flight: never sent by disjoint
    /// benchmark ranges, so seeing it is a correlation failure.
    DuplicateId,
    /// The client's own id range ran out before the call.
    IdRangeExhausted,
    /// Any other JSON-RPC error.
    RpcError,
    /// Any other tool error result.
    ToolError,
    /// A redirect, refused rather than followed.
    Redirect,
}

impl Refusal {
    pub fn name(self) -> &'static str {
        match self {
            Refusal::BusyBridge => "busy_bridge",
            Refusal::BusyAdapter => "busy_adapter",
            Refusal::RateLimited => "rate_limited",
            Refusal::DuplicateId => "duplicate_id",
            Refusal::IdRangeExhausted => "id_range_exhausted",
            Refusal::RpcError => "rpc_error",
            Refusal::ToolError => "tool_error",
            Refusal::Redirect => "redirect",
        }
    }
}

impl OpError {
    pub fn reason(self) -> String {
        match self {
            OpError::Timeout => "timeout".into(),
            OpError::Transport => "transport".into(),
            OpError::Status(s) => format!("status_{s}"),
            OpError::Malformed => "malformed".into(),
            OpError::Incomplete => "incomplete".into(),
            OpError::Oversized => "oversized".into(),
            OpError::NotSupported => "not_supported".into(),
            OpError::Refused(r) => r.name().into(),
        }
    }
}

/// Percent-encode a path segment or query value. RFC 3986's unreserved
/// characters stay as they are (MailSlurper's router does not decode path
/// parameters, so `-` must not become `%2D`).
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// A target-supplied id as one path segment (0132 §7, spec AC7): every
/// byte outside `A-Z a-z 0-9 - _ ~` is percent-encoded, `.` and `/`
/// included, so no id can become a separator. An id made only of dots (or
/// empty) is refused: a URL parser normalizes `%2E` and `%2E%2E` back into
/// the dot segments they spell, so no encoding of it reaches the intended
/// route. `-` stays literal for MailSlurper's router, which does not decode.
pub fn segment(id: &str) -> Result<String, OpError> {
    if id.bytes().all(|b| b == b'.') {
        return Err(OpError::Malformed);
    }
    let mut out = String::with_capacity(id.len());
    for b in id.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    Ok(out)
}

/// The URL `base` + `path` names, refused if parsing it changes the path or
/// query the client built (a dot segment resolved, a separator introduced):
/// the request must reach exactly the route it was written for.
pub fn exact_url(base: &str, path: &str) -> Result<reqwest::Url, OpError> {
    let url = reqwest::Url::parse(&format!("{base}{path}")).map_err(|_| OpError::Malformed)?;
    let (want_path, want_query) = match path.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (path, None),
    };
    if url.path() != want_path || url.query() != want_query || url.fragment().is_some() {
        return Err(OpError::Malformed);
    }
    Ok(url)
}

/// What one operation has spent: bytes over all its responses (bounded by
/// [`MAX_OPERATION_BYTES`]) and requests.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Spend {
    pub bytes: u64,
    pub requests: u32,
}

/// One worker's keep-alive HTTP connection to one target. It never reads
/// proxy variables, never follows a redirect, caps every response and
/// bounds every request by the caller's absolute deadline: a nested call
/// gets only what remains of the operation's budget.
pub struct Conn {
    client: reqwest::Client,
    base: String,
    requests: AtomicU64,
}

impl Conn {
    pub fn new(addr: SocketAddr) -> Conn {
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .pool_max_idle_per_host(1)
            .tcp_nodelay(true)
            .build()
            .expect("a plain loopback client builds");
        Conn {
            client,
            base: format!("http://{addr}"),
            requests: AtomicU64::new(0),
        }
    }

    pub fn requests(&self) -> u64 {
        self.requests.load(Ordering::Relaxed)
    }

    /// One request: its status and body, within `deadline` and the byte
    /// bounds. `body` is `(content type, bytes)`.
    pub async fn call(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<(&str, Vec<u8>)>,
        deadline: Instant,
        spend: &mut Spend,
    ) -> Result<(u16, Vec<u8>), OpError> {
        let left = deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or(OpError::Timeout)?;
        // Refused before dispatch: a path the URL parser would rewrite.
        let url = exact_url(&self.base, path)?;
        self.requests.fetch_add(1, Ordering::Relaxed);
        spend.requests += 1;
        let mut req = self.client.request(method, url).timeout(left);
        if let Some((ctype, bytes)) = body {
            req = req.header("content-type", ctype).body(bytes);
        }
        let fut = async {
            let mut resp = req.send().await.map_err(classify)?;
            let status = resp.status().as_u16();
            if resp.status().is_redirection() {
                return Err(OpError::Status(status));
            }
            // A declared length over a bound refuses before any byte is
            // read; a streamed body is accounted, checked, before append.
            let over = |reply: u64, more: u64, spent: u64| {
                reply > MAX_RESPONSE_BYTES
                    || spent
                        .checked_add(more)
                        .is_none_or(|t| t > MAX_OPERATION_BYTES)
            };
            if resp
                .content_length()
                .is_some_and(|n| over(n, n, spend.bytes))
            {
                return Err(OpError::Oversized);
            }
            let mut out = Vec::new();
            while let Some(chunk) = resp.chunk().await.map_err(classify)? {
                let len = chunk.len() as u64;
                if over(out.len() as u64 + len, len, spend.bytes) {
                    return Err(OpError::Oversized);
                }
                spend.bytes += len;
                out.extend_from_slice(&chunk);
            }
            Ok((status, out))
        };
        tokio::time::timeout_at(deadline, fut)
            .await
            .map_err(|_| OpError::Timeout)?
    }

    /// GET a JSON body; any status but 200 is an error.
    pub async fn get_json(
        &self,
        path: &str,
        deadline: Instant,
        spend: &mut Spend,
    ) -> Result<Value, OpError> {
        let (status, body) = self
            .call(reqwest::Method::GET, path, None, deadline, spend)
            .await?;
        if status != 200 {
            return Err(OpError::Status(status));
        }
        serde_json::from_slice(&body).map_err(|_| OpError::Malformed)
    }

    /// GET raw bytes; any status but 200 is an error.
    pub async fn get_bytes(
        &self,
        path: &str,
        deadline: Instant,
        spend: &mut Spend,
    ) -> Result<Vec<u8>, OpError> {
        let (status, body) = self
            .call(reqwest::Method::GET, path, None, deadline, spend)
            .await?;
        if status != 200 {
            return Err(OpError::Status(status));
        }
        Ok(body)
    }

    /// A mutation whose documented success statuses are `ok`.
    pub async fn mutate(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<(&str, Vec<u8>)>,
        ok: &[u16],
        deadline: Instant,
        spend: &mut Spend,
    ) -> Result<(), OpError> {
        let (status, _) = self.call(method, path, body, deadline, spend).await?;
        if ok.contains(&status) {
            Ok(())
        } else {
            Err(OpError::Status(status))
        }
    }
}

fn classify(e: reqwest::Error) -> OpError {
    if e.is_timeout() {
        OpError::Timeout
    } else {
        OpError::Transport
    }
}

/// One decoded listing page: the count of everything the request matches
/// and the page's items.
pub type Page = (u64, Vec<Summary>);

/// How a family pages a listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Paging {
    /// `start` or `offset` in messages, and a page size.
    Offset { size: u32 },
    /// A 1-based page number, and a page size.
    Number { size: u32 },
    /// One unpaged answer: its length is its own count.
    Whole,
}

/// Page `path(start, page_number, size)` over HTTP to completion: see
/// [`scan_pages`].
pub async fn scan(
    conn: &Conn,
    paging: Paging,
    path: impl Fn(u64, u64, u32) -> String,
    decode: impl Fn(&Value, u64) -> Option<Page>,
    deadline: Instant,
    spend: &mut Spend,
) -> Result<Vec<Summary>, OpError> {
    scan_pages(
        paging,
        |start, page, size, mut so_far| {
            let p = path(start, page, size);
            async move {
                let v = conn.get_json(&p, deadline, &mut so_far).await?;
                Ok((v, so_far))
            }
        },
        decode,
        spend,
    )
    .await
}

/// Fetch pages from `fetch(start, page_number, size, spent_so_far)` to
/// completion, over
/// any transport (HTTP here, MCP tool calls in the eval crate). Complete
/// means the pages reached the matching count with every id distinct, every
/// page before the last full, the count unchanged and no more pages than the
/// largest store needs. Anything else is an error, never a shorter success.
pub async fn scan_pages<F, Fut>(
    paging: Paging,
    mut fetch: F,
    decode: impl Fn(&Value, u64) -> Option<Page>,
    spend: &mut Spend,
) -> Result<Vec<Summary>, OpError>
where
    F: FnMut(u64, u64, u32, Spend) -> Fut,
    Fut: std::future::Future<Output = Result<(Value, Spend), OpError>>,
{
    // Each page is handed the operation's spend so far and returns it with
    // its own reply added: one shared budget, checked by the transport
    // before any byte past it is buffered, and again here.
    let account = |after: Spend, spend: &mut Spend| {
        *spend = after;
        if spend.bytes > MAX_OPERATION_BYTES {
            Err(OpError::Oversized)
        } else {
            Ok(())
        }
    };
    let size = match paging {
        Paging::Offset { size } | Paging::Number { size } => size,
        Paging::Whole => {
            let (v, one) = fetch(0, 1, 0, *spend).await?;
            account(one, spend)?;
            let (total, items) = decode(&v, 0).ok_or(OpError::Malformed)?;
            return complete(items, total);
        }
    };
    let max_pages = MAX_STORE / u64::from(size) + 2;
    let mut out: Vec<Summary> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut expected = None;
    for page in 0..max_pages {
        // The next offset is what arrived, never a fixed stride.
        let start = out.len() as u64;
        let (v, one) = fetch(start, page + 1, size, *spend).await?;
        account(one, spend)?;
        let (total, items) = decode(&v, start).ok_or(OpError::Malformed)?;
        if total > MAX_STORE || *expected.get_or_insert(total) != total {
            return Err(OpError::Incomplete);
        }
        let got = items.len();
        if got > size as usize {
            return Err(OpError::Malformed);
        }
        for s in items {
            if !seen.insert(s.id.clone()) {
                return Err(OpError::Incomplete);
            }
            out.push(s);
        }
        match (out.len() as u64).cmp(&total) {
            std::cmp::Ordering::Equal => return Ok(out),
            std::cmp::Ordering::Greater => return Err(OpError::Incomplete),
            std::cmp::Ordering::Less if got < size as usize => return Err(OpError::Incomplete),
            std::cmp::Ordering::Less => {}
        }
    }
    Err(OpError::Oversized)
}

/// An unpaged answer: distinct ids, as many as it states.
pub fn complete(items: Vec<Summary>, total: u64) -> Result<Vec<Summary>, OpError> {
    let mut seen = std::collections::HashSet::new();
    if items.len() as u64 != total || !items.iter().all(|s| seen.insert(s.id.clone())) {
        return Err(OpError::Incomplete);
    }
    Ok(items)
}

/// Only the items addressed to exactly `address` (ASCII case-insensitive,
/// as domains compare), so a server filter's substring match never lets a
/// look-alike recipient through.
pub fn exactly_to(items: Vec<Summary>, address: &str) -> Vec<Summary> {
    items
        .into_iter()
        .filter(|s| s.to.iter().any(|t| t.eq_ignore_ascii_case(address)))
        .collect()
}

/// A JSON id: a non-empty string or an integer.
pub fn id_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) if n.is_u64() || n.is_i64() => Some(n.to_string()),
        _ => None,
    }
}

/// An address list as a family states it: an array of strings or of
/// objects holding the address under `key`, or one comma-separated string.
/// Each is reduced to its bare `local@domain`.
pub fn addresses(v: &Value, key: Option<&str>) -> Option<Vec<String>> {
    match v {
        Value::Array(items) => items
            .iter()
            .map(|i| match (i, key) {
                (Value::String(s), _) => Some(bare(s)),
                (Value::Object(_), Some(k)) => i.get(k)?.as_str().map(bare),
                _ => None,
            })
            .collect(),
        Value::String(s) => Some(s.split(',').map(bare).filter(|a| !a.is_empty()).collect()),
        _ => None,
    }
}

/// `Name <a@b>`, `<a@b>` or `a@b` → `a@b`.
pub fn bare(s: &str) -> String {
    let s = s.trim();
    match (s.rfind('<'), s.rfind('>')) {
        (Some(a), Some(b)) if a < b => s[a + 1..b].trim().to_owned(),
        _ => s.to_owned(),
    }
}

/// A required string field.
pub fn text_of(v: &Value, key: &str) -> Option<String> {
    v.get(key)?.as_str().map(str::to_owned)
}

/// A client of one target for one worker: its family's operations over one
/// keep-alive connection. Every operation takes the absolute deadline of
/// its own budget and the [`Spend`] it accounts into.
pub struct Client {
    pub family: Family,
    pub target: String,
    pub conn: Conn,
}

impl Client {
    pub fn new(target: &str, addr: SocketAddr) -> Option<Client> {
        Some(Client {
            family: Family::of(target)?,
            target: target.to_owned(),
            conn: Conn::new(addr),
        })
    }

    /// Every message of `scope`'s recipient, complete and filtered exactly.
    pub async fn list(
        &self,
        scope: &Scope,
        deadline: Instant,
        spend: &mut Spend,
    ) -> Result<Vec<Summary>, OpError> {
        let all = self.candidates(scope, None, deadline, spend).await?;
        Ok(exactly_to(all, &scope.address))
    }

    /// The messages of `scope`'s recipient whose subject contains `token`:
    /// recipient AND subject, complete and filtered exactly.
    pub async fn search(
        &self,
        scope: &Scope,
        token: &str,
        deadline: Instant,
        spend: &mut Spend,
    ) -> Result<Vec<Summary>, OpError> {
        let hits = self.candidates(scope, Some(token), deadline, spend).await?;
        Ok(exactly_to(hits, &scope.address)
            .into_iter()
            .filter(|s| s.subject.contains(token))
            .collect())
    }

    /// What the family's documented listing returns for `scope` — with the
    /// subject `token`, its search — complete, before the exact match that
    /// [`list`](Self::list) and [`search`](Self::search) apply. This is the
    /// boundary where the reference suite counts other tests' mail that the
    /// tool's own filter let through (0132 §8.5, `foreign_seen`).
    pub async fn candidates(
        &self,
        scope: &Scope,
        token: Option<&str>,
        deadline: Instant,
        spend: &mut Spend,
    ) -> Result<Vec<Summary>, OpError> {
        let c = &self.conn;
        Ok(match (self.family, token) {
            (Family::Swarmail, t) => swarmail::list(c, scope, t, deadline, spend).await?,
            (Family::Mailpit, None) => mailpit::all(c, deadline, spend).await?,
            (Family::Mailpit, Some(t)) => mailpit::search(c, t, deadline, spend).await?,
            (Family::Mailhog, _) => mailhog::to(c, scope, deadline, spend).await?,
            (Family::Mailslurper, _) => mailslurper::to(c, scope, deadline, spend).await?,
            (Family::Smtp4dev, None) => {
                smtp4dev::search(c, &scope.address, deadline, spend).await?
            }
            (Family::Smtp4dev, Some(t)) => smtp4dev::search(c, t, deadline, spend).await?,
            (Family::Inbucket, _) => inbucket::list(c, scope, deadline, spend).await?,
            (Family::Greenmail, _) => greenmail::list(c, scope, deadline, spend).await?,
        })
    }

    /// Whether [`candidates`](Self::candidates) answers the same for every
    /// recipient scope: a store-wide listing or a subject search that takes
    /// no recipient. Then one call serves every address a test owns.
    pub fn scope_free(&self, token: Option<&str>) -> bool {
        matches!(
            (self.family, token),
            (Family::Mailpit, _) | (Family::Smtp4dev, Some(_))
        )
    }

    /// Whether ids are local to a mailbox (Inbucket's and GreenMail's
    /// per-recipient stores): a message is then named by its mailbox and
    /// its id together.
    pub fn mailbox_local_ids(&self) -> bool {
        matches!(self.family, Family::Inbucket | Family::Greenmail)
    }

    /// One message by id. `scope` is the mailbox the id lives in: only the
    /// per-mailbox stores (Inbucket, GreenMail) address a message by it.
    pub async fn get(
        &self,
        scope: &Scope,
        id: &str,
        deadline: Instant,
        spend: &mut Spend,
    ) -> Result<Fetched, OpError> {
        let c = &self.conn;
        let mut f = match self.family {
            Family::Swarmail => swarmail::get(c, id, deadline, spend).await?,
            Family::Mailpit => mailpit::get(c, id, deadline, spend).await?,
            Family::Mailhog => mailhog::get(c, id, deadline, spend).await?,
            Family::Mailslurper => mailslurper::get(c, id, deadline, spend).await?,
            Family::Smtp4dev => smtp4dev::get(c, id, deadline, spend).await?,
            Family::Inbucket => inbucket::get(c, scope, id, deadline, spend).await?,
            Family::Greenmail => greenmail::get(c, scope, id, deadline, spend).await?,
        };
        f.id = id.to_owned();
        Ok(f)
    }

    /// Delete one message by id.
    pub async fn delete(
        &self,
        scope: &Scope,
        id: &str,
        deadline: Instant,
        spend: &mut Spend,
    ) -> Result<(), OpError> {
        let c = &self.conn;
        match self.family {
            Family::Swarmail => swarmail::delete(c, id, deadline, spend).await,
            Family::Mailpit => mailpit::delete(c, Some(id), deadline, spend).await,
            Family::Smtp4dev => smtp4dev::delete(c, id, deadline, spend).await,
            Family::Inbucket => inbucket::delete(c, scope, Some(id), deadline, spend).await,
            Family::Mailhog | Family::Mailslurper | Family::Greenmail => Err(OpError::NotSupported),
        }
    }

    /// The narrowest documented clear: `scope`'s recipient where the tool
    /// has one, else the whole store.
    pub async fn clear(
        &self,
        scope: &Scope,
        deadline: Instant,
        spend: &mut Spend,
    ) -> Result<(), OpError> {
        let c = &self.conn;
        match self.family {
            Family::Swarmail => swarmail::clear(c, scope, deadline, spend).await,
            Family::Mailpit => mailpit::delete(c, None, deadline, spend).await,
            Family::Smtp4dev => smtp4dev::clear(c, deadline, spend).await,
            Family::Inbucket => inbucket::delete(c, scope, None, deadline, spend).await,
            Family::Greenmail => greenmail::clear(c, deadline, spend).await,
            Family::Mailhog | Family::Mailslurper => Err(OpError::NotSupported),
        }
    }

    /// Untimed verification: every id of the whole store, where the family
    /// has one listing of it. A per-mailbox store answers `None`; its
    /// callers sum the mailboxes they addressed instead.
    pub async fn all_ids(&self, deadline: Instant) -> Result<Option<Vec<String>>, OpError> {
        let mut spend = Spend::default();
        let c = &self.conn;
        let all = match self.family {
            Family::Swarmail | Family::Inbucket | Family::Greenmail => return Ok(None),
            Family::Mailpit => mailpit::all(c, deadline, &mut spend).await?,
            Family::Mailhog => mailhog::all(c, deadline, &mut spend).await?,
            Family::Mailslurper => mailslurper::all(c, deadline, &mut spend).await?,
            Family::Smtp4dev => smtp4dev::search(c, "", deadline, &mut spend).await?,
        };
        Ok(Some(all.into_iter().map(|s| s.id).collect()))
    }
}

/// Whether `f` is a MailSlurper answer that kept only a multipart's HTML
/// part: no text part, an HTML one (operator ruling 2026-11-18, under v3's
/// record that MailSlurper keeps one decoded body).
pub fn from_kept_html(family: Family, f: &Fetched) -> bool {
    family == Family::Mailslurper
        && f.text.as_deref().is_none_or(|t| t.trim().is_empty())
        && f.html.as_deref().is_some_and(|h| !h.trim().is_empty())
}

/// What the frozen extractors read: the text part, or, for an answer that
/// kept only the HTML part ([`from_kept_html`]), that HTML as text.
fn extractor_text(family: Family, f: &Fetched) -> Option<String> {
    if from_kept_html(family, f) {
        return f.html.as_deref().map(html_text);
    }
    f.text.clone()
}

/// An HTML body as the text it shows: a paragraph, line break, division,
/// list item, table row or heading tag ends a line, every other tag is
/// dropped, and the five XML entities are decoded. The frozen extractors
/// then read it unchanged, so a code or a labelled link in the kept HTML is
/// found as it would be on its text line.
pub fn html_text(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(lt) = rest.find('<') {
        out.push_str(&rest[..lt]);
        let Some(gt) = rest[lt..].find('>') else {
            out.push_str(&rest[lt..]);
            rest = "";
            break;
        };
        let name: String = rest[lt + 1..lt + gt]
            .trim_start_matches('/')
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect::<String>()
            .to_ascii_lowercase();
        let block = matches!(
            name.as_str(),
            "p" | "br" | "div" | "li" | "tr" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
        );
        if block && !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        rest = &rest[lt + gt + 1..];
    }
    out.push_str(rest);
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// The code answer of a read message: Swarmail's first ingest code, or the
/// frozen extractor over the text part (or the kept HTML, as text).
pub fn answer_code(family: Family, f: &Fetched) -> Option<String> {
    match family {
        Family::Swarmail => f.codes.as_ref()?.first().cloned(),
        _ => extractor_text(family, f).and_then(|t| extract::six_digit_code(&t)),
    }
}

/// The link answer of a read message: Swarmail's one distinct ingest link
/// (several distinct links are no single answer), or the frozen extractor
/// over the text part (or the kept HTML, as text) under `label`.
pub fn answer_link(family: Family, f: &Fetched, label: &str) -> Option<String> {
    match family {
        Family::Swarmail => {
            let mut links = f.links.clone()?;
            links.sort();
            links.dedup();
            (links.len() == 1).then(|| links.remove(0))
        }
        _ => extractor_text(family, f).and_then(|t| extract::labelled_url(&t, label)),
    }
}

/// What one MCP future wait answered, read from its own envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitAnswer {
    /// The wait returned within its window with these messages.
    Found(Vec<Summary>),
    /// A 0129 adapter's poll first saw a match after the cutoff.
    Late,
    /// The window closed with no match: Swarmail's `timed_out`, an
    /// adapter's `timeout`.
    TimedOut,
    /// The envelope is not the tool's documented shape.
    Malformed,
}

/// Decode a wait tool's answer. Swarmail's `swarmail_wait_for_email`
/// answers `{emails}`, or `{timed_out: true}`; the 0129 adapters' poll
/// (`benches/mail_compare/mcp.rs`, `poll_wait`) answers `{status,
/// messages_count, messages, scans, requests, method}` whose messages are
/// the family's own native page items, read here by the same family page
/// decoder the native client uses.
pub fn wait_answer(family: Family, v: &Value) -> WaitAnswer {
    let found = |page: Option<Page>| match page {
        Some((total, items)) if total == items.len() as u64 => WaitAnswer::Found(items),
        _ => WaitAnswer::Malformed,
    };
    if family == Family::Swarmail {
        if v.get("timed_out").and_then(Value::as_bool) == Some(true) {
            return WaitAnswer::TimedOut;
        }
        let Some(emails) = v.get("emails").and_then(Value::as_array) else {
            return WaitAnswer::Malformed;
        };
        return found(swarmail::decode_page(
            &serde_json::json!({"total": emails.len(), "emails": emails}),
            0,
        ));
    }
    if !matches!(
        family,
        Family::Mailpit | Family::Mailhog | Family::Mailslurper
    ) {
        return WaitAnswer::Malformed;
    }
    let status = v.get("status").and_then(Value::as_str);
    let (Some(count), Some(messages)) = (
        v.get("messages_count").and_then(Value::as_u64),
        v.get("messages").and_then(Value::as_array),
    ) else {
        return WaitAnswer::Malformed;
    };
    match status {
        Some("found") => {}
        Some("late") => return WaitAnswer::Late,
        Some("timeout") => return WaitAnswer::TimedOut,
        _ => return WaitAnswer::Malformed,
    }
    if count != messages.len() as u64 {
        return WaitAnswer::Malformed;
    }
    found(match family {
        Family::Mailpit => mailpit::decode_page(
            &serde_json::json!({"start": 0, "messages_count": count, "messages": messages}),
            0,
        ),
        Family::Mailhog => mailhog::decode_page(
            &serde_json::json!({"start": 0, "total": count, "items": messages}),
            0,
        ),
        _ => mailslurper::decode_page(
            &serde_json::json!({"totalRecords": count, "mailItems": messages}),
            0,
        ),
    })
}

/// The MCP surface (0132 §6.2): Swarmail's own `POST /mcp` tools, and the
/// 0129 adapters' tools behind the eval runner's bridge, every one at
/// revision 2025-03-26. The adapters offer no delete or clear (0092 §3, 0129
/// §5), and nothing on this surface falls back to native HTTP.
pub fn mcp_capability(target: &str, op: Operation) -> Capability {
    let tool = |doc: &'static str, needle: &'static str| Cite { doc, needle };
    let swarmail = |name: &'static str| tool("repo:src/mcp.rs", name);
    let adapter_none = |doc: &'static str, needle: &'static str| Capability::NotSupported {
        reason: "the adapter offers no delete, clear or admin tool (0092 §3, 0129 §5)",
        cites: vec![tool(doc, needle)],
    };
    if target == "mailslurper" {
        return Capability::DoesNotStart;
    }
    if !has_mcp_surface(target) {
        return Capability::NoMcpSurface;
    }
    let s = |method, routes: &[&'static str], cites: &[Cite]| supported(method, routes, cites);
    match (target, op) {
        ("swarmail", Operation::List) => s(
            Method::Inbox,
            &["tools/call swarmail_search_emails {inbox}"],
            &[swarmail("\"swarmail_search_emails\"")],
        ),
        ("swarmail", Operation::Search) => s(
            Method::Inbox,
            &["tools/call swarmail_search_emails {inbox, subject}"],
            &[swarmail("\"swarmail_search_emails\"")],
        ),
        ("swarmail", Operation::Get) => s(
            Method::ById,
            &["tools/call swarmail_get_email {id}"],
            &[swarmail("\"swarmail_get_email\"")],
        ),
        ("swarmail", Operation::ExtractCode) => s(
            Method::Ingest,
            &["tools/call swarmail_extract_codes {id}"],
            &[swarmail("\"swarmail_extract_codes\"")],
        ),
        ("swarmail", Operation::ExtractLink) => s(
            Method::Ingest,
            &["tools/call swarmail_extract_links {id}"],
            &[swarmail("\"swarmail_extract_links\"")],
        ),
        ("swarmail", Operation::Delete) => s(
            Method::ById,
            &["tools/call swarmail_delete_email {id}"],
            &[swarmail("\"swarmail_delete_email\"")],
        ),
        ("swarmail", Operation::Clear) => Capability::Supported {
            method: Method::Inbox,
            routes: vec!["tools/call swarmail_clear_inbox {inbox}"],
            cites: vec![swarmail("\"swarmail_clear_inbox\"")],
            scope: Some(ClearScope::Recipient),
        },
        ("swarmail", Operation::Wait) => s(
            Method::Inbox,
            &["tools/call swarmail_wait_for_email {inbox, subject}"],
            &[swarmail("\"swarmail_wait_for_email\"")],
        ),
        ("mailpit", op) => {
            let doc = "repo:benches/mail_compare/mailpit_mcp.rs";
            match op {
                Operation::List => s(
                    Method::ClientScan,
                    &["tools/call mailpit_list {start, limit}"],
                    &[tool(doc, "\"name\": \"mailpit_list\"")],
                ),
                Operation::Search => s(
                    Method::ServerFilter,
                    &["tools/call mailpit_search {query, start, limit}"],
                    &[tool(doc, "\"name\": \"mailpit_search\"")],
                ),
                Operation::Get | Operation::ExtractCode | Operation::ExtractLink => s(
                    if op == Operation::Get {
                        Method::ById
                    } else {
                        Method::ClientExtract
                    },
                    &["tools/call mailpit_read {id}"],
                    &[tool(doc, "\"name\": \"mailpit_read\"")],
                ),
                Operation::Wait => s(
                    Method::ServerFilter,
                    &["tools/call mailpit_wait {query} (a labelled client-side poll)"],
                    &[tool(doc, "\"name\": \"mailpit_wait\"")],
                ),
                Operation::Delete | Operation::Clear => adapter_none(doc, "no admin, send, delete"),
            }
        }
        ("mailhog", op) => {
            let doc = "repo:benches/mail_compare/mailhog_mcp.rs";
            match op {
                Operation::List | Operation::Search => s(
                    Method::ServerFilter,
                    &["tools/call mailhog_search {kind: to, query, start, limit}"],
                    &[tool(doc, "\"name\": \"mailhog_search\"")],
                ),
                Operation::Get | Operation::ExtractCode | Operation::ExtractLink => s(
                    if op == Operation::Get {
                        Method::ClientScan
                    } else {
                        Method::ClientExtract
                    },
                    &["tools/call mailhog_list {start, limit} until the id"],
                    &[tool(doc, "There is no read-by-id tool.")],
                ),
                Operation::Wait => s(
                    Method::ServerFilter,
                    &["tools/call mailhog_wait {kind, query} (a labelled client-side poll)"],
                    &[tool(doc, "\"name\": \"mailhog_wait\"")],
                ),
                Operation::Delete | Operation::Clear => adapter_none(doc, "no admin, send,"),
            }
        }
        (_, op) => {
            let doc = "repo:benches/mail_compare/mailslurper_mcp.rs";
            match op {
                Operation::List | Operation::Search => s(
                    Method::ServerFilter,
                    &["tools/call mailslurper_search {to, page}"],
                    &[tool(doc, "\"name\": \"mailslurper_search\"")],
                ),
                Operation::Get | Operation::ExtractCode | Operation::ExtractLink => s(
                    if op == Operation::Get {
                        Method::ById
                    } else {
                        Method::ClientExtract
                    },
                    &["tools/call mailslurper_read {id}"],
                    &[tool(doc, "\"name\": \"mailslurper_read\"")],
                ),
                Operation::Wait => s(
                    Method::ServerFilter,
                    &["tools/call mailslurper_wait {to} (a labelled client-side poll)"],
                    &[tool(doc, "\"name\": \"mailslurper_wait\"")],
                ),
                Operation::Delete | Operation::Clear => adapter_none(
                    doc,
                    "no delete, prune, attachment, login or version operation",
                ),
            }
        }
    }
}

/// A capability on either surface.
pub fn capability(target: &str, surface: Surface, op: Operation) -> Capability {
    match surface {
        Surface::Native => native_capability(target, op),
        Surface::Mcp => mcp_capability(target, op),
    }
}
