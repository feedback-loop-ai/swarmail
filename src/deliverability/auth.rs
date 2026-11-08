//! SPF and DMARC evaluation (issue #255, decision 0126).
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
                "The connecting address is not authorized by the SPF record of the domain of the envelope sender, which ends in -all; add the address to the record or send from an authorized host (RFC 7208 section 2.6.4)."
            }
            Self::SpfSoftfail => {
                "The connecting address is probably not authorized by the SPF record of the domain of the envelope sender (~all); add the address to the record or send from an authorized host (RFC 7208 section 2.6.5)."
            }
            Self::SpfNeutral => {
                "No mechanism of the SPF record of the domain of the envelope sender matched the connecting address, or the record says ?all; add the address to the record (RFC 7208 section 2.6.2)."
            }
            Self::SpfNone => {
                "Declare a v=spf1 TXT record for the domain of the envelope sender in the DNS policy; none is declared, or the domain is not a valid name (RFC 7208 section 2.6.1)."
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
                "Declare exactly one v=DMARC1 record with a p= policy at the _dmarc name of the From domain or of its organizational domain in the DNS policy; none applies, and two records at one name apply none (RFC 9989 section 4.10)."
            }
            Self::DmarcTemperror => {
                "A DNS lookup the DMARC policy discovery needed failed temporarily, so the message was not evaluated; fix the DNS answer and retry (RFC 9989 section 4.10)."
            }
            Self::DmarcPermerror => {
                "Fix the DMARC policy record so that its tags are valid (RFC 9989 section 4.10)."
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
                "DMARC was not evaluated: the From header cannot be judged the way the Swarmail header scan reads it, or SPF or DKIM could not be evaluated and no aligned pass was found (RFC 9989 section 4.10)."
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
            weight: 0,
        }
    }
}

/// Whether every term of a `v=spf1` record is a mechanism of RFC 7208 section
/// 5 (with an optional qualifier) or a modifier `name=value` (section 6). The
/// crate skips a bare word it does not know, and swallows the term after it, so
/// `v=spf1 inlcude:x.example -all` would read as neutral instead of the
/// permanent error section 4.6 requires: that decision is Swarmail's. Only the
/// term names are judged here; the crate parses each term's arguments.
fn known_terms(record: &str) -> bool {
    const MECHANISMS: [&str; 8] = ["all", "include", "a", "mx", "ptr", "ip4", "ip6", "exists"];
    record
        .split(' ')
        .skip(1)
        .filter(|term| !term.is_empty())
        .all(|term| {
            let body = term.strip_prefix(['+', '-', '~', '?']);
            let qualified = body.is_some();
            let body = body.unwrap_or(term);
            match body.find([':', '/', '=']) {
                Some(end) if body.as_bytes()[end] == b'=' => {
                    let name = &body.as_bytes()[..end];
                    !qualified
                        && name.first().is_some_and(u8::is_ascii_alphabetic)
                        && name
                            .iter()
                            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(b))
                }
                end => {
                    let name = &body[..end.unwrap_or(body.len())];
                    MECHANISMS
                        .iter()
                        .any(|known| known.eq_ignore_ascii_case(name))
                }
            }
        })
}

/// The bytes after any leading spaces and tabs.
fn skip_blanks(bytes: &[u8]) -> &[u8] {
    let blanks = bytes
        .iter()
        .take_while(|b| matches!(b, b' ' | b'\t'))
        .count();
    &bytes[blanks..]
}

/// Which record type a name's values are selected for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Spf,
    Dmarc,
}

impl Kind {
    /// Whether a TXT value is a record of this type at all: it begins with the
    /// version tag and the tag ends there. SPF is `v=spf1`, case-insensitive,
    /// followed by a space or the end (RFC 7208 section 4.5), so `v=spf10` is not
    /// an SPF record. DMARC follows the ABNF of RFC 7489 section 6.4: `v`, any
    /// spaces or tabs, `=`, any spaces or tabs, then the case-exact `DMARC1`,
    /// followed by a space, a tab, `;` or the end, so `v=dmarc1` and `v=DMARC10`
    /// are not DMARC records and `v = DMARC1; p=reject` is.
    fn is_ours(self, value: &str) -> bool {
        let bytes = value.as_bytes();
        match self {
            Self::Spf => {
                let tag = b"v=spf1";
                bytes.len() >= tag.len()
                    && bytes[..tag.len()].eq_ignore_ascii_case(tag)
                    && bytes.get(tag.len()).is_none_or(|next| *next == b' ')
            }
            Self::Dmarc => {
                let rest = match bytes.split_first() {
                    Some((v, rest)) if v.eq_ignore_ascii_case(&b'v') => skip_blanks(rest),
                    _ => return false,
                };
                let Some((b'=', rest)) = rest.split_first() else {
                    return false;
                };
                skip_blanks(rest)
                    .strip_prefix(b"DMARC1")
                    .is_some_and(|after| {
                        after
                            .first()
                            .is_none_or(|next| matches!(next, b' ' | b'\t' | b';'))
                    })
            }
        }
    }

