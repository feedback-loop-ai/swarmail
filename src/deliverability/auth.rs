//! SPF and DMARC evaluation (issue #255, proposed decision 0126).
//!
//! Both run inside the lazy report evaluator, on `mail-auth` 0.14.0 exactly as
//! #254 pinned it, and read DNS from one place only: the declared map, through
//! `Lookups` (decision 0120). The crate parses records, walks the DMARC tree and
//! decides alignment; this module owns everything that decides a verdict around
//! it: which of a name's values is "the" record, which identity SPF judges, what
//! the crate is never allowed to ask for, and the mapping from its results to
//! stable, versioned rule ids named the way Authentication-Results (RFC 8601)
//! names them.
//!
//! The crate has a resolver compiled in and it is unreachable: every cache below
//! answers every query, and the one authenticator has no name servers. SPF terms
//! that need A, AAAA, MX or PTR answers (`a`, `mx`, `ptr`, `exists`, `%{p}`) get
//! a fixed negative and mark the evaluation *unsupported*: Swarmail did not look,
//! so the verdict is `spf.unsupported_mechanism.v1` and the `spf_pass` fact is
//! Unknown, never a failure (0126 §4).
//!
//! Ambient reads (0126 §8). SPF's `check_host` starts a monotonic timer and
//! refuses a DNS-querying term once 20 s have passed on it: the one reachable
//! clock read, held for the operator. Report sampling (`ra=`) reads the wall
//! clock and is made unreachable by clearing `ra=` from every record the adapter
//! hands over; `%{t}` is unreachable because an explanation is fetched as a
//! different record type than the one the adapter builds; DMARC reads no clock.

use super::dkim::{self, AUTHENTICATOR, Prepared, name_of, not_found, owner_of};
use super::dns::{Lookups, Txt};
use super::{
    AnalysisError, AuthEvidence, Collector, Fact, Finding, Headers, Location, Severity,
    header_location, values,
};
use crate::model::Email;
use futures_util::FutureExt;
use mail_auth::dmarc::verify::DmarcParameters;
use mail_auth::dmarc::{DmarcRecord, Policy};
use mail_auth::dns::TxtRecordParser;
use mail_auth::hickory_resolver::proto::op::ResponseCode;
use mail_auth::spf::SpfRecord;
use mail_auth::spf::verify::SpfParameters;
use mail_auth::{
    DkimOutput, DmarcResult, DnsCache, DnsError, Error, Negative, Parameters, RecordSet,
    ResolverCache, SpfOutput, SpfResult, TxtRecord,
};
use std::borrow::Borrow;
use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Instant;

/// The name of the host doing the check (SPF's `%{r}`): a constant, never the
/// configured hostname, so a report does not depend on configuration. `.invalid`
/// is reserved by RFC 2606 and never resolves.
const HOST: &str = "swarmail.invalid";

/// The SMTP envelope Swarmail observed for a message (decision 0126 §2), copied
/// out of the stored `Email`. Never read from a header.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Envelope<'e> {
    /// MAIL FROM's reverse path; `Some("")` is the null reverse path.
    pub(super) mail_from: Option<&'e str>,
    /// The connecting address, canonical.
    pub(super) client_ip: Option<IpAddr>,
    /// The last HELO/EHLO argument.
    pub(super) helo: Option<&'e str>,
}

impl<'e> Envelope<'e> {
    pub(super) fn of(email: &'e Email) -> Self {
        Self {
            mail_from: email.envelope_from.as_deref(),
            client_ip: email.smtp_client_ip,
            helo: email.smtp_helo.as_deref(),
        }
    }
}

/// The closed rule table. The guide lists the same ids and a test binds the two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rule {
    SpfFail,
    SpfSoftfail,
    SpfNeutral,
    SpfNone,
    SpfTemperror,
    SpfPermerror,
    SpfUnsupported,
    SpfNotEvaluated,
    DmarcFromInvalid,
    DmarcNone,
    DmarcTemperror,
    DmarcPermerror,
    DmarcFailNone,
    DmarcFailQuarantine,
    DmarcFailReject,
    DmarcNotEvaluated,
}

