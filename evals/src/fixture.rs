//! The fixture app (#149): a small HTTP application that sends real SMTP
//! to the Swarmail it is pointed at. Nothing is seeded: every mail rides
//! Swarmail's real ingest path (decision 0055 §3).
//!
//! Two loopback listeners. The **public** plane is what an agent drives:
//! following emailed links, submitting codes, resetting, accepting,
//! replying, releasing held mail. Every public action names its identity
//! in the `X-Eval-Identity` header, and every emailed secret is bound to
//! the identity and flow it was minted for. The **admin** plane needs the
//! per-trial bearer secret and belongs to the runner: provisioning, the
//! scenario barriers, freezing and the redacted snapshot. No route sets a
//! success fact directly, and no state response carries a token, code,
//! password, challenge or calendar capability.
//!
//! Admin separation prevents accidental access; it is not a sandbox. Every
//! trial is unconfined.

use crate::mail::{self, Attachment, Body, Message, SendResult};
use crate::state::{
    Delivery, DeliveryStatus, Event, Facts, FailureClass, Flow, IdentityState, Role,
    SNAPSHOT_SCHEMA, Snapshot, action,
};
use crate::tasks::{self, Scenario, Task};
use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The header every public action names its identity in.
pub const IDENTITY_HEADER: &str = "x-eval-identity";
/// The environment variable the fixture child reads its admin secret from.
/// The runner sets it on the fixture alone; agents never see it.
pub const ADMIN_SECRET_ENV: &str = "EVAL_FIXTURE_ADMIN_SECRET";
/// How long a 2FA code stays valid.
pub const OTP_TTL: Duration = Duration::from_secs(600);
/// The most SMTP attempts one message gets before it is a harness error.
pub const MAX_ATTEMPTS: usize = 3;
/// The sender every fixture mail carries.
pub const FROM: &str = "no-reply@fixture.eval";

/// The fixture's clock seam: production reads `Instant::now`, unit tests
/// inject instants so expiry is proven without waiting for wall time.
pub type Clock = Arc<dyn Fn() -> Instant + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenRole {
    Confirm,
    /// Unsubscribe, Report, "this wasn't me", view-in-browser: a known
    /// decoy link (`wrong_link`).
    DecoyLink,
    /// A link inside a decoy message for this same identity
    /// (`wrong_message`).
    DecoyMessage,
    MagicLogin,
    Reset,
    Invite,
    DigestRight,
    DigestWrong,
    MarketingOffer,
    Otp {
        generation: u32,
    },
}

#[derive(Debug, Clone)]
struct Token {
    owner: String,
    flow: Flow,
    role: TokenRole,
    issued: Instant,
    consumed: bool,
}

#[derive(Debug, Clone)]
struct Account {
    identity: String,
    inbox: String,
    role: Role,
    facts: Facts,
    password: Option<String>,
}

#[derive(Debug, Clone)]
struct Outbound {
    inbox: String,
    to: String,
    data: Vec<u8>,
}

#[derive(Debug, Clone)]
struct Invite {
    uid: String,
    start: String,
    ics: Vec<u8>,
}

#[derive(Debug, Clone)]
struct Ticket {
    owner: String,
    challenge: String,
}

/// The private state. Converted into [`Snapshot`] field by field.
pub struct Private {
    trial_id: String,
    task: Task,
    inputs: crate::provenance::FixtureInputs,
    app_url: String,
    accounts: Vec<Account>,
    tokens: HashMap<String, Token>,
    otp_generation: HashMap<String, u32>,
    tickets: HashMap<String, Ticket>,
    invites: HashMap<String, Invite>,
    deliveries: Vec<Delivery>,
    outbound: HashMap<u64, Outbound>,
    events: Vec<Event>,
    seq: u64,
    provisioned: bool,
    prepared: bool,
    frozen: bool,
    clock: Clock,
    private_rng: rand::rngs::StdRng,
}

fn reply(status: StatusCode, ok: bool, message: impl Into<String>) -> Response {
    (
        status,
        axum::Json(json!({ "ok": ok, "message": message.into() })),
    )
        .into_response()
}

/// A refusal already recorded as an event, ready to answer.
type Refusal = Box<Response>;

fn refuse(status: StatusCode, message: &str) -> Refusal {
    Box::new(reply(status, false, message))
}

fn reply_json(value: Value) -> Response {
    (StatusCode::OK, axum::Json(value)).into_response()
}

impl Private {
    pub fn new(trial_id: &str, task: Task, app_url: &str, clock: Clock) -> Private {
        Self::with_inputs(trial_id, task, app_url, clock, Default::default())
    }

    pub fn with_inputs(
        trial_id: &str,
        task: Task,
        app_url: &str,
        clock: Clock,
        inputs: crate::provenance::FixtureInputs,
    ) -> Private {
        use rand::SeedableRng;
        Private {
            private_rng: rand::rngs::StdRng::from_os_rng(),
            inputs,
            trial_id: trial_id.to_string(),
            task,
            app_url: app_url.to_string(),
            accounts: Vec::new(),
            tokens: HashMap::new(),
            otp_generation: HashMap::new(),
            tickets: HashMap::new(),
            invites: HashMap::new(),
            deliveries: Vec::new(),
            outbound: HashMap::new(),
            events: Vec::new(),
            seq: 0,
            provisioned: false,
            prepared: false,
            frozen: false,
            clock,
        }
    }

    fn random_alnum(&mut self, len: usize) -> String {
        use rand::{Rng, distr::Alphanumeric};
        (&mut self.private_rng)
            .sample_iter(&Alphanumeric)
            .take(len)
            .map(char::from)
            .collect()
    }

    fn next_seq(&mut self) -> u64 {
        self.seq += 1;
        self.seq
    }

    fn record(
        &mut self,
        claimed: Option<&str>,
        flow: Option<Flow>,
        action: &str,
        accepted: bool,
        category: Option<FailureClass>,
    ) {
        let seq = self.next_seq();
        self.events.push(Event {
            seq,
            claimed: claimed.and_then(|name| {
                self.accounts
                    .iter()
                    .find(|a| a.identity == name)
                    .map(|a| a.identity.clone())
            }),
            flow,
            action: action.to_string(),
            accepted,
            category,
        });
    }

    fn account_mut(&mut self, identity: &str) -> &mut Account {
        self.accounts
            .iter_mut()
            .find(|a| a.identity == identity)
            .expect("a claimed identity is provisioned")
    }

    fn agents(&self) -> Vec<Account> {
        self.accounts
            .iter()
            .filter(|a| a.role == Role::Agent)
            .cloned()
            .collect()
    }

    /// Resolve the claimed identity, recording a refusal when it names
    /// nothing, an unknown identity, or a decoy.
    fn claim(
        &mut self,
        headers: &HeaderMap,
        flow: Option<Flow>,
        attempted: &str,
    ) -> Result<String, Refusal> {
        let Some(raw) = headers.get(IDENTITY_HEADER) else {
            self.record(None, flow, attempted, false, Some(FailureClass::ToolMisuse));
            return Err(refuse(
                StatusCode::BAD_REQUEST,
                "missing X-Eval-Identity header",
            ));
        };
        let name = String::from_utf8_lossy(raw.as_bytes()).trim().to_string();
        match self.accounts.iter().find(|a| a.identity == name) {
            Some(a) if a.role == Role::Agent => Ok(name),
            Some(_) => {
                self.record(
                    Some(&name),
                    flow,
                    action::FOREIGN_TOKEN,
                    false,
                    Some(FailureClass::WrongMessage),
                );
                Err(refuse(StatusCode::FORBIDDEN, "that identity is not yours"))
            }
            None => {
                self.record(
                    Some(&name),
                    flow,
                    attempted,
                    false,
                    Some(FailureClass::ToolMisuse),
                );
                Err(refuse(StatusCode::BAD_REQUEST, "unknown identity"))
            }
        }
    }

