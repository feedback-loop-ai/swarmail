//! The API section's MCP surface (accepted decision 0132 §6.2, §7.4, story
//! B): the same scheduler, fixtures and exact checks as the native surface
//! (`api.rs`, shared by path), with every operation a `tools/call` through
//! the target's MCP endpoint: Swarmail's own `POST /mcp`, or a 0129 adapter
//! behind the runner's bridge, so the bridge hop is in the time. Nothing on
//! this surface falls back to native HTTP; the answers are decoded by the
//! same family decoders the native clients use.
//!
//! Each worker is one `McpClient` with its own keep-alive connection and a
//! disjoint range of JSON-RPC ids, initialization included, so up to 32
//! workers share one bridge without colliding. Busy refusals (the bridge's
//! in-flight bound, an adapter's queue) are failure rows by source: never
//! retried and never serialized around.

use crate::api::{self, Done, Outcome, PlannedCell, Role, WaitTimes};
use crate::clients::{self, Family, OpError, Page, Paging, Scope, Spend, Summary};
use crate::mcp::McpClient;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;

/// The ids each benchmark client may use: client `n` gets
/// `n * IDS_PER_CLIENT + 1 ..` up to the next client's first id.
pub const IDS_PER_CLIENT: u64 = 1 << 32;

/// Client `n`'s id range: disjoint from every other client's.
pub fn id_range(n: u64) -> std::ops::Range<u64> {
    n * IDS_PER_CLIENT + 1..(n + 1) * IDS_PER_CLIENT
}

/// A tool call's failure, by source: a deadline is a timeout; a busy
/// refusal names who refused; anything else is the class of error.
pub fn classify(e: &str, past_deadline: bool) -> Outcome {
    if past_deadline || e.contains("operation timed out") || e.contains("deadline") {
        return Outcome::Timeout;
    }
    let reason = if e.contains("rate exceeds") {
        "rate_limited"
    } else if e.contains("busy") {
        if e.contains("-32600") || e.contains("in flight") {
            "busy_bridge"
        } else {
            "busy_adapter"
        }
    } else if e.contains("JSON-RPC error") {
        "rpc_error"
    } else if e.contains("tool error") {
        "tool_error"
    } else if e.contains("exhausted") {
        "id_range_exhausted"
    } else if e.contains(crate::mcp::OVERSIZED) {
        "oversized"
    } else if e.contains(crate::mcp::REDIRECT) {
        "redirect"
    } else {
        "transport"
    };
    Outcome::Failed {
        reason: reason.into(),
    }
}

/// One worker's MCP session against one target.
pub struct Worker {
    pub client: Mutex<McpClient>,
    pub target: String,
}

impl Worker {
    async fn call(
        &self,
        tool: &str,
        args: Value,
        deadline: Instant,
        spend: &mut Spend,
    ) -> Result<Value, OpError> {
        let left = deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or(OpError::Timeout)?;
        spend.requests += 1;
        let mut c = self.client.lock().await;
        // Every reply of the operation charges its one shared byte budget,
        // checked before append in the transport (spec AC7, R11).
        let call = c.call_spending(tool, args, Some(left), &mut spend.bytes);
        match tokio::time::timeout(left, call).await {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(match classify(&e, Instant::now() >= deadline) {
                Outcome::Timeout => OpError::Timeout,
                Outcome::Failed { reason } => match reason.as_str() {
                    "transport" => OpError::Transport,
                    "oversized" => OpError::Oversized,
                    "redirect" => OpError::Status(302),
                    // A refusal by the server or the path to it: the class is
                    // kept as a status the row names (429 a rate limit, 503
                    // a busy bridge or adapter, 502 any other tool or RPC
                    // error), never retried.
                    "rate_limited" => OpError::Status(429),
                    "busy_bridge" | "busy_adapter" => OpError::Status(503),
                    _ => OpError::Status(502),
                },
                _ => OpError::Status(502),
            }),
            Err(_) => Err(OpError::Timeout),
        }
    }

    async fn paged(
        &self,
        tool: &'static str,
        paging: Paging,
        args: impl Fn(u64, u64, u32) -> Value,
        decode: impl Fn(&Value, u64) -> Option<Page>,
        deadline: Instant,
        spend: &mut Spend,
    ) -> Result<Vec<Summary>, OpError> {
        clients::scan_pages(
            paging,
            |start, page, size, mut so_far| {
                let a = args(start, page, size);
                async move {
                    let v = self.call(tool, a, deadline, &mut so_far).await?;
                    Ok((v, so_far))
                }
            },
            decode,
            spend,
        )
        .await
    }

    fn family(&self) -> Family {
        Family::of(&self.target).expect("an mcp target has a family")
    }