impl Rule {
    #[cfg(test)]
    const ALL: [Rule; 16] = [
        Self::SpfFail,
        Self::SpfSoftfail,
        Self::SpfNeutral,
        Self::SpfNone,
        Self::SpfTemperror,
        Self::SpfPermerror,
        Self::SpfUnsupported,
        Self::SpfNotEvaluated,
        Self::DmarcFromInvalid,
        Self::DmarcNone,
        Self::DmarcTemperror,
        Self::DmarcPermerror,
        Self::DmarcFailNone,
        Self::DmarcFailQuarantine,
        Self::DmarcFailReject,
        Self::DmarcNotEvaluated,
    ];

    const fn id(self) -> &'static str {
        match self {
            Self::SpfFail => "spf.fail.v1",
            Self::SpfSoftfail => "spf.softfail.v1",
            Self::SpfNeutral => "spf.neutral.v1",
            Self::SpfNone => "spf.none.v1",
            Self::SpfTemperror => "spf.temperror.v1",
            Self::SpfPermerror => "spf.permerror.v1",
            Self::SpfUnsupported => "spf.unsupported_mechanism.v1",
            Self::SpfNotEvaluated => "spf.not_evaluated.v1",
            Self::DmarcFromInvalid => "dmarc.from_invalid.v1",
            Self::DmarcNone => "dmarc.none.v1",
            Self::DmarcTemperror => "dmarc.temperror.v1",
            Self::DmarcPermerror => "dmarc.permerror.v1",
            Self::DmarcFailNone => "dmarc.fail.none.v1",
            Self::DmarcFailQuarantine => "dmarc.fail.quarantine.v1",
            Self::DmarcFailReject => "dmarc.fail.reject.v1",
            Self::DmarcNotEvaluated => "dmarc.not_evaluated.v1",
        }
    }

    /// Everything that could not be judged is an error, except that SPF was not
    /// run at all: that has no verdict to disqualify and is information.
    const fn severity(self) -> Severity {
        match self {
            Self::SpfNotEvaluated => Severity::Info,
            _ => Severity::Error,
        }
    }

    /// The fix hints are static: they never echo a domain, an address or a
    /// record from the message (decision 0089 §7).
    const fn hint(self) -> &'static str {
        match self {
            Self::SpfFail => {
                "The connecting address is not authorized by the SPF record of the envelope sender's domain, which ends in -all; add the address to the record or send from an authorized host (RFC 7208 section 2.6.4)."
            }
            Self::SpfSoftfail => {
                "The connecting address is probably not authorized by the SPF record of the envelope sender's domain (~all); add the address to the record or send from an authorized host (RFC 7208 section 2.6.5)."
            }
            Self::SpfNeutral => {
                "No mechanism of the SPF record of the envelope sender's domain matched the connecting address, or the record says ?all; add the address to the record (RFC 7208 section 2.6.2)."
            }
            Self::SpfNone => {
                "Declare a v=spf1 TXT record for the envelope sender's domain in the DNS policy; none is declared, or the domain is not a valid name (RFC 7208 section 2.6.1)."
            }
            Self::SpfTemperror => {
                "A DNS lookup the SPF check needed failed temporarily, so the check was not evaluated; fix the DNS answer and retry (RFC 7208 section 2.6.6)."
            }
            Self::SpfPermerror => {
                "Fix the SPF record: one v=spf1 record per name, valid syntax, declared include: and redirect= targets, and at most 10 DNS-querying terms (RFC 7208 sections 4.5, 4.6.4 and 2.6.7)."
            }
            Self::SpfUnsupported => {
                "The SPF record reaches a, mx, ptr, exists or a %{p} macro, which need address records the DNS policy cannot declare; Swarmail did not evaluate it, so this is not a failure. Put an ip4: or ip6: term before it, or declare spf_client_ip and an ip4:/ip6: record (RFC 7208 section 5)."
            }
            Self::SpfNotEvaluated => {
                "SPF was not evaluated: the message has no observed SMTP connection address, no known envelope sender, or a null sender without a HELO name. Send it over SMTP with a HELO and a MAIL FROM (RFC 7208 section 2.2)."
            }
            Self::DmarcFromInvalid => {
                "Send exactly one From header field whose addresses share one domain (RFC 5322 section 3.6.2, RFC 9989 section 4.4)."
            }
            Self::DmarcNone => {
                "Declare a v=DMARC1 record with a p= policy at _dmarc.<the From domain or its organizational domain> in the DNS policy; none applies (RFC 9989 section 4.10)."
            }
            Self::DmarcTemperror => {
                "A DNS lookup the DMARC policy discovery needed failed temporarily, so the message was not evaluated; fix the DNS answer and retry (RFC 9989 section 4.10)."
            }
            Self::DmarcPermerror => {
                "Fix the DMARC policy record: exactly one v=DMARC1 record per _dmarc name, with valid tags (RFC 9989 section 4.10)."
            }
            Self::DmarcFailNone => {
                "Neither SPF nor DKIM passed with a domain aligned to the From domain; the applied policy is none. Align a passing DKIM d= or SPF MAIL FROM domain with the From domain (RFC 9989 section 4.10)."
            }
            Self::DmarcFailQuarantine => {
                "Neither SPF nor DKIM passed with a domain aligned to the From domain; the applied policy is quarantine. Align a passing DKIM d= or SPF MAIL FROM domain with the From domain (RFC 9989 section 4.10)."
            }
            Self::DmarcFailReject => {
                "Neither SPF nor DKIM passed with a domain aligned to the From domain; the applied policy is reject. Align a passing DKIM d= or SPF MAIL FROM domain with the From domain (RFC 9989 section 4.10)."
            }
            Self::DmarcNotEvaluated => {
                "DMARC was not evaluated: the From header cannot be judged the way Swarmail's own header scan reads it, or SPF or DKIM could not be evaluated and no aligned pass was found (RFC 9989 section 4.10)."
            }
        }
    }

    /// SPF judges the SMTP envelope, which no stored header holds: its findings
    /// name the reverse path (RFC 5321 section 4.4) with no occurrence. DMARC
    /// findings sit at the root `From` field.
    fn location(self, fields: &Headers) -> Location {
        match self {
            Self::SpfFail
            | Self::SpfSoftfail
            | Self::SpfNeutral
            | Self::SpfNone
            | Self::SpfTemperror
            | Self::SpfPermerror
            | Self::SpfUnsupported
            | Self::SpfNotEvaluated => Location::Header {
                name: "return-path".into(),
                occurrence: None,
            },
            _ => header_location(fields, "from"),
        }
    }

    fn finding(self, fields: &Headers) -> Finding {
        Finding {
            rule_id: self.id().into(),
            location: self.location(fields),
            severity: self.severity(),
            fix_hint: self.hint().into(),
        }
    }
}