    /// The record of a name's declared values. Swarmail selects it, not the
    /// crate (whose default keeps the first value that parses): no value of the
    /// type is "no record". Two or more SPF values are a parse error (RFC 7208
    /// section 4.5). Two or more DMARC values are no record at that name, so the
    /// tree walk goes on (RFC 7489 section 6.6.3 applies no policy for multiple
    /// records; a single malformed record stays an error). SPF's `ra=` is cleared
    /// so the crate's report sampling, which reads the clock, is never reached.
    fn select(self, values: Vec<String>) -> TxtRecord {
        let mut ours = values.into_iter().filter(|value| self.is_ours(value));
        match (ours.next(), ours.next(), self) {
            (None, _, _) | (Some(_), Some(_), Self::Dmarc) => not_found(),
            (Some(_), Some(_), Self::Spf) => TxtRecord::Error(Error::Parse),
            (Some(value), None, Self::Spf) if !known_terms(&value) => {
                TxtRecord::Error(Error::Parse)
            }
            (Some(value), None, Self::Spf) => SpfRecord::parse(value.as_bytes())
                .map(|mut record| {
                    record.ra = None;
                    record
                })
                .into(),
            (Some(value), None, Self::Dmarc) => DmarcRecord::parse(value.as_bytes()).into(),
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

#[cfg(test)]
mod tests {
    use super::super::dkim::tests::{BODY, Kind as KeyKind, Sign, message, signed};
    use super::super::dns::{self, DnsRecord, Effective, Outcome, Source};
    use super::super::{Inputs, Report, evaluate_at, headers};
    use super::*;
    use mail_auth::Mx;
    use mail_auth::dns::UnwrapTxtRecord;
    use std::sync::Arc;

    const RECEIPT: u64 = 1_793_000_000;

    /// A frozen policy: `(name, values)` TXT records, `tempfail` names and the
    /// optional declared client address.
    fn world(
        records: &[(&str, &[&str])],
        tempfail: &[&str],
        client_ip: Option<&str>,
    ) -> Arc<Effective> {
        let txt: Vec<_> = records
            .iter()
            .map(|(name, values)| serde_json::json!({"name": name, "values": values}))
            .collect();
        let mut declaration = serde_json::json!({"txt": txt, "tempfail": tempfail});
        if let Some(ip) = client_ip {
            declaration["spf_client_ip"] = ip.into();
        }
        let policy = dns::parse(declaration.to_string().as_bytes()).unwrap();
        Effective::new(Source::Inbox, policy)
    }

    /// The envelope of a loopback SMTP session: the sender, 127.0.0.1 and a HELO.
    fn env(mail_from: &str) -> Envelope<'_> {
        Envelope {
            mail_from: Some(mail_from),
            client_ip: Some("127.0.0.1".parse().unwrap()),
            helo: Some("mail.example.com"),
        }
    }

    /// A message with the given From header value and a body.
    fn msg(from: &str) -> Vec<u8> {
        format!(
            "From: {from}\r\nTo: bob@example.org\r\nSubject: Hello\r\nDate: Mon, 02 Nov 2026 10:00:00 +0000\r\nMessage-ID: <1@example.com>\r\n\r\nHello Bob,\r\n"
        )
        .into_bytes()
    }

    /// What one evaluation produced: the facts, the findings of this module and
    /// the policy and lookups record.
    struct Run {
        facts: AuthEvidence,
        findings: Vec<Finding>,
        dns: DnsRecord,
    }

    impl Run {
        /// The SPF and DMARC rule ids, sorted.
        fn ids(&self) -> Vec<&str> {
            let mut ids: Vec<&str> = self
                .findings
                .iter()
                .map(|f| f.rule_id.as_str())
                .filter(|id| id.starts_with("spf.") || id.starts_with("dmarc."))
                .collect();
            ids.sort_unstable();
            ids
        }

        fn names(&self) -> Vec<&str> {
            self.dns.lookups.iter().map(|l| l.name.as_str()).collect()
        }
    }

    fn run(raw: &[u8], envelope: &Envelope<'_>, world: &Arc<Effective>) -> Run {
        let (fields, size) = headers(raw).unwrap();
        let mut lookups = Lookups::new(world.clone());
        let mut out = Collector(Vec::new());
        let facts = collect(
            raw,
            &fields,
            size,
            RECEIPT,
            envelope,
            &mut lookups,
            &mut out,
        )
        .unwrap();
        let mut findings = out.0;
        findings.sort();
        Run {
            facts,
            findings,
            dns: lookups.finish(),
        }
    }

    /// The whole report with the real evaluation (no fact override).
    fn report(raw: &[u8], envelope: &Envelope<'_>, world: &Arc<Effective>) -> Report {
        let inputs = Inputs {
            dns: world.clone(),
            auth: None,
            ..Inputs::default()
        };
        evaluate_at(raw, RECEIPT, envelope, &inputs).unwrap()
    }

    const OPEN: &str = "v=spf1 ip4:127.0.0.1 -all";
    const REJECT: &str = "v=DMARC1; p=reject";

    /// Run the example.com sender through one SPF record and return the SPF ids.
    fn spf_ids(record: &[&str]) -> (Vec<String>, Fact) {
        let w = world(&[("example.com", record)], &[], None);
        let r = run(&msg("a@example.com"), &env("a@example.com"), &w);
        let ids = r
            .ids()
            .into_iter()
            .filter(|id| id.starts_with("spf."))
            .map(str::to_string)
            .collect();
        (ids, r.facts.spf_pass)
    }

    use Fact::{False as F, True as T, Unknown as U};

    /// AC6: a passing and a failing fixture for every SPF result; a pass
    /// reports nothing, and the fact follows the result.
    #[test]
    fn spf_results_map_one_to_one() {
        for (record, ids, pass) in [
            (OPEN, vec![], T),
            ("v=spf1 ip4:127.0.0.0/8 -all", vec![], T),
            ("v=spf1 ip4:192.0.2.1 -all", vec!["spf.fail.v1"], F),
            ("v=spf1 ip4:192.0.2.1 ~all", vec!["spf.softfail.v1"], F),
            ("v=spf1 ip4:192.0.2.1 ?all", vec!["spf.neutral.v1"], F),
            // No term matches and there is no `all`: neutral (RFC 7208 4.7).
            ("v=spf1 ip4:192.0.2.1", vec!["spf.neutral.v1"], F),
            ("v=spf1", vec!["spf.neutral.v1"], F),
            // A syntax error is a permanent error.
            (
                "v=spf1 ip4:not-an-address -all",
                vec!["spf.permerror.v1"],
                F,
            ),
            ("v=spf1 bogus -all", vec!["spf.permerror.v1"], F),
        ] {
            let (found, fact) = spf_ids(&[record]);
            assert_eq!(found, ids, "{record}");
            assert_eq!(fact, pass, "{record}");
        }
    }

    #[test]
    fn spf_none_and_temperror() {
        // No record at the identity's domain, and a domain that is no name.
        let w = world(&[], &[], None);
        let r = run(&msg("a@example.com"), &env("a@example.com"), &w);
        let spf = |r: &Run| -> Vec<String> {
            r.ids()
                .into_iter()
                .filter(|id| id.starts_with("spf."))
                .map(str::to_string)
                .collect()
        };
        assert_eq!(spf(&r), ["spf.none.v1"]);
        assert_eq!(r.facts.spf_pass, F);
        let r = run(&msg("a@example.com"), &env("a@bad..domain"), &w);
        assert!(r.ids().contains(&"spf.none.v1"));
        assert!(
            r.names().iter().all(|n| n.starts_with("_dmarc.")),
            "{:?}",
            r.names()
        );
        // A declared temporary failure is a temporary error and not a failure.
        let w = world(&[], &["example.com"], None);
        let r = run(&msg("a@example.com"), &env("a@example.com"), &w);
        assert!(r.ids().contains(&"spf.temperror.v1"));
        assert_eq!(r.facts.spf_pass, U);
        assert!(r.dns.lookups.iter().any(|l| l.outcome == Outcome::TempFail));
    }

    /// AC7: the record is Swarmail's pick. Two `v=spf1` values are a permanent
    /// error, one beside unrelated values is evaluated, and `v=spf10` is not an
    /// SPF record. The crate's own first-parse-wins default never decides.
    #[test]
    fn spf_record_selection() {
        for (values, ids, pass) in [
            (vec![OPEN, "v=spf1 -all"], vec!["spf.permerror.v1"], F),
            (
                vec![OPEN, "google-site-verification=abc", "hello"],
                vec![],
                T,
            ),
            (vec!["v=spf10 ip4:127.0.0.1 -all"], vec!["spf.none.v1"], F),
            (vec!["v=spf1x ip4:127.0.0.1 -all"], vec!["spf.none.v1"], F),
            (vec!["V=SPF1 IP4:127.0.0.1 -ALL"], vec![], T),
            (vec!["ip4:127.0.0.1 v=spf1"], vec!["spf.none.v1"], F),
            (vec![OPEN, "v=spf10 -all"], vec![], T),
        ] {
            let (found, fact) = spf_ids(&values);
            assert_eq!(found, ids, "{values:?}");
            assert_eq!(fact, pass, "{values:?}");
        }
    }

    /// Every term of a record is a mechanism or a modifier: the crate would
    /// skip an unknown bare word and swallow the term after it.
    #[test]
    fn spf_unknown_terms_are_permanent_errors() {
        for (record, known) in [
            ("v=spf1", true),
            ("v=spf1 all", true),
            ("v=spf1  +ip4:127.0.0.1   ~ALL", true),
            (
                "v=spf1 a a:x.example a/24 mx mx:x.example/24//64 ptr ptr:x.example",
                true,
            ),
            (
                "v=spf1 include:x.example exists:%{i}.x.example ip6:::1 -ip4:1.2.3.4/24",
                true,
            ),
            ("v=spf1 redirect=x.example exp=why.x.example", true),
            (
                "v=spf1 ra=postmaster rp=10 rr=e:f custom-mod_1.x=value",
                true,
            ),
            ("v=spf1 inlcude:x.example -all", false),
            ("v=spf1 bogus -all", false),
            // A known name with bad arguments is the crate's to refuse.
            ("v=spf1 ip4 -all", true),
            ("v=spf1 :x -all", false),
            ("v=spf1 -redirect=x.example", false),
            ("v=spf1 =x", false),
            ("v=spf1 1bad=value", false),
            ("v=spf1 ba!d=value", false),
        ] {
            assert_eq!(known_terms(record), known, "{record}");
        }
        for record in ["v=spf1 inlcude:x.example -all", "v=spf1 ip4 -all"] {
            let (found, fact) = spf_ids(&[record]);
            assert_eq!((found, fact), (vec!["spf.permerror.v1".to_string()], F));
        }
    }

    /// One identity per message (decision 0126 §5): MAIL FROM, or the HELO name
    /// for the null reverse path; the HELO identity's record decides then.
    #[test]
    fn spf_identity_is_mail_from_or_helo_for_the_null_sender() {
        let w = world(
            &[
                ("example.com", &["v=spf1 ip4:192.0.2.1 -all"]),
                ("helo.example.net", &[OPEN]),
            ],
            &[],
            None,
        );
        let helo = Some("helo.example.net");
        let at = |mail_from: Option<&str>, client_ip: Option<&str>, helo: Option<&str>| {
            let envelope = Envelope {
                mail_from,
                client_ip: client_ip.map(|ip| ip.parse().unwrap()),
                helo,
            };
            let r = run(&msg("a@example.com"), &envelope, &w);
            (
                r.ids()
                    .into_iter()
                    .filter(|i| i.starts_with("spf."))
                    .map(str::to_string)
                    .collect::<Vec<_>>(),
                r.facts.spf_pass,
            )
        };
        let fail = (vec!["spf.fail.v1".to_string()], F);
        let pass = (Vec::<String>::new(), T);
        let none = (vec!["spf.not_evaluated.v1".to_string()], U);
        // A sender: its domain's record, whatever the HELO name says.
        assert_eq!(at(Some("a@example.com"), Some("127.0.0.1"), helo), fail);
        assert_eq!(at(Some("a@example.com"), Some("127.0.0.1"), None), fail);
        // The null sender: the HELO name's record.
        assert_eq!(at(Some(""), Some("127.0.0.1"), helo), pass);
        assert_eq!(at(Some(""), Some("192.0.2.1"), helo), fail);
        // Not evaluated: no peer, no known sender, a null sender with no HELO.
        assert_eq!(at(Some("a@example.com"), None, helo), none);
        assert_eq!(at(None, Some("127.0.0.1"), helo), none);
        assert_eq!(at(None, Some("127.0.0.1"), None), none);
        assert_eq!(at(Some(""), Some("127.0.0.1"), None), none);
        assert_eq!(at(None, None, None), none);
        // A sender with no domain part uses the HELO name as its domain.
        assert_eq!(at(Some("postmaster"), Some("127.0.0.1"), helo), pass);
        // The information finding is at the envelope, never a header, and the
        // evaluation made no lookup for the SPF side.
        let r = run(&msg("a@example.com"), &Envelope::default(), &w);
        let finding = r
            .findings
            .iter()
            .find(|f| f.rule_id == "spf.not_evaluated.v1");
        assert_eq!(
            serde_json::to_value(finding.unwrap()).unwrap()["location"],
            serde_json::json!({"kind": "header", "name": "return-path", "occurrence": null})
        );
        assert_eq!(finding.unwrap().severity, Severity::Info);
        assert!(r.names().iter().all(|n| n.starts_with("_dmarc.")));
    }

    /// AC5: the declared address replaces the observed one for SPF, for every
    /// peer, and never makes a message with no observed peer evaluable.
    #[test]
    fn spf_client_ip_overrides_the_observed_address() {
        let only_declared = world(
            &[("example.com", &["v=spf1 ip4:192.0.2.9 -all"])],
            &[],
            Some("192.0.2.9"),
        );
        let only_loopback = world(&[("example.com", &[OPEN])], &[], Some("192.0.2.9"));
        let v6 = world(
            &[("example.com", &["v=spf1 ip6:2001:db8::25 -all"])],
            &[],
            Some("2001:DB8::25"),
        );
        let mapped = world(
            &[("example.com", &["v=spf1 ip4:192.0.2.9 -all"])],
            &[],
            Some("::ffff:192.0.2.9"),
        );
        let id = |w: &Arc<Effective>, e: &Envelope<'_>| {
            run(&msg("a@example.com"), e, w)
                .ids()
                .into_iter()
                .filter(|i| i.starts_with("spf."))
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        let loopback = env("a@example.com");
        assert!(id(&only_declared, &loopback).is_empty());
        assert_eq!(id(&only_loopback, &loopback), ["spf.fail.v1"]);
        assert!(id(&v6, &loopback).is_empty());
        assert!(id(&mapped, &loopback).is_empty());
        // Every peer, loopback or not.
        let remote = Envelope {
            client_ip: Some("203.0.113.5".parse().unwrap()),
            ..loopback
        };
        assert!(id(&only_declared, &remote).is_empty());
        // An observed peer is required: the declaration cannot create one.
        let unobserved = Envelope {
            client_ip: None,
            ..loopback
        };
        assert_eq!(id(&only_declared, &unobserved), ["spf.not_evaluated.v1"]);
    }

    /// The SPF ids and `spf_pass` for the sender, one record per owner.
    fn spf_over(records: &[(String, String)], peer: &str) -> (Run, Vec<String>) {
        let declared: Vec<(&str, Vec<&str>)> = records
            .iter()
            .map(|(name, value)| (name.as_str(), vec![value.as_str()]))
            .collect();
        let refs: Vec<(&str, &[&str])> = declared.iter().map(|(n, v)| (*n, v.as_slice())).collect();
        let w = world(&refs, &[], None);
        let envelope = Envelope {
            client_ip: Some(peer.parse().unwrap()),
            ..env("a@example.com")
        };
        let r = run(&msg("a@example.com"), &envelope, &w);
        let ids = r
            .ids()
            .into_iter()
            .filter(|i| i.starts_with("spf."))
            .map(str::to_string)
            .collect();
        (r, ids)
    }

    fn one(record: &str) -> Vec<(String, String)> {
        vec![("example.com".into(), record.into())]
    }

    /// AC8: a term that needs an A, AAAA, MX or PTR answer is neither a pass nor
    /// a failure: the verdict is Swarmail's own, and `spf_pass` is Unknown.
    #[test]
    fn unsupported_mechanisms_are_unknown_not_failures() {
        for record in [
            "v=spf1 a -all",
            "v=spf1 a:other.example.com -all",
            "v=spf1 a/24 -all",
            "v=spf1 mx -all",
            "v=spf1 mx:other.example.com/24 -all",
            "v=spf1 ptr -all",
            "v=spf1 exists:x.example.com -all",
            "v=spf1 include:%{p}.example.net -all",
            "v=spf1 a a a -all",
            "v=spf1 ip4:192.0.2.1 a -all",
            // Reached before a term that would have matched.
            "v=spf1 a ip4:127.0.0.1 -all",
            "v=spf1 a -all exp=why.example.com",
        ] {
            for peer in ["127.0.0.1", "::1"] {
                let (r, ids) = spf_over(&one(record), peer);
                assert_eq!(ids, ["spf.unsupported_mechanism.v1"], "{record} {peer}");
                assert_eq!(r.facts.spf_pass, U, "{record} {peer}");
            }
        }
        // Through an include and a redirect, too.
        let inner = vec![
            (
                "example.com".to_string(),
                "v=spf1 include:inner.example.com -all".to_string(),
            ),
            (
                "inner.example.com".to_string(),
                "v=spf1 mx -all".to_string(),
            ),
        ];
        let (r, ids) = spf_over(&inner, "127.0.0.1");
        assert_eq!(
            (ids, r.facts.spf_pass),
            (vec!["spf.unsupported_mechanism.v1".into()], U)
        );
        let redirected = vec![
            (
                "example.com".to_string(),
                "v=spf1 redirect=other.example.com".to_string(),
            ),
            (
                "other.example.com".to_string(),
                "v=spf1 exists:x.example.com".to_string(),
            ),
        ];
        let (r, ids) = spf_over(&redirected, "127.0.0.1");
        assert_eq!(
            (ids, r.facts.spf_pass),
            (vec!["spf.unsupported_mechanism.v1".into()], U)
        );
        // A term after the first match is never evaluated (RFC 7208 4.6.2): the
        // match decides, and the unsupported term is not reached.
        for record in [
            "v=spf1 ip4:127.0.0.1 a -all",
            "v=spf1 +all a",
            "v=spf1 ip4:127.0.0.1 mx ptr exists:x.example.com",
        ] {
            let (r, ids) = spf_over(&one(record), "127.0.0.1");
            assert!(ids.is_empty(), "{record}: {ids:?}");
            assert_eq!(r.facts.spf_pass, T, "{record}");
        }
        // The not-evaluated checklist entry, not a failure, comes of it.
        let w = world(
            &[
                ("example.com", &["v=spf1 a -all"]),
                ("_dmarc.example.com", &[REJECT]),
            ],
            &[],
            None,
        );
        let findings: Vec<String> = report(&msg("a@example.com"), &env("a@example.com"), &w)
            .findings
            .into_iter()
            .map(|f| f.rule_id)
            .filter(|id| id.starts_with("bulk.") && id.contains("sender"))
            .collect();
        assert_eq!(findings, ["bulk.authenticated_sender.not_evaluated.v1"]);
    }

    /// The names an SPF evaluation looked up (the DMARC walk's are not SPF's).
    fn spf_names(r: &Run) -> Vec<&str> {
        r.names()
            .into_iter()
            .filter(|n| !n.starts_with("_dmarc."))
            .collect()
    }

    /// `n` includes that never match, then `tail`.
    fn chain(n: usize, tail: &str, extra: &[(&str, &str)]) -> Vec<(String, String)> {
        let includes: String = (1..=n)
            .map(|k| format!(" include:i{k}.example.com"))
            .collect();
        let mut records = vec![(
            "example.com".to_string(),
            format!("v=spf1{includes} {tail}"),
        )];
        records.extend((1..=n).map(|k| (format!("i{k}.example.com"), "v=spf1 -all".to_string())));
        records.extend(extra.iter().map(|(n, v)| (n.to_string(), v.to_string())));
        records
    }

    /// AC9: ten DNS-querying terms are admitted and the eleventh is a permanent
    /// error; an `include:` or `redirect=` target that is not declared is a
    /// permanent error at its first void answer; at most twelve names are looked
    /// up by one evaluation.
    #[test]
    fn spf_lookup_limits() {
        let verdict = |records: &[(String, String)]| {
            let (r, ids) = spf_over(records, "127.0.0.1");
            let names = spf_names(&r).len();
            (ids, r.facts.spf_pass, names)
        };
        // Ten includes, then a match: the record plus ten targets are looked up.
        assert_eq!(
            verdict(&chain(10, "ip4:127.0.0.1 -all", &[])),
            (vec![], T, 11)
        );
        // The eleventh refuses before it looks anything up.
        assert_eq!(
            verdict(&chain(11, "ip4:127.0.0.1 -all", &[])),
            (vec!["spf.permerror.v1".to_string()], F, 11)
        );
        // A redirect is a DNS-querying term too: the tenth is admitted, the
        // eleventh is not.
        let target = [("r.example.com", OPEN)];
        assert_eq!(
            verdict(&chain(9, "redirect=r.example.com", &target)),
            (vec![], T, 11)
        );
        assert_eq!(
            verdict(&chain(10, "redirect=r.example.com", &target)),
            (vec!["spf.permerror.v1".to_string()], F, 11)
        );
        // A target that is not declared is void: a permanent error at the first.
        let void = vec![(
            "example.com".to_string(),
            "v=spf1 include:gone.example.com -all".to_string(),
        )];
        let (r, ids) = spf_over(&void, "127.0.0.1");
        assert_eq!(ids, ["spf.permerror.v1"]);
        assert_eq!(spf_names(&r), ["example.com", "gone.example.com"]);
        let void = vec![(
            "example.com".to_string(),
            "v=spf1 redirect=gone.example.com".to_string(),
        )];
        assert_eq!(spf_over(&void, "127.0.0.1").1, ["spf.permerror.v1"]);
        // A declared temporary failure on a target is a temporary error.
        let w = world(
            &[("example.com", &["v=spf1 include:down.example.com -all"])],
            &["down.example.com"],
            None,
        );
        let r = run(&msg("a@example.com"), &env("a@example.com"), &w);
        assert!(r.ids().contains(&"spf.temperror.v1"));
        assert_eq!(r.facts.spf_pass, U);
        // Twelve names at most: the record, ten terms and the explanation.
        let explained = chain(
            10,
            "-all exp=why.example.com",
            &[("why.example.com", "Because %{t} %{d}")],
        );
        let (r, ids) = spf_over(&explained, "127.0.0.1");
        assert_eq!(ids, ["spf.fail.v1"]);
        assert_eq!(spf_names(&r).len(), 12);
        assert!(spf_names(&r).contains(&"why.example.com"));
        // A void-lookup flood is vacuous: every term that can be void and is not
        // an include or redirect is unsupported before a void answer decides.
        let voids = one("v=spf1 a:v1.example.com a:v2.example.com a:v3.example.com -all");
        assert_eq!(
            spf_over(&voids, "127.0.0.1").1,
            ["spf.unsupported_mechanism.v1"]
        );
    }

    /// The SPF-and-DMARC world for example.com: its sender is the loopback,
    /// and its DMARC record is `dmarc`.
    fn example(dmarc: &str) -> Arc<Effective> {
        world(
            &[("example.com", &[OPEN]), ("_dmarc.example.com", &[dmarc])],
            &[],
            None,
        )
    }

    /// The `dmarc.*` ids of a run.
    fn dmarc_ids(r: &Run) -> Vec<&str> {
        r.ids()
            .into_iter()
            .filter(|i| i.starts_with("dmarc."))
            .collect()
    }

    fn location_of(r: &Run, id: &str) -> serde_json::Value {
        let finding = r.findings.iter().find(|f| f.rule_id == id).unwrap();
        serde_json::to_value(&finding.location).unwrap()
    }

    /// AC12: From validity is Swarmail's own, from the root header list, and
    /// `Email.from` (which falls back to the envelope) is never read.
    #[test]
    fn from_validity() {
        let w = example(REJECT);
        let sender = env("a@example.com");
        // Valid: one field, one domain (two addresses of it are still one).
        for from in [
            "a@example.com",
            "Alice <a@example.com>",
            "a@example.com, b@example.com",
        ] {
            let r = run(&msg(from), &sender, &w);
            assert!(dmarc_ids(&r).is_empty(), "{from}");
            assert_eq!(r.facts.valid_from, T, "{from}");
        }
        let header = |location: serde_json::Value| serde_json::json!({"kind": "header", "name": "from", "occurrence": location});
        // No From field: an envelope sender whose domain would pass DMARC
        // changes nothing.
        let none = b"To: bob@example.org\r\nSubject: Hello\r\n\r\nHello\r\n";
        let r = run(none, &sender, &w);
        assert_eq!(dmarc_ids(&r), ["dmarc.from_invalid.v1"]);
        assert_eq!(
            location_of(&r, "dmarc.from_invalid.v1"),
            header(serde_json::Value::Null)
        );
        assert_eq!((r.facts.valid_from, r.facts.policy_present), (F, U));
        assert_eq!((r.facts.dkim_aligned, r.facts.spf_aligned), (U, U));
        assert!(
            r.names().iter().all(|n| !n.starts_with("_dmarc.")),
            "no walk without a From"
        );
        // Two From fields: the finding is at the second (occurrence 1).
        let two = b"From: a@example.com\r\nFrom: b@example.com\r\nSubject: Hello\r\n\r\nHello\r\n";
        let r = run(two, &sender, &w);
        assert_eq!(dmarc_ids(&r), ["dmarc.from_invalid.v1"]);
        assert_eq!(location_of(&r, "dmarc.from_invalid.v1"), header(1.into()));
        assert_eq!(r.facts.valid_from, F);
        // Two distinct domains in one field: no single Author Domain.
        let r = run(&msg("a@example.com, b@example.org"), &sender, &w);
        assert_eq!(dmarc_ids(&r), ["dmarc.from_invalid.v1"]);
        assert_eq!(location_of(&r, "dmarc.from_invalid.v1"), header(0.into()));
        assert_eq!((r.facts.valid_from, r.facts.policy_present), (F, U));
        // A From with no domain part at all.
        let r = run(&msg("nobody"), &sender, &w);
        assert_eq!(dmarc_ids(&r), ["dmarc.from_invalid.v1"]);
    }

    /// AC12: a From that the crate reads where Swarmail's own scan ends the
    /// headers differently cannot be judged.
    #[test]
    fn from_the_crate_reads_differently_is_not_evaluated() {
        let w = example(REJECT);
        let sender = env("a@example.com");
        let stray = b"From: a@example.com\r\nstray line\r\nTo: bob@example.org\r\n\r\nHello\r\n";
        let r = run(stray, &sender, &w);
        assert_eq!(dmarc_ids(&r), ["dmarc.not_evaluated.v1"]);
        assert_eq!(location_of(&r, "dmarc.not_evaluated.v1")["occurrence"], 0);
        assert_eq!(r.facts.valid_from, U);
        assert_eq!(r.facts.policy_present, U);
        assert!(r.names().iter().all(|n| !n.starts_with("_dmarc.")));
        // The same message without the stray line is judged.
        let clean = run(&msg("a@example.com"), &sender, &w);
        assert!(dmarc_ids(&clean).is_empty());
    }

    /// A world where example.com's SPF record fails the loopback, so only the
    /// DMARC record decides what a message gets.
    fn failing(dmarc: &[&str]) -> Arc<Effective> {
        world(
            &[
                ("example.com", &["v=spf1 ip4:192.0.2.1 -all"]),
                ("_dmarc.example.com", dmarc),
            ],
            &[],
            None,
        )
    }

    fn mail(w: &Arc<Effective>) -> Run {
        run(&msg("a@example.com"), &env("a@example.com"), w)
    }

    /// AC10: discovery. A passing and a failing fixture for none, temperror and
    /// permerror, each with the fact it sets.
    #[test]
    fn dmarc_discovery_results() {
        // Applies and passes: nothing is reported.
        let r = mail(&example(REJECT));
        assert!(dmarc_ids(&r).is_empty());
        assert_eq!(r.facts.policy_present, T);
        // None: no record, a value that is not a DMARC record, or a record
        // with neither p= nor rua= (it does not apply).
        for values in [
            None,
            Some("v=DMARC10; p=reject"),
            Some("v=DMARC1; fo=1"),
            Some("hello"),
        ] {
            let w = match values {
                None => world(&[("example.com", &[OPEN])], &[], None),
                Some(v) => example(v),
            };
            let r = mail(&w);
            assert_eq!(dmarc_ids(&r), ["dmarc.none.v1"], "{values:?}");
            assert_eq!(r.facts.policy_present, F, "{values:?}");
            assert_eq!((r.facts.dkim_aligned, r.facts.spf_aligned), (U, U));
            assert_eq!(r.facts.valid_from, T);
        }
        // Temperror: a walked name is declared tempfail, the Author Domain's own
        // or a parent's, even when a record was found below it. It never
        // qualifies, at any threshold.
        for (records, down) in [
            (vec![("example.com", &[OPEN][..])], "_dmarc.example.com"),
            (
                vec![
                    ("example.com", &[OPEN][..]),
                    ("_dmarc.example.com", &[REJECT][..]),
                ],
                "_dmarc.com",
            ),
        ] {
            let w = world(&records, &[down], None);
            let r = mail(&w);
            assert_eq!(dmarc_ids(&r), ["dmarc.temperror.v1"], "{down}");
            assert_eq!(r.facts.policy_present, U, "{down}");
            assert!(r.dns.lookups.iter().any(|l| l.outcome == Outcome::TempFail));
            let whole = report(&msg("a@example.com"), &env("a@example.com"), &w);
            assert!(!whole.qualifies(Severity::Error));
        }
        // Permerror: one record that does not parse. (Two records at a name are
        // no record at it: `two_dmarc_records_at_a_name_are_no_record`.)
        for values in [
            vec!["v=DMARC1; p=bogus"],
            vec!["v=DMARC1; p=reject; adkim=x"],
        ] {
            let w = world(
                &[("example.com", &[OPEN]), ("_dmarc.example.com", &values)],
                &[],
                None,
            );
            let r = mail(&w);
            assert_eq!(dmarc_ids(&r), ["dmarc.permerror.v1"], "{values:?}");
            assert_eq!(
                (r.facts.policy_present, r.facts.valid_from),
                (F, T),
                "{values:?}"
            );
        }
        // One DMARC value beside unrelated ones is the record.
        let w = example(REJECT);
        let w2 = world(
            &[
                ("example.com", &[OPEN]),
                ("_dmarc.example.com", &[REJECT, "unrelated", "v=DMARC10"]),
            ],
            &[],
            None,
        );
        assert_eq!(mail(&w).facts, mail(&w2).facts);
        // A single-label Author Domain has no tree to walk.
        let r = run(&msg("a@localhost"), &env("a@localhost"), &w);
        assert_eq!(dmarc_ids(&r), ["dmarc.none.v1"]);
    }

    /// AC21: the version tag follows the ABNF of RFC 7489 section 6.4.
    #[test]
    fn dmarc_version_tag_grammar() {
        let applies = |value: &str| {
            let r = mail(&failing(&[value]));
            dmarc_ids(&r) == ["dmarc.fail.reject.v1"]
        };
        // Spaces around `=` and before `;` still make the record.
        for value in [
            "v=DMARC1; p=reject",
            "V=DMARC1; p=reject",
            "v = DMARC1; p=reject",
            "v=DMARC1 ;p=reject",
        ] {
            assert!(applies(value), "{value:?}");
        }
        // The tag is case-exact, ends there, and starts the value.
        for value in [
            "v=dmarc1; p=reject",
            "v=DMARC10; p=reject",
            "vv=DMARC1; p=reject",
            "xv=DMARC1; p=reject",
            "v DMARC1; p=reject",
            "v=; p=reject",
            "v=DMARC",
            "v",
        ] {
            assert!(!applies(value), "{value:?}");
            let r = mail(&failing(&[value]));
            assert_eq!(dmarc_ids(&r), ["dmarc.none.v1"], "{value:?}");
            assert_eq!(r.facts.policy_present, F, "{value:?}");
        }
        // A declared value is printable ASCII, so tabs and the empty value are
        // judged on the selector itself.
        for value in [
            "v\t=\tDMARC1; p=reject",
            "v=DMARC1\t;p=reject",
            "v=DMARC1",
            "v=DMARC1;",
        ] {
            assert!(Kind::Dmarc.is_ours(value), "{value:?}");
        }
        for value in ["", "\tv=DMARC1", "v=DMARC1x", "v=dmarc1"] {
            assert!(!Kind::Dmarc.is_ours(value), "{value:?}");
        }
        // SPF is unchanged: case-insensitive, then a space or the end.
        assert!(Kind::Spf.is_ours("V=SPF1 -all"));
        assert!(Kind::Spf.is_ours("v=spf1"));
        assert!(!Kind::Spf.is_ours("v=spf10 -all"));
        assert!(!Kind::Spf.is_ours("v=spf1;"));
        assert!(!Kind::Spf.is_ours("v=spf"));
    }

    /// AC10, AC21: two DMARC records at one name apply no policy there (RFC 7489
    /// section 6.6.3), so the walk goes on; a single malformed record is an
    /// error, and two SPF records stay a permerror.
    #[test]
    fn two_dmarc_records_at_a_name_are_no_record() {
        let two = ["v=DMARC1; p=reject", "v=DMARC1; p=none"];
        // Alone: no record, nothing applies.
        let r = mail(&failing(&two));
        assert_eq!(dmarc_ids(&r), ["dmarc.none.v1"]);
        assert_eq!(r.facts.policy_present, F);
        // Beneath a parent that has one record: the parent applies, sp= included.
        let w = world(
            &[
                ("example.com", &["v=spf1 ip4:192.0.2.1 -all"]),
                ("_dmarc.mail.example.com", &two),
                ("_dmarc.example.com", &["v=DMARC1; p=reject; sp=quarantine"]),
            ],
            &[],
            None,
        );
        let r = run(&msg("a@mail.example.com"), &env("a@mail.example.com"), &w);
        assert_eq!(dmarc_ids(&r), ["dmarc.fail.quarantine.v1"]);
        assert_eq!(r.facts.policy_present, T);
        // Both names, with their values, are in the lookups.
        assert_eq!(
            r.names(),
            [
                "_dmarc.com",
                "_dmarc.example.com",
                "_dmarc.mail.example.com",
                "mail.example.com"
            ]
        );
        let values = |name: &str| {
            r.dns
                .lookups
                .iter()
                .find(|l| l.name == name)
                .map(|l| l.values.clone())
        };
        assert_eq!(
            values("_dmarc.mail.example.com").unwrap(),
            ["v=DMARC1; p=none", "v=DMARC1; p=reject"].map(String::from)
        );
        // A malformed single record stays a permerror.
        let r = mail(&failing(&["v=DMARC1; p=bogus"]));
        assert_eq!(dmarc_ids(&r), ["dmarc.permerror.v1"]);
        // Two SPF records stay a permerror.
        let w = world(
            &[("example.com", &["v=spf1 -all", "v=spf1 +all"])],
            &[],
            None,
        );
        let r = mail(&w);
        assert!(r.ids().contains(&"spf.permerror.v1"));
    }

    /// AC23: the null sender's DMARC baseline is the HELO identity (0126 §5).
    #[test]
    fn null_sender_aligns_through_the_helo_identity() {
        let w = world(
            &[
                ("mx.example.com", &[OPEN]),
                ("mx.example.net", &[OPEN]),
                ("_dmarc.example.com", &[REJECT]),
            ],
            &[],
            None,
        );
        let at = |helo: &'static str| {
            let envelope = Envelope {
                mail_from: Some(""),
                helo: Some(helo),
                ..env("")
            };
            run(&msg("a@example.com"), &envelope, &w)
        };
        let aligned = at("mx.example.com");
        assert_eq!(aligned.facts.spf_pass, T);
        assert_eq!(aligned.facts.spf_aligned, T);
        assert!(dmarc_ids(&aligned).is_empty());
        let stranger = at("mx.example.net");
        assert_eq!(stranger.facts.spf_pass, T);
        assert_eq!(stranger.facts.spf_aligned, F);
        assert_eq!(dmarc_ids(&stranger), ["dmarc.fail.reject.v1"]);
    }

    /// AC10: the applied disposition rides the id. `pct=` is not a tag of
    /// RFC 9989 and changes nothing; the record's rua= alone applies as p=none.
    #[test]
    fn dmarc_failures_carry_the_applied_disposition() {
        for (record, id) in [
            ("v=DMARC1; p=none", "dmarc.fail.none.v1"),
            ("v=DMARC1; p=quarantine", "dmarc.fail.quarantine.v1"),
            ("v=DMARC1; p=reject", "dmarc.fail.reject.v1"),
            ("v=DMARC1; p=reject; pct=50", "dmarc.fail.reject.v1"),
            ("v=DMARC1; p=reject; pct=0", "dmarc.fail.reject.v1"),
            ("v=DMARC1; rua=mailto:r@example.com", "dmarc.fail.none.v1"),
            // t=y steps the policy down once (RFC 9989).
            ("v=DMARC1; p=reject; t=y", "dmarc.fail.quarantine.v1"),
            ("v=DMARC1; p=quarantine; t=y", "dmarc.fail.none.v1"),
            ("v=DMARC1; p=none; t=y", "dmarc.fail.none.v1"),
        ] {
            let r = mail(&failing(&[record]));
            assert_eq!(dmarc_ids(&r), [id], "{record}");
            assert_eq!(r.facts.policy_present, T);
            assert_eq!(
                (r.facts.spf_pass, r.facts.spf_aligned),
                (F, T),
                "aligned, not authenticated"
            );
            assert_eq!((r.facts.dkim_pass, r.facts.dkim_aligned), (F, F));
            // Deterministic: a sampling tag would differ between runs.
            assert_eq!(dmarc_ids(&mail(&failing(&[record]))), [id]);
        }
        // The same records pass when SPF does.
        for record in [
            "v=DMARC1; p=reject; pct=50",
            "v=DMARC1; p=none",
            "v=DMARC1; rua=mailto:r@example.com",
        ] {
            assert!(dmarc_ids(&mail(&example(record))).is_empty(), "{record}");
        }
    }

    /// AC10: p= for the Author Domain's own record, sp= for a record found
    /// above it, np= for an Author Domain that does not exist, which is a fixed
    /// rule over the declared names (RFC 8020).
    #[test]
    fn dmarc_policy_scope_p_sp_np() {
        let sub = |records: &[(&str, &[&str])]| {
            let mut all = vec![("mail.example.com", &["v=spf1 -all"][..])];
            all.extend_from_slice(records);
            let r = run(
                &msg("a@mail.example.com"),
                &env("a@mail.example.com"),
                &world(&all, &[], None),
            );
            (
                dmarc_ids(&r)
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>(),
                r.facts.policy_present,
            )
        };
        let id = |s: &str| (vec![s.to_string()], T);
        // Only the organizational record: found, and sp= applies (sp defaults
        // to p).
        assert_eq!(
            sub(&[("_dmarc.example.com", &["v=DMARC1; p=reject; sp=none"])]),
            id("dmarc.fail.none.v1")
        );
        assert_eq!(
            sub(&[("_dmarc.example.com", &["v=DMARC1; p=reject; sp=quarantine"])]),
            id("dmarc.fail.quarantine.v1")
        );
        assert_eq!(
            sub(&[("_dmarc.example.com", &["v=DMARC1; p=reject"])]),
            id("dmarc.fail.reject.v1")
        );
        // The Author Domain's own record: p= applies, not the parent's sp=.
        assert_eq!(
            sub(&[
                ("_dmarc.example.com", &["v=DMARC1; p=none; sp=none"]),
                ("_dmarc.mail.example.com", &["v=DMARC1; p=quarantine"]),
            ]),
            id("dmarc.fail.quarantine.v1")
        );
        // np= for a subdomain that does not exist: nothing is declared at or
        // below it. Declared (a record at it, or only below it), sp= applies.
        let org = (
            "_dmarc.example.com",
            &["v=DMARC1; p=reject; sp=quarantine; np=none"][..],
        );
        let ghost = |extra: &[(&str, &[&str])]| {
            let mut all = vec![org];
            all.extend_from_slice(extra);
            let r = run(
                &msg("a@ghost.example.com"),
                &env("a@ghost.example.com"),
                &world(&all, &[], None),
            );
            dmarc_ids(&r)
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        assert_eq!(ghost(&[]), ["dmarc.fail.none.v1"]);
        assert_eq!(
            ghost(&[("ghost.example.com", &["v=spf1 -all"])]),
            ["dmarc.fail.quarantine.v1"]
        );
        assert_eq!(
            ghost(&[("deep.in.ghost.example.com", &["x"])]),
            ["dmarc.fail.quarantine.v1"]
        );
        assert_eq!(
            ghost(&[("other.example.com", &["x"])]),
            ["dmarc.fail.none.v1"]
        );
        let down = {
            let w = world(&[org], &["deep.ghost.example.com"], None);
            let r = run(&msg("a@ghost.example.com"), &env("a@ghost.example.com"), &w);
            dmarc_ids(&r)
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            down,
            ["dmarc.fail.quarantine.v1"],
            "a name that only fails temporarily exists"
        );
    }

    /// AC10: the organizational domain is decided by the declared `_dmarc`
    /// records alone (no public-suffix list): a `psd=y` record stops the walk
    /// and puts the organizational domain one label below it.
    #[test]
    fn organizational_domain_comes_from_the_declared_walk() {
        let aligned_with = |records: &[(&str, &[&str])], from: &str, sender: &str| {
            let mut all = vec![(sender.rsplit('@').next().unwrap(), &[OPEN][..])];
            all.extend_from_slice(records);
            let r = run(&msg(from), &env(sender), &world(&all, &[], None));
            (
                dmarc_ids(&r)
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>(),
                r.facts.spf_aligned,
            )
        };
        let psd = [("_dmarc.co.uk", &["v=DMARC1; p=reject; psd=y"][..])];
        // Under a declared public suffix: the sender's subdomain aligns, a
        // sibling registrant does not.
        assert_eq!(
            aligned_with(&psd, "a@brand.co.uk", "a@mail.brand.co.uk"),
            (vec![], T)
        );
        assert_eq!(
            aligned_with(&psd, "a@brand.co.uk", "a@other.co.uk"),
            (vec!["dmarc.fail.reject.v1".to_string()], F)
        );
        // Without psd=y the same records make co.uk the organizational domain,
        // and a sibling aligns: only the declared world can say otherwise.
        let flat = [("_dmarc.co.uk", &["v=DMARC1; p=reject"][..])];
        assert_eq!(
            aligned_with(&flat, "a@brand.co.uk", "a@other.co.uk"),
            (vec![], T)
        );
        // psd=n marks the organizational domain itself.
        let org = [
            ("_dmarc.example.com", &["v=DMARC1; p=reject; psd=n"][..]),
            ("_dmarc.com", &["v=DMARC1; p=none; psd=y"][..]),
        ];
        assert_eq!(
            aligned_with(&org, "a@a.example.com", "a@b.example.com"),
            (vec![], T)
        );
        // A walk is at most eight names, however many labels the From has.
        let deep = "a@l1.l2.l3.l4.l5.l6.l7.l8.l9.l10.example.com";
        let w = world(&[], &[], None);
        let r = run(&msg(deep), &Envelope::default(), &w);
        let walked: Vec<&str> = r
            .names()
            .into_iter()
            .filter(|n| n.starts_with("_dmarc."))
            .collect();
        assert_eq!(walked.len(), 8, "{walked:?}");
        assert_eq!(dmarc_ids(&r), ["dmarc.none.v1"]);
        // The sender's own walk is at most eight more, once.
        let r = run(
            &msg("a@example.com"),
            &env("a@l1.l2.l3.l4.l5.l6.l7.l8.l9.l10.example.com"),
            &example(REJECT),
        );
        let walked = r
            .names()
            .into_iter()
            .filter(|n| n.starts_with("_dmarc."))
            .count();
        assert!(walked <= 8 + 8, "{walked}");
    }

    fn aligned_pair(r: &Run) -> (Fact, Fact) {
        (r.facts.spf_pass, r.facts.spf_aligned)
    }

    /// AC11: relaxed and strict alignment for SPF, each a pair, with a
    /// subdomain identity on either side.
    #[test]
    fn spf_alignment_relaxed_and_strict() {
        let at = |dmarc: &str, from: &str, sender: &str| {
            let w = world(
                &[
                    ("example.com", &[OPEN]),
                    ("mail.example.com", &[OPEN]),
                    ("_dmarc.example.com", &[dmarc]),
                ],
                &[],
                None,
            );
            let r = run(&msg(from), &env(sender), &w);
            (
                dmarc_ids(&r)
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>(),
                aligned_pair(&r),
            )
        };
        let pass = (vec![], (T, T));
        let fail = (vec!["dmarc.fail.reject.v1".to_string()], (T, F));
        // Relaxed (the default): the same organizational domain aligns.
        for dmarc in ["v=DMARC1; p=reject", "v=DMARC1; p=reject; aspf=r"] {
            assert_eq!(at(dmarc, "a@example.com", "b@mail.example.com"), pass);
            assert_eq!(at(dmarc, "a@mail.example.com", "b@example.com"), pass);
            assert_eq!(at(dmarc, "a@example.com", "b@example.com"), pass);
        }
        // Strict: the domains must be equal.
        let strict = "v=DMARC1; p=reject; aspf=s";
        assert_eq!(at(strict, "a@example.com", "b@mail.example.com"), fail);
        assert_eq!(at(strict, "a@mail.example.com", "b@example.com"), fail);
        assert_eq!(at(strict, "a@example.com", "b@example.com"), pass);
        // A different organizational domain never aligns.
        assert_eq!(
            at("v=DMARC1; p=reject", "a@example.com", "b@example.org").0,
            ["dmarc.fail.reject.v1"]
        );
    }

    /// The signed-message world for DKIM alignment: the key for `signer` and
    /// the DMARC record at example.com.
    fn signed_world(signer: &str, dmarc: &str) -> Arc<Effective> {
        let key = KeyKind::Ed25519.record();
        let name = format!("sel._domainkey.{signer}");
        world(
            &[
                (name.as_str(), &[key.as_str()]),
                ("_dmarc.example.com", &[dmarc]),
            ],
            &[],
            None,
        )
    }

    fn dkim_pair(r: &Run) -> (Fact, Fact) {
        (r.facts.dkim_pass, r.facts.dkim_aligned)
    }

    /// AC11: relaxed and strict alignment for DKIM, with a subdomain `d=`.
    #[test]
    fn dkim_alignment_relaxed_and_strict() {
        let signed_by = |domain: &'static str| {
            signed(
                &Sign {
                    domain,
                    ..Sign::default()
                },
                &message(BODY),
            )
        };
        let at = |signer: &'static str, dmarc: &str| {
            let r = run(
                &signed_by(signer),
                &Envelope::default(),
                &signed_world(signer, dmarc),
            );
            (
                dmarc_ids(&r)
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>(),
                dkim_pair(&r),
            )
        };
        let pass = (vec![], (T, T));
        assert_eq!(at("example.com", "v=DMARC1; p=reject"), pass);
        assert_eq!(at("mail.example.com", "v=DMARC1; p=reject"), pass);
        assert_eq!(at("mail.example.com", "v=DMARC1; p=reject; adkim=r"), pass);
        // Strict: a signing subdomain does not align; SPF is not evaluated, so
        // the result cannot be called a failure either.
        let strict = "v=DMARC1; p=reject; adkim=s";
        assert_eq!(at("example.com", strict), pass);
        let (ids, pair) = at("mail.example.com", strict);
        assert_eq!(
            (ids, pair),
            (vec!["dmarc.not_evaluated.v1".to_string()], (T, F))
        );
        // With SPF evaluated and failing, it is a failure.
        let r = run(
            &signed_by("mail.example.com"),
            &env("a@example.com"),
            &world(
                &[
                    ("example.com", &["v=spf1 -all"]),
                    (
                        "sel._domainkey.mail.example.com",
                        &[KeyKind::Ed25519.record().as_str()],
                    ),
                    ("_dmarc.example.com", &[strict]),
                ],
                &[],
                None,
            ),
        );
        assert_eq!(dmarc_ids(&r), ["dmarc.fail.reject.v1"]);
        assert_eq!(dkim_pair(&r), (T, F));
    }