    /// Every message of `scope`, complete and filtered exactly.
    pub async fn list(
        &self,
        scope: &Scope,
        deadline: Instant,
        spend: &mut Spend,
    ) -> Result<Vec<Summary>, OpError> {
        let all = match self.family() {
            Family::Swarmail => {
                let inbox = scope.inbox.clone();
                self.paged(
                    "swarmail_search_emails",
                    Paging::Offset { size: 250 },
                    move |start, _, size| json!({"inbox": inbox, "limit": size, "offset": start}),
                    clients::swarmail::decode_page,
                    deadline,
                    spend,
                )
                .await?
            }
            Family::Mailpit => {
                self.paged(
                    "mailpit_list",
                    Paging::Offset { size: 250 },
                    |start, _, size| json!({"start": start, "limit": size}),
                    clients::mailpit::decode_page,
                    deadline,
                    spend,
                )
                .await?
            }
            Family::Mailhog => {
                let q = scope.address.clone();
                self.paged(
                    "mailhog_search",
                    Paging::Offset { size: 250 },
                    move |start, _, size| json!({"kind": "to", "query": q, "start": start, "limit": size}),
                    clients::mailhog::decode_page,
                    deadline,
                    spend,
                )
                .await?
            }
            _ => {
                let to = scope.address.clone();
                self.paged(
                    "mailslurper_search",
                    Paging::Number {
                        size: clients::mailslurper::PAGE,
                    },
                    move |_, page, _| json!({"to": to, "page": page}),
                    clients::mailslurper::decode_page,
                    deadline,
                    spend,
                )
                .await?
            }
        };
        Ok(clients::exactly_to(all, &scope.address))
    }

    /// Recipient AND subject token, complete and filtered exactly.
    pub async fn search(
        &self,
        scope: &Scope,
        token: &str,
        deadline: Instant,
        spend: &mut Spend,
    ) -> Result<Vec<Summary>, OpError> {
        let hits = match self.family() {
            Family::Swarmail => {
                let (inbox, t) = (scope.inbox.clone(), token.to_owned());
                self.paged(
                    "swarmail_search_emails",
                    Paging::Offset { size: 250 },
                    move |start, _, size| json!({"inbox": inbox, "subject": t, "limit": size, "offset": start}),
                    clients::swarmail::decode_page,
                    deadline,
                    spend,
                )
                .await?
            }
            Family::Mailpit => {
                let t = token.to_owned();
                self.paged(
                    "mailpit_search",
                    Paging::Offset { size: 250 },
                    move |start, _, size| json!({"query": t, "start": start, "limit": size}),
                    clients::mailpit::decode_page,
                    deadline,
                    spend,
                )
                .await?
            }
            _ => self.list(scope, deadline, spend).await?,
        };
        Ok(clients::exactly_to(hits, &scope.address)
            .into_iter()
            .filter(|s| s.subject.contains(token))
            .collect())
    }

    /// One message by id: the read tool, or (MailHog, which has none) the
    /// list tool paged until the id.
    pub async fn get(
        &self,
        id: &str,
        deadline: Instant,
        spend: &mut Spend,
    ) -> Result<clients::Fetched, OpError> {
        let f = match self.family() {
            Family::Swarmail => {
                let v = self
                    .call("swarmail_get_email", json!({"id": id}), deadline, spend)
                    .await?;
                clients::swarmail::decode_message(&v)
            }
            Family::Mailpit => {
                let v = self
                    .call("mailpit_read", json!({"id": id}), deadline, spend)
                    .await?;
                clients::mailpit::decode_message(&v)
            }
            Family::Mailhog => {
                let mut start = 0u64;
                // Finite paging: no more pages than the largest store needs.
                let mut pages = clients::MAX_STORE / 250 + 2;
                loop {
                    pages = pages.checked_sub(1).ok_or(OpError::Oversized)?;
                    let v = self
                        .call(
                            "mailhog_list",
                            json!({"start": start, "limit": 250}),
                            deadline,
                            spend,
                        )
                        .await?;
                    let items = v
                        .get("items")
                        .and_then(Value::as_array)
                        .ok_or(OpError::Malformed)?;
                    if let Some(m) = items
                        .iter()
                        .find(|m| m.get("ID").and_then(clients::id_text).as_deref() == Some(id))
                    {
                        break clients::mailhog::decode_item(m);
                    }
                    let total = v
                        .get("total")
                        .and_then(Value::as_u64)
                        .ok_or(OpError::Malformed)?;
                    start += items.len() as u64;
                    if start >= total || items.is_empty() {
                        return Err(OpError::Status(404));
                    }
                }
            }
            _ => {
                let v = self
                    .call("mailslurper_read", json!({"id": id}), deadline, spend)
                    .await?;
                clients::mailslurper::decode_message(&v)
            }
        };
        f.ok_or(OpError::Malformed)
    }
}

/// The boxed closure type the scheduler takes.
type Op = Arc<dyn Fn(u32, u32, Instant) -> api::OpFuture + Send + Sync>;

/// The MCP surface: one worker per concurrency slot, the authored ledger,
/// and what a future wait needs to send its own message.
pub struct McpSurface {
    pub workers: Arc<Vec<Worker>>,
    /// Untimed verification (postchecks) reads through the target's native
    /// API: it is not a measurement, and must not spend the measured
    /// surface's own rate or queue.
    pub verify: clients::Client,
    pub fixtures: Arc<Vec<api::Authored>>,
    pub smtp: std::net::SocketAddr,
    pub seed: u64,
    pub clock: api::Clock,
}