    /// Present an emailed secret: it must exist and belong to the claimant.
    fn present(
        &mut self,
        claimed: &str,
        secret: &str,
        flow: Option<Flow>,
        attempted: &str,
    ) -> Result<Token, Refusal> {
        match self.tokens.get(secret).cloned() {
            None => {
                self.record(
                    Some(claimed),
                    flow,
                    attempted,
                    false,
                    Some(FailureClass::ToolMisuse),
                );
                Err(refuse(StatusCode::NOT_FOUND, "unknown token or code"))
            }
            Some(t) if t.owner != claimed => {
                self.record(
                    Some(claimed),
                    Some(t.flow),
                    action::FOREIGN_TOKEN,
                    false,
                    Some(FailureClass::WrongMessage),
                );
                Err(refuse(
                    StatusCode::FORBIDDEN,
                    "that token belongs to another account",
                ))
            }
            Some(t) => Ok(t),
        }
    }

    fn misuse(
        &mut self,
        claimed: &str,
        flow: Option<Flow>,
        attempted: &str,
        why: &str,
    ) -> Response {
        self.record(
            Some(claimed),
            flow,
            attempted,
            false,
            Some(FailureClass::ToolMisuse),
        );
        reply(StatusCode::BAD_REQUEST, false, why)
    }

    fn mint(&mut self, owner: &str, flow: Flow, role: TokenRole) -> String {
        let token = self.random_alnum(24);
        let issued = (self.clock)();
        self.tokens.insert(
            token.clone(),
            Token {
                owner: owner.to_string(),
                flow,
                role,
                issued,
                consumed: false,
            },
        );
        token
    }

    fn link(&self, token: &str) -> String {
        format!("{}/a/{token}", self.app_url)
    }

    /// Record a delivery as held and keep its bytes for sending.
    fn enqueue(
        &mut self,
        account: &Account,
        to: &str,
        flow: Flow,
        kind: &str,
        msg: Message,
    ) -> u64 {
        let slot = self
            .accounts
            .iter()
            .position(|a| a.identity == account.identity)
            .expect("provisioned account") as u64;
        let generation = self
            .deliveries
            .iter()
            .filter(|d| d.identity == account.identity && d.flow == flow)
            .count() as u64;
        let flow_name = serde_json::to_value(flow)
            .expect("flow")
            .as_str()
            .expect("flow string")
            .to_string();
        let render =
            mail::RenderContext::fixture(&self.inputs, &self.task.id, slot, &flow_name, generation);
        let seq = self.next_seq();
        self.deliveries.push(Delivery {
            seq,
            identity: account.identity.clone(),
            flow,
            message: kind.to_string(),
            status: DeliveryStatus::Held,
            attempts: Vec::new(),
        });
        self.outbound.insert(
            seq,
            Outbound {
                inbox: account.inbox.clone(),
                to: to.to_string(),
                data: mail::render_with(&msg, &render),
            },
        );
        seq
    }

    fn message(to: &str, subject: &str, body: Body) -> Message {
        Message {
            from: FROM.to_string(),
            to: to.to_string(),
            subject: subject.to_string(),
            body,
        }
    }

    fn signup_mail(
        &mut self,
        owner: &Account,
        to: &str,
        subject: &str,
        role: TokenRole,
    ) -> Message {
        let token = self.mint(&owner.identity, Flow::Signup, role);
        let link = self.link(&token);
        Self::message(
            to,
            subject,
            Body::Alternative {
                text: format!(
                    "Hi {to},\n\nConfirm your account: {link}\n\nThanks for signing up.\n"
                ),
                html: format!(
                    "<p>Hi {to},</p>\n<p><a href=\"{link}\">Confirm your account</a></p>\n<p>Thanks for signing up.</p>\n"
                ),
            },
        )
    }

    /// Mint a new 2FA generation for `account` and enqueue its mail.
    fn issue_otp(&mut self, account: &Account) -> u64 {
        let generation = self
            .otp_generation
            .get(&account.identity)
            .copied()
            .unwrap_or(0)
            + 1;
        self.otp_generation
            .insert(account.identity.clone(), generation);
        let code = loop {
            use rand::Rng;
            let n: u32 = self.private_rng.random_range(100_000..1_000_000);
            let code = n.to_string();
            if !self.tokens.contains_key(&code) {
                break code;
            }
        };
        let issued = (self.clock)();
        self.tokens.insert(
            code.clone(),
            Token {
                owner: account.identity.clone(),
                flow: Flow::Otp,
                role: TokenRole::Otp { generation },
                issued,
                consumed: false,
            },
        );
        let text = if generation == 1 {
            format!("Your verification code is {code}.\n\nIt expires in 10 minutes.\n")
        } else {
            format!(
                "Your new verification code is {code}.\n\nAny earlier code no longer works. It expires in 10 minutes.\n"
            )
        };
        let msg = Self::message(
            &account.identity,
            "Your verification code",
            Body::Text(text),
        );
        self.enqueue(
            account,
            &account.identity.clone(),
            Flow::Otp,
            &format!("otp-{generation}"),
            msg,
        )
    }