/// Which record type a name's values are selected for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Spf,
    Dmarc,
}

impl Kind {
    /// Whether a TXT value is a record of this type at all: it begins with the
    /// version tag and the tag ends there (RFC 7208 section 4.5 for `v=spf1`
    /// followed by a space or the end; RFC 9989 for `v=DMARC1` followed by `;`,
    /// a space or the end). A `v=spf10` value is not an SPF record.
    fn is_ours(self, value: &str) -> bool {
        let (tag, delimiters): (&str, &[u8]) = match self {
            Self::Spf => ("v=spf1", b" "),
            Self::Dmarc => ("v=dmarc1", b"; "),
        };
        let bytes = value.as_bytes();
        bytes.len() >= tag.len()
            && bytes[..tag.len()].eq_ignore_ascii_case(tag.as_bytes())
            && bytes
                .get(tag.len())
                .is_none_or(|next| delimiters.contains(next))
    }

    /// The record of a name's declared values. Swarmail selects it, not the
    /// crate (whose default keeps the first value that parses): no value of the
    /// type is "no record" and more than one is a parse error (RFC 7208 section
    /// 4.5; RFC 9989). SPF's `ra=` is cleared so the crate's report sampling,
    /// which reads the clock, is never reached.
    fn select(self, values: Vec<String>) -> TxtRecord {
        let mut ours = values.into_iter().filter(|value| self.is_ours(value));
        match (ours.next(), ours.next()) {
            (None, _) => not_found(),
            (Some(_), Some(_)) => TxtRecord::Error(Error::Parse),
            (Some(value), None) => match self {
                Self::Spf => SpfRecord::parse(value.as_bytes())
                    .map(|mut record| {
                        record.ra = None;
                        record
                    })
                    .into(),
                Self::Dmarc => DmarcRecord::parse(value.as_bytes()).into(),
            },
        }
    }
}