fn id_of(a: &api::Authored) -> String {
    a.id.clone().unwrap_or_default()
}

/// The scope of a future wait's own attempt: fresh per attempt (spec AC7).
pub fn wait_scope(ordinal: u32, slot: u32) -> Scope {
    Scope::new(&format!("apiwt{ordinal:01}{slot:04}x"))
}

/// A wait's answer as summaries: Swarmail's `{emails}`, or the adapter's
/// page in its family's shape.
fn wait_summaries(family: Family, v: &Value) -> Option<Vec<Summary>> {
    match family {
        Family::Swarmail => clients::swarmail::decode_page(
            &json!({"total": v.get("emails")?.as_array()?.len(), "emails": v.get("emails")?}),
            0,
        )
        .map(|(_, s)| s),
        Family::Mailpit => {
            clients::mailpit::decode_page(v, v.get("start")?.as_u64()?).map(|(_, s)| s)
        }
        Family::Mailhog => {
            clients::mailhog::decode_page(v, v.get("start")?.as_u64()?).map(|(_, s)| s)
        }
        _ => clients::mailslurper::decode_page(v, 0).map(|(_, s)| s),
    }
}

impl McpSurface {
    fn of(&self, role: Role, scope: Option<&Scope>) -> Vec<api::Authored> {
        self.fixtures
            .iter()
            .filter(|f| f.role == role && scope.is_none_or(|s| &f.scope == s))
            .cloned()
            .collect()
    }
}

/// How long the SMTP QUIT after a wait's delivery may take: bounded apart
/// from the instants it can no longer move.
pub const QUIT_BOUND: std::time::Duration = std::time::Duration::from_secs(2);

/// One future wait (spec AC7): the wait request is started first, then the
/// attempt's own message is sent; no sleep orders them and no claim is made
/// that the server registered the waiter. Exactly that message must return.
pub async fn wait_once(
    w: &Worker,
    (smtp, clock): (std::net::SocketAddr, api::Clock),
    a: api::Authored,
    deadline: Instant,
) -> Done {
    let family = w.family();
    let left_ms = deadline
        .saturating_duration_since(Instant::now())
        .as_millis()
        .min(20_000) as u64;
    let args = match family {
        Family::Swarmail => {
            json!({"inbox": a.scope.inbox, "subject": a.token, "timeout_ms": left_ms})
        }
        Family::Mailpit => json!({"query": a.token, "timeout_ms": left_ms}),
        Family::Mailhog => json!({"kind": "containing", "query": a.token, "timeout_ms": left_ms}),
        _ => json!({"to": a.scope.address, "timeout_ms": left_ms}),
    };
    let tool = match family {
        Family::Swarmail => "swarmail_wait_for_email",
        Family::Mailpit => "mailpit_wait",
        Family::Mailhog => "mailhog_wait",
        _ => "mailslurper_wait",
    };
    let mut spend = Spend::default();
    let call_ns = clock.now_ns();
    // The return is stamped inside the waiting future the instant the call
    // returns (spec AC7, R3): whatever the sending side still does — the
    // SMTP QUIT — cannot shift it.
    let waiting = async {
        let got = w.call(tool, args, deadline, &mut spend).await;
        (got, clock.now_ns())
    };
    let sending = async {
        // Started after the wait future's first poll: `join!` polls it first.
        tokio::task::yield_now().await;
        let send_ns = clock.now_ns();
        let mut s = match clients::smtp::Session::open(smtp, api::SMTP_STEP).await {
            Ok(s) => s,
            Err(_) => return (send_ns, Err("fixture_not_sent")),
        };
        if family == Family::Swarmail && s.auth(&a.scope.inbox).await.is_err() {
            return (send_ns, Err("fixture_not_sent"));
        }
        let sent = s
            .send(api::SENDER, &a.scope.address, &api::fixture_bytes(&a))
            .await;
        // The acknowledgement is stamped at the `250` itself, before QUIT.
        let ack = match sent {
            clients::smtp::Sent::Accepted => Ok(clock.now_ns()),
            clients::smtp::Sent::Refused(_) | clients::smtp::Sent::Failed => Err("fixture_refused"),
            clients::smtp::Sent::Ambiguous => Err("fixture_ambiguous"),
        };
        // QUIT is bounded on its own and never moves the instants above.
        let _ = tokio::time::timeout(QUIT_BOUND, s.quit()).await;
        (send_ns, ack)
    };
    let ((got, return_ns), (send_ns, ack)) = tokio::join!(waiting, sending);
    let ack_ns = ack.ok();
    let outcome = match (got, ack) {
        // A delivery whose fate is unknown outranks the wait's own answer:
        // it is mutation uncertainty (R1).
        (_, Err(why)) => Outcome::Failed { reason: why.into() },
        (Err(e), _) => api::from_error(e),
        (Ok(v), _) => match wait_summaries(family, &v) {
            None => Outcome::Failed {
                reason: "malformed".into(),
            },
            Some(hits) => {
                let mine: Vec<&Summary> = hits
                    .iter()
                    .filter(|s| s.subject.contains(&a.token))
                    .collect();
                if mine.len() == 1
                    && hits.len() == 1
                    && mine[0]
                        .to
                        .iter()
                        .any(|t| t.eq_ignore_ascii_case(&a.scope.address))
                {
                    Outcome::Exact
                } else {
                    Outcome::WrongResult {
                        detail: format!(
                            "{} returned, {} of them the attempt's own",
                            hits.len(),
                            mine.len()
                        ),
                    }
                }
            }
        },
    };
    Done {
        outcome,
        spend,
        wait: Some(WaitTimes {
            call_ns,
            send_ns,
            ack_ns,
            return_ns,
            ack_to_return_ns: ack_ns.map(|a| return_ns as i64 - a as i64),
        }),
    }
}