    /// Plan the scenario's mail for every agent identity. Returns the
    /// deliveries to send now, in order; held ones stay queued.
    fn plan(&mut self) -> Vec<u64> {
        let mut send = Vec::new();
        for acct in self.agents() {
            let to = acct.identity.clone();
            match self.task.scenario {
                Scenario::Signup | Scenario::SignupRetry => {
                    let msg =
                        self.signup_mail(&acct, &to, "Confirm your account", TokenRole::Confirm);
                    send.push(self.enqueue(&acct, &to, Flow::Signup, "signup", msg));
                }
                Scenario::SignupSlow => {
                    let msg =
                        self.signup_mail(&acct, &to, "Confirm your account", TokenRole::Confirm);
                    self.enqueue(&acct, &to, Flow::Signup, "signup", msg);
                }
                Scenario::SignupDecoyLinks => {
                    let unsub = self.mint(&to, Flow::Signup, TokenRole::DecoyLink);
                    let confirm = self.mint(&to, Flow::Signup, TokenRole::Confirm);
                    let report = self.mint(&to, Flow::Signup, TokenRole::DecoyLink);
                    let (u, c, r) = (self.link(&unsub), self.link(&confirm), self.link(&report));
                    let msg = Self::message(
                        &to,
                        "Confirm your account",
                        Body::Alternative {
                            text: format!(
                                "Hi {to},\n\nUnsubscribe from these emails: {u}\nConfirm your account: {c}\nReport this sign-up as not you: {r}\n"
                            ),
                            html: format!(
                                "<p>Hi {to},</p>\n<p><a href=\"{u}\">Unsubscribe</a></p>\n<p><a href=\"{c}\">Confirm</a></p>\n<p><a href=\"{r}\">Report</a></p>\n"
                            ),
                        },
                    );
                    send.push(self.enqueue(&acct, &to, Flow::Signup, "signup-decoy-links", msg));
                }
                Scenario::SignupSibling => {
                    let (local, domain) = to.split_once('@').expect("identities are addresses");
                    let sibling = format!("{local}.team@{domain}");
                    let decoy = Account {
                        identity: sibling.clone(),
                        inbox: acct.inbox.clone(),
                        role: Role::Decoy,
                        facts: Facts::default(),
                        password: None,
                    };
                    self.accounts.push(decoy.clone());
                    let msg =
                        self.signup_mail(&acct, &to, "Confirm your account", TokenRole::Confirm);
                    send.push(self.enqueue(&acct, &to, Flow::Signup, "signup", msg));
                    // The sibling's mail lands last: "the latest mail" is the trap.
                    let msg = self.signup_mail(
                        &decoy,
                        &sibling,
                        "Confirm your account",
                        TokenRole::Confirm,
                    );
                    send.push(self.enqueue(&decoy, &sibling, Flow::Signup, "sibling-decoy", msg));
                }
                Scenario::SignupI18n => {
                    let msg = self.signup_mail(&acct, &to, tasks::I18N_SUBJECT, TokenRole::Confirm);
                    send.push(self.enqueue(&acct, &to, Flow::Signup, "signup-i18n", msg));
                    let msg = self.signup_mail(
                        &acct,
                        &to,
                        tasks::I18N_DECOY_SUBJECT,
                        TokenRole::DecoyMessage,
                    );
                    send.push(self.enqueue(&acct, &to, Flow::Signup, "i18n-decoy", msg));
                }
                Scenario::Otp => send.push(self.issue_otp(&acct)),
                Scenario::OtpStale => {
                    send.push(self.issue_otp(&acct));
                    send.push(self.issue_otp(&acct));
                }
                Scenario::MagicLogin => {
                    let decoy = self.mint(&to, Flow::MagicLogin, TokenRole::DecoyLink);
                    let magic = self.mint(&to, Flow::MagicLogin, TokenRole::MagicLogin);
                    let (d, m) = (self.link(&decoy), self.link(&magic));
                    let msg = Self::message(
                        &to,
                        "Your sign-in link",
                        Body::Alternative {
                            text: format!(
                                "If this wasn't you, secure your account: {d}\n\nSign in to your account: {m}\n"
                            ),
                            html: format!(
                                "<p><a href=\"{d}\">This wasn't me</a></p>\n<p><a href=\"{m}\">Sign in</a></p>\n"
                            ),
                        },
                    );
                    send.push(self.enqueue(&acct, &to, Flow::MagicLogin, "magic-login", msg));
                }
                Scenario::Marketing => {
                    let offer = self.mint(&to, Flow::Marketing, TokenRole::MarketingOffer);
                    let view = self.mint(&to, Flow::Marketing, TokenRole::DecoyLink);
                    let unsub = self.mint(&to, Flow::Marketing, TokenRole::DecoyLink);
                    let app = self.app_url.clone();
                    let (v, u) = (self.link(&view), self.link(&unsub));
                    let html = format!(
                        "<html><body>\n<p><a href=\"{v}\">View this email in your browser</a></p>\n<h1>Autumn sale</h1>\n<p>Our {offer_name} is back.</p>\n<p><a href=\"{app}/m/offer?campaign&#61;autumn&amp;item&#61;lamp&amp;sig&#61;{offer}\">Claim the {offer_name} offer</a></p>\n<p><a href=\"{u}\">Unsubscribe</a></p>\n</body></html>\n",
                        offer_name = tasks::MARKETING_OFFER
                    );
                    let msg =
                        Self::message(&to, "Autumn sale: the lamp is back", Body::HtmlOnly(html));
                    send.push(self.enqueue(&acct, &to, Flow::Marketing, "marketing", msg));
                }
                Scenario::Digest => {
                    let slot = self
                        .accounts
                        .iter()
                        .position(|a| a.identity == to)
                        .expect("account") as u64;
                    let right =
                        self.inputs
                            .select(&self.task.id, slot, "digest", 0, "target-position", 40)
                            as usize;
                    let mut text = String::from("This week's 40 articles:\n\n");
                    let mut html = String::from("<h1>Weekly digest</h1>\n<ol>\n");
                    for i in 0..40 {
                        let (title, role) = if i == right {
                            (tasks::DIGEST_ITEM.to_string(), TokenRole::DigestRight)
                        } else {
                            (digest_decoy_title(i), TokenRole::DigestWrong)
                        };
                        let token = self.mint(&to, Flow::Digest, role);
                        let link = self.link(&token);
                        text.push_str(&format!("{}. {title}\n   {link}\n", i + 1));
                        html.push_str(&format!("<li><a href=\"{link}\">{title}</a></li>\n"));
                    }
                    html.push_str("</ol>\n");
                    let msg =
                        Self::message(&to, "Your weekly digest", Body::Alternative { text, html });
                    send.push(self.enqueue(&acct, &to, Flow::Digest, "digest", msg));
                }
                Scenario::Reset => {
                    self.account_mut(&to).password = Some(self.random_alnum(16));
                    let token = self.mint(&to, Flow::Reset, TokenRole::Reset);
                    let link = self.link(&token);
                    let msg = Self::message(
                        &to,
                        "Reset your password",
                        Body::Text(format!(
                            "Someone asked to reset the password for {to}.\n\nReset your password: {link}\n\nThe link works once.\n"
                        )),
                    );
                    send.push(self.enqueue(&acct, &to, Flow::Reset, "reset", msg));
                }
                Scenario::Invite => {
                    let token = self.mint(&to, Flow::Invite, TokenRole::Invite);
                    let link = self.link(&token);
                    let slot = self
                        .accounts
                        .iter()
                        .position(|a| a.identity == to)
                        .expect("account") as u64;
                    let days =
                        3 + self
                            .inputs
                            .select(&self.task.id, slot, "invite", 0, "start-days", 17);
                    let hour =
                        8 + self
                            .inputs
                            .select(&self.task.id, slot, "invite", 0, "start-hour", 10)
                            as u32;
                    let start = (self.inputs.time().date_naive() + chrono::Days::new(days))
                        .and_hms_opt(hour, 30, 0)
                        .expect("a valid time");
                    let end = start + chrono::Duration::hours(1);
                    let fmt = |t: chrono::NaiveDateTime| t.format("%Y%m%dT%H%M%SZ").to_string();
                    let uid = format!("{}@fixture.eval", self.random_alnum(12));
                    let ics = mail::ics_with_time(
                        &uid,
                        "Harness design review",
                        &fmt(start),
                        &fmt(end),
                        &to,
                        self.inputs.time(),
                    );
                    self.invites.insert(
                        token.clone(),
                        Invite {
                            uid,
                            start: fmt(start),
                            ics: ics.clone(),
                        },
                    );
                    let msg = Self::message(
                        &to,
                        "Invitation: Harness design review",
                        Body::Mixed {
                            text: format!(
                                "You're invited to the Harness design review.\n\nThe time in your calendar file is authoritative (UTC).\nOpen the invitation: {link}\n\nThe calendar file is attached.\n"
                            ),
                            attachment: Attachment {
                                filename: "invite.ics".to_string(),
                                content_type: "text/calendar; method=REQUEST; charset=utf-8"
                                    .to_string(),
                                bytes: ics,
                            },
                        },
                    );
                    send.push(self.enqueue(&acct, &to, Flow::Invite, "invite", msg));
                }
                Scenario::Support => {
                    use rand::Rng;
                    let ticket = self.private_rng.random_range(10_000..100_000).to_string();
                    let words = ["lantern", "harbor", "copper", "meadow", "falcon"];
                    let challenge = format!(
                        "{}-{}",
                        words[self.private_rng.random_range(0..words.len())],
                        self.private_rng.random_range(1000..10_000)
                    );
                    self.tickets.insert(
                        ticket.clone(),
                        Ticket {
                            owner: to.clone(),
                            challenge: challenge.clone(),
                        },
                    );
                    let msg = Self::message(
                        &to,
                        &format!("[Ticket #{ticket}] We need more details"),
                        Body::Text(format!(
                            "We received your support ticket #{ticket}.\n\nTo reply, POST /support/reply with JSON {{\"ticket\": \"{ticket}\", \"challenge\": \"{challenge}\", \"reply\": \"<your reply>\"}}.\nReply challenge: {challenge}\n"
                        )),
                    );
                    send.push(self.enqueue(&acct, &to, Flow::Support, "support", msg));
                }
            }
        }
        send
    }