    fn facts_of(r: &Run) -> [Fact; 6] {
        let f = &r.facts;
        [
            f.policy_present,
            f.valid_from,
            f.dkim_pass,
            f.dkim_aligned,
            f.spf_pass,
            f.spf_aligned,
        ]
    }

    fn signed_header(m: &[u8]) -> Vec<u8> {
        let all = signed(&Sign::default(), m);
        all[..all.len() - m.len()].to_vec()
    }

    /// AC11: what DKIM contributes is #254's verdict, not the crate's. An
    /// unverified signature whose `d=` equals the From domain does not align; a
    /// signature the crate verifies but #254 refuses (From not signed) does not
    /// either; the `l=` partial warning verified and still aligns; a key lookup
    /// that failed temporarily leaves the alignment unknown.
    #[test]
    fn only_signatures_swarmail_accepts_align() {
        let w = signed_world("example.com", REJECT);
        let m = message(BODY);
        let failing_spf = || {
            let key = KeyKind::Ed25519.record();
            world(
                &[
                    ("example.com", &["v=spf1 -all"]),
                    ("sel._domainkey.example.com", &[key.as_str()]),
                    ("_dmarc.example.com", &[REJECT]),
                ],
                &[],
                None,
            )
        };
        // Passing: verified, aligned.
        let good = signed(&Sign::default(), &m);
        let r = run(&good, &Envelope::default(), &w);
        assert_eq!(dkim_pair(&r), (T, T));
        assert!(dmarc_ids(&r).is_empty());
        // Failing: the body changed after signing. d= equals the From domain,
        // and the crate's output for it is a failure Swarmail reports.
        let altered = String::from_utf8(good.clone())
            .unwrap()
            .replace("this  is a test", "this is a test!");
        let w2 = failing_spf();
        let r = run(altered.as_bytes(), &env("a@example.com"), &w2);
        assert_eq!(dkim_pair(&r), (F, F));
        assert_eq!(dmarc_ids(&r), ["dmarc.fail.reject.v1"]);
        let whole = report(altered.as_bytes(), &env("a@example.com"), &w2);
        assert!(
            whole
                .findings
                .iter()
                .any(|f| f.rule_id == "dkim.body_hash_mismatch.v1")
        );
        // A signature over h=subject only: the crate verifies it, #254 refuses it.
        let unsigned_from = signed(
            &Sign {
                headers: vec!["subject".into()],
                ..Sign::default()
            },
            &m,
        );
        let r = run(&unsigned_from, &env("a@example.com"), &w2);
        assert_eq!(dkim_pair(&r), (F, F));
        let whole = report(&unsigned_from, &env("a@example.com"), &w2);
        assert!(
            whole
                .findings
                .iter()
                .any(|f| f.rule_id == "dkim.from_unsigned.v1")
        );
        // The l= warning: text appended after the signed length. It verified.
        let mut partial = signed(
            &Sign {
                body_length: true,
                ..Sign::default()
            },
            &m,
        );
        partial.extend_from_slice(b"Pay me now.\r\n");
        let r = run(&partial, &Envelope::default(), &w);
        assert_eq!(dkim_pair(&r), (T, T));
        // A key lookup that failed temporarily: not evaluated, not a failure.
        let down = world(
            &[
                ("_dmarc.example.com", &[REJECT]),
                ("example.com", &["v=spf1 -all"]),
            ],
            &["sel._domainkey.example.com"],
            None,
        );
        let r = run(&good, &env("a@example.com"), &down);
        assert_eq!(dkim_pair(&r), (U, U));
        assert_eq!(dmarc_ids(&r), ["dmarc.not_evaluated.v1"]);
        // No key declared: a definite failure.
        let none = world(
            &[
                ("_dmarc.example.com", &[REJECT]),
                ("example.com", &["v=spf1 -all"]),
            ],
            &[],
            None,
        );
        let r = run(&good, &env("a@example.com"), &none);
        assert_eq!(dkim_pair(&r), (F, F));
        assert_eq!(dmarc_ids(&r), ["dmarc.fail.reject.v1"]);
        // A signature from a different domain verifies and does not align.
        let other = signed(
            &Sign {
                domain: "example.net",
                ..Sign::default()
            },
            &m,
        );
        let key = KeyKind::Ed25519.record();
        let w3 = world(
            &[
                ("sel._domainkey.example.net", &[key.as_str()]),
                ("example.com", &["v=spf1 -all"]),
                ("_dmarc.example.com", &[REJECT]),
            ],
            &[],
            None,
        );
        let r = run(&other, &env("a@example.com"), &w3);
        assert_eq!(dkim_pair(&r), (T, F));
        assert_eq!(dmarc_ids(&r), ["dmarc.fail.reject.v1"]);
    }