impl api::Surfacer for McpSurface {
    fn op(&self, cell: &PlannedCell, offset: u32) -> Op {
        let workers = self.workers.clone();
        let reads = Arc::new(self.of(Role::Read, None));
        let op = cell.operation;
        match op {
            clients::Operation::List => {
                let expected: BTreeSet<String> = reads.iter().map(id_of).collect();
                Arc::new(move |w, _i, d| {
                    let (workers, expected) = (workers.clone(), expected.clone());
                    Box::pin(async move {
                        let mut spend = Spend::default();
                        let got = workers[w as usize]
                            .list(&api::read_scope(), d, &mut spend)
                            .await;
                        Done::of(api::check_list(got, &expected), spend)
                    })
                })
            }
            clients::Operation::Search
            | clients::Operation::Get
            | clients::Operation::ExtractCode
            | clients::Operation::ExtractLink => Arc::new(move |w, i, d| {
                let (workers, reads) = (workers.clone(), reads.clone());
                Box::pin(async move {
                    let a = &reads[((i + offset) % api::K) as usize];
                    let wk = &workers[w as usize];
                    let mut spend = Spend::default();
                    let id = id_of(a);
                    let swarmail = wk.family() == Family::Swarmail;
                    let outcome = match op {
                        clients::Operation::Search => api::check_search(
                            wk.search(&a.scope, &a.token, d, &mut spend).await,
                            &id,
                        ),
                        clients::Operation::Get => {
                            api::check_get(wk.family(), wk.get(&id, d, &mut spend).await, a)
                        }
                        clients::Operation::ExtractCode if swarmail => {
                            let v = wk
                                .call("swarmail_extract_codes", json!({"id": id}), d, &mut spend)
                                .await;
                            api::check_value(v.and_then(|v| first_string(&v, "codes")), &a.code)
                        }
                        clients::Operation::ExtractLink if swarmail => {
                            let v = wk
                                .call("swarmail_extract_links", json!({"id": id}), d, &mut spend)
                                .await;
                            let f = v.map(|v| clients::Fetched {
                                links: strings(&v, "links"),
                                ..Default::default()
                            });
                            api::check_value(
                                f.map(|f| {
                                    clients::answer_link(Family::Swarmail, &f, api::LINK_LABEL)
                                }),
                                &a.link,
                            )
                        }
                        clients::Operation::ExtractCode => {
                            let f = wk.get(&id, d, &mut spend).await;
                            api::check_value(
                                f.map(|f| clients::answer_code(wk.family(), &f)),
                                &a.code,
                            )
                        }
                        _ => {
                            let f = wk.get(&id, d, &mut spend).await;
                            api::check_value(
                                f.map(|f| clients::answer_link(wk.family(), &f, api::LINK_LABEL)),
                                &a.link,
                            )
                        }
                    };
                    Done::of(outcome, spend)
                })
            }),
            clients::Operation::Wait => {
                let (smtp, clock, seed, ordinal) = (self.smtp, self.clock, self.seed, cell.ordinal);
                Arc::new(move |w, i, d| {
                    let workers = workers.clone();
                    Box::pin(async move {
                        let slot = i + offset;
                        let a = api::author(
                            seed,
                            Role::Future,
                            ordinal * 1_000 + slot,
                            &wait_scope(ordinal, slot),
                        );
                        wait_once(&workers[w as usize], (smtp, clock), a, d).await
                    })
                })
            }
            clients::Operation::Delete => {
                let victims =
                    Arc::new(self.of(Role::Victim, Some(&api::delete_scope(cell.ordinal))));
                Arc::new(move |w, i, d| {
                    let (workers, victims) = (workers.clone(), victims.clone());
                    Box::pin(async move {
                        let mut spend = Spend::default();
                        let a = &victims[(i + offset) as usize];
                        let r = workers[w as usize]
                            .call(
                                "swarmail_delete_email",
                                json!({"id": id_of(a)}),
                                d,
                                &mut spend,
                            )
                            .await;
                        Done::of(r.map_or_else(api::from_error, |_| Outcome::Exact), spend)
                    })
                })
            }
            clients::Operation::Clear => {
                let ordinal = cell.ordinal;
                Arc::new(move |w, i, d| {
                    let workers = workers.clone();
                    Box::pin(async move {
                        let mut spend = Spend::default();
                        let scope = api::clear_scope(ordinal, i + offset);
                        let r = workers[w as usize]
                            .call(
                                "swarmail_clear_inbox",
                                json!({"inbox": scope.inbox}),
                                d,
                                &mut spend,
                            )
                            .await;
                        Done::of(r.map_or_else(api::from_error, |_| Outcome::Exact), spend)
                    })
                })
            }
        }
    }