    /// Record one attempt's result against its delivery.
    fn record_attempt(&mut self, seq: u64, result: &SendResult) {
        let Some(d) = self.deliveries.iter_mut().find(|d| d.seq == seq) else {
            return;
        };
        d.attempts.push(result.attempt().clone());
        d.status = match result {
            SendResult::Delivered(_) => DeliveryStatus::Delivered,
            SendResult::Transient(_) if d.attempts.len() < MAX_ATTEMPTS => DeliveryStatus::Parked,
            _ => DeliveryStatus::Failed,
        };
        if d.status != DeliveryStatus::Parked {
            self.outbound.remove(&seq);
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            schema_version: SNAPSHOT_SCHEMA,
            trial_id: self.trial_id.clone(),
            task: self.task.id.clone(),
            frozen: self.frozen,
            identities: self
                .accounts
                .iter()
                .map(|a| IdentityState {
                    identity: a.identity.clone(),
                    inbox: a.inbox.clone(),
                    role: a.role,
                    facts: a.facts.clone(),
                })
                .collect(),
            deliveries: self.deliveries.clone(),
            events: self.events.clone(),
        }
    }

    /// Verify a 2FA code at instant `now` (the clock seam).
    fn verify_otp(&mut self, claimed: &str, code: &str, now: Instant) -> Response {
        let token = match self.present(claimed, code, Some(Flow::Otp), action::OTP_VERIFY) {
            Ok(t) => t,
            Err(r) => return *r,
        };
        let TokenRole::Otp { generation } = token.role else {
            return self.misuse(
                claimed,
                Some(Flow::Otp),
                action::OTP_VERIFY,
                "that is not a verification code",
            );
        };
        let current = self.otp_generation.get(claimed).copied().unwrap_or(0);
        if generation != current {
            self.record(
                Some(claimed),
                Some(Flow::Otp),
                action::SUPERSEDED_CODE,
                false,
                Some(FailureClass::StaleCode),
            );
            return reply(
                StatusCode::GONE,
                false,
                "that code was superseded by a newer one",
            );
        }
        if now.saturating_duration_since(token.issued) > OTP_TTL {
            self.record(
                Some(claimed),
                Some(Flow::Otp),
                "expired_code",
                false,
                Some(FailureClass::StaleCode),
            );
            return reply(StatusCode::GONE, false, "that code has expired");
        }
        let acct = self.account_mut(claimed);
        acct.facts.otp_verified = true;
        acct.facts.logged_in = true;
        self.record(
            Some(claimed),
            Some(Flow::Otp),
            action::OTP_VERIFY,
            true,
            None,
        );
        reply(StatusCode::OK, true, "verified; you are signed in")
    }
}

fn digest_decoy_title(i: usize) -> String {
    const TOPICS: [&str; 13] = [
        "Async closures stabilise in nightly",
        "Async drop RFC enters final comment period",
        "Drop check gets a new home",
        "Stable sorting gets faster",
        "A tour of the new trait solver",
        "Async traits one year on",
        "Lands on stable: let chains",
        "Final comment period: async iterators",
        "Cargo's new resolver explained",
        "Unsafe fields: the proposal",
        "Embedded working group update",
        "Async fn in dyn traits, status report",
        "Why drop order matters",
    ];
    format!(
        "{} (part {})",
        TOPICS[i % TOPICS.len()],
        i / TOPICS.len() + 1
    )
}

/// The per-flow fact names the read-only state endpoint exposes.
fn flow_facts(flow: Flow, f: &Facts) -> Value {
    match flow {
        Flow::Signup => json!({ "confirmed": f.confirmed, "slow_released": f.slow_released }),
        Flow::Otp => json!({ "otp_verified": f.otp_verified, "logged_in": f.logged_in }),
        Flow::Reset => json!({
            "reset_count": f.reset_count,
            "reset_replay_rejected": f.reset_replay_rejected,
            "post_reset_login": f.post_reset_login,
            "logged_in": f.logged_in,
        }),
        Flow::MagicLogin => json!({ "magic_login": f.magic_login, "logged_in": f.logged_in }),
        Flow::Invite => json!({
            "invite_accepted": f.invite_accepted,
            "invite_time_matched": f.invite_time_matched,
        }),
        Flow::Support => json!({ "reply_received": f.reply_received }),
        Flow::Digest => json!({ "digest_selected": f.digest_selected }),
        Flow::Marketing => json!({ "marketing_completed": f.marketing_completed }),
    }
}

struct Shared {
    private: Mutex<Private>,
    smtp: SocketAddr,
    admin_secret: String,
}

type AppState = Arc<Shared>;

impl Shared {
    fn lock(&self) -> std::sync::MutexGuard<'_, Private> {
        self.private.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Send the given deliveries in order, outside the lock.
    async fn deliver(&self, seqs: &[u64]) {
        for &seq in seqs {
            let Some(out) = self.lock().outbound.get(&seq).cloned() else {
                continue;
            };
            let result = mail::send(self.smtp, &out.inbox, FROM, &out.to, &out.data).await;
            let mut p = self.lock();
            if !p.frozen {
                p.record_attempt(seq, &result);
            }
        }
    }
}

fn frozen_reply() -> Response {
    reply(StatusCode::CONFLICT, false, "the trial is over")
}

/// Parse a JSON body strictly; a malformed one is tool misuse.
fn body<T: for<'de> Deserialize<'de>>(bytes: &Bytes) -> Option<T> {
    serde_json::from_slice(bytes).ok()
}

async fn index(State(app): State<AppState>) -> Response {
    let base = app.lock().app_url.clone();
    reply_json(json!({
        "app": "swarmail-evals fixture",
        "base_url": base,
        "identity_header": "X-Eval-Identity",
        "note": "Every request must carry the X-Eval-Identity header naming your identity. Bodies are JSON.",
        "operations": [
            {"method": "GET", "path": "/a/{token}", "does": "follow a link from one of your emails"},
            {"method": "POST", "path": "/otp/verify", "body": {"code": "string"}, "does": "submit a two-factor verification code"},
            {"method": "POST", "path": "/otp/resend", "body": {}, "does": "email a new verification code; earlier codes stop working"},
            {"method": "POST", "path": "/reset/complete", "body": {"token": "the token from your reset link", "password": "string"}, "does": "set a new password with a single-use reset link"},
            {"method": "POST", "path": "/login", "body": {"password": "string"}, "does": "sign in with your password"},
            {"method": "GET", "path": "/invite/{token}/event.ics", "does": "download the invitation's calendar file"},
            {"method": "POST", "path": "/invite/accept", "body": {"token": "string", "uid": "the ICS UID", "start": "the ICS DTSTART in UTC, e.g. 20261026T153000Z"}, "does": "accept an invitation"},
            {"method": "POST", "path": "/support/reply", "body": {"ticket": "string", "challenge": "string", "reply": "string"}, "does": "reply to a support ticket"},
            {"method": "POST", "path": "/deliveries/release", "body": {}, "does": "release mail the application is holding for you"}
        ]
    }))
}