    /// The checklist entries that read the facts, by rule id.
    fn checklist(report: &Report) -> Vec<&str> {
        report
            .findings
            .iter()
            .map(|f| f.rule_id.as_str())
            .filter(|id| {
                [
                    "bulk.dmarc_presence.",
                    "bulk.from_alignment.",
                    "bulk.authenticated_sender.",
                ]
                .iter()
                .any(|prefix| id.starts_with(prefix))
            })
            .collect()
    }

    /// AC11: aligned is not authenticated. A MAIL FROM at the From domain whose
    /// record rejects the sender aligns and does not pass: `bulk.from_alignment`
    /// is clean and `bulk.authenticated_sender` is not.
    #[test]
    fn aligned_but_not_authenticated() {
        let whole = report(
            &msg("a@example.com"),
            &env("a@example.com"),
            &failing(&[REJECT]),
        );
        assert_eq!(checklist(&whole), ["bulk.authenticated_sender.v1"]);
        assert!(
            whole
                .findings
                .iter()
                .any(|f| f.rule_id == "dmarc.fail.reject.v1")
        );
        // With an authenticated sender the whole checklist entry set is clean.
        let whole = report(
            &msg("a@example.com"),
            &env("a@example.com"),
            &example(REJECT),
        );
        assert_eq!(checklist(&whole), Vec::<&str>::new());
        assert!(
            whole
                .findings
                .iter()
                .all(|f| !f.rule_id.starts_with("dmarc."))
        );
        // An aligned SPF record that is not evaluable is not evaluated, no failure.
        let w = world(
            &[
                ("example.com", &["v=spf1 a -all"]),
                ("_dmarc.example.com", &[REJECT]),
            ],
            &[],
            None,
        );
        let whole = report(&msg("a@example.com"), &env("a@example.com"), &w);
        assert_eq!(
            checklist(&whole),
            ["bulk.authenticated_sender.not_evaluated.v1"]
        );
    }