    fn postcheck<'a>(
        &'a self,
        cell: &'a PlannedCell,
        rows: &'a mut Vec<api::OpRow>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, String>> + Send + 'a>> {
        Box::pin(async move {
            let wk = &self.verify;
            let deadline = Instant::now() + api::VERIFY_DEADLINE;
            let mut spend = Spend::default();
            let reason = |e: OpError| format!("postcheck listing: {}", e.reason());
            let ids = |v: Vec<Summary>| v.into_iter().map(|s| s.id).collect::<BTreeSet<_>>();
            match cell.operation {
                clients::Operation::Delete => {
                    let scope = api::delete_scope(cell.ordinal);
                    let retained = ids(wk
                        .list(&scope, deadline, &mut spend)
                        .await
                        .map_err(reason)?);
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
                    Ok(api::postcheck_delete(
                        &retained,
                        &survivors,
                        &victims,
                        rows,
                        cell.warmup,
                    ))
                }
                clients::Operation::Clear => {
                    let mut counts = Vec::new();
                    for s in 0..cell.warmup + cell.planned {
                        counts.push(
                            wk.list(&api::clear_scope(cell.ordinal, s), deadline, &mut spend)
                                .await
                                .map_err(reason)?
                                .len() as u64,
                        );
                    }
                    let sentinel = wk
                        .list(&api::sentinel_scope(), deadline, &mut spend)
                        .await
                        .map_err(reason)?;
                    let read = ids(wk
                        .list(&api::read_scope(), deadline, &mut spend)
                        .await
                        .map_err(reason)?);
                    let want: BTreeSet<String> =
                        self.of(Role::Read, None).iter().map(id_of).collect();
                    Ok(api::postcheck_clear(
                        &counts,
                        rows,
                        cell.warmup,
                        sentinel.len() as u32 == api::SURVIVORS && read == want,
                    ))
                }
                _ => Ok(Vec::new()),
            }
        })
    }
}

fn strings(v: &Value, key: &str) -> Option<Vec<String>> {
    v.get(key)?
        .as_array()?
        .iter()
        .map(|x| x.as_str().map(str::to_owned))
        .collect()
}

fn first_string(v: &Value, key: &str) -> Result<Option<String>, OpError> {
    strings(v, key)
        .map(|s| s.into_iter().next())
        .ok_or(OpError::Malformed)
}

// ============================================================================
// `swarmail-evals api-mcp`: one MCP API chunk (0132 §11.1)
// ============================================================================

/// The held target's control pipe, as the meter of every cell's window: the
/// window opens and closes on the holder's clock, never this process's.
pub struct ControlMeter {
    pipe: Mutex<(
        tokio::process::ChildStdin,
        tokio::io::BufReader<tokio::process::ChildStdout>,
    )>,
    /// Local send and receive instants of every command, for the record.
    pub brackets: std::sync::Mutex<Vec<(String, u64, u64)>>,
    clock: api::Clock,
}

impl ControlMeter {
    /// Send one control line and read its one reply line: an `error` reply,
    /// or no reply, is a refusal.
    pub async fn command(&self, line: &Value) -> Result<Value, String> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
        let mut p = self.pipe.lock().await;
        let sent = self.clock.now_ns();
        let mut bytes = line.to_string().into_bytes();
        bytes.push(b'\n');
        p.0.write_all(&bytes)
            .await
            .map_err(|e| format!("control: {e}"))?;
        p.0.flush().await.map_err(|e| format!("control: {e}"))?;
        let mut reply = String::new();
        let n = tokio::time::timeout(
            std::time::Duration::from_secs(60),
            p.1.read_line(&mut reply),
        )
        .await
        .map_err(|_| "control: no reply".to_owned())?
        .map_err(|e| format!("control: {e}"))?;
        let got = self.clock.now_ns();
        if n == 0 {
            return Err("control: the holder ended".into());
        }
        let v: Value = serde_json::from_str(&reply).map_err(|e| format!("control reply: {e}"))?;
        if v.get("ok").is_none() {
            return Err(format!("control refused: {}", v));
        }
        self.brackets.lock().expect("brackets lock").push((
            line["cmd"].as_str().unwrap_or_default().to_owned(),
            sent,
            got,
        ));
        Ok(v)
    }
}