/// The crate's TXT cache over the declared map. `get` always answers, so the
/// crate never reaches its resolver; every answered name is recorded by `Lookups`.
struct Txts<'a, 'l> {
    lookups: &'a RefCell<&'l mut Lookups>,
    kind: Kind,
}

impl Txts<'_, '_> {
    fn answer(&self, name: &str) -> TxtRecord {
        // A name outside 0120's grammar is never looked up and never recorded.
        let Some(owner) = owner_of(name) else {
            return not_found();
        };
        match self.lookups.borrow_mut().resolve_txt(&owner) {
            Txt::Records(values) => self.kind.select(values),
            Txt::NoRecords => not_found(),
            Txt::TempFail => {
                TxtRecord::Error(Error::Dns(DnsError::Resolver("declared tempfail".into())))
            }
        }
    }
}

impl ResolverCache<Box<str>, TxtRecord> for Txts<'_, '_> {
    fn get<Q>(&self, name: &Q) -> Option<TxtRecord>
    where
        Box<str>: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Some(self.answer(&name_of(name).unwrap_or_default()))
    }

    fn remove<Q>(&self, _: &Q) -> Option<TxtRecord>
    where
        Box<str>: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        None
    }

    fn insert(&self, _: Box<str>, _: TxtRecord, _: Instant) {}
}

/// MX, AAAA and PTR (and A for SPF): a fixed negative for every query. For SPF
/// the answer also marks the evaluation unsupported (0126 §4); DMARC never asks.
struct Absent<'a> {
    mark: Option<&'a Cell<bool>>,
}

impl<K, T> ResolverCache<K, Result<RecordSet<T>, Negative>> for Absent<'_> {
    fn get<Q>(&self, _: &Q) -> Option<Result<RecordSet<T>, Negative>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if let Some(mark) = self.mark {
            mark.set(true);
        }
        Some(Err(Negative {
            code: ResponseCode::NXDomain,
        }))
    }

    fn remove<Q>(&self, _: &Q) -> Option<Result<RecordSet<T>, Negative>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        None
    }

    fn insert(&self, _: K, _: Result<RecordSet<T>, Negative>, _: Instant) {}
}

/// The A cache. DMARC asks one A question, whether the Author Domain exists,
/// which decides `np=` (RFC 9989 section 4.10, RFC 8020): a fixed rule over the
/// declared map answers it (0126 §4). SPF has no map to consult and a mark.
struct Exists<'a, 'l> {
    lookups: Option<&'a RefCell<&'l mut Lookups>>,
    mark: Option<&'a Cell<bool>>,
}