    /// AC13: the six facts per scenario, in the order policy_present,
    /// valid_from, dkim_pass, dkim_aligned, spf_pass, spf_aligned.
    #[test]
    fn the_six_facts_follow_the_table() {
        let m = message(BODY);
        let good = signed(&Sign::default(), &m);
        let key = KeyKind::Ed25519.record();
        let keyed = |spf: &'static str| {
            world(
                &[
                    ("example.com", &[spf]),
                    ("sel._domainkey.example.com", &[key.as_str()]),
                    ("_dmarc.example.com", &[REJECT]),
                ],
                &[],
                None,
            )
        };
        let sender = env("alice@example.com");
        let facts = |raw: &[u8], envelope: &Envelope<'_>, w: &Arc<Effective>| {
            facts_of(&run(raw, envelope, w))
        };
        let alice = msg("alice@example.com");
        // SPF evaluated, no signature.
        assert_eq!(facts(&alice, &sender, &example(REJECT)), [T, T, F, F, T, T]);
        assert_eq!(
            facts(&alice, &sender, &failing(&[REJECT])),
            [T, T, F, F, F, T]
        );
        assert_eq!(
            facts(&alice, &env("a@example.org"), &example(REJECT)),
            [T, T, F, F, F, F]
        );
        let other = world(
            &[("example.org", &[OPEN]), ("_dmarc.example.com", &[REJECT])],
            &[],
            None,
        );
        assert_eq!(
            facts(&alice, &env("a@example.org"), &other),
            [T, T, F, F, T, F]
        );
        // SPF not evaluated, or evaluated to something that is not a verdict.
        assert_eq!(
            facts(&alice, &Envelope::default(), &example(REJECT)),
            [T, T, F, F, U, U]
        );
        let down = world(&[("_dmarc.example.com", &[REJECT])], &["example.com"], None);
        assert_eq!(facts(&alice, &sender, &down), [T, T, F, F, U, T]);
        let unsupported = world(
            &[
                ("example.com", &["v=spf1 mx -all"]),
                ("_dmarc.example.com", &[REJECT]),
            ],
            &[],
            None,
        );
        assert_eq!(facts(&alice, &sender, &unsupported), [T, T, F, F, U, T]);
        // No applicable policy: alignment is defined against the policy.
        let nothing = world(&[("example.com", &[OPEN])], &[], None);
        assert_eq!(facts(&alice, &sender, &nothing), [F, T, F, U, T, U]);
        // A From that cannot be judged: the policy is unknown, SPF is not.
        let none = b"To: bob@example.org\r\n\r\nHello\r\n";
        assert_eq!(facts(none, &sender, &example(REJECT)), [U, F, F, U, T, U]);
        // DKIM alone, SPF not evaluated.
        assert_eq!(
            facts(&good, &Envelope::default(), &keyed(OPEN)),
            [T, T, T, T, U, U]
        );
        // DKIM and SPF both aligned passes.
        assert_eq!(facts(&good, &sender, &keyed(OPEN)), [T, T, T, T, T, T]);
        // A signature that does not verify, beside a failing SPF.
        let broken = String::from_utf8(good.clone())
            .unwrap()
            .replace("Hello", "Hullo");
        assert_eq!(
            facts(broken.as_bytes(), &sender, &keyed("v=spf1 -all")),
            [T, T, F, F, F, T]
        );
        // A deferred key lookup: DKIM is neither a pass nor a failure.
        let deferred = world(
            &[("example.com", &[OPEN]), ("_dmarc.example.com", &[REJECT])],
            &["sel._domainkey.example.com"],
            None,
        );
        assert_eq!(facts(&good, &sender, &deferred), [T, T, U, U, T, T]);
        // Bounds and boundaries: 17 fields stop DKIM at its bound, a stray line
        // stops everything at the boundary.
        let header = signed_header(&m);
        let mut many = header.repeat(17);
        many.extend_from_slice(&m);
        assert_eq!(facts(&many, &sender, &keyed(OPEN)), [T, T, U, U, T, T]);
        let mut stray = header.clone();
        stray.extend_from_slice(b"stray line\r\n");
        stray.extend_from_slice(&m);
        assert_eq!(facts(&stray, &sender, &keyed(OPEN)), [U, U, U, U, T, U]);
        let r = run(&stray, &sender, &keyed(OPEN));
        assert_eq!(dmarc_ids(&r), ["dmarc.not_evaluated.v1"]);
        let whole = report(&stray, &sender, &keyed(OPEN));
        assert!(
            whole
                .findings
                .iter()
                .any(|f| f.rule_id == "dkim.header_boundary_ambiguous.v1")
        );
        // The HELO identity aligns the way MAIL FROM does.
        let helo_world = world(
            &[("example.com", &[OPEN]), ("_dmarc.example.com", &[REJECT])],
            &[],
            None,
        );
        let null = Envelope {
            mail_from: Some(""),
            helo: Some("example.com"),
            ..sender
        };
        assert_eq!(facts(&alice, &null, &helo_world), [T, T, F, F, T, T]);
    }

    /// The same bytes and policy give the same report, twice, and the facts
    /// have no other input (decision 0126 §6).
    #[test]
    fn evaluation_is_deterministic() {
        let m = message(BODY);
        let good = signed(&Sign::default(), &m);
        let w = signed_world(
            "example.com",
            "v=DMARC1; p=reject; rua=mailto:r@example.com",
        );
        let first = serde_json::to_vec(&report(&good, &env("alice@example.com"), &w)).unwrap();
        let second = serde_json::to_vec(&report(&good, &env("alice@example.com"), &w)).unwrap();
        assert_eq!(first, second);
        let a = run(&good, &env("alice@example.com"), &w);
        let b = run(&good, &env("alice@example.com"), &w);
        assert_eq!(facts_of(&a), facts_of(&b));
        assert_eq!(a.dns, b.dns);
    }

    fn shared_over(w: &Arc<Effective>) -> Lookups {
        Lookups::new(w.clone())
    }

    /// AC15: every cache the crate may ask answers every query, so it never
    /// reaches its resolver, and the ones the crate never asks are inert.
    #[test]
    fn every_cache_answers_every_query() {
        let w = world(
            &[
                ("example.com", &[OPEN]),
                ("dup.example.com", &[OPEN, "v=spf1 -all"]),
                ("_dmarc.example.com", &[REJECT]),
                ("sub.deep.example.org", &["x"]),
            ],
            &["down.example.com"],
            None,
        );
        let mut lookups = shared_over(&w);
        let shared = RefCell::new(&mut lookups);
        let txt = |kind| Txts {
            lookups: &shared,
            kind,
        };
        let ask = |kind, name: &str| {
            ResolverCache::<Box<str>, TxtRecord>::get::<str>(&txt(kind), name).unwrap()
        };
        assert!(matches!(ask(Kind::Spf, "example.com."), TxtRecord::Spf(_)));
        assert!(matches!(ask(Kind::Spf, "EXAMPLE.com"), TxtRecord::Spf(_)));
        assert!(matches!(
            ask(Kind::Dmarc, "_dmarc.example.com."),
            TxtRecord::Dmarc(_)
        ));
        // The record type is the caller's choice: a name holds what it holds.
        assert!(matches!(
            ask(Kind::Dmarc, "example.com."),
            TxtRecord::Error(Error::Dns(DnsError::RecordNotFound(_)))
        ));
        assert!(matches!(
            ask(Kind::Spf, "_dmarc.example.com."),
            TxtRecord::Error(Error::Dns(DnsError::RecordNotFound(_)))
        ));
        assert!(matches!(
            ask(Kind::Spf, "dup.example.com."),
            TxtRecord::Error(Error::Parse)
        ));
        assert!(matches!(
            ask(Kind::Spf, "down.example.com."),
            TxtRecord::Error(Error::Dns(DnsError::Resolver(_)))
        ));
        for name in ["missing.example.com.", "bad..name.", "", "a b."] {
            assert!(
                matches!(
                    ask(Kind::Spf, name),
                    TxtRecord::Error(Error::Dns(DnsError::RecordNotFound(_)))
                ),
                "{name:?}"
            );
        }
        // insert and remove are inert, as the crate's own caching would be.
        let cache = txt(Kind::Spf);
        ResolverCache::<Box<str>, TxtRecord>::insert(
            &cache,
            "a.example.".into(),
            not_found(),
            Instant::now(),
        );
        assert!(
            ResolverCache::<Box<str>, TxtRecord>::remove::<str>(&cache, "example.com.").is_none()
        );

        // The negative caches answer MX, A, AAAA and PTR, and mark SPF's.
        let mark = Cell::new(false);
        let quiet = Absent { mark: None };
        let marking = Absent { mark: Some(&mark) };
        fn nx<T>() -> Result<T, Negative> {
            Err(Negative {
                code: ResponseCode::NXDomain,
            })
        }
        assert_eq!(
            ResolverCache::<Box<str>, Result<RecordSet<Mx>, Negative>>::get::<str>(
                &quiet,
                "a.example."
            ),
            Some(nx())
        );
        assert!(!mark.get());
        assert_eq!(
            ResolverCache::<Box<str>, Result<RecordSet<std::net::Ipv6Addr>, Negative>>::get::<str>(
                &marking,
                "a.example."
            ),
            Some(nx())
        );
        assert!(mark.get());
        mark.set(false);
        let ptr = ResolverCache::<IpAddr, Result<RecordSet<Box<str>>, Negative>>::get(
            &marking,
            &"::1".parse().unwrap(),
        );
        assert_eq!(ptr, Some(nx()));
        assert!(mark.get());
        ResolverCache::<IpAddr, Result<RecordSet<Box<str>>, Negative>>::insert(
            &marking,
            "::1".parse().unwrap(),
            nx(),
            Instant::now(),
        );
        assert!(
            ResolverCache::<IpAddr, Result<RecordSet<Box<str>>, Negative>>::remove(
                &marking,
                &"::1".parse().unwrap()
            )
            .is_none()
        );

        // The A cache: DMARC's existence rule, SPF's mark.
        let exists = Exists {
            lookups: Some(&shared),
            mark: None,
        };
        let a = |cache: &Exists<'_, '_>, name: &str| {
            ResolverCache::<Box<str>, Result<RecordSet<Ipv4Addr>, Negative>>::get::<str>(
                cache, name,
            )
        };
        let nodata = Some(Err(Negative {
            code: ResponseCode::NoError,
        }));
        assert_eq!(a(&exists, "example.com."), nodata);
        assert_eq!(
            a(&exists, "org."),
            nodata,
            "a name above a declared one exists"
        );
        assert_eq!(a(&exists, "down.example.com."), nodata);
        assert_eq!(a(&exists, "ghost.example.com."), Some(nx()));
        assert_eq!(a(&exists, "bad..name."), Some(nx()));
        assert_eq!(a(&exists, ""), Some(nx()));
        let spf_side = Exists {
            lookups: None,
            mark: Some(&mark),
        };
        mark.set(false);
        assert_eq!(a(&spf_side, "example.com."), Some(nx()));
        assert!(mark.get());
        ResolverCache::<Box<str>, Result<RecordSet<Ipv4Addr>, Negative>>::insert(
            &exists,
            "a.example.".into(),
            nx(),
            Instant::now(),
        );
        assert!(
            ResolverCache::<Box<str>, Result<RecordSet<Ipv4Addr>, Negative>>::remove::<str>(
                &exists,
                "a.example."
            )
            .is_none()
        );

        // The surface hands the crate every cache.
        let caches = Caches {
            txt: txt(Kind::Dmarc),
            exists,
            absent: quiet,
        };
        assert!(caches.txt().is_some());
        assert!(caches.mx().is_some());
        assert!(caches.ipv4().is_some());
        assert!(caches.ipv6().is_some());
        assert!(caches.ptr().is_some());
        // Only the TXT names that were asked and declared were recorded.
        let names: Vec<String> = lookups
            .finish()
            .lookups
            .into_iter()
            .map(|l| l.name)
            .collect();
        assert_eq!(
            names,
            [
                "_dmarc.example.com",
                "down.example.com",
                "dup.example.com",
                "example.com",
                "missing.example.com"
            ]
            .map(String::from)
        );
    }

    /// AC15: report sampling (`ra=`/`rp=`) reads the wall clock inside the crate,
    /// so no record the adapter hands over carries `ra=`; evaluation is
    /// unchanged by it, and `%{t}` is never expanded.
    #[test]
    fn ra_is_cleared_before_the_crate_sees_a_record() {
        let record = "v=spf1 ra=postmaster rp=0 rr=all ip4:127.0.0.1 -all";
        let parsed = SpfRecord::unwrap_txt(Kind::Spf.select(vec![record.into()])).unwrap();
        assert!(parsed.ra.is_none());
        assert_eq!(parsed.rp, 0, "everything else is the crate's parse");
        // What the crate itself would have kept, to prove the adapter cleared it.
        let raw = SpfRecord::parse(record.as_bytes()).unwrap();
        assert!(raw.ra.is_some());
        // And a failing record with ra= and a zero sample evaluates normally,
        // with no report address anywhere.
        let (r, ids) = spf_over(
            &one("v=spf1 ra=postmaster rp=0 ip4:192.0.2.1 -all"),
            "127.0.0.1",
        );
        assert_eq!(ids, ["spf.fail.v1"]);
        assert_eq!(r.facts.spf_pass, F);
        // DMARC records are the crate's, and their reporting tags change nothing.
        let r = mail(&failing(&[
            "v=DMARC1; p=reject; rua=mailto:r@example.com; ruf=mailto:f@example.com; fo=1",
        ]));
        assert_eq!(dmarc_ids(&r), ["dmarc.fail.reject.v1"]);
    }

    /// The closed rule table: ids, severities, locations and static hints.
    #[test]
    fn the_rule_table_is_closed() {
        let ids: Vec<&str> = Rule::ALL.iter().map(|r| r.id()).collect();
        assert_eq!(
            ids,
            [
                "spf.fail.v1",
                "spf.softfail.v1",
                "spf.neutral.v1",
                "spf.none.v1",
                "spf.temperror.v1",
                "spf.permerror.v1",
                "spf.unsupported_mechanism.v1",
                "spf.not_evaluated.v1",
                "dmarc.from_invalid.v1",
                "dmarc.none.v1",
                "dmarc.temperror.v1",
                "dmarc.permerror.v1",
                "dmarc.fail.none.v1",
                "dmarc.fail.quarantine.v1",
                "dmarc.fail.reject.v1",
                "dmarc.not_evaluated.v1",
            ]
        );
        let fields: Headers = vec![(b"from".to_vec(), b" a@example.com".to_vec())];
        for rule in Rule::ALL {
            let finding = rule.finding(&fields);
            assert_eq!(finding.rule_id, rule.id());
            assert_eq!(
                finding.severity,
                if rule == Rule::SpfNotEvaluated {
                    Severity::Info
                } else {
                    Severity::Error
                }
            );
            // The hint is a constant: nothing a message holds can reach it, and no
            // character the UI or a JSON string has to escape is in it.
            assert!(
                !finding.fix_hint.contains(['\'', '"', '<', '>', '&', '\\']),
                "{}",
                rule.id()
            );
            assert!(finding.fix_hint.ends_with('.'), "{}", rule.id());
            let location = serde_json::to_value(&finding.location).unwrap();
            if rule.id().starts_with("spf.") {
                assert_eq!(
                    location,
                    serde_json::json!({"kind": "header", "name": "return-path", "occurrence": null})
                );
            } else {
                assert_eq!(
                    location,
                    serde_json::json!({"kind": "header", "name": "from", "occurrence": 0})
                );
            }
        }
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), 16);
    }

    /// AC1, AC13: the service reads the observed envelope from the stored row.
    /// A row with an observed session is judged; a row without one (seeded, or
    /// stored before the columns) is not evaluated for SPF and still gets DMARC
    /// through DKIM alone.
    #[tokio::test]
    async fn the_service_reads_the_stored_envelope() {
        let service = super::super::Service::default();
        service.auth.set(None).unwrap();
        let m = message(BODY);
        let good = signed(&Sign::default(), &m);
        let key = KeyKind::Ed25519.record();
        let w = world(
            &[
                ("example.com", &[OPEN]),
                ("sel._domainkey.example.com", &[key.as_str()]),
                ("_dmarc.example.com", &[REJECT]),
            ],
            &[],
            None,
        );
        let row = |observed: bool| {
            let mut email = crate::store::email("row", "box", "bob@example.org");
            email.raw = good.clone();
            if observed {
                email.envelope_from = Some("alice@example.com".into());
                email.smtp_client_ip = Some("127.0.0.1".parse().unwrap());
                email.smtp_helo = Some("mail.example.com".into());
            }
            Arc::new(email)
        };
        let ids = |report: &Report| -> Vec<String> {
            report
                .findings
                .iter()
                .map(|f| f.rule_id.clone())
                .filter(|id| {
                    ["spf.", "dmarc.", "dkim."]
                        .iter()
                        .any(|prefix| id.starts_with(prefix))
                })
                .collect()
        };
        let seen = service
            .report_under(row(true), w.clone(), false)
            .await
            .unwrap();
        assert_eq!(ids(&seen), Vec::<String>::new());
        let unseen = service
            .report_under(row(false), w.clone(), false)
            .await
            .unwrap();
        assert_eq!(ids(&unseen), ["spf.not_evaluated.v1"]);
        // Through DKIM alone the sender is authenticated all the same.
        assert!(checklist(&unseen).is_empty());
        // A sender the record does not list, over the same row shape.
        let mut stranger = (*row(true)).clone();
        stranger.smtp_client_ip = Some("203.0.113.9".parse().unwrap());
        let report = service
            .report_under(Arc::new(stranger), w, false)
            .await
            .unwrap();
        assert_eq!(ids(&report), ["spf.fail.v1"]);
        assert!(checklist(&report).is_empty(), "DKIM still authenticates it");
    }

    /// The guide lists every rule exactly once, with the hint the code emits,
    /// and the bounds and tables it states are the ones the code enforces.
    #[test]
    fn guide_lists_every_auth_rule() {
        let guide = include_str!("../../docs/guide/deliverability.md");
        for rule in Rule::ALL {
            let heading = format!("### `{}`\n", rule.id());
            assert_eq!(guide.matches(&heading).count(), 1, "{}", rule.id());
            let start = guide.find(&heading).unwrap();
            let rest = &guide[start + heading.len()..];
            let section = &rest[..rest.find("\n##").unwrap()];
            let word = format!("{:?}", rule.severity()).to_lowercase();
            assert!(
                section.contains(&format!("**Severity:** {word}.")),
                "{}",
                rule.id()
            );
            assert!(
                section.contains(&format!("**Hint:** {}", rule.hint())),
                "{}",
                rule.id()
            );
            assert!(section.contains("**Passing:**"));
            assert!(section.contains("**Failing:**"));
        }
        assert_eq!(guide.matches("\n### `spf.").count(), 8);
        assert_eq!(guide.matches("\n### `dmarc.").count(), 8);
        for fact in [
            "## SPF and DMARC (`swarmail-html/8`)",
            "**The observed envelope.**",
            "**Record types: TXT only.**",
            "**Limits (RFC 7208 section 4.6.4).**",
            "Ten DNS-querying terms are admitted and the\neleventh is `spf.permerror.v1`",
            "at most twelve names",
            "**Authentication-Results correspondence.**",
            "**The facts, and the checklist.**",
            "determined offline by the declared records alone",
            "`swarmail-html/7` (the sixteen SPF and DMARC rules of proposed 0126",
            "monotonic timer",
        ] {
            assert!(guide.contains(fact), "{fact}");
        }
        assert_eq!(HOST, "swarmail.invalid");
    }
}