impl api::Meter for ControlMeter {
    fn open<'a>(&'a self, cell: &'a str) -> api::MeterFuture<'a> {
        Box::pin(async move {
            self.command(&json!({"cmd": "window_start", "id": cell}))
                .await
                .map(|_| ())
        })
    }

    fn close<'a>(&'a self, cell: &'a str) -> api::MeterFuture<'a> {
        Box::pin(async move {
            self.command(&json!({"cmd": "window_end", "id": cell}))
                .await
                .map(|_| ())
        })
    }
}

/// `api.mcp.<target>.r<rep>.f<fill>`: the coordinate's target, repetition
/// and fill. Anything else refuses; the holder rechecks it against the plan.
pub fn parse_chunk(text: &str) -> Result<(String, u32, u32), String> {
    let parts: Vec<&str> = text.split('.').collect();
    let num = |s: &str, p: char| {
        s.strip_prefix(p)
            .filter(|n| !n.starts_with('0') || *n == "0")
            .and_then(|n| n.parse::<u32>().ok())
            .ok_or_else(|| format!("chunk {text:?}: {s:?} is not {p}<n>"))
    };
    match parts.as_slice() {
        ["api", "mcp", target, rep, fill] => {
            if !clients::has_mcp_surface(target) {
                return Err(format!("chunk {text:?}: {target} has no MCP surface"));
            }
            Ok(((*target).to_owned(), num(rep, 'r')?, num(fill, 'f')?))
        }
        _ => Err(format!("{text:?} is not an api.mcp chunk")),
    }
}

/// The adapter mode of a bridged target.
pub fn adapter_mode(target: &str) -> Option<&'static str> {
    match target {
        "mailpit" => Some("mailpit-mcp"),
        "mailhog" => Some("mailhog-mcp"),
        "mailslurper-source-build" => Some("mailslurper-mcp"),
        _ => None,
    }
}

/// The adapter's command line: its mode, the lock, the endpoint of the held
/// backend and — when the chunk was given one — the document root the
/// holder already verified (spec AC2, R2), so a draft lock whose documents
/// live apart from it serves all three adapters without relocation. The
/// adapter rechecks the same bytes before it serves.
pub fn adapter_command(
    driver: &std::path::Path,
    mode: &str,
    sources: &std::path::Path,
    documents: Option<&std::path::Path>,
    http: std::net::SocketAddr,
) -> tokio::process::Command {
    let mut a = tokio::process::Command::new(driver);
    a.arg(mode)
        .arg("--sources")
        .arg(sources)
        .args(["--endpoint", &format!("http://{http}")]);
    if let Some(d) = documents {
        a.arg("--documents").arg(d);
    }
    a
}

/// `--flag value` pairs; `--bin` and `--runtime` once each.
pub fn flags(args: &[String]) -> Result<std::collections::BTreeMap<String, String>, String> {
    const KNOWN: [&str; 13] = [
        "driver",
        "mode",
        "protocol",
        "sources",
        "documents",
        "chunk",
        "bin",
        "runtime",
        "output",
        "scratch",
        "run-id",
        "commit",
        "dirty",
    ];
    let mut out = std::collections::BTreeMap::new();
    let mut it = args.iter();
    while let Some(f) = it.next() {
        let name = f
            .strip_prefix("--")
            .ok_or_else(|| format!("unexpected argument {f:?}"))?;
        if !KNOWN.contains(&name) && name != "systemd" {
            return Err(format!("unknown flag --{name}"));
        }
        let v = it.next().ok_or_else(|| format!("--{name} needs a value"))?;
        if out.insert(name.to_owned(), v.clone()).is_some() {
            return Err(format!("--{name} given twice"));
        }
    }
    for need in [
        "driver", "mode", "protocol", "sources", "chunk", "bin", "output", "scratch", "run-id",
    ] {
        if !out.contains_key(need) {
            return Err(format!("--{need} is required"));
        }
    }
    Ok(out)
}

/// Run one MCP API chunk; the exit code is the holder's (0 clean, 1 a
/// target's own failure, sealed; 2 an infrastructure or control failure,
/// unsealed). Every process it starts is torn down before it returns.
pub async fn api_mcp(args: &[String]) -> i32 {
    match api_mcp_inner(args).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("swarmail-evals api-mcp: {e}");
            2
        }
    }
}

/// How long the holder has to print its receipt once its control input
/// closes, and then to exit: its own teardown is bounded inside these.
const RECEIPT_BOUND: std::time::Duration = std::time::Duration::from_secs(120);