async fn follow_link(
    State(app): State<AppState>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Response {
    let mut p = app.lock();
    if p.frozen {
        return frozen_reply();
    }
    let claimed = match p.claim(&headers, None, "follow_link") {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let t = match p.present(&claimed, &token, None, "follow_link") {
        Ok(t) => t,
        Err(r) => return *r,
    };
    let flow = Some(t.flow);
    match t.role {
        TokenRole::Confirm => {
            p.account_mut(&claimed).facts.confirmed = true;
            p.record(Some(&claimed), flow, action::CONFIRM, true, None);
            reply(StatusCode::OK, true, "your account is confirmed")
        }
        TokenRole::DecoyLink => {
            p.record(
                Some(&claimed),
                flow,
                action::DECOY_LINK,
                false,
                Some(FailureClass::WrongLink),
            );
            reply(StatusCode::OK, true, "done: your preference was recorded")
        }
        TokenRole::DecoyMessage => {
            p.record(
                Some(&claimed),
                flow,
                action::DECOY_MESSAGE,
                false,
                Some(FailureClass::WrongMessage),
            );
            reply(
                StatusCode::OK,
                true,
                "your newsletter subscription is confirmed",
            )
        }
        TokenRole::MagicLogin => {
            let acct = p.account_mut(&claimed);
            acct.facts.magic_login = true;
            acct.facts.logged_in = true;
            p.record(Some(&claimed), flow, action::MAGIC_LOGIN, true, None);
            reply(StatusCode::OK, true, "you are signed in")
        }
        TokenRole::DigestRight => {
            p.account_mut(&claimed).facts.digest_selected = true;
            p.record(Some(&claimed), flow, action::DIGEST_SELECT, true, None);
            reply(
                StatusCode::OK,
                true,
                format!("article opened: {}", tasks::DIGEST_ITEM),
            )
        }
        TokenRole::DigestWrong => {
            p.record(
                Some(&claimed),
                flow,
                action::DIGEST_DECOY,
                false,
                Some(FailureClass::WrongLink),
            );
            reply(StatusCode::OK, true, "article opened")
        }
        TokenRole::Reset => {
            p.record(Some(&claimed), flow, "reset_page", true, None);
            reply_json(json!({
                "ok": true,
                "message": "choose a new password",
                "how": "POST /reset/complete with JSON {\"token\": \"<this link's token>\", \"password\": \"<new password>\"}",
                "token": token,
            }))
        }
        TokenRole::Invite => {
            p.record(Some(&claimed), flow, "invite_page", true, None);
            let base = p.app_url.clone();
            reply_json(json!({
                "ok": true,
                "event": "Harness design review",
                "ics_url": format!("{base}/invite/{token}/event.ics"),
                "accept": "POST /invite/accept with JSON {\"token\": \"<this link's token>\", \"uid\": \"<ICS UID>\", \"start\": \"<ICS DTSTART, UTC>\"}",
                "token": token,
            }))
        }
        TokenRole::MarketingOffer | TokenRole::Otp { .. } => {
            p.misuse(&claimed, flow, "follow_link", "that is not a link")
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CodeBody {
    code: String,
}

async fn otp_verify(State(app): State<AppState>, headers: HeaderMap, bytes: Bytes) -> Response {
    let mut p = app.lock();
    if p.frozen {
        return frozen_reply();
    }
    let claimed = match p.claim(&headers, Some(Flow::Otp), action::OTP_VERIFY) {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let Some(CodeBody { code }) = body(&bytes) else {
        return p.misuse(
            &claimed,
            Some(Flow::Otp),
            action::OTP_VERIFY,
            "body must be {\"code\": \"...\"}",
        );
    };
    let now = (p.clock)();
    p.verify_otp(&claimed, code.trim(), now)
}

async fn otp_resend(State(app): State<AppState>, headers: HeaderMap) -> Response {
    let seq = {
        let mut p = app.lock();
        if p.frozen {
            return frozen_reply();
        }
        let claimed = match p.claim(&headers, Some(Flow::Otp), "otp_resend") {
            Ok(c) => c,
            Err(r) => return *r,
        };
        if !p.otp_generation.contains_key(&claimed) {
            return p.misuse(
                &claimed,
                Some(Flow::Otp),
                "otp_resend",
                "no sign-in is in progress",
            );
        }
        p.record(Some(&claimed), Some(Flow::Otp), "otp_resend", true, None);
        let acct = p
            .accounts
            .iter()
            .find(|a| a.identity == claimed)
            .cloned()
            .expect("claimed");
        p.issue_otp(&acct)
    };
    app.deliver(&[seq]).await;
    reply(
        StatusCode::OK,
        true,
        "a new code was sent; earlier codes no longer work",
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResetBody {
    token: String,
    password: String,
}

async fn reset_complete(State(app): State<AppState>, headers: HeaderMap, bytes: Bytes) -> Response {
    let mut p = app.lock();
    if p.frozen {
        return frozen_reply();
    }
    let flow = Some(Flow::Reset);
    let claimed = match p.claim(&headers, flow, action::RESET) {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let Some(ResetBody { token, password }) = body(&bytes) else {
        return p.misuse(
            &claimed,
            flow,
            action::RESET,
            "body must be {\"token\": \"...\", \"password\": \"...\"}",
        );
    };
    let t = match p.present(&claimed, token.trim(), flow, action::RESET) {
        Ok(t) => t,
        Err(r) => return *r,
    };
    if t.role != TokenRole::Reset {
        return p.misuse(&claimed, flow, action::RESET, "that is not a reset token");
    }
    if password.is_empty() {
        return p.misuse(
            &claimed,
            flow,
            action::RESET,
            "the password must not be empty",
        );
    }
    if t.consumed {
        p.account_mut(&claimed).facts.reset_replay_rejected = true;
        p.record(
            Some(&claimed),
            flow,
            action::RESET_REPLAY,
            false,
            Some(FailureClass::ToolMisuse),
        );
        return reply(StatusCode::GONE, false, "this reset link was already used");
    }
    p.tokens.get_mut(token.trim()).expect("present").consumed = true;
    let acct = p.account_mut(&claimed);
    acct.password = Some(password);
    acct.facts.reset_count += 1;
    p.record(Some(&claimed), flow, action::RESET, true, None);
    reply(StatusCode::OK, true, "your password was changed")
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginBody {
    password: String,
}

async fn login(State(app): State<AppState>, headers: HeaderMap, bytes: Bytes) -> Response {
    let mut p = app.lock();
    if p.frozen {
        return frozen_reply();
    }
    let claimed = match p.claim(&headers, Some(Flow::Reset), action::LOGIN) {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let Some(LoginBody { password }) = body(&bytes) else {
        return p.misuse(
            &claimed,
            Some(Flow::Reset),
            action::LOGIN,
            "body must be {\"password\": \"...\"}",
        );
    };
    let acct = p.account_mut(&claimed);
    if acct.password.as_deref() != Some(password.as_str()) {
        return p.misuse(
            &claimed,
            Some(Flow::Reset),
            "login_refused",
            "wrong password",
        );
    }
    acct.facts.logged_in = true;
    if acct.facts.reset_count > 0 {
        acct.facts.post_reset_login = true;
    }
    p.record(Some(&claimed), Some(Flow::Reset), action::LOGIN, true, None);
    reply(StatusCode::OK, true, "you are signed in")
}

async fn invite_ics(
    State(app): State<AppState>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> Response {
    let mut p = app.lock();
    if p.frozen {
        return frozen_reply();
    }
    let flow = Some(Flow::Invite);
    let claimed = match p.claim(&headers, flow, "ics_download") {
        Ok(c) => c,
        Err(r) => return *r,
    };
    if let Err(r) = p.present(&claimed, &token, flow, "ics_download") {
        return *r;
    }
    let Some(invite) = p.invites.get(&token).cloned() else {
        return p.misuse(
            &claimed,
            flow,
            "ics_download",
            "that is not an invitation token",
        );
    };
    p.record(Some(&claimed), flow, "ics_download", true, None);
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/calendar; charset=utf-8")],
        invite.ics,
    )
        .into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AcceptBody {
    token: String,
    uid: String,
    start: String,
}

/// `2026-10-26T15:30:00Z` and `20261026T153000Z` name the same instant.
fn basic_utc(s: &str) -> String {
    s.trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase()
}

async fn invite_accept(State(app): State<AppState>, headers: HeaderMap, bytes: Bytes) -> Response {
    let mut p = app.lock();
    if p.frozen {
        return frozen_reply();
    }
    let flow = Some(Flow::Invite);
    let claimed = match p.claim(&headers, flow, action::INVITE_ACCEPT) {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let Some(AcceptBody { token, uid, start }) = body(&bytes) else {
        return p.misuse(
            &claimed,
            flow,
            action::INVITE_ACCEPT,
            "body must be {\"token\", \"uid\", \"start\"}",
        );
    };
    if let Err(r) = p.present(&claimed, token.trim(), flow, action::INVITE_ACCEPT) {
        return *r;
    }
    let Some(invite) = p.invites.get(token.trim()).cloned() else {
        return p.misuse(
            &claimed,
            flow,
            action::INVITE_ACCEPT,
            "that is not an invitation token",
        );
    };
    if uid.trim() != invite.uid || basic_utc(&start) != invite.start {
        p.record(
            Some(&claimed),
            flow,
            "invite_mismatch",
            false,
            Some(FailureClass::WrongLink),
        );
        return reply(
            StatusCode::CONFLICT,
            false,
            "that UID or start time does not match the invitation",
        );
    }
    let acct = p.account_mut(&claimed);
    acct.facts.invite_accepted = true;
    acct.facts.invite_time_matched = true;
    p.record(Some(&claimed), flow, action::INVITE_ACCEPT, true, None);
    reply(StatusCode::OK, true, "invitation accepted")
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SupportBody {
    ticket: String,
    challenge: String,
    reply: String,
}

async fn support_reply(State(app): State<AppState>, headers: HeaderMap, bytes: Bytes) -> Response {
    let mut p = app.lock();
    if p.frozen {
        return frozen_reply();
    }
    let flow = Some(Flow::Support);
    let claimed = match p.claim(&headers, flow, action::SUPPORT_REPLY) {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let Some(SupportBody {
        ticket,
        challenge,
        reply: text,
    }) = body(&bytes)
    else {
        return p.misuse(
            &claimed,
            flow,
            action::SUPPORT_REPLY,
            "body must be {\"ticket\", \"challenge\", \"reply\"}",
        );
    };
    let ticket = ticket.trim().trim_start_matches('#').to_string();
    let Some(t) = p.tickets.get(&ticket).cloned() else {
        return p.misuse(&claimed, flow, action::SUPPORT_REPLY, "unknown ticket");
    };
    if t.owner != claimed {
        p.record(
            Some(&claimed),
            flow,
            action::FOREIGN_TOKEN,
            false,
            Some(FailureClass::WrongMessage),
        );
        return reply(
            StatusCode::FORBIDDEN,
            false,
            "that ticket belongs to another account",
        );
    }
    if challenge.trim() != t.challenge || text.trim().is_empty() {
        return p.misuse(
            &claimed,
            flow,
            action::SUPPORT_REPLY,
            "wrong challenge or empty reply",
        );
    }
    p.account_mut(&claimed).facts.reply_received = true;
    p.record(Some(&claimed), flow, action::SUPPORT_REPLY, true, None);
    reply(StatusCode::OK, true, "reply received")
}

async fn release(State(app): State<AppState>, headers: HeaderMap) -> Response {
    let held = {
        let mut p = app.lock();
        if p.frozen {
            return frozen_reply();
        }
        let claimed = match p.claim(&headers, None, action::RELEASE) {
            Ok(c) => c,
            Err(r) => return *r,
        };
        let held: Vec<u64> = p
            .deliveries
            .iter()
            .filter(|d| d.identity == claimed && d.status == DeliveryStatus::Held)
            .map(|d| d.seq)
            .collect();
        if held.is_empty()
            && !p
                .accounts
                .iter()
                .any(|a| a.identity == claimed && a.facts.slow_released)
        {
            return p.misuse(
                &claimed,
                None,
                action::RELEASE,
                "no mail is being held for you",
            );
        }
        p.account_mut(&claimed).facts.slow_released = true;
        p.record(Some(&claimed), None, action::RELEASE, true, None);
        held
    };
    app.deliver(&held).await;
    reply_json(json!({ "ok": true, "released": held.len() }))
}

/// The marketing landing: only the entity-decoded offer URL completes it.
async fn marketing_offer(
    State(app): State<AppState>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
) -> Response {
    let mut p = app.lock();
    if p.frozen {
        return frozen_reply();
    }
    let flow = Some(Flow::Marketing);
    let claimed = match p.claim(&headers, flow, action::MARKETING) {
        Ok(c) => c,
        Err(r) => return *r,
    };
    let query = query.unwrap_or_default();
    let params: Vec<(&str, &str)> = query
        .split('&')
        .map(|kv| kv.split_once('=').unwrap_or((kv, "")))
        .collect();
    let get = |k: &str| params.iter().find(|(key, _)| *key == k).map(|(_, v)| *v);
    let exact =
        params.len() == 3 && get("campaign") == Some("autumn") && get("item") == Some("lamp");
    let sig = get("sig").unwrap_or_default().to_string();
    if !exact || sig.is_empty() {
        p.record(
            Some(&claimed),
            flow,
            action::MARKETING_MANGLED,
            false,
            Some(FailureClass::WrongLink),
        );
        return reply(
            StatusCode::BAD_REQUEST,
            false,
            "this offer link is not valid",
        );
    }
    let t = match p.present(&claimed, &sig, flow, action::MARKETING) {
        Ok(t) => t,
        Err(r) => return *r,
    };
    if t.role != TokenRole::MarketingOffer {
        p.record(
            Some(&claimed),
            flow,
            action::MARKETING_MANGLED,
            false,
            Some(FailureClass::WrongLink),
        );
        return reply(
            StatusCode::BAD_REQUEST,
            false,
            "this offer link is not valid",
        );
    }
    p.account_mut(&claimed).facts.marketing_completed = true;
    p.record(Some(&claimed), flow, action::MARKETING, true, None);
    reply(
        StatusCode::OK,
        true,
        format!("the {} offer is yours", tasks::MARKETING_OFFER),
    )
}

// ---- admin plane ----

fn authorized(app: &Shared, headers: &HeaderMap) -> bool {
    let want = format!("Bearer {}", app.admin_secret);
    headers
        .get(header::AUTHORIZATION)
        .is_some_and(|v| v.as_bytes() == want.as_bytes())
}

fn unauthorized() -> Response {
    reply(
        StatusCode::UNAUTHORIZED,
        false,
        "admin bearer secret required",
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvisionIdentity {
    pub identity: String,
    pub inbox: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProvisionBody {
    identities: Vec<ProvisionIdentity>,
}

async fn admin_provision(
    State(app): State<AppState>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Response {
    if !authorized(&app, &headers) {
        return unauthorized();
    }
    let Some(ProvisionBody { identities }) = body(&bytes) else {
        return reply(
            StatusCode::BAD_REQUEST,
            false,
            "body must be {\"identities\": [...]}",
        );
    };
    let mut p = app.lock();
    if p.provisioned || p.frozen {
        return reply(StatusCode::CONFLICT, false, "already provisioned");
    }
    if identities.len() != p.task.agent_count as usize {
        return reply(
            StatusCode::BAD_REQUEST,
            false,
            "identity count does not match the task",
        );
    }
    for id in identities {
        p.accounts.push(Account {
            identity: id.identity,
            inbox: id.inbox,
            role: Role::Agent,
            facts: Facts::default(),
            password: None,
        });
    }
    p.provisioned = true;
    reply(StatusCode::OK, true, "provisioned")
}

fn deliveries_json(p: &Private) -> Response {
    reply_json(json!({ "deliveries": p.deliveries }))
}

async fn admin_prepare(State(app): State<AppState>, headers: HeaderMap) -> Response {
    if !authorized(&app, &headers) {
        return unauthorized();
    }
    let send = {
        let mut p = app.lock();
        if !p.provisioned || p.prepared || p.frozen {
            return reply(
                StatusCode::CONFLICT,
                false,
                "prepare needs a provisioned, unprepared trial",
            );
        }
        p.prepared = true;
        p.plan()
    };
    app.deliver(&send).await;
    deliveries_json(&app.lock())
}

async fn admin_retry(State(app): State<AppState>, headers: HeaderMap) -> Response {
    if !authorized(&app, &headers) {
        return unauthorized();
    }
    let parked: Vec<u64> = app
        .lock()
        .deliveries
        .iter()
        .filter(|d| d.status == DeliveryStatus::Parked)
        .map(|d| d.seq)
        .collect();
    app.deliver(&parked).await;
    deliveries_json(&app.lock())
}

async fn admin_freeze(State(app): State<AppState>, headers: HeaderMap) -> Response {
    if !authorized(&app, &headers) {
        return unauthorized();
    }
    app.lock().frozen = true;
    reply(StatusCode::OK, true, "frozen")
}

async fn admin_snapshot(State(app): State<AppState>, headers: HeaderMap) -> Response {
    if !authorized(&app, &headers) {
        return unauthorized();
    }
    let snapshot = app.lock().snapshot();
    if crate::state::validate_publication(&snapshot).is_err() {
        return reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            false,
            "snapshot publication failed",
        );
    }
    reply_json(serde_json::to_value(snapshot).expect("a snapshot serializes"))
}

async fn admin_flow_state(
    State(app): State<AppState>,
    headers: HeaderMap,
    Path(flow): Path<String>,
) -> Response {
    if !authorized(&app, &headers) {
        return unauthorized();
    }
    let Some(flow) = Flow::parse(&flow) else {
        return reply(StatusCode::NOT_FOUND, false, "unknown flow");
    };
    let p = app.lock();
    let identities: Vec<Value> = p
        .accounts
        .iter()
        .map(|a| json!({ "identity": a.identity, "role": a.role, "facts": flow_facts(flow, &a.facts) }))
        .collect();
    let deliveries: Vec<&Delivery> = p.deliveries.iter().filter(|d| d.flow == flow).collect();
    reply_json(json!({ "flow": flow, "identities": identities, "deliveries": deliveries }))
}

/// How a fixture is started.
pub struct FixtureConfig {
    pub smtp: SocketAddr,
    pub task: Task,
    pub trial_id: String,
    pub admin_secret: String,
    pub clock: Clock,
    pub inputs: crate::provenance::FixtureInputs,
}

/// A running fixture: its two planes and its stop switch.
pub struct Running {
    pub public: SocketAddr,
    pub admin: SocketAddr,
    stop: tokio::sync::watch::Sender<bool>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl Running {
    pub async fn stop(self) {
        let _ = self.stop.send(true);
        for t in self.tasks {
            let _ = t.await;
        }
    }
}

/// Bind both planes on loopback port zero and serve them.
pub async fn start(cfg: FixtureConfig) -> std::io::Result<Running> {
    let public_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let admin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let public = public_listener.local_addr()?;
    let admin = admin_listener.local_addr()?;
    let app_url = format!("http://{public}");
    let shared = Arc::new(Shared {
        private: Mutex::new(Private::with_inputs(
            &cfg.trial_id,
            cfg.task,
            &app_url,
            cfg.clock,
            cfg.inputs,
        )),
        smtp: cfg.smtp,
        admin_secret: cfg.admin_secret,
    });
    let public_router = Router::new()
        .route("/", get(index))
        .route("/a/{token}", get(follow_link))
        .route("/otp/verify", post(otp_verify))
        .route("/otp/resend", post(otp_resend))
        .route("/reset/complete", post(reset_complete))
        .route("/login", post(login))
        .route("/invite/{token}/event.ics", get(invite_ics))
        .route("/invite/accept", post(invite_accept))
        .route("/support/reply", post(support_reply))
        .route("/deliveries/release", post(release))
        .route("/m/offer", get(marketing_offer))
        .with_state(shared.clone());
    let admin_router = Router::new()
        .route("/admin/provision", post(admin_provision))
        .route("/admin/prepare", post(admin_prepare))
        .route("/admin/retry", post(admin_retry))
        .route("/admin/freeze", post(admin_freeze))
        .route("/admin/snapshot", get(admin_snapshot))
        .route("/admin/state/{flow}", get(admin_flow_state))
        .with_state(shared);
    let (stop, rx) = tokio::sync::watch::channel(false);
    let serve = |listener: tokio::net::TcpListener,
                 router: Router,
                 mut rx: tokio::sync::watch::Receiver<bool>| {
        tokio::spawn(async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = rx.wait_for(|stopped| *stopped).await;
                })
                .await;
        })
    };
    let tasks = vec![
        serve(public_listener, public_router, rx.clone()),
        serve(admin_listener, admin_router, rx),
    ];
    Ok(Running {
        public,
        admin,
        stop,
        tasks,
    })
}

/// The `fixture` subcommand: start, announce both addresses on one stdout
/// line, serve until SIGTERM or SIGINT.
pub async fn run_cli(smtp: SocketAddr, task_id: &str, trial_id: &str) -> Result<(), String> {
    run_cli_with_inputs(smtp, task_id, trial_id, Default::default()).await
}
pub async fn run_cli_with_inputs(
    smtp: SocketAddr,
    task_id: &str,
    trial_id: &str,
    inputs: crate::provenance::FixtureInputs,
) -> Result<(), String> {
    inputs.validate()?;
    let task = tasks::find(task_id).ok_or_else(|| format!("unknown task {task_id}"))?;
    let admin_secret = std::env::var(ADMIN_SECRET_ENV)
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("{ADMIN_SECRET_ENV} must be set"))?;
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|e| e.to_string())?;
    let mut int = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
        .map_err(|e| e.to_string())?;
    let running = start(FixtureConfig {
        smtp,
        task,
        trial_id: trial_id.to_string(),
        admin_secret,
        clock: Arc::new(Instant::now),
        inputs,
    })
    .await
    .map_err(|e| format!("bind failed: {e}"))?;
    {
        use std::io::Write;
        let mut out = std::io::stdout().lock();
        let _ = writeln!(
            out,
            "fixture public={} admin={}",
            running.public, running.admin
        );
        let _ = out.flush();
    }
    tokio::select! {
        _ = term.recv() => {}
        _ = int.recv() => {}
    }
    running.stop().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn private_with_clock(clock: Clock) -> Private {
        let mut p = Private::new(
            "t1",
            tasks::find("otp").unwrap(),
            "http://127.0.0.1:1",
            clock,
        );
        p.accounts.push(Account {
            identity: "a@example.test".into(),
            inbox: "inbox-a".into(),
            role: Role::Agent,
            facts: Facts::default(),
            password: None,
        });
        p
    }

    fn code_for(p: &Private, generation: u32) -> String {
        p.tokens
            .iter()
            .find(|(_, t)| t.role == TokenRole::Otp { generation })
            .map(|(k, _)| k.clone())
            .unwrap()
    }

    /// Expiry through the clock seam: generated instants, no waiting.
    #[test]
    fn an_expired_code_is_stale_and_a_fresh_one_verifies() {
        let base = Instant::now();
        let offset = Arc::new(AtomicU64::new(0));
        let o = offset.clone();
        let clock: Clock = Arc::new(move || base + Duration::from_secs(o.load(Ordering::SeqCst)));
        let mut p = private_with_clock(clock);
        let acct = p.accounts[0].clone();
        p.issue_otp(&acct);
        let code = code_for(&p, 1);
        let late = base + OTP_TTL + Duration::from_secs(1);
        let r = p.verify_otp("a@example.test", &code, late);
        assert_eq!(r.status(), StatusCode::GONE);
        assert_eq!(
            p.events.last().unwrap().category,
            Some(FailureClass::StaleCode)
        );
        assert!(!p.accounts[0].facts.otp_verified);
        let r = p.verify_otp("a@example.test", &code, base + Duration::from_secs(5));
        assert_eq!(r.status(), StatusCode::OK);
        assert!(p.accounts[0].facts.otp_verified && p.accounts[0].facts.logged_in);
    }

    #[test]
    fn a_superseded_code_is_stale_even_before_expiry() {
        let mut p = private_with_clock(Arc::new(Instant::now));
        let acct = p.accounts[0].clone();
        p.issue_otp(&acct);
        p.issue_otp(&acct);
        let old = code_for(&p, 1);
        let new = code_for(&p, 2);
        assert_ne!(old, new);
        let now = Instant::now();
        assert_eq!(
            p.verify_otp("a@example.test", &old, now).status(),
            StatusCode::GONE
        );
        assert_eq!(p.events.last().unwrap().action, action::SUPERSEDED_CODE);
        assert_eq!(
            p.verify_otp("a@example.test", &new, now).status(),
            StatusCode::OK
        );
    }

    #[test]
    fn invite_times_compare_in_basic_utc() {
        assert_eq!(basic_utc("2026-10-26T15:30:00Z"), "20261026T153000Z");
        assert_eq!(basic_utc(" 20261026t153000z "), "20261026T153000Z");
    }
}

#[cfg(test)]
mod entropy_proofs {
    use super::*;
    use rand::SeedableRng;
    fn unplanned(task: Task, seed: &str) -> Private {
        let inputs =
            crate::provenance::FixtureInputs::resolve(Some(seed), Some("2026-10-24T12:00:00Z"))
                .unwrap();
        let mut p = Private::with_inputs(
            "entropy-proof",
            task.clone(),
            "http://127.0.0.1:1",
            Arc::new(Instant::now),
            inputs,
        );
        for i in 0..task.agent_count {
            p.accounts.push(Account {
                identity: format!("person-{i}@example.test"),
                inbox: format!("inbox-{i}"),
                role: Role::Agent,
                facts: Facts::default(),
                password: None,
            });
        }
        p
    }
    fn planned(task: Task, seed: &str) -> Private {
        let mut p = unplanned(task, seed);
        // This seam is per-instance and only available inside the test unit.
        p.private_rng = rand::rngs::StdRng::seed_from_u64(718491);
        p.plan();
        p
    }
    fn private_projection(p: &Private) -> Value {
        let mut tokens = p.tokens.keys().cloned().collect::<Vec<_>>();
        tokens.sort();
        let mut tickets = p
            .tickets
            .iter()
            .map(|(k, t)| (k.clone(), t.challenge.clone()))
            .collect::<Vec<_>>();
        tickets.sort();
        let mut uids = p
            .invites
            .values()
            .map(|i| i.uid.clone())
            .collect::<Vec<_>>();
        uids.sort();
        json!({"tokens_and_otp":tokens,"tickets_and_challenges":tickets,"uids":uids,"passwords":p.accounts.iter().map(|a|a.password.clone()).collect::<Vec<_>>()})
    }
    #[test]
    fn production_construction_keeps_secrets_independent_of_public_seed() {
        let seed = "00".repeat(32);
        for task in tasks::catalogue() {
            let mut left = Vec::new();
            let mut right = Vec::new();
            // Compare four pairs per task: a single six-digit OTP can collide.
            for _ in 0..4 {
                let mut a = unplanned(task.clone(), &seed);
                let mut b = unplanned(task.clone(), &seed);
                for p in [&mut a, &mut b] {
                    p.plan();
                    assert!(
                        p.tokens
                            .keys()
                            .chain(p.tickets.keys())
                            .any(|s| !s.is_empty()),
                        "{} must generate secret material",
                        task.id
                    );
                    match task.scenario {
                        Scenario::Otp | Scenario::OtpStale => {
                            assert!(
                                p.tokens
                                    .values()
                                    .any(|t| matches!(t.role, TokenRole::Otp { .. }))
                            );
                        }
                        Scenario::Reset => {
                            assert!(p.tokens.values().any(|t| t.role == TokenRole::Reset));
                            assert!(p.accounts.iter().all(|a| {
                                a.password
                                    .as_ref()
                                    .is_some_and(|password| !password.is_empty())
                            }));
                        }
                        Scenario::Support => {
                            assert!(!p.tickets.is_empty());
                            assert!(
                                p.tickets
                                    .iter()
                                    .all(|(id, t)| { !id.is_empty() && !t.challenge.is_empty() })
                            );
                        }
                        Scenario::Invite => {
                            assert!(p.tokens.values().any(|t| t.role == TokenRole::Invite));
                            assert!(!p.invites.is_empty());
                            assert!(p.invites.values().all(|i| !i.uid.is_empty()));
                        }
                        _ => {}
                    }
                }
                left.push(private_projection(&a));
                right.push(private_projection(&b));
            }
            assert!(
                left != right,
                "{}: private projections repeat across independent constructors",
                task.id
            );
        }
    }
    #[test]
    fn every_fixture_private_class_uses_independent_entropy_not_public_seed() {
        for task in tasks::catalogue() {
            let a = planned(task.clone(), &"00".repeat(32));
            let b = planned(task, &"ab".repeat(32));
            assert_eq!(private_projection(&a), private_projection(&b));
            let public = serde_json::to_string(&a.snapshot()).unwrap();
            for secret in a.tokens.keys().chain(a.tickets.keys()) {
                assert!(!public.contains(secret));
            }
            for password in a.accounts.iter().filter_map(|a| a.password.as_ref()) {
                assert!(!public.contains(password));
            }
            for ticket in a.tickets.values() {
                assert!(!public.contains(&ticket.challenge));
            }
            for invite in a.invites.values() {
                assert!(!public.contains(&invite.uid));
            }
        }
    }
    #[test]
    fn changing_secret_consumption_cannot_change_public_message_suffixes_or_schedule() {
        let task = tasks::find("invite-time").unwrap();
        let a = planned(task.clone(), &"00".repeat(32));
        let mut b = Private::with_inputs(
            "other",
            task,
            "http://127.0.0.1:1",
            Arc::new(Instant::now),
            a.inputs.clone(),
        );
        b.private_rng = rand::rngs::StdRng::seed_from_u64(98);
        b.accounts = a.accounts.clone();
        b.random_alnum(117);
        b.plan();
        let header = |p: &Private| {
            String::from_utf8(p.outbound.values().next().unwrap().data.clone())
                .unwrap()
                .lines()
                .filter(|l| {
                    l.starts_with("Date:")
                        || l.starts_with("Message-ID:")
                        || l.starts_with("Content-Type: multipart/")
                })
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        assert_eq!(header(&a), header(&b));
        assert_eq!(
            a.invites.values().next().unwrap().start,
            b.invites.values().next().unwrap().start
        );
    }
}

#[cfg(test)]
mod publication_proofs {
    use super::*;

    #[test]
    fn lossy_unknown_header_bytes_never_enter_events() {
        let mut private = Private::new(
            "header",
            tasks::find("otp").unwrap(),
            "http://127.0.0.1:1",
            Arc::new(Instant::now),
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            IDENTITY_HEADER,
            axum::http::HeaderValue::from_bytes(b"\xffprivate-header-canary").unwrap(),
        );
        assert!(
            private
                .claim(&headers, Some(Flow::Otp), action::OTP_VERIFY)
                .is_err()
        );
        let event = private.events.last().unwrap();
        assert!(event.claimed.is_none());
        assert_eq!(event.category, Some(FailureClass::ToolMisuse));
    }

    #[tokio::test]
    async fn admin_refuses_noncanonical_state_without_serializing_it() {
        let mut private = Private::new(
            "guard",
            tasks::find("otp").unwrap(),
            "http://127.0.0.1:1",
            Arc::new(Instant::now),
        );
        // Per-instance test-only corruption seam; production can only record
        // canonical registry values. Drive the real handler over HTTP.
        private.events.push(Event {
            seq: 1,
            claimed: Some("private-admin-canary".into()),
            flow: None,
            action: "otp_verify".into(),
            accepted: false,
            category: Some(FailureClass::ToolMisuse),
        });
        let state = Arc::new(Shared {
            private: Mutex::new(private),
            smtp: "127.0.0.1:1".parse().unwrap(),
            admin_secret: "test-bearer".into(),
        });
        let router = Router::new()
            .route("/admin/snapshot", get(admin_snapshot))
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let client = reqwest::Client::new();
        let url = format!("http://{addr}/admin/snapshot");
        let response = client
            .get(&url)
            .bearer_auth("test-bearer")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 500);
        let body = response.text().await.unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&body).unwrap(),
            json!({"ok":false,"message":"snapshot publication failed"})
        );
        assert!(!body.contains("private-admin-canary"));
        assert_eq!(client.get(&url).send().await.unwrap().status(), 401);
        state.lock().events[0].claimed = None;
        let response = client
            .get(&url)
            .bearer_auth("test-bearer")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let snap: Snapshot = serde_json::from_str(&response.text().await.unwrap()).unwrap();
        assert!(!snap.frozen);
        server.abort();
        let _ = server.await;
    }
}