impl ResolverCache<Box<str>, Result<RecordSet<Ipv4Addr>, Negative>> for Exists<'_, '_> {
    fn get<Q>(&self, name: &Q) -> Option<Result<RecordSet<Ipv4Addr>, Negative>>
    where
        Box<str>: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        if let Some(mark) = self.mark {
            mark.set(true);
        }
        let exists = self.lookups.is_some_and(|lookups| {
            name_of(name)
                .as_deref()
                .and_then(owner_of)
                .is_some_and(|owner| lookups.borrow().exists(&owner))
        });
        Some(Err(Negative {
            // NODATA when the name exists with no address, NXDOMAIN when not.
            code: if exists {
                ResponseCode::NoError
            } else {
                ResponseCode::NXDomain
            },
        }))
    }

    fn remove<Q>(&self, _: &Q) -> Option<Result<RecordSet<Ipv4Addr>, Negative>>
    where
        Box<str>: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        None
    }

    fn insert(&self, _: Box<str>, _: Result<RecordSet<Ipv4Addr>, Negative>, _: Instant) {}
}

/// Every cache the crate may ask, all answering.
struct Caches<'a, 'l> {
    txt: Txts<'a, 'l>,
    exists: Exists<'a, 'l>,
    absent: Absent<'a>,
}

impl<'a, 'l> DnsCache for Caches<'a, 'l> {
    type Txt = Txts<'a, 'l>;
    type Mx = Absent<'a>;
    type Ipv4 = Exists<'a, 'l>;
    type Ipv6 = Absent<'a>;
    type Ptr = Absent<'a>;

    fn txt(&self) -> Option<&Self::Txt> {
        Some(&self.txt)
    }

    fn mx(&self) -> Option<&Self::Mx> {
        Some(&self.absent)
    }

    fn ipv4(&self) -> Option<&Self::Ipv4> {
        Some(&self.exists)
    }

    fn ipv6(&self) -> Option<&Self::Ipv6> {
        Some(&self.absent)
    }

    fn ptr(&self) -> Option<&Self::Ptr> {
        Some(&self.absent)
    }
}

/// What the SPF step did.
#[derive(Debug, PartialEq, Eq)]
enum Spf {
    NotEvaluated,
    Done {
        /// The identity's domain, as the crate checked it.
        domain: String,
        result: SpfResult,
        /// A term needed an address record the policy cannot hold.
        unsupported: bool,
    },
}

impl Spf {
    /// The rule that reports the outcome; a pass reports nothing.
    fn rule(&self) -> Option<Rule> {
        match self {
            Self::NotEvaluated => Some(Rule::SpfNotEvaluated),
            Self::Done {
                unsupported: true, ..
            } => Some(Rule::SpfUnsupported),
            Self::Done { result, .. } => match result {
                SpfResult::Pass => None,
                SpfResult::Fail => Some(Rule::SpfFail),
                SpfResult::SoftFail => Some(Rule::SpfSoftfail),
                SpfResult::Neutral => Some(Rule::SpfNeutral),
                SpfResult::None => Some(Rule::SpfNone),
                SpfResult::TempError => Some(Rule::SpfTemperror),
                SpfResult::PermError => Some(Rule::SpfPermerror),
            },
        }
    }

    /// `spf_pass`: a definite pass is True; a temporary failure, an unsupported
    /// term or no evaluation is Unknown (Swarmail did not look, so the sender
    /// did not fail); every other result is False.
    fn pass(&self) -> Fact {
        match self {
            Self::NotEvaluated
            | Self::Done {
                unsupported: true, ..
            }
            | Self::Done {
                result: SpfResult::TempError,
                ..
            } => Fact::Unknown,
            Self::Done {
                result: SpfResult::Pass,
                ..
            } => Fact::True,
            Self::Done { .. } => Fact::False,
        }
    }
}