/// Check the holder's cleanup receipt (spec AC8, R12): one JSON object whose
/// launch and cleanup both name `target` and the same leaf (the endpoints'
/// leaf, when they named one), and whose cleanup proves the leaf removed,
/// the process reaped, its group gone, its listeners closed and its scratch
/// removed. A missing, foreign or unproved receipt never seals.
pub fn validate_receipt(v: &Value, target: &str, leaf: Option<&str>) -> Result<(), String> {
    let launch = v.get("launch").filter(|l| l.is_object());
    let cleanup = v.get("cleanup").filter(|c| c.is_object());
    let (Some(launch), Some(cleanup)) = (launch, cleanup) else {
        return Err("the receipt has no launch and cleanup".into());
    };
    if launch["target"] != json!(target) || cleanup["target"] != json!(target) {
        return Err(format!("the receipt is not of {target}"));
    }
    let named = launch["leaf"]
        .as_str()
        .filter(|l| !l.is_empty())
        .ok_or("the receipt's launch names no leaf")?;
    if cleanup["leaf"] != json!(named) || leaf.is_some_and(|l| l != named) {
        return Err("the receipt's cleanup is of another leaf".into());
    }
    for proof in [
        "cgroup_removed",
        "reaped",
        "group_gone",
        "listeners_closed",
        "scratch_removed",
    ] {
        if cleanup[proof] != json!(true) {
            return Err(format!("the receipt does not prove {proof}"));
        }
    }
    Ok(())
}

/// The chunk's exit from the holder's status and receipt, and whether this
/// side completed its part: 0 or 1 only when the holder said so and its
/// receipt proves the cleanup; 2 otherwise (spec AC8, R12).
pub fn chunk_exit(
    holder: Option<i32>,
    receipt: Result<(), String>,
    ours: Result<(), String>,
) -> (i32, Option<String>) {
    match (ours, receipt, holder) {
        (Err(e), _, _) => (2, Some(e)),
        (_, Err(e), _) => (2, Some(format!("cleanup receipt: {e}"))),
        (_, _, Some(c @ (0 | 1))) => (c, None),
        (_, _, other) => (2, Some(format!("the holder exited {other:?}"))),
    }
}

async fn api_mcp_inner(args: &[String]) -> Result<i32, String> {
    use tokio::io::AsyncBufReadExt;
    let f = flags(args)?;
    let (target, repetition, fill) = parse_chunk(&f["chunk"])?;
    let protocol: Value = serde_json::from_slice(
        &std::fs::read(&f["protocol"]).map_err(|e| format!("--protocol: {e}"))?,
    )
    .map_err(|e| format!("--protocol: {e}"))?;
    let seed = protocol["corpus"]["seed"]
        .as_u64()
        .ok_or("--protocol has no corpus seed")?;
    // The holder: the one owner of the backend, its leaf and its sampler.
    // From here on every path — a refusal, a malformed line, a rejected
    // record, an interruption — closes its control input and awaits its
    // receipt within a bound (R12). Nothing relies on kill-on-drop.
    let mut hold = tokio::process::Command::new(&f["driver"]);
    hold.arg("hold").args(["--control", "api-v1"]);
    for (k, v) in &f {
        if k != "driver" {
            hold.arg(format!("--{k}")).arg(v);
        }
    }
    hold.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());
    let mut holder = hold.spawn().map_err(|e| format!("hold: {e}"))?;
    let stdin = holder.stdin.take().ok_or("hold: no stdin")?;
    let stdout = tokio::io::BufReader::new(holder.stdout.take().ok_or("hold: no stdout")?);
    let clock = api::Clock(std::time::Instant::now());
    let meter = ControlMeter {
        pipe: Mutex::new((stdin, stdout)),
        brackets: std::sync::Mutex::new(Vec::new()),
        clock,
    };
    let mut adapter = None;
    let mut bridge = None;
    let mut first_line = None;
    let mut leaf = None;
    let ours: Result<(), String> = async {
        let mut line = String::new();
        let read = {
            let mut p = meter.pipe.lock().await;
            tokio::time::timeout(RECEIPT_BOUND, p.1.read_line(&mut line)).await
        };
        let endpoints: Value = match read {
            Ok(Ok(n)) if n > 0 => {
                serde_json::from_str(&line).map_err(|e| format!("hold endpoints: {e}"))?
            }
            _ => return Err("the holder ended before it announced".into()),
        };
        if endpoints.get("cleanup").is_some() {
            // It refused or its target did not come up: its first line is
            // already its receipt.
            first_line = Some(endpoints);
            return Ok(());
        }
        leaf = endpoints["leaf"].as_str().map(str::to_owned);
        let http: std::net::SocketAddr = endpoints["http"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .ok_or("hold: no http endpoint")?;
        let smtp: std::net::SocketAddr = endpoints["smtp"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .ok_or("hold: no smtp endpoint")?;
        // The MCP endpoint: Swarmail's own, or the adapter behind one bridge.
        let url = match adapter_mode(&target) {
            None => format!("http://{http}/mcp"),
            Some(mode) => {
                let mut a = adapter_command(
                    std::path::Path::new(&f["driver"]),
                    mode,
                    std::path::Path::new(&f["sources"]),
                    f.get("documents").map(std::path::Path::new),
                    http,
                );
                a.stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped());
                let mut child = a.spawn().map_err(|e| format!("adapter: {e}"))?;
                let io = (child.stdin.take(), child.stdout.take());
                adapter = Some(child);
                let (Some(to), Some(from)) = io else {
                    return Err("adapter: no stdio".into());
                };
                let b = crate::bridge::Bridge::start(to, tokio::io::BufReader::new(from))
                    .await
                    .map_err(|e| format!("bridge: {e}"))?;
                let url = b.url();
                bridge = Some(b);
                url
            }
        };
        let rec = measure(
            &target,
            (repetition, fill, seed),
            (smtp, http, &url),
            &meter,
            clock,
        )
        .await?;
        meter
            .command(&json!({"cmd": "record", "api": rec}))
            .await
            .map(|_| ())
    }
    .await;
    // Teardown, in order and bounded: the bridge and adapter, the control
    // input, then the holder's receipt and exit.
    if let Some(b) = bridge {
        b.stop().await;
    }
    if let Some(mut a) = adapter {
        if tokio::time::timeout(std::time::Duration::from_secs(10), a.wait())
            .await
            .is_err()
        {
            let _ = a.kill().await;
        }
    }
    let (stdin, mut stdout) = meter.pipe.into_inner();
    drop(stdin);
    let receipt = match first_line {
        Some(v) => Some(v),
        None => {
            // A late control reply may precede the receipt: the receipt is
            // the line that carries a cleanup.
            let mut found = None;
            for _ in 0..4 {
                let mut line = String::new();
                match tokio::time::timeout(RECEIPT_BOUND, stdout.read_line(&mut line)).await {
                    Ok(Ok(n)) if n > 0 => {
                        if let Ok(v) = serde_json::from_str::<Value>(&line)
                            && v.get("cleanup").is_some()
                        {
                            found = Some(v);
                            break;
                        }
                    }
                    _ => break,
                }
            }
            found
        }
    };
    let status = match tokio::time::timeout(RECEIPT_BOUND, holder.wait()).await {
        Ok(Ok(s)) => s.code(),
        _ => {
            let _ = holder.kill().await;
            None
        }
    };
    let receipt = receipt
        .ok_or_else(|| "missing".to_owned())
        .and_then(|v| validate_receipt(&v, &target, leaf.as_deref()));
    let (code, why) = chunk_exit(status, receipt, ours);
    if let Some(e) = why {
        eprintln!("swarmail-evals api-mcp: {e}");
    }
    Ok(code)
}