/// One `check_host` for one identity (decision 0126 §5): MAIL FROM, or the HELO
/// name when MAIL FROM is the null reverse path (RFC 7208 section 2.4). The
/// address is the declared `spf_client_ip` when the policy declares one, else
/// the one the session observed; the declaration never makes a message without
/// an observed peer evaluable.
fn spf(envelope: &Envelope<'_>, shared: &RefCell<&mut Lookups>) -> Spf {
    let Some(observed) = envelope.client_ip else {
        return Spf::NotEvaluated;
    };
    let ip = shared.borrow().spf_client_ip().unwrap_or(observed);
    let parameters = match (envelope.mail_from, envelope.helo) {
        (None | Some(""), None) | (None, Some(_)) => return Spf::NotEvaluated,
        (Some(""), Some(helo)) => SpfParameters::helo(ip, helo, HOST),
        (Some(sender), helo) => SpfParameters::mail_from(ip, helo.unwrap_or(""), HOST, sender),
    };
    let unsupported = Cell::new(false);
    let caches = Caches {
        txt: Txts {
            lookups: shared,
            kind: Kind::Spf,
        },
        exists: Exists {
            lookups: None,
            mark: Some(&unsupported),
        },
        absent: Absent {
            mark: Some(&unsupported),
        },
    };
    let output = AUTHENTICATOR
        .check_host(Parameters::new(parameters).with_cache(&caches))
        .now_or_never()
        .expect("a declared map answers on the first poll");
    Spf::Done {
        domain: output.domain().to_string(),
        result: output.result(),
        unsupported: unsupported.get(),
    }
}

/// What the DMARC step found before the result is derived from the facts.
struct Dmarc {
    /// The finding that already settles it, when the message was not judged.
    rule: Option<Rule>,
    policy_present: Fact,
    valid_from: Fact,
    dkim_aligned: Fact,
    spf_aligned: Fact,
    /// The disposition the crate applied when a record applies.
    applied: Policy,
}

impl Dmarc {
    fn settled(rule: Rule, valid_from: Fact) -> Self {
        Self {
            rule: Some(rule),
            policy_present: Fact::Unknown,
            valid_from,
            dkim_aligned: Fact::Unknown,
            spf_aligned: Fact::Unknown,
            applied: Policy::None,
        }
    }
}

/// An alignment fact from the crate's per-mechanism result. `known` is false
/// when the mechanism could not be judged at all (SPF not evaluated; DKIM
/// stopped at a bound), which is Unknown whatever the crate reports for an
/// empty input.
fn aligned(result: &DmarcResult, known: bool) -> Fact {
    match result {
        _ if !known => Fact::Unknown,
        DmarcResult::Pass => Fact::True,
        DmarcResult::Fail(_) | DmarcResult::None => Fact::False,
        DmarcResult::TempError(_) | DmarcResult::PermError(_) => Fact::Unknown,
    }
}

/// DMARC for the one root From field (decision 0126 §7). From validity is
/// Swarmail's first: exactly one root field, which the crate parses where
/// Swarmail's own scan ends the headers, and whose addresses share one domain.
fn dmarc(
    fields: &Headers,
    size: usize,
    prepared: &Prepared<'_>,
    outputs: &[DkimOutput<'_>],
    dkim_known: bool,
    spf: &Spf,
    shared: &RefCell<&mut Lookups>,
) -> Dmarc {
    if values(fields, b"from").take(2).count() != 1 {
        return Dmarc::settled(Rule::DmarcFromInvalid, Fact::False);
    }
    let Some(message) = prepared
        .message()
        .filter(|message| message.body_offset() == size)
    else {
        return Dmarc::settled(Rule::DmarcNotEvaluated, Fact::Unknown);
    };
    // The identity's domain with a pass: the crate reports whether that identity
    // *aligns*, whatever SPF's own result was. SPF's result stays check_host's.
    let (mail_from_domain, spf_output) = match spf {
        Spf::Done { domain, .. } => (
            domain.as_str(),
            SpfOutput::new(domain.clone()).with_result(SpfResult::Pass),
        ),
        Spf::NotEvaluated => ("", SpfOutput::new(String::new())),
    };
    let caches = Caches {
        txt: Txts {
            lookups: shared,
            kind: Kind::Dmarc,
        },
        exists: Exists {
            lookups: Some(shared),
            mark: None,
        },
        absent: Absent { mark: None },
    };
    let output = AUTHENTICATOR
        .verify_dmarc(
            Parameters::new(DmarcParameters::new(
                message,
                outputs,
                mail_from_domain,
                &spf_output,
            ))
            .with_cache(&caches),
        )
        .now_or_never()
        .expect("a declared map answers on the first poll");
    // No domain, or several distinct domains in one field.
    if output.domain().is_empty() {
        return Dmarc::settled(Rule::DmarcFromInvalid, Fact::False);
    }
    if output.record().is_none() {
        // The walk's own error sets both mechanism results; otherwise nothing
        // applies. No alignment is defined without a record.
        let (rule, present) = match output.dkim_result() {
            DmarcResult::TempError(_) => (Rule::DmarcTemperror, Fact::Unknown),
            DmarcResult::PermError(_) => (Rule::DmarcPermerror, Fact::False),
            _ => (Rule::DmarcNone, Fact::False),
        };
        return Dmarc {
            policy_present: present,
            ..Dmarc::settled(rule, Fact::True)
        };
    }
    Dmarc {
        rule: None,
        policy_present: Fact::True,
        valid_from: Fact::True,
        dkim_aligned: aligned(output.dkim_result(), dkim_known),
        spf_aligned: aligned(output.spf_result(), matches!(spf, Spf::Done { .. })),
        applied: output.policy(),
    }
}

/// Evaluate DKIM (#254), SPF and DMARC for one stored message, push their
/// findings and return the six facts the `bulk.*` checklist reads (decision
/// 0126 §6). Every input is the stored bytes, the row's immutable envelope
/// observation, the receipt second and the frozen policy.
pub(super) fn collect(
    raw: &[u8],
    fields: &Headers,
    size: usize,
    receipt: u64,
    envelope: &Envelope<'_>,
    lookups: &mut Lookups,
    out: &mut Collector,
) -> Result<AuthEvidence, AnalysisError> {
    let prepared = dkim::prepare(raw, fields, size, out)?;
    // One borrow of the policy for the three evaluations, which run one after
    // the other: the DKIM outputs live on into DMARC, so their cache must too.
    let shared = RefCell::new(lookups);
    let adapter = dkim::Adapter { lookups: &shared };
    let (outputs, dkim_pass) = match &prepared {
        Prepared::Ready(message) => {
            let verified = dkim::verify(message, &adapter, receipt, out)?;
            (verified.outputs, verified.pass)
        }
        Prepared::Unsigned(_) => (Vec::new(), Fact::False),
        Prepared::Stopped | Prepared::Limited(_) => (Vec::new(), Fact::Unknown),
    };
    let dkim_known = !matches!(prepared, Prepared::Stopped | Prepared::Limited(_));
    let spf = spf(envelope, &shared);
    if let Some(rule) = spf.rule() {
        out.push(rule.finding(fields))?;
    }
    let found = dmarc(fields, size, &prepared, &outputs, dkim_known, &spf, &shared);
    let facts = AuthEvidence {
        policy_present: found.policy_present,
        valid_from: found.valid_from,
        dkim_pass,
        dkim_aligned: found.dkim_aligned,
        spf_pass: spf.pass(),
        spf_aligned: found.spf_aligned,
    };
    // The DMARC result is 0089 section 5's formula over the facts; Swarmail
    // computes no second verdict. Only a message whose record applies has one.
    let rule = found
        .rule
        .or_else(|| match (facts.dmarc_pass(), found.applied) {
            (Fact::True, _) => None,
            (Fact::False, Policy::Reject) => Some(Rule::DmarcFailReject),
            (Fact::False, Policy::Quarantine) => Some(Rule::DmarcFailQuarantine),
            (Fact::False, _) => Some(Rule::DmarcFailNone),
            (Fact::Unknown, _) => Some(Rule::DmarcNotEvaluated),
        });
    if let Some(rule) = rule {
        out.push(rule.finding(fields))?;
    }
    Ok(facts)
}