/// Fill, idle point, fixtures and every MCP cell; the record.
async fn measure(
    target: &str,
    (repetition, fill, seed): (u32, u32, u64),
    (smtp, http, url): (std::net::SocketAddr, std::net::SocketAddr, &str),
    meter: &ControlMeter,
    clock: api::Clock,
) -> Result<api::ApiRecord, String> {
    let surface = clients::Surface::Mcp;
    let caps = |op| clients::mcp_capability(target, op);
    let (cells, capabilities) = api::plan_cells(surface, caps);
    let growth = api::fixture_growth(seed, &cells);
    api::within_cap(fill, growth)?;
    // Every worker initializes before any clock (spec AC5), each with its own
    // id range, initialization included.
    let workers = cells
        .iter()
        .filter_map(|c| c.concurrency)
        .max()
        .unwrap_or(1);
    let mut pool = Vec::new();
    for n in 0..workers {
        let c = McpClient::connect_range(url, id_range(u64::from(n))).await?;
        pool.push(Worker {
            client: Mutex::new(c),
            target: target.to_owned(),
        });
    }
    // Setup is untimed and outside every window: the fill and the fixture
    // ids are read through the target's native API.
    let setup = clients::Client::new(target, http).ok_or("no family")?;
    let tag = format!("mcp-{target}-r{repetition}-f{fill}");
    let mut rec = api::ApiRecord {
        surface,
        target: target.to_owned(),
        repetition,
        fill: api::fill(smtp, &setup, (seed, &tag), fill, clock).await,
        fixtures: 0,
        growth,
        capabilities,
        cells: Vec::new(),
        abandoned: None,
        store: api::store_label(setup.family).to_owned(),
    };
    let block = |why: &str, rec: &mut api::ApiRecord| {
        rec.cells = cells
            .iter()
            .map(|c| api::CellRecord::blocked(c, &caps(c.operation), why, clock.now_ns()))
            .collect();
    };
    if !rec.fill.reached {
        block("fill_not_reached", &mut rec);
        return Ok(rec);
    }
    meter.command(&json!({"cmd": "idle_fill"})).await?;
    let mut fixtures = api::plan_fixtures(seed, &cells);
    if let Err(e) = api::prepare(smtp, &setup, &mut fixtures).await {
        block(&format!("fixtures_not_prepared: {e}"), &mut rec);
        return Ok(rec);
    }
    rec.fixtures = fixtures.len() as u64;
    let s = McpSurface {
        workers: Arc::new(pool),
        verify: clients::Client::new(target, http).ok_or("no family")?,
        fixtures: Arc::new(fixtures),
        smtp,
        seed,
        clock,
    };
    let (recs, abandoned) = api::run_cells(&cells, &caps, &s, meter, clock, None).await?;
    rec.cells = recs;
    rec.abandoned = abandoned;
    Ok(rec)
}
