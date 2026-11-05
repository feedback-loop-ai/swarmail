//! Offline DKIM verification (issue #254, proposed decision 0125).
//!
//! Each `DKIM-Signature` field of the stored bytes is verified by `mail-auth`
//! against keys that come from one place only: the declared DNS map, through
//! `Lookups` (decision 0120). The crate parses, canonicalizes, hashes and checks
//! the signature; this module owns everything that decides a verdict around it:
//! which key source answers, what time a signature is judged at, the bounds,
//! and the mapping from the crate's result to a stable, versioned rule id.
//!
//! The crate has a DNS resolver compiled in (it cannot be built without one) and
//! it is unreachable: every name it asks for is answered by [`Adapter`], which
//! never misses, and the one [`MessageAuthenticator`] has no name servers.
//!
//! Time. Swarmail reads no clock. A signature is judged at the message's receipt
//! second (`x=` at or before it is expired), so a report is a function of the
//! bytes, the receipt time and the policy. The crate itself compares `x=` with
//! the wall clock inside `verify_dkim`; that read is the clock residual, and its
//! radius is stated in decision 0125 section 8: once the host clock reaches an
//! `x=` that is after the receipt second, every verdict the crate would have
//! decided for that field becomes `dkim.expired_since_receipt.v1`, and the key
//! lookups for it disappear.
//!
//! One body, one field list. Before anything is verified, two things must hold
//! of the crate's strict parse: it ends the headers where Swarmail does, so DKIM
//! hashes the body every other collector sees, and it lists exactly as many
//! `DKIM-Signature` fields as Swarmail's root list does, so a finding's
//! occurrence is the root occurrence. If either fails (a colon-less line that
//! ends the headers early, or a field name such as `DKIM-Signature :` that only
//! one of the two names a signature), the message gets one
//! `dkim.header_boundary_ambiguous.v1` and nothing else. A passing guard proves
//! those two facts and no more.

use super::dns::{Lookups, OwnerName, Txt};
use super::{AnalysisError, Collector, Finding, Headers, Location, Severity};
use futures_util::FutureExt;
use mail_auth::crypto::{Algorithm, CryptoError};
use mail_auth::dkim::{
    AtpsRecord, Canonicalization, DkimError, DomainKey, Signature, VerifySignature,
};
use mail_auth::dns::TxtRecordParser;
use mail_auth::headers::Writer;
use mail_auth::hickory_resolver::config::{ResolverConfig, ResolverOpts};
use mail_auth::hickory_resolver::proto::op::ResponseCode;
use mail_auth::{
    AuthenticatedMessage, DkimOutput, DkimResult, DnsCache, DnsError, Error, MessageAuthenticator,
    NoCache, Parameters, ResolverCache, TxtRecord,
};
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};
use std::sync::LazyLock;
use std::time::Instant;

/// More fields than this give one `limits_exceeded` finding and no verdicts.
const MAX_DKIM_SIGNATURES: usize = 16;
/// More `h=` names than this in one signature do the same. Real oversigning
/// stays far below it; the crate's header selection is O(names x headers).
const MAX_DKIM_SIGNED_HEADERS: usize = 128;

/// The closed rule table. The guide lists the same ids and a test binds the two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rule {
    Malformed,
    Unsupported,
    FromUnsigned,
    HeaderBoundaryAmbiguous,
    LimitsExceeded,
    SignatureExpired,
    ExpiredSinceReceipt,
    SelectorInvalid,
    KeyMissing,
    KeyInvalid,
    KeyRevoked,
    NotEvaluated,
    BodyHashMismatch,
    SignatureMismatch,
    AuidMismatch,
    AtpsUnauthorized,
    BodyLengthExceeds,
    BodyLengthPartial,
}

impl Rule {
    #[cfg(test)]
    const ALL: [Rule; 18] = [
        Self::Malformed,
        Self::Unsupported,
        Self::FromUnsigned,
        Self::HeaderBoundaryAmbiguous,
        Self::LimitsExceeded,
        Self::SignatureExpired,
        Self::ExpiredSinceReceipt,
        Self::SelectorInvalid,
        Self::KeyMissing,
        Self::KeyInvalid,
        Self::KeyRevoked,
        Self::NotEvaluated,
        Self::BodyHashMismatch,
        Self::SignatureMismatch,
        Self::AuidMismatch,
        Self::AtpsUnauthorized,
        Self::BodyLengthExceeds,
        Self::BodyLengthPartial,
    ];

    const fn id(self) -> &'static str {
        match self {
            Self::Malformed => "dkim.malformed.v1",
            Self::Unsupported => "dkim.unsupported.v1",
            Self::FromUnsigned => "dkim.from_unsigned.v1",
            Self::HeaderBoundaryAmbiguous => "dkim.header_boundary_ambiguous.v1",
            Self::LimitsExceeded => "dkim.limits_exceeded.v1",
            Self::SignatureExpired => "dkim.signature_expired.v1",
            Self::ExpiredSinceReceipt => "dkim.expired_since_receipt.v1",
            Self::SelectorInvalid => "dkim.selector_invalid.v1",
            Self::KeyMissing => "dkim.key_missing.v1",
            Self::KeyInvalid => "dkim.key_invalid.v1",
            Self::KeyRevoked => "dkim.key_revoked.v1",
            Self::NotEvaluated => "dkim.not_evaluated.v1",
            Self::BodyHashMismatch => "dkim.body_hash_mismatch.v1",
            Self::SignatureMismatch => "dkim.signature_mismatch.v1",
            Self::AuidMismatch => "dkim.auid_mismatch.v1",
            Self::AtpsUnauthorized => "dkim.atps_unauthorized.v1",
            Self::BodyLengthExceeds => "dkim.body_length_exceeds.v1",
            Self::BodyLengthPartial => "dkim.body_length_partial.v1",
        }
    }

    /// Every outcome that leaves a signature unverified is an error. The one
    /// warning is a signature that verified but covers only a prefix of the body.
    const fn severity(self) -> Severity {
        match self {
            Self::BodyLengthPartial => Severity::Warning,
            _ => Severity::Error,
        }
    }

    const fn hint(self) -> &'static str {
        match self {
            Self::Malformed => {
                "Fix the DKIM-Signature tag list: each tag once; v=, a=, d=, s=, b=, bh= and h= present; valid base64; t=, x= and l= plain decimal numbers; and x= later than t= (RFC 6376 sections 3.2 and 3.5)."
            }
            Self::Unsupported => {
                "Use v=1, a=rsa-sha256 or ed25519-sha256, c= simple or relaxed, and no l=0 (RFC 6376 section 3.5, RFC 8301 section 3.1, RFC 8463)."
            }
            Self::FromUnsigned => "Sign the From header: list from in h= (RFC 6376 section 5.4).",
            Self::HeaderBoundaryAmbiguous => {
                "A line in the header block that is not a header field (no colon, or only whitespace at the very top), or a DKIM-Signature field name that not every parser accepts (such as a space before the colon), ends or splits the headers differently at different receivers; remove it, fold it into the previous field, or write the name as DKIM-Signature: (RFC 5322 sections 2.2 and 2.2.3)."
            }
            Self::LimitsExceeded => {
                "Send at most 16 DKIM-Signature fields, each with at most 128 header names in h=."
            }
            Self::SignatureExpired => {
                "Sign again with an x= expiration later than the time the message was received (RFC 6376 section 3.5)."
            }
            Self::ExpiredSinceReceipt => {
                "The signature's x= has passed on this machine's clock since the message was received; sign with a later x= (RFC 6376 section 3.5)."
            }
            Self::SelectorInvalid => {
                "Use an s= and d= that form a valid DNS name, <s>._domainkey.<d> (RFC 6376 section 3.6.2.1)."
            }
            Self::KeyMissing => {
                "Declare a TXT record at <s>._domainkey.<d> in the DNS policy; none is declared (RFC 6376 section 3.6.2.1)."
            }
            Self::KeyInvalid => {
                "Publish a v=DKIM1 key record with a valid p= and a k= that matches a= (RFC 6376 section 3.6.1, RFC 8463)."
            }
            Self::KeyRevoked => {
                "The key record has an empty p=, which revokes the key; publish a non-empty p= (RFC 6376 section 3.6.1)."
            }
            Self::NotEvaluated => {
                "The key lookup failed temporarily, so the signature was not evaluated; fix the DNS answer and retry (RFC 6376 section 6.1.2)."
            }
            Self::BodyHashMismatch => {
                "The body differs from what was signed; sign the final body with the same c= (RFC 6376 section 3.7)."
            }
            Self::SignatureMismatch => {
                "A signed header changed after signing, b= was made with another key, or p= is not a usable public key; sign the final headers with the declared key (RFC 6376 section 3.7)."
            }
            Self::AuidMismatch => {
                "Make the i= domain equal to d= or a subdomain of it, and equal to d= when the key has t=s (RFC 6376 sections 3.5 and 3.6.1)."
            }
            Self::AtpsUnauthorized => {
                "Declare the RFC 6541 ATPS record for the signing domain, or remove atps= from the signature (RFC 6541 section 4)."
            }
            Self::BodyLengthExceeds => {
                "Remove l= or set it to the body length; l= is longer than the body (RFC 6376 section 3.4.5)."
            }
            Self::BodyLengthPartial => {
                "Remove l=: it signs only a prefix of the body, so text can be appended unsigned (RFC 6376 section 8.2)."
            }
        }
    }

    fn finding(self, occurrence: usize) -> Finding {
        self.located(Some(occurrence as u64))
    }

    /// The finding at an occurrence of the root header list, or at `None` (0085
    /// section 1's value for a missing field) when the list holds none.
    fn located(self, occurrence: Option<u64>) -> Finding {
        Finding {
            rule_id: self.id().into(),
            location: Location::Header {
                name: "dkim-signature".into(),
                occurrence,
            },
            severity: self.severity(),
            fix_hint: self.hint().into(),
        }
    }
}

/// The only authenticator: a resolver with no name servers, so even a lookup that
/// somehow escaped the adapter would have nowhere to send a packet.
static AUTHENTICATOR: LazyLock<MessageAuthenticator> = LazyLock::new(|| {
    MessageAuthenticator::new(
        ResolverConfig::from_parts(None, Vec::new(), Vec::new()),
        ResolverOpts::default(),
    )
    .expect("a resolver with no name servers builds")
});

/// What `Hash` writes for a `str`: its bytes, then 0xff, which UTF-8 never holds.
/// `ResolverCache::get` is generic over the borrowed key, so this is the only
/// stable way to read the name back; any other shape yields `None`.
struct NameRecorder(Vec<u8>);

impl Hasher for NameRecorder {
    fn write(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes);
    }

    fn finish(&self) -> u64 {
        0
    }
}

fn name_of<Q: Hash + ?Sized>(key: &Q) -> Option<String> {
    let mut recorder = NameRecorder(Vec::new());
    key.hash(&mut recorder);
    (recorder.0.pop() == Some(0xff))
        .then_some(recorder.0)
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

/// The crate's TXT cache over 0120's declared map. `get` always answers, so the
/// crate never reaches its resolver; every answered name is recorded by `Lookups`.
struct Adapter<'l> {
    lookups: RefCell<&'l mut Lookups>,
}

/// The one judge of a DNS name. The crate asks for `<s>._domainkey.<d>.` with a
/// trailing dot; `OwnerName::parse` strips exactly one, so a `d=` that itself ends
/// in a dot leaves an empty label and is outside the grammar. The adapter and the
/// verdict both ask this function, so what is looked up and what is reported
/// cannot disagree.
fn owner_of(name: &str) -> Option<OwnerName> {
    OwnerName::parse(name)
}

fn not_found() -> TxtRecord {
    TxtRecord::Error(Error::Dns(DnsError::RecordNotFound(ResponseCode::NXDomain)))
}

impl Adapter<'_> {
    fn answer(&self, name: &str) -> TxtRecord {
        // A name outside 0120's grammar is never looked up and never recorded.
        let Some(owner) = owner_of(name) else {
            return not_found();
        };
        match self.lookups.borrow_mut().resolve_txt(&owner) {
            Txt::Records(values) => record(owner.as_str(), values),
            Txt::NoRecords => not_found(),
            Txt::TempFail => {
                TxtRecord::Error(Error::Dns(DnsError::Resolver("declared tempfail".into())))
            }
        }
    }
}

/// The record type is chosen by the shape of the name, because the crate asks
/// the cache by name only. There are two types: a name with an `_atps` label and no
/// `_domainkey` one is an ATPS record (and fails closed as `key_invalid` if the
/// signature's selector put it there), and every other name is a key record. A
/// report record is never built: the crate's RFC 6651 report sampling, which reads
/// the clock, is reached only through one (0125 section 8), so even a failing
/// `r=y` signature's `_report._domainkey.<d>` name is looked up, recorded and
/// answered as a key record, which the crate's report loop discards unused.
fn record(name: &str, values: Vec<String>) -> TxtRecord {
    let records: Vec<Vec<u8>> = values.into_iter().map(String::into_bytes).collect();
    if !name.contains("._domainkey.") && name.contains("._atps.") {
        AtpsRecord::parse_records(&records).into()
    } else {
        DomainKey::parse_records(&records).into()
    }
}

impl ResolverCache<Box<str>, TxtRecord> for Adapter<'_> {
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

impl DnsCache for Adapter<'_> {
    type Txt = Self;
    type Mx = NoCache;
    type Ipv4 = NoCache;
    type Ipv6 = NoCache;
    type Ptr = NoCache;

    fn txt(&self) -> Option<&Self::Txt> {
        Some(self)
    }

    fn mx(&self) -> Option<&Self::Mx> {
        None
    }

    fn ipv4(&self) -> Option<&Self::Ipv4> {
        None
    }

    fn ipv6(&self) -> Option<&Self::Ipv6> {
        None
    }

    fn ptr(&self) -> Option<&Self::Ptr> {
        None
    }
}

/// A byte counter: the canonical length is counted, never materialized.
struct Counter(u64);

impl Writer for Counter {
    fn write(&mut self, buf: &[u8]) {
        self.0 += buf.len() as u64;
    }
}

fn is_gap(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}

fn line_breaks(sink: &mut impl Writer, count: usize) {
    for _ in 0..count {
        sink.write(b"\r\n");
    }
}

/// The canonical body (RFC 6376 section 3.4.3 and 3.4.4) as `mail-auth` 0.14.0
/// writes it, bare LF and lone CR included, because its `bh=` is the arbiter of
/// what a signature covers. The public crate cannot give a body's canonical
/// length, so this mirrors the observable behaviour, and a test holds it equal
/// to the crate's own body hash over every short body of an awkward alphabet.
fn canonicalize(body: &[u8], c: Canonicalization, sink: &mut impl Writer) {
    match c {
        // CR dropped, LF as CRLF, trailing line breaks dropped, one CRLF added.
        Canonicalization::Simple => {
            let end = body.iter().rposition(|b| !matches!(b, b'\r' | b'\n'));
            for byte in &body[..end.map_or(0, |i| i + 1)] {
                match byte {
                    b'\r' => {}
                    b'\n' => line_breaks(sink, 1),
                    other => sink.write(std::slice::from_ref(other)),
                }
            }
            line_breaks(sink, 1);
        }
        Canonicalization::Relaxed => {
            // `breaks` waits for the next text; `last` is the last gap or text byte.
            let (mut breaks, mut last, mut empty) = (0, 0, true);
            let mut at = 0;
            while at < body.len() {
                let gap = body[at..].iter().take_while(|b| is_gap(**b)).count();
                if gap > 0 {
                    let (mut carried, mut pending, mut blank) = (breaks, 0, false);
                    for &byte in &body[at..at + gap] {
                        match byte {
                            b'\n' => pending += 1,
                            b' ' | b'\t' => {
                                carried += pending;
                                pending = 0;
                                blank = true;
                            }
                            _ => {}
                        }
                        last = byte;
                    }
                    if blank {
                        line_breaks(sink, carried);
                        breaks = pending;
                        empty = false;
                    } else {
                        breaks = carried + pending;
                    }
                    at += gap;
                } else {
                    let text = body[at..].iter().take_while(|b| !is_gap(**b)).count();
                    line_breaks(sink, breaks);
                    breaks = 0;
                    if last == b' ' || last == b'\t' {
                        sink.write(b" ");
                    }
                    sink.write(&body[at..at + text]);
                    empty = false;
                    last = body[at + text - 1];
                    at += text;
                }
            }
            if !empty {
                line_breaks(sink, 1);
            }
        }
    }
}

/// The canonicalized length of the first `limit` raw body bytes (all of them when
/// `limit` is 0), the way the crate hashes them: its `l=` counts raw bytes.
fn canonical_len(body: &[u8], c: Canonicalization, limit: u64) -> u64 {
    let body = if limit == 0 {
        body
    } else {
        &body[..body.len().min(usize::try_from(limit).unwrap_or(usize::MAX))]
    };
    let mut counter = Counter(0);
    canonicalize(body, c, &mut counter);
    counter.0
}

/// What the crate's public struct cannot say about one field: the tag list read
/// once more, narrowly (RFC 6376 section 3.2).
///
/// It covers only what the crate defaults silently: a repeated tag, a missing
/// `v=` or `a=` (the crate reads them as version 0 and rsa-sha256), and a `t=`,
/// `x=` or `l=` that is not a plain number (the crate reads garbage as absent).
#[derive(Debug, PartialEq, Eq)]
struct Scan {
    duplicate: bool,
    has_v: bool,
    has_a: bool,
    has_t: bool,
    has_x: bool,
    has_l: bool,
    /// Every `t=`, `x=` and `l=` is one or more ASCII digits that fit `u64`, once
    /// the folding whitespace around it is trimmed. Then the crate's value is
    /// the literal one.
    numbers_ok: bool,
    h_names: usize,
}

fn number_fits(value: &[u8]) -> bool {
    let value = value.trim_ascii();
    !value.is_empty()
        && value.iter().all(u8::is_ascii_digit)
        && std::str::from_utf8(value).is_ok_and(|digits| digits.parse::<u64>().is_ok())
}

fn scan(value: &[u8]) -> Scan {
    let mut scan = Scan {
        duplicate: false,
        has_v: false,
        has_a: false,
        has_t: false,
        has_x: false,
        has_l: false,
        numbers_ok: true,
        h_names: 0,
    };
    let mut seen = BTreeSet::new();
    for tag in value.split(|b| *b == b';') {
        let Some(eq) = tag.iter().position(|b| *b == b'=') else {
            continue;
        };
        // The crate ignores whitespace inside a tag name and folds its case.
        let name: Vec<u8> = tag[..eq]
            .iter()
            .filter(|b| !b.is_ascii_whitespace())
            .map(u8::to_ascii_lowercase)
            .collect();
        scan.duplicate |= !name.is_empty() && !seen.insert(name.clone());
        let tag_value = &tag[eq + 1..];
        match name.as_slice() {
            b"v" => scan.has_v = true,
            b"a" => scan.has_a = true,
            b"t" => scan.has_t = true,
            b"x" => scan.has_x = true,
            b"l" => scan.has_l = true,
            b"h" => scan.h_names += tag_value.iter().filter(|b| **b == b':').count() + 1,
            _ => {}
        }
        if matches!(name.as_slice(), b"t" | b"x" | b"l") {
            scan.numbers_ok &= number_fits(tag_value);
        }
    }
    scan
}

fn is_dkim_signature(name: &[u8]) -> bool {
    let folded: Vec<u8> = name
        .iter()
        .filter(|b| !b.is_ascii_whitespace())
        .map(u8::to_ascii_lowercase)
        .collect();
    folded == b"dkim-signature"
}

/// One `DKIM-Signature` field as the crate sees it, in header order.
struct Field<'m> {
    value: &'m [u8],
    parsed: Result<&'m Signature, &'m Error>,
    scan: Scan,
}

/// The crate's own view of the fields, so that its outputs and ours line up by
/// construction: parsed signatures and rejected fields, sorted by position.
fn fields<'m>(message: &'m AuthenticatedMessage<'m>) -> Vec<Field<'m>> {
    let mut fields: Vec<Field<'m>> = message
        .dkim_signatures()
        .iter()
        .map(|header| Field {
            value: header.value,
            parsed: Ok(&header.header),
            scan: scan(header.value),
        })
        .chain(
            message
                .errors()
                .iter()
                .filter(|header| is_dkim_signature(header.name))
                .map(|header| Field {
                    value: header.value,
                    parsed: Err(&header.header),
                    scan: scan(header.value),
                }),
        )
        .collect();
    fields.sort_by_key(|field| field.value.as_ptr() as usize);
    fields
}

/// What `verdict` reads besides the field: the crate's outputs, the message body,
/// and the receipt second.
struct Context<'a, 'm> {
    outputs: &'a [DkimOutput<'m>],
    body: &'a [u8],
    receipt: u64,
}

/// The verdict for one field, or `None` for a signature that verified cleanly.
/// The first matching row of the specification's evaluation table wins.
fn verdict(field: &Field<'_>, context: &Context<'_, '_>) -> Option<Rule> {
    let scan = &field.scan;
    // Row a: what the crate would default or read as absent. A repeated tag, no
    // `v=` (version 0) or no `a=` (rsa-sha256) break RFC 6376 sections 3.2 and
    // 3.5; a `t=`, `x=` or `l=` that is no number reads as 0, "absent".
    if scan.duplicate || !scan.has_v || !scan.has_a || !scan.numbers_ok {
        return Some(Rule::Malformed);
    }
    let signature = match field.parsed {
        Err(Error::Dkim(
            DkimError::UnsupportedVersion
            | DkimError::UnsupportedAlgorithm
            | DkimError::UnsupportedCanonicalization,
        )) => return Some(Rule::Unsupported),
        Err(_) => return Some(Rule::Malformed),
        Ok(signature) => signature,
    };
    if signature.a == Algorithm::RsaSha1 || (scan.has_l && signature.l == 0) {
        return Some(Rule::Unsupported);
    }
    if !signature
        .h
        .iter()
        .any(|name| name.eq_ignore_ascii_case("from"))
    {
        return Some(Rule::FromUnsigned);
    }
    if scan.has_x && scan.has_t && signature.x <= signature.t {
        return Some(Rule::Malformed);
    }
    if scan.has_x && signature.x <= context.receipt {
        return Some(Rule::SignatureExpired);
    }
    // The output is the one that holds this very signature, never one picked by
    // position: the crate lists an output for rejected fields too.
    let output = context
        .outputs
        .iter()
        .find(|output| {
            output
                .signature()
                .is_some_and(|found| std::ptr::eq(found, signature))
        })
        .expect("every parsed signature has one output");
    match output.result() {
        DkimResult::Pass => body_length(signature, context.body),
        DkimResult::Neutral(Error::Dkim(DkimError::SignatureExpired)) => {
            Some(Rule::ExpiredSinceReceipt)
        }
        DkimResult::TempError(_) => Some(Rule::NotEvaluated),
        DkimResult::PermError(_) if output.is_atps() => Some(Rule::AtpsUnauthorized),
        // The name the crate asked for, trailing dot included, is the name judged.
        DkimResult::PermError(Error::Dns(DnsError::RecordNotFound(_))) => {
            Some(if owner_of(&signature.domain_key()).is_none() {
                Rule::SelectorInvalid
            } else {
                Rule::KeyMissing
            })
        }
        DkimResult::PermError(Error::Dkim(DkimError::PublicKeyRevoked)) => Some(Rule::KeyRevoked),
        DkimResult::PermError(_) => Some(Rule::KeyInvalid),
        DkimResult::Fail(Error::Dkim(DkimError::AuidMismatch)) => Some(Rule::AuidMismatch),
        DkimResult::Fail(Error::Crypto(CryptoError::IncompatibleAlgorithms)) => {
            Some(Rule::KeyInvalid)
        }
        // The crate does not check key bytes before use, and the ed25519 backend
        // words a failed check as a library error, not `FailedVerification`: any
        // other failure is a signature that does not verify against the key.
        DkimResult::Fail(_) => Some(Rule::SignatureMismatch),
        // The crate returns no other neutral for a parsed, unexpired signature
        // and never `None`; both are read as the body hash not matching.
        DkimResult::Neutral(_) | DkimResult::None => Some(Rule::BodyHashMismatch),
    }
}

/// A signature the crate verified: judge how much of the body its `l=` covers.
/// `l=` counts raw body bytes in the crate (its signer and its verifier agree),
/// and the hash covers `min(l, body)`. A longer `l=` than the body is crafted;
/// a shorter one that leaves canonical content out is RFC 6376 section 8.2.
fn body_length(signature: &Signature, body: &[u8]) -> Option<Rule> {
    if signature.l == 0 {
        return None;
    }
    let total = canonical_len(body, signature.cb, 0);
    if signature.l > (body.len() as u64).max(total) {
        Some(Rule::BodyLengthExceeds)
    } else if canonical_len(body, signature.cb, signature.l) < total {
        Some(Rule::BodyLengthPartial)
    } else {
        None
    }
}

/// Verify every `DKIM-Signature` of `raw` against the declared map. `root` and
/// `size` are the root header list and body offset every other collector uses.
pub(super) fn collect(
    raw: &[u8],
    root: &Headers,
    size: usize,
    receipt: u64,
    lookups: &mut Lookups,
    out: &mut Collector,
) -> Result<(), AnalysisError> {
    // The strict parse bounds the work first: it hashes the body once per
    // distinct canonicalization of an `l=`-less signature (at most four) and
    // rejects every `l=` signature unhashed. The fields it lists are the crate's
    // own, so one that a header scanner of ours would not recognise (a name folded
    // across lines, say) cannot slip past the bound.
    let parsed = AuthenticatedMessage::parse_with_opts(raw, None, true);
    let first = parsed.as_ref().map_or_else(Vec::new, fields);
    // Unsigned mail is none of DKIM's business, whatever its header block holds.
    if first.is_empty() && !root.iter().any(|(name, _)| is_dkim_signature(name)) {
        return Ok(());
    }
    // One body and one field list: the crate must end the headers where Swarmail
    // does, or its body is not the one every other collector and the UI see (a
    // colon-less line ends the crate's headers and not Swarmail's, or the other
    // way round), and it must list as many DKIM-Signature fields as the root list
    // does, or the occurrence of a finding is not the root's (a name with a space
    // before the colon is a signature to the crate and no field to Swarmail). The
    // boundary finding itself sits at root occurrence 0, or at `null` when the root
    // list holds no DKIM-Signature field to point at.
    let root_count = root
        .iter()
        .filter(|(name, _)| is_dkim_signature(name))
        .count();
    let Some(strict) = parsed
        .as_ref()
        .filter(|message| message.body_offset() == size && first.len() == root_count)
    else {
        return out.push(Rule::HeaderBoundaryAmbiguous.located((root_count > 0).then_some(0)));
    };
    let by_count = (first.len() > MAX_DKIM_SIGNATURES).then_some(MAX_DKIM_SIGNATURES);
    let by_names = first
        .iter()
        .position(|f| f.scan.h_names > MAX_DKIM_SIGNED_HEADERS);
    if let Some(over) = [by_count, by_names].into_iter().flatten().min() {
        return out.push(Rule::LimitsExceeded.finding(over));
    }
    // A signature with `l=` is judged, not ignored: parse again without strict.
    let full;
    let message = if first.iter().any(|f| f.scan.has_l) {
        full = AuthenticatedMessage::parse_with_opts(raw, None, false)
            .expect("a message that parsed strictly parses without strict");
        &full
    } else {
        strict
    };
    let adapter = Adapter {
        lookups: RefCell::new(lookups),
    };
    let outputs = AUTHENTICATOR
        .verify_dkim(Parameters::new(message).with_cache(&adapter))
        .now_or_never()
        .expect("a declared map answers on the first poll");
    let context = Context {
        outputs: &outputs,
        body: message.raw_body(),
        receipt,
    };
    for (occurrence, field) in fields(message).iter().enumerate() {
        if let Some(rule) = verdict(field, &context) {
            out.push(rule.finding(occurrence))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::dns::{self, DnsRecord, Effective, Outcome, Source};
    use super::super::{Inputs, Report, evaluate_at};
    use super::*;
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD as B64;
    use mail_auth::crypto::{Ed25519Key, RsaKey, Sha256, SigningKey};
    use mail_auth::dkim::DkimSigner;
    use mail_auth::headers::HeaderWriter;
    use rustls::pki_types::{PrivateKeyDer, PrivatePkcs1KeyDer};
    use std::sync::Arc;
    use std::time::Duration;

    // Throwaway test-only keys, committed on purpose: no key is generated at run
    // time and none ever signs anything outside these tests. The RSA key is
    // PKCS#1 private DER with its SPKI public half; the ed25519 key is PKCS#8
    // with its raw public key (RFC 8463 section 3.3). aws-lc will not sign with a
    // 1,024-bit RSA key (it verifies them), so that size is proved by the
    // published RFC 8463 vector instead.
    const RSA2048_PRIVATE: &str = "MIIEpAIBAAKCAQEA0Cd1o2mJkXDFNQ9SdX9KtAWW71salDqwwb5QF6kRzuGkCSADrGCsFuKD7AWi4EhkAtsVoL0KEEVTAVW6Bu0uzYkLoQFVdjWdBGS3ZMotq2R1H6QkLMZFRQminfxX4huvJ+QDCxbI7BH3b15dmGJ250Q7fOPnJyWmxhaqQcm5+8A+GAMC4+u3Th8KuZUhwY9tiFSqtb4XosqvNMffZdzC1/Y0aZr8wpyvLHwp94++MrvDlqLfDhbbeg+cZvrTNUqaY7PiGm3CqjJLOB5tTQ8y7jCqXvcoquq/78MSGBhvjVhVjmZnME0f6vJfh9jgI1JdAF0fD5vNU5JfpJRZgEjFxQIDAQABAoIBAAPrycweWZNPMMPSpZouqOugAObUlWbpokJm6C9nDs/tY7QxTQPXlMdyS4uHZhRy3BO4aLjs0PwbaHz714g4XsknUUVMMocKfZNjbrxAfgUaChhZdKu1ZJNL40aRYZVtaJO6Zii0o1nKN9oe9mmbIF9YY3LqJNcIjNvOwv4SemnNRsBRJn7xpgOkpKTDi8bxL+WkHICWfYaR6WNNggH2ZEOeBUcclSRvMcMBANZ1dqbpRznL/nEE/5mjPqhW1KgK4wzrX1aE8VEy6M2aFI7UMUEPUd63ZxJVfwQW7WvdZ/y5XnIPjGhBV3bdj8POThcek515Iu0RMhKgAJJJI+pJfbkCgYEA50jblWAnMk5QpbkeXM5QL0y9GpsemCl+75NkLSQGvVKElVKxJ6rILJZ0E4bID76sJ0sIfMSYg4x9uxr7aUYOoEsu2KEn59ntCO1aF50awcaAYHUcqh0L0lvUPo9S+AJ8kT5ih5t/TjltF8QSckuLXuqC6hc7RYNDpQ3FOOes5t0CgYEA5mXVMcJVGzgtTvV6i5Yj1mpWHeaPoDIG61y+VQP7BNhxKW04n342r8Px8nw9uxlYfkdG4n7QTzhVw6uvOTrcoPnpiJx1mts+WUzEuyrGO/FwmQBGHdQSeYJv5OogzhkNViCS+hkcWarZlZlD+SvK1uIKUl+3Uipc1VqEpDi4yAkCgYEAn75Js61axTskJqJqXy08Yr786bBz/CLFApHpoef9I6wn2L7pjv09DtbHy2jPK1jFrMn9Tbe3ozxY5u9m8DosBfiHnaYJWvDA7+aH0EF5ZpdEudqJdtFjBeLZAHCbnKKTCPpusTzsO8ocGEsbdofI9kDBNQ/gp01XXlvZeUlCuZ0CgYB4P7v23YOH0lkrdcnA4trPhO5vaq+Zn6Km4t/gBVTVIZZmVcAQ1lH6iJ1VvuyFI2RhfBgODLvyLOfuWUP57bPLPMRbbbWf3j/7YaroORn/nTL0WAVAejguPRrjciMvDjo8soGjkMmQLzCi+LxWZznVz2ItvhEzDDMpk7XY9FXU6QKBgQC7MQbuTqgCrYLxGxA9VYDM+sJjQpq5eT0EcivvRnqlmLcGEAAMtXMXAPVmP2XnA/vKSIl/SLItE1FkpDXdc0kPnlok3tzCJz9+zfa0tuDxMmHDXBYcl8Z3jLszfQuqdOuD4HOr8WQ8Byklo8Oc+AwqqaeVBxAdKi1EXWzVj2+rTw==";
    const RSA2048_PUBLIC: &str = "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA0Cd1o2mJkXDFNQ9SdX9KtAWW71salDqwwb5QF6kRzuGkCSADrGCsFuKD7AWi4EhkAtsVoL0KEEVTAVW6Bu0uzYkLoQFVdjWdBGS3ZMotq2R1H6QkLMZFRQminfxX4huvJ+QDCxbI7BH3b15dmGJ250Q7fOPnJyWmxhaqQcm5+8A+GAMC4+u3Th8KuZUhwY9tiFSqtb4XosqvNMffZdzC1/Y0aZr8wpyvLHwp94++MrvDlqLfDhbbeg+cZvrTNUqaY7PiGm3CqjJLOB5tTQ8y7jCqXvcoquq/78MSGBhvjVhVjmZnME0f6vJfh9jgI1JdAF0fD5vNU5JfpJRZgEjFxQIDAQAB";
    const ED25519_PRIVATE: &str =
        "MC4CAQAwBQYDK2VwBCIEIAtvV61x8r6mErlreyiGrmEK48yeio3jk3mzi+LgprfU";
    const ED25519_PUBLIC: &str = "UodvhlES5fJESplf9URbrh0U2TXyG7w5hvtVVBMrQRg=";
    // The two public keys RFC 8463 appendix A publishes (no private half is held
    // here): a different key of each type, for the wrong-key proofs.
    const RFC8463_ED25519_PUBLIC: &str = "11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=";
    const RFC8463_RSA1024_PUBLIC: &str = "MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDkHlOQoBTzWRiGs5V6NpP3idY6Wk08a5qhdR6wy5bdOKb2jLQiY/J16JYi0Qvx/byYzCNb3W91y3FutACDfzwQ/BC/e/8uBsCR+yz1Lxj+PL6lHvqMKrM3rG4hstT5QjvHO9PzoxZyVYLzBfO2EeC3Ip3G+2kryOTIKT+l/K4w3QIDAQAB";

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Kind {
        Rsa2048,
        Ed25519,
    }

    impl Kind {
        const ALL: [Kind; 2] = [Kind::Rsa2048, Kind::Ed25519];

        fn record(self) -> String {
            match self {
                Kind::Rsa2048 => format!("v=DKIM1; k=rsa; p={RSA2048_PUBLIC}"),
                Kind::Ed25519 => format!("v=DKIM1; k=ed25519; p={ED25519_PUBLIC}"),
            }
        }

        fn algorithm(self) -> &'static str {
            match self {
                Kind::Ed25519 => "ed25519-sha256",
                _ => "rsa-sha256",
            }
        }
    }

    fn rsa(private: &str) -> RsaKey<Sha256> {
        let der = B64.decode(private).unwrap();
        RsaKey::from_key_der(PrivateKeyDer::Pkcs1(PrivatePkcs1KeyDer::from(der))).unwrap()
    }

    fn ed25519() -> Ed25519Key {
        Ed25519Key::from_pkcs8_maybe_unchecked_der(&B64.decode(ED25519_PRIVATE).unwrap()).unwrap()
    }

    /// What the signer writes: everything the crate's `DkimSigner` can set.
    #[derive(Clone)]
    struct Sign {
        kind: Kind,
        domain: &'static str,
        selector: &'static str,
        ch: Canonicalization,
        cb: Canonicalization,
        headers: Vec<String>,
        identity: Option<&'static str>,
        expiration: Option<u64>,
        body_length: bool,
        atps: Option<&'static str>,
        reporting: bool,
    }

    impl Default for Sign {
        fn default() -> Self {
            Self {
                kind: Kind::Ed25519,
                domain: "example.com",
                selector: "sel",
                ch: Canonicalization::Relaxed,
                cb: Canonicalization::Relaxed,
                headers: vec!["from".into(), "subject".into()],
                identity: None,
                expiration: None,
                body_length: false,
                atps: None,
                reporting: false,
            }
        }
    }

    fn sign_with<K: SigningKey>(key: K, o: &Sign, message: &[u8]) -> Signature {
        let mut signer = DkimSigner::from_key(key)
            .domain(o.domain)
            .selector(o.selector)
            .headers(o.headers.iter().cloned())
            .header_canonicalization(o.ch)
            .body_canonicalization(o.cb)
            .body_length(o.body_length)
            .reporting(o.reporting);
        if let Some(identity) = o.identity {
            signer = signer.identity(identity);
        }
        if let Some(seconds) = o.expiration {
            signer = signer.expiration(Duration::from_secs(seconds));
        }
        if let Some(atps) = o.atps {
            signer = signer.atps(atps);
        }
        signer.sign(message).unwrap()
    }

    fn signature(o: &Sign, message: &[u8]) -> Signature {
        match o.kind {
            Kind::Rsa2048 => sign_with(rsa(RSA2048_PRIVATE), o, message),
            Kind::Ed25519 => sign_with(ed25519(), o, message),
        }
    }

    fn crlf(text: &str) -> Vec<u8> {
        text.replace('\n', "\r\n").into_bytes()
    }

    const BODY: &str = "Hello Bob,\nthis  is a test.\n";

    fn message(body: &str) -> Vec<u8> {
        crlf(&format!(
            "From: Alice <alice@example.com>\nTo: bob@example.org\nSubject: Hello\nDate: Mon, 02 Nov 2026 10:00:00 +0000\nMessage-ID: <1@example.com>\n\n{body}"
        ))
    }

    /// The signature header followed by the message.
    fn signed(o: &Sign, message: &[u8]) -> Vec<u8> {
        let mut raw = signature(o, message).to_header().into_bytes();
        raw.extend_from_slice(message);
        raw
    }

    /// Sign the message, then splice `edit` over the header text: the message
    /// stays as signed, the header does not.
    fn tampered(o: &Sign, message: &[u8], edit: impl FnOnce(String) -> String) -> Vec<u8> {
        let header = signature(o, message).to_header();
        let mut raw = edit(header).into_bytes();
        raw.extend_from_slice(message);
        raw
    }

    /// A signature with chosen tags (`t=`, `x=`, ...) in front of `message`,
    /// validly signed: the tag list is written by hand with a placeholder `b=`,
    /// the crate canonicalizes it, and the same key signs those bytes. When the
    /// crate declines to canonicalize it (its `x=` is not after `t=`), the
    /// placeholder stays: such a field never reaches the key.
    fn forged(o: &Sign, tags: &str, message: &[u8]) -> Vec<u8> {
        forged_with(o, tags, true, message)
    }

    /// `forged`, optionally without the `a=` tag (RFC 6376 section 3.5 requires it).
    fn forged_with(o: &Sign, tags: &str, algorithm: bool, message: &[u8]) -> Vec<u8> {
        let template = signature(o, message);
        let names: Vec<String> = template.h.clone();
        let c = |c: Canonicalization| match c {
            Canonicalization::Relaxed => "relaxed",
            Canonicalization::Simple => "simple",
        };
        let head = format!(
            "DKIM-Signature: v=1; {}c={}/{}; d={}; s={}; {tags}; h={}; bh={}; b=",
            if algorithm {
                format!("a={}; ", o.kind.algorithm())
            } else {
                String::new()
            },
            c(o.ch),
            c(o.cb),
            o.domain,
            o.selector,
            names.join(":"),
            B64.encode(&template.bh),
        );
        let mut raw = format!("{head}AAAA\r\n").into_bytes();
        raw.extend_from_slice(message);
        let canonical = AuthenticatedMessage::parse_with_opts(&raw, None, false)
            .unwrap()
            .canonicalized_dkim_headers();
        let Some(canonical) = canonical else {
            return raw;
        };
        let b = match o.kind {
            Kind::Rsa2048 => rsa(RSA2048_PRIVATE).sign(canonical.as_slice()),
            Kind::Ed25519 => ed25519().sign(canonical.as_slice()),
        }
        .unwrap();
        let mut out = format!("{head}{}\r\n", B64.encode(b)).into_bytes();
        out.extend_from_slice(message);
        out
    }

    /// A frozen policy: one record per `(name, value)`, and `tempfail` names.
    fn policy(records: &[(&str, String)], tempfail: &[&str]) -> Arc<Effective> {
        let txt: Vec<_> = records
            .iter()
            .map(|(name, value)| serde_json::json!({"name": name, "values": [value]}))
            .collect();
        let declaration = serde_json::json!({"txt": txt, "tempfail": tempfail});
        let policy = dns::parse(declaration.to_string().as_bytes()).unwrap();
        Effective::new(Source::Inbox, policy)
    }

    fn key_of(kind: Kind) -> Arc<Effective> {
        policy(&[("sel._domainkey.example.com", kind.record())], &[])
    }

    fn report(raw: &[u8], receipt: u64, world: &Arc<Effective>) -> Report {
        let inputs = Inputs {
            dns: world.clone(),
            ..Inputs::default()
        };
        evaluate_at(raw, receipt, &inputs).unwrap()
    }

    /// The DKIM findings only: `(rule id, occurrence, severity)`, by occurrence.
    fn dkim(report: &Report) -> Vec<(String, u64, Severity)> {
        let mut found: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.rule_id.starts_with("dkim."))
            .map(|f| {
                let location = serde_json::to_value(&f.location).unwrap();
                assert_eq!(location["kind"], "header");
                assert_eq!(location["name"], "dkim-signature");
                (
                    f.rule_id.clone(),
                    location["occurrence"].as_u64().unwrap(),
                    f.severity,
                )
            })
            .collect();
        found.sort_by(|a, b| (a.1, &a.0).cmp(&(b.1, &b.0)));
        found
    }

    /// The report with only its DKIM findings, for judging them in isolation.
    fn only_dkim(mut report: Report) -> Report {
        report.findings.retain(|f| f.rule_id.starts_with("dkim."));
        report
    }

    fn ids(report: &Report) -> Vec<String> {
        dkim(report).into_iter().map(|(id, ..)| id).collect()
    }

    fn lookups(record: &DnsRecord) -> Vec<(String, Outcome)> {
        record
            .lookups
            .iter()
            .map(|l| (l.name.clone(), l.outcome))
            .collect()
    }

    #[test]
    fn kit_signs_verifiably() {
        let message = message(BODY);
        for kind in Kind::ALL {
            let o = Sign {
                kind,
                ..Sign::default()
            };
            let raw = signed(&o, &message);
            let report = report(&raw, 0, &key_of(kind));
            assert_eq!(dkim(&report), vec![], "{kind:?}");
            assert_eq!(
                lookups(&report.dns),
                vec![("sel._domainkey.example.com".into(), Outcome::Records)]
            );
        }
        // The same bytes with no key declared are a named finding, which proves
        // the verifier read the world and not the signature alone.
        let raw = signed(&Sign::default(), &message);
        let empty = policy(&[], &[]);
        assert_eq!(ids(&report(&raw, 0, &empty)), ["dkim.key_missing.v1"]);
    }

    fn declared_lookups(world: &Arc<Effective>) -> Lookups {
        Lookups::new(world.clone())
    }

    fn ask(adapter: &Adapter<'_>, name: &str) -> TxtRecord {
        ResolverCache::<Box<str>, TxtRecord>::get::<str>(adapter, name).unwrap()
    }

    #[test]
    fn adapter_answers_without_resolver() {
        let atps = "example.com._atps.signer.example";
        let world = policy(
            &[
                ("sel._domainkey.example.com", Kind::Ed25519.record()),
                ("garbage._domainkey.example.com", "v=DKIM1; p=!!!!".into()),
                ("revoked._domainkey.example.com", "v=DKIM1; p=".into()),
                (
                    "_report._domainkey.example.com",
                    "v=DKIM1; ra=postmaster".into(),
                ),
                ("_report._domainkey.broken.example", "nonsense".into()),
                (atps, "v=ATPS1; d=example.com".into()),
                ("odd._atps.signer.example", "nonsense".into()),
                ("not-a-key.example.com", "v=spf1 -all".into()),
            ],
            &["down._domainkey.example.com"],
        );
        let mut lookups = declared_lookups(&world);
        let adapter = Adapter {
            lookups: RefCell::new(&mut lookups),
        };
        // Every shape of name gets the record type its shape selects.
        assert!(matches!(
            ask(&adapter, "sel._domainkey.example.com."),
            TxtRecord::DomainKey(_)
        ));
        assert!(matches!(
            ask(&adapter, "SEL._DomainKey.Example.COM."),
            TxtRecord::DomainKey(_)
        ));
        // A report record is never built, even for a `_report` name: a report-shaped
        // string there is a key record without a key, and a nonsense one is no key
        // either (measured: `DomainKey::parse_records` gives `InvalidRecordType` for
        // both `v=DKIM1; ra=postmaster` and `nonsense`).
        for name in [
            "_report._domainkey.example.com.",
            "_report._domainkey.broken.example.",
        ] {
            assert!(
                matches!(
                    ask(&adapter, name),
                    TxtRecord::Error(Error::Dns(DnsError::InvalidRecordType))
                ),
                "{name}"
            );
        }
        assert!(matches!(
            ask(&adapter, &format!("{atps}.")),
            TxtRecord::Atps(_)
        ));
        assert!(matches!(
            ask(&adapter, "odd._atps.signer.example."),
            TxtRecord::Error(Error::Dns(DnsError::InvalidRecordType))
        ));
        // A name with neither shape reads as a key record: an SPF string is no key.
        assert!(matches!(
            ask(&adapter, "not-a-key.example.com."),
            TxtRecord::Error(Error::Dns(DnsError::InvalidRecordType))
        ));
        assert!(matches!(
            ask(&adapter, "garbage._domainkey.example.com."),
            TxtRecord::Error(_)
        ));
        assert!(matches!(
            ask(&adapter, "revoked._domainkey.example.com."),
            TxtRecord::Error(Error::Dkim(DkimError::PublicKeyRevoked))
        ));
        // A definite negative, a declared failure, and a name outside the grammar.
        assert!(matches!(
            ask(&adapter, "undeclared._domainkey.example.com."),
            TxtRecord::Error(Error::Dns(DnsError::RecordNotFound(_)))
        ));
        assert!(matches!(
            ask(&adapter, "down._domainkey.example.com."),
            TxtRecord::Error(Error::Dns(DnsError::Resolver(_)))
        ));
        assert!(matches!(
            ask(&adapter, "bad*sel._domainkey.example.com."),
            TxtRecord::Error(Error::Dns(DnsError::RecordNotFound(_)))
        ));
        assert!(matches!(
            ask(&adapter, ""),
            TxtRecord::Error(Error::Dns(DnsError::RecordNotFound(_)))
        ));
        let names: Vec<_> = lookups
            .finish()
            .lookups
            .into_iter()
            .map(|l| (l.name, l.outcome))
            .collect();
        // The invalid names are asked of nobody and recorded nowhere.
        assert!(names.iter().all(|(name, _)| !name.contains('*')));
        assert!(names.iter().all(|(name, _)| !name.is_empty()));
        assert_eq!(names.len(), 10);
        assert!(names.contains(&("down._domainkey.example.com".into(), Outcome::TempFail)));
        assert!(names.contains(&(
            "undeclared._domainkey.example.com".into(),
            Outcome::NoRecords
        )));
    }

    #[test]
    fn adapter_surface_and_name_recovery() {
        let world = policy(&[], &[]);
        let mut lookups = declared_lookups(&world);
        let adapter = Adapter {
            lookups: RefCell::new(&mut lookups),
        };
        // The crate never inserts, removes or asks another cache; each is inert.
        ResolverCache::<Box<str>, TxtRecord>::insert(
            &adapter,
            "a.example.".into(),
            not_found(),
            Instant::now(),
        );
        assert!(
            ResolverCache::<Box<str>, TxtRecord>::remove::<str>(&adapter, "a.example.").is_none()
        );
        assert!(adapter.txt().is_some());
        assert!(adapter.mx().is_none());
        assert!(adapter.ipv4().is_none());
        assert!(adapter.ipv6().is_none());
        assert!(adapter.ptr().is_none());
        assert!(lookups.finish().lookups.is_empty());
        // A `str` key is read back; any other shape, or bytes that are no text,
        // is no name at all.
        assert_eq!(
            name_of("sel._domainkey.example.com."),
            Some("sel._domainkey.example.com.".into())
        );
        assert_eq!(name_of(""), Some(String::new()));
        assert_eq!(name_of(&7u32), None);
        // The recorder is only ever written to; its digest is unused and fixed.
        assert_eq!(NameRecorder(Vec::new()).finish(), 0);
        assert_eq!(name_of([0xc0u8, 0xff].as_slice()), None);
    }

    #[test]
    fn prescan_table() {
        let scan = |value: &str| scan(value.as_bytes());
        let clean = Scan {
            duplicate: false,
            has_v: true,
            has_a: false,
            has_t: false,
            has_x: false,
            has_l: false,
            numbers_ok: true,
            h_names: 2,
        };
        assert_eq!(scan(" v=1; d=a.example; h=from:to; b=AA"), clean);
        assert_eq!(
            scan("v=1;h=from"),
            Scan {
                h_names: 1,
                ..clean
            }
        );
        assert!(!scan("d=a; h=from").has_v);
        assert!(scan(" V =1").has_v);
        assert!(!scan("vv=1; b=v=1").has_v);
        // A duplicate is any repeated name, whatever the case or the folding.
        assert!(scan("d=a; s=b; d=c").duplicate);
        assert!(scan("d=a;\r\n D=c").duplicate);
        assert!(scan("d=a; d\r\n =c").duplicate);
        assert!(!scan("d=a; s=b; dd=c").duplicate);
        // Empty names and fragments with no `=` are not tags.
        assert!(!scan("=1; =2; ;; stray; d=a").duplicate);
        assert!(!scan("").duplicate);
        // `l=` is seen however it is spelled; its value is the crate's to read.
        for value in [
            "l=0",
            "l=00",
            " l = 5",
            "L=5",
            "d=a;\r\n l=\r\n 7",
            "l=abc",
            "l=",
        ] {
            assert!(scan(value).has_l, "{value:?}");
        }
        assert!(!scan("ll=5; b=l=5; d=a").has_l);
        // `a=`, `t=` and `x=` are seen however they are spelled, and only as names.
        for (value, a, t, x) in [
            ("a=rsa-sha256", true, false, false),
            (" A = ed25519-sha256", true, false, false),
            ("t=1; x=2", false, true, true),
            ("T=1;\r\n X =2", false, true, true),
            ("aa=1; tt=2; xx=3; b=a=t=x=", false, false, false),
        ] {
            let found = scan(value);
            assert_eq!(
                (found.has_a, found.has_t, found.has_x),
                (a, t, x),
                "{value:?}"
            );
        }
        // A number is one or more digits that fit `u64`, trimmed of folding
        // whitespace; the crate would read anything else as 0, which means absent.
        for good in [
            "t=0",
            "t=1000",
            "x= 2000 ",
            "x=\r\n 2000\r\n ",
            "l=18446744073709551615",
            "t=1; x=2; l=3",
            "d=a; b=abc",
        ] {
            assert!(scan(good).numbers_ok, "{good:?}");
        }
        for bad in [
            "t=",
            "t= ",
            "t=abc",
            "x=12a",
            "x=-1",
            "x=1 2",
            "l=+5",
            "l=0x10",
            "l=1.5",
            "x=18446744073709551616",
            "x=99999999999999999999",
            "t=1; x=abc",
            "t=\u{661}",
        ] {
            assert!(!scan(bad).numbers_ok, "{bad:?}");
        }
        // The `h=` count is an upper bound: colons plus one, summed over tags.
        assert_eq!(scan("h=").h_names, 1);
        assert_eq!(scan("h=a:b:c:d").h_names, 4);
        assert_eq!(scan("h=a:b; h=c:d:e").h_names, 5);
        assert_eq!(scan("H = a : b \r\n : c").h_names, 3);
        assert_eq!(scan("d=a:b:c").h_names, 0);
    }

    fn hash_of(bytes: &[u8]) -> Vec<u8> {
        HashAlgorithmForTest::Sha256.hash(bytes).as_ref().to_vec()
    }

    use mail_auth::crypto::HashAlgorithm as HashAlgorithmForTest;

    fn canonical(body: &[u8], c: Canonicalization) -> Vec<u8> {
        let mut out = Vec::new();
        canonicalize(body, c, &mut out);
        out
    }

    #[test]
    fn canonical_bodies_follow_the_rfc_examples() {
        let body = b" C \r\nD \t E\r\n\r\n\r\n";
        assert_eq!(
            canonical(body, Canonicalization::Simple),
            b" C \r\nD \t E\r\n"
        );
        assert_eq!(canonical(body, Canonicalization::Relaxed), b" C\r\nD E\r\n");
        // An empty body is one CRLF under simple and nothing under relaxed.
        assert_eq!(canonical(b"", Canonicalization::Simple), b"\r\n");
        assert_eq!(canonical(b"", Canonicalization::Relaxed), b"");
        // No final CRLF gets one; a bare LF is a line break; a lone CR is dropped.
        assert_eq!(canonical(b"abc", Canonicalization::Simple), b"abc\r\n");
        assert_eq!(
            canonical(b"a\nb\n", Canonicalization::Simple),
            b"a\r\nb\r\n"
        );
        assert_eq!(canonical(b"a\rb", Canonicalization::Simple), b"ab\r\n");
        assert_eq!(
            canonical(b"\r\n\r\nabc", Canonicalization::Simple),
            b"\r\n\r\nabc\r\n"
        );
        assert_eq!(
            canonical(b"a \t b  \n", Canonicalization::Relaxed),
            b"a b\r\n"
        );
        assert_eq!(canonical(b"\n\n", Canonicalization::Relaxed), b"");
        for c in [Canonicalization::Simple, Canonicalization::Relaxed] {
            let body = b"one\r\ntwo three\r\n\r\n";
            assert_eq!(canonical_len(body, c, 0), canonical(body, c).len() as u64);
            // `l=` is a raw prefix: the first 5 bytes are "one\r\n".
            assert_eq!(
                canonical_len(body, c, 5),
                canonical(&body[..5], c).len() as u64
            );
            // A limit past the end is the whole body.
            assert_eq!(canonical_len(body, c, 10_000), canonical_len(body, c, 0));
            assert_eq!(canonical_len(body, c, u64::MAX), canonical_len(body, c, 0));
        }
    }

    #[test]
    fn canonicalizer_equals_the_crate_on_every_short_body() {
        // Every body of up to six bytes over the bytes that canonicalization
        // treats specially, plus text and a control byte: 55,987 bodies, both modes.
        let alphabet = [b'a', b' ', b'\t', b'\r', b'\n', 0x01];
        let mut bodies: Vec<Vec<u8>> = vec![Vec::new()];
        let mut start = 0;
        for _ in 0..6 {
            let end = bodies.len();
            for i in start..end {
                for byte in alphabet {
                    let mut body = bodies[i].clone();
                    body.push(byte);
                    bodies.push(body);
                }
            }
            start = end;
        }
        assert_eq!(bodies.len(), 55_987);
        let o = |cb| Sign {
            headers: vec!["from".into()],
            cb,
            ..Sign::default()
        };
        let signers = [Canonicalization::Simple, Canonicalization::Relaxed].map(|cb| {
            DkimSigner::from_key(ed25519())
                .domain("example.com")
                .selector("sel")
                .headers(o(cb).headers)
                .header_canonicalization(Canonicalization::Relaxed)
                .body_canonicalization(cb)
        });
        for body in &bodies {
            let mut message = b"From: a@example.com\r\n\r\n".to_vec();
            message.extend_from_slice(body);
            for (c, signer) in [Canonicalization::Simple, Canonicalization::Relaxed]
                .into_iter()
                .zip(&signers)
            {
                assert_eq!(
                    hash_of(&canonical(body, c)),
                    signer.sign(&message).unwrap().bh,
                    "{c:?} {body:?}"
                );
            }
        }
    }

    const ERROR: Severity = Severity::Error;

    fn one(id: &str, occurrence: u64, severity: Severity) -> Vec<(String, u64, Severity)> {
        vec![(id.to_string(), occurrence, severity)]
    }

    fn verdicts(raw: &[u8], receipt: u64, world: &Arc<Effective>) -> Vec<(String, u64, Severity)> {
        dkim(&report(raw, receipt, world))
    }

    fn msg() -> Vec<u8> {
        message(BODY)
    }

    fn base() -> Vec<u8> {
        signed(&Sign::default(), &msg())
    }

    #[test]
    fn unsigned_mail_has_no_verdict_and_no_lookup() {
        let world = key_of(Kind::Ed25519);
        for raw in [msg(), b"\r\nno headers at all\r\n".to_vec()] {
            let report = report(&raw, 0, &world);
            assert_eq!(dkim(&report), vec![]);
            assert!(report.dns.lookups.is_empty());
        }
        // The crate refuses a DKIM-Signature header it does not read as one.
        let mut raw = b"X-DKIM-Signature: v=1; d=a; s=b\r\n".to_vec();
        raw.extend_from_slice(&msg());
        assert_eq!(verdicts(&raw, 0, &world), vec![]);
    }

    #[test]
    fn rule_malformed() {
        let world = key_of(Kind::Ed25519);
        let o = Sign::default();
        // Passing fixture: a well-formed signature.
        assert_eq!(verdicts(&base(), 0, &world), vec![]);
        // A required tag missing, bad base64, a repeated tag, and `x=` at `t=`.
        let missing = tampered(&o, &msg(), |h| h.replacen("bh=", "xh=", 1));
        let base64 = tampered(&o, &msg(), |h| h.replacen("bh=", "bh=!!", 1));
        let twice = tampered(&o, &msg(), |h| {
            format!("{} d=example.com;\r\n", h.trim_end())
        });
        let expiry = forged(&o, "t=1000; x=1000", &msg());
        // No version tag: the crate would read it as a valid signature.
        let unversioned = tampered(&o, &msg(), |h| h.replacen("v=1; ", "", 1));
        assert!(!String::from_utf8_lossy(&unversioned).contains("v=1"));
        // No algorithm tag: the crate would read it as rsa-sha256 (RFC 6376 3.5).
        let unalgorithmed = tampered(&o, &msg(), |h| h.replacen("a=ed25519-sha256; ", "", 1));
        assert!(!String::from_utf8_lossy(&unalgorithmed).contains("a=ed25519"));
        // Numbers that are no numbers: the crate reads each as 0, "absent".
        let numbers = [
            "t=abc",
            "x=abc",
            "l=abc",
            "x=99999999999999999999",
            "t=1000; x=1000",
        ]
        .map(|tags| forged(&o, tags, &msg()));
        for raw in [missing, base64, twice, expiry, unversioned, unalgorithmed]
            .into_iter()
            .chain(numbers)
        {
            assert_eq!(
                verdicts(&raw, 0, &world),
                one("dkim.malformed.v1", 0, ERROR),
                "{}",
                String::from_utf8_lossy(&raw)
            );
        }
        // A repeated tag is malformed even if the crate would verify the rest,
        // and the first spelling's case does not matter.
        let upper = tampered(&o, &msg(), |h| format!("{} S=other;\r\n", h.trim_end()));
        assert_eq!(
            verdicts(&upper, 0, &world),
            one("dkim.malformed.v1", 0, ERROR)
        );
    }

    #[test]
    fn rule_unsupported() {
        let world = key_of(Kind::Ed25519);
        let o = Sign::default();
        // Passing fixture: ed25519-sha256 (and `l=` absent).
        assert_eq!(verdicts(&base(), 0, &world), vec![]);
        // rsa-sha1 is refused whatever the key says (RFC 8301); the crate cannot
        // sign it, so the algorithm tag is rewritten.
        let sha1 = tampered(&o, &msg(), |h| {
            h.replacen("a=ed25519-sha256", "a=rsa-sha1", 1)
        });
        assert_eq!(
            verdicts(&sha1, 0, &world),
            one("dkim.unsupported.v1", 0, ERROR)
        );
        // A literal `l=0` is indistinguishable from no `l=` to the crate.
        let zero = tampered(&o, &msg(), |h| format!("{} l=0;\r\n", h.trim_end()));
        assert_eq!(
            verdicts(&zero, 0, &world),
            one("dkim.unsupported.v1", 0, ERROR)
        );
        // `l=abc` is no number at all, which is malformed, not unsupported.
        let nonsense = tampered(&o, &msg(), |h| format!("{} l=abc;\r\n", h.trim_end()));
        assert_eq!(
            verdicts(&nonsense, 0, &world),
            one("dkim.malformed.v1", 0, ERROR)
        );
        // An unknown version, algorithm or canonicalization.
        for (from, to) in [
            ("v=1", "v=2"),
            ("a=ed25519-sha256", "a=rsa-md5"),
            ("c=relaxed/relaxed", "c=bogus/relaxed"),
        ] {
            let raw = tampered(&o, &msg(), |h| h.replacen(from, to, 1));
            assert_eq!(
                verdicts(&raw, 0, &world),
                one("dkim.unsupported.v1", 0, ERROR),
                "{to}"
            );
        }
    }

    #[test]
    fn rule_from_unsigned() {
        let world = key_of(Kind::Ed25519);
        // Passing fixture: h=from:subject. The name is matched case-insensitively.
        assert_eq!(verdicts(&base(), 0, &world), vec![]);
        let shouted = Sign {
            headers: vec!["FROM".into(), "Subject".into()],
            ..Sign::default()
        };
        assert_eq!(verdicts(&signed(&shouted, &msg()), 0, &world), vec![]);
        // Failing fixture: a valid signature that leaves the From header out.
        let subject_only = Sign {
            headers: vec!["subject".into()],
            ..Sign::default()
        };
        let raw = signed(&subject_only, &msg());
        assert_eq!(
            verdicts(&raw, 0, &world),
            one("dkim.from_unsigned.v1", 0, ERROR)
        );
    }

    /// The header block `o` signs `message` with, without the message.
    fn header_of(o: &Sign, message: &[u8]) -> Vec<u8> {
        signature(o, message).to_header().into_bytes()
    }

    fn many_fields(count: usize) -> Vec<u8> {
        let message = msg();
        let mut raw = Vec::new();
        for _ in 0..count {
            raw.extend_from_slice(&header_of(&Sign::default(), &message));
        }
        raw.extend_from_slice(&message);
        raw
    }

    fn parsed(raw: &[u8]) -> Signature {
        AuthenticatedMessage::parse_with_opts(raw, None, false)
            .unwrap()
            .dkim_signatures()[0]
            .header
            .clone()
    }

    fn padded(names: usize) -> Sign {
        let mut headers = vec!["from".to_string()];
        headers.extend((1..names).map(|i| format!("x-pad-{i}")));
        Sign {
            headers,
            ..Sign::default()
        }
    }

    #[test]
    fn rule_limits_exceeded_by_count() {
        let world = key_of(Kind::Ed25519);
        // Passing fixture: sixteen valid fields, each judged.
        let sixteen = report(&many_fields(MAX_DKIM_SIGNATURES), 0, &world);
        assert_eq!(dkim(&sixteen), vec![]);
        assert_eq!(lookups(&sixteen.dns).len(), 1);
        // Failing fixture: one more gives one finding at the seventeenth field,
        // no other DKIM finding and no lookup, even though every field is bad.
        let seventeen = report(&many_fields(MAX_DKIM_SIGNATURES + 1), 0, &policy(&[], &[]));
        assert_eq!(dkim(&seventeen), one("dkim.limits_exceeded.v1", 16, ERROR));
        assert!(seventeen.dns.lookups.is_empty());
        // Fields the crate rejects count too: they are the crate's own fields.
        let mut raw = Vec::new();
        for _ in 0..=MAX_DKIM_SIGNATURES {
            raw.extend_from_slice(b"DKIM-Signature: v=1; d=a.example\r\n");
        }
        raw.extend_from_slice(&msg());
        assert_eq!(
            verdicts(&raw, 0, &world),
            one("dkim.limits_exceeded.v1", 16, ERROR)
        );
    }

    #[test]
    fn rule_limits_exceeded_by_signed_names() {
        let world = key_of(Kind::Ed25519);
        // Passing fixture: 128 names, all of them evaluated.
        let raw = signed(&padded(MAX_DKIM_SIGNED_HEADERS), &msg());
        let at_limit = report(&raw, 0, &world);
        assert_eq!(dkim(&at_limit), vec![]);
        assert_eq!(lookups(&at_limit.dns).len(), 1);
        // Failing fixture: 129 names, judged at the field that has them, with no lookup.
        let raw = signed(&padded(MAX_DKIM_SIGNED_HEADERS + 1), &msg());
        let over = report(&raw, 0, &world);
        assert_eq!(dkim(&over), one("dkim.limits_exceeded.v1", 0, ERROR));
        assert!(over.dns.lookups.is_empty());
        // A later field is named by its own occurrence, and nothing else is judged.
        let mut raw = b"DKIM-Signature: v=1\r\n".to_vec();
        raw.extend_from_slice(&header_of(&padded(MAX_DKIM_SIGNED_HEADERS + 1), &msg()));
        raw.extend_from_slice(&base());
        let both = report(&raw, 0, &world);
        assert_eq!(dkim(&both), one("dkim.limits_exceeded.v1", 1, ERROR));
        assert!(both.dns.lookups.is_empty());
        // The count and the names are separate bounds; the earlier field wins.
        let mut raw = header_of(&padded(MAX_DKIM_SIGNED_HEADERS + 1), &msg());
        for _ in 0..=MAX_DKIM_SIGNATURES {
            raw.extend_from_slice(b"DKIM-Signature: v=1\r\n");
        }
        raw.extend_from_slice(&msg());
        assert_eq!(
            verdicts(&raw, 0, &world),
            one("dkim.limits_exceeded.v1", 0, ERROR)
        );
    }

    #[test]
    fn rule_signature_expired() {
        let world = key_of(Kind::Ed25519);
        let o = Sign {
            expiration: Some(86_400),
            ..Sign::default()
        };
        let raw = signed(&o, &msg());
        // The fixtures sit at the poles of the receipt: no clock is read here.
        // Passing fixture: x = receipt + 1. Failing fixture: x = receipt.
        let sig = parsed(&raw);
        assert_eq!(verdicts(&raw, sig.x - 1, &world), vec![]);
        assert_eq!(
            verdicts(&raw, sig.x, &world),
            one("dkim.signature_expired.v1", 0, ERROR)
        );
        assert_eq!(
            verdicts(&raw, sig.x + 1, &world),
            one("dkim.signature_expired.v1", 0, ERROR)
        );
        // A signature that never expires is never expired, whatever the receipt.
        assert_eq!(verdicts(&base(), u64::MAX, &world), vec![]);
    }

    #[test]
    fn rule_expired_since_receipt() {
        let world = key_of(Kind::Ed25519);
        let o = Sign::default();
        // Passing fixture: x is a day after the wall clock; receipt at t.
        let day = Sign {
            expiration: Some(86_400),
            ..Sign::default()
        };
        let raw = signed(&day, &msg());
        let sig = parsed(&raw);
        assert_eq!(verdicts(&raw, sig.t, &world), vec![]);
        // Failing fixture: t=1000, x=2000, received at 1500: the message was
        // received inside its window, but the crate's own clock is long past 2000.
        let raw = forged(&o, "t=1000; x=2000", &msg());
        assert_eq!(
            verdicts(&raw, 1500, &world),
            one("dkim.expired_since_receipt.v1", 0, ERROR)
        );
        // At and after x the receipt rule answers first.
        assert_eq!(
            verdicts(&raw, 2000, &world),
            one("dkim.signature_expired.v1", 0, ERROR)
        );
        // Before t is no problem for the crate (t is not enforced).
        assert_eq!(
            verdicts(&raw, 0, &world),
            one("dkim.expired_since_receipt.v1", 0, ERROR)
        );
    }

    #[test]
    fn expired_since_receipt_hides_key_and_lookup() {
        // The clock residual's radius (decision 0125 section 8): the crate stops at
        // the expiry, before the body hash and before it asks for any key. Every
        // verdict the later rows could give, a missing key included, is replaced.
        let raw = forged(&Sign::default(), "t=1000; x=2000", &msg());
        let nothing = policy(&[], &[]);
        let undeclared = report(&raw, 1500, &nothing);
        assert_eq!(
            dkim(&undeclared),
            one("dkim.expired_since_receipt.v1", 0, ERROR)
        );
        assert!(undeclared.dns.lookups.is_empty());
        // The same bytes with the key declared read the same, and ask nothing.
        let declared = report(&raw, 1500, &key_of(Kind::Ed25519));
        assert_eq!(
            dkim(&declared),
            one("dkim.expired_since_receipt.v1", 0, ERROR)
        );
        assert!(declared.dns.lookups.is_empty());
        // An altered body is still the one verdict.
        let altered = String::from_utf8(raw)
            .unwrap()
            .replace("this  is", "this was");
        assert_eq!(
            verdicts(altered.as_bytes(), 1500, &nothing),
            one("dkim.expired_since_receipt.v1", 0, ERROR)
        );
    }

    #[test]
    fn rule_selector_invalid() {
        // Passing fixture: s=sel.
        assert_eq!(verdicts(&base(), 0, &key_of(Kind::Ed25519)), vec![]);
        // Failing fixture: a selector that cannot be a DNS label is never looked up.
        let o = Sign {
            selector: "bad*sel",
            ..Sign::default()
        };
        let report = report(&signed(&o, &msg()), 0, &key_of(Kind::Ed25519));
        assert_eq!(dkim(&report), one("dkim.selector_invalid.v1", 0, ERROR));
        assert!(report.dns.lookups.is_empty());
        // A domain that is no DNS name gets the same verdict.
        let o = Sign {
            domain: "bad_domain!.example",
            ..Sign::default()
        };
        assert_eq!(
            verdicts(&signed(&o, &msg()), 0, &key_of(Kind::Ed25519)),
            one("dkim.selector_invalid.v1", 0, ERROR)
        );
    }

    #[test]
    fn rule_key_missing() {
        // Passing fixture: the key is declared. Failing fixture: it is declared nowhere.
        assert_eq!(verdicts(&base(), 0, &key_of(Kind::Ed25519)), vec![]);
        let elsewhere = policy(
            &[("other._domainkey.example.com", Kind::Ed25519.record())],
            &[],
        );
        let report = report(&base(), 0, &elsewhere);
        assert_eq!(dkim(&report), one("dkim.key_missing.v1", 0, ERROR));
        // The resolver was never asked: the declared map answered "no records".
        assert_eq!(
            lookups(&report.dns),
            vec![("sel._domainkey.example.com".to_string(), Outcome::NoRecords)]
        );
    }

    #[test]
    fn rule_key_invalid() {
        let rsa = Sign {
            kind: Kind::Rsa2048,
            ..Sign::default()
        };
        let at = |value: &str| policy(&[("sel._domainkey.example.com", value.to_string())], &[]);
        // Passing fixtures: a valid key of each type.
        assert_eq!(
            verdicts(&signed(&rsa, &msg()), 0, &key_of(Kind::Rsa2048)),
            vec![]
        );
        assert_eq!(verdicts(&base(), 0, &key_of(Kind::Ed25519)), vec![]);
        // Failing fixtures: a key type that does not match a= (the crate refuses
        // to verify across types), an unsupported k=, and a record that is no key.
        let wrong_type = (signed(&rsa, &msg()), key_of(Kind::Ed25519));
        let ed_for_rsa_tag = (
            tampered(&Sign::default(), &msg(), |h| {
                h.replacen("a=ed25519-sha256", "a=rsa-sha256", 1)
            }),
            key_of(Kind::Ed25519),
        );
        let unsupported_k = (base(), at("v=DKIM1; k=dsa; p=AAAA"));
        let not_a_key = (base(), at("v=spf1 -all"));
        let no_p = (base(), at("v=DKIM1; k=ed25519"));
        for (raw, world) in [wrong_type, ed_for_rsa_tag, unsupported_k, not_a_key, no_p] {
            assert_eq!(
                verdicts(&raw, 0, &world),
                one("dkim.key_invalid.v1", 0, ERROR)
            );
        }
        // A p= that is not even base64 reads as no key at all.
        let garbage = (base(), at("v=DKIM1; k=ed25519; p=!!!!"));
        assert_eq!(
            verdicts(&garbage.0, 0, &garbage.1),
            one("dkim.key_invalid.v1", 0, ERROR)
        );
    }

    #[test]
    fn key_bytes_the_crate_never_checks_fail_verification() {
        // The crate builds a key from any base64 bytes and learns they are no key
        // only when verification fails, so such a record is `signature_mismatch`
        // for both algorithms, never a pass (decision 0125 section 5).
        let rsa = Sign {
            kind: Kind::Rsa2048,
            ..Sign::default()
        };
        for (raw, value) in [
            (signed(&rsa, &msg()), "v=DKIM1; k=rsa; p=AAAA"),
            (base(), "v=DKIM1; k=ed25519; p=AAAA"),
        ] {
            let world = policy(&[("sel._domainkey.example.com", value.into())], &[]);
            assert_eq!(
                verdicts(&raw, 0, &world),
                one("dkim.signature_mismatch.v1", 0, ERROR)
            );
        }
    }

    #[test]
    fn rule_key_revoked() {
        // Passing fixture: a non-empty p=. Failing fixture: `v=DKIM1; p=`.
        assert_eq!(verdicts(&base(), 0, &key_of(Kind::Ed25519)), vec![]);
        let revoked = policy(&[("sel._domainkey.example.com", "v=DKIM1; p=".into())], &[]);
        assert_eq!(
            verdicts(&base(), 0, &revoked),
            one("dkim.key_revoked.v1", 0, ERROR)
        );
    }

    #[test]
    fn rule_not_evaluated_is_never_clean() {
        // Passing fixture: the key is declared.
        assert_eq!(verdicts(&base(), 0, &key_of(Kind::Ed25519)), vec![]);
        // Failing fixture: the key name fails temporarily.
        let down = policy(&[], &["sel._domainkey.example.com"]);
        let report = report(&base(), 0, &down);
        assert_eq!(dkim(&report), one("dkim.not_evaluated.v1", 0, ERROR));
        assert_eq!(
            lookups(&report.dns),
            vec![("sel._domainkey.example.com".to_string(), Outcome::TempFail)]
        );
        // The report never qualifies, at any threshold: a temp_fail is never clean.
        for threshold in [Severity::Info, Severity::Warning, Severity::Error] {
            assert!(!report.qualifies(threshold), "{threshold:?}");
        }
        // The same when only the ATPS name fails, though the signature verified.
        let o = Sign {
            domain: "signer.example",
            atps: Some("example.com"),
            ..Sign::default()
        };
        let down = policy(
            &[("sel._domainkey.signer.example", Kind::Ed25519.record())],
            &["signer.example._atps.example.com"],
        );
        let report = report_of(&signed(&o, &msg()), &down);
        assert_eq!(dkim(&report), one("dkim.not_evaluated.v1", 0, ERROR));
        assert!(!report.qualifies(Severity::Error));
    }

    fn report_of(raw: &[u8], world: &Arc<Effective>) -> Report {
        report(raw, 0, world)
    }

    #[test]
    fn rule_body_hash_mismatch() {
        for kind in Kind::ALL {
            let o = Sign {
                kind,
                ..Sign::default()
            };
            let raw = signed(&o, &msg());
            let world = key_of(kind);
            // Passing fixture: the intact body.
            assert_eq!(verdicts(&raw, 0, &world), vec![], "{kind:?}");
            // Failing fixture: one body byte altered. The crate stops before the
            // key, so nothing is looked up.
            let altered = String::from_utf8(raw).unwrap().replace("test.", "tesT.");
            let report = report_of(altered.as_bytes(), &world);
            assert_eq!(
                dkim(&report),
                one("dkim.body_hash_mismatch.v1", 0, ERROR),
                "{kind:?}"
            );
            assert!(report.dns.lookups.is_empty());
        }
    }

    #[test]
    fn rule_signature_mismatch() {
        for kind in Kind::ALL {
            let o = Sign {
                kind,
                ..Sign::default()
            };
            let raw = signed(&o, &msg());
            let world = key_of(kind);
            // Passing fixture: intact headers. Failing fixture: a signed Subject altered.
            assert_eq!(verdicts(&raw, 0, &world), vec![], "{kind:?}");
            let altered = String::from_utf8(raw)
                .unwrap()
                .replace("Subject: Hello", "Subject: Hellp");
            let report = report_of(altered.as_bytes(), &world);
            assert_eq!(
                dkim(&report),
                one("dkim.signature_mismatch.v1", 0, ERROR),
                "{kind:?}"
            );
            assert_eq!(lookups(&report.dns).len(), 1);
        }
        // A key of the other type cannot check the signature at all: the key is
        // unusable for it, which is its own rule.
        let other_pair = policy(
            &[("sel._domainkey.example.com", Kind::Rsa2048.record())],
            &[],
        );
        assert_eq!(
            verdicts(&signed(&Sign::default(), &msg()), 0, &other_pair),
            one("dkim.key_invalid.v1", 0, ERROR)
        );
        // A different key of the same type, published by RFC 8463, is the verdict:
        // the signature does not verify against the declared key.
        for (kind, other) in [
            (
                Kind::Ed25519,
                format!("v=DKIM1; k=ed25519; p={RFC8463_ED25519_PUBLIC}"),
            ),
            (
                Kind::Rsa2048,
                format!("v=DKIM1; k=rsa; p={RFC8463_RSA1024_PUBLIC}"),
            ),
        ] {
            let o = Sign {
                kind,
                ..Sign::default()
            };
            let raw = signed(&o, &msg());
            let wrong = policy(&[("sel._domainkey.example.com", other)], &[]);
            let report = report_of(&raw, &wrong);
            assert_eq!(
                dkim(&report),
                one("dkim.signature_mismatch.v1", 0, ERROR),
                "{kind:?}"
            );
            assert_eq!(lookups(&report.dns).len(), 1, "{kind:?}");
            // The same bytes verify against the key that signed them.
            assert_eq!(verdicts(&raw, 0, &key_of(kind)), vec![], "{kind:?}");
        }
    }

    #[test]
    fn rule_auid_mismatch() {
        let world = key_of(Kind::Ed25519);
        let with = |identity| Sign {
            identity: Some(identity),
            ..Sign::default()
        };
        // Passing fixtures: i= at d= and at a subdomain of d=.
        for identity in ["@example.com", "joe@example.com", "@mail.example.com"] {
            assert_eq!(
                verdicts(&signed(&with(identity), &msg()), 0, &world),
                vec![],
                "{identity}"
            );
        }
        // Failing fixture: i= at another domain; a t=s key forbids a subdomain.
        let other = signed(&with("@other.example"), &msg());
        assert_eq!(
            verdicts(&other, 0, &world),
            one("dkim.auid_mismatch.v1", 0, ERROR)
        );
        let strict = policy(
            &[(
                "sel._domainkey.example.com",
                format!("v=DKIM1; t=s; k=ed25519; p={ED25519_PUBLIC}"),
            )],
            &[],
        );
        let sub = signed(&with("@mail.example.com"), &msg());
        assert_eq!(
            verdicts(&sub, 0, &strict),
            one("dkim.auid_mismatch.v1", 0, ERROR)
        );
    }

    #[test]
    fn rule_atps_unauthorized() {
        let o = Sign {
            domain: "signer.example",
            atps: Some("example.com"),
            ..Sign::default()
        };
        let key = ("sel._domainkey.signer.example", Kind::Ed25519.record());
        let authorization = (
            "signer.example._atps.example.com",
            "v=ATPS1; d=signer.example".to_string(),
        );
        let raw = signed(&o, &msg());
        // Passing fixture: the ATPS record is declared. Both names are looked up.
        let both = policy(&[key.clone(), authorization], &[]);
        let report = report_of(&raw, &both);
        assert_eq!(dkim(&report), vec![]);
        assert_eq!(lookups(&report.dns).len(), 2);
        // Failing fixture: it is not.
        let report = report_of(&raw, &policy(std::slice::from_ref(&key), &[]));
        assert_eq!(dkim(&report), one("dkim.atps_unauthorized.v1", 0, ERROR));
        // An ATPS name that is no valid name, and a record that is no ATPS record.
        let weird = Sign {
            atps: Some("exa*mple.com"),
            ..o.clone()
        };
        let from_weird = b"From: a@exa*mple.com\r\nSubject: Hello\r\n\r\nBody\r\n".to_vec();
        let raw = signed(&weird, &from_weird);
        let report = report_of(&raw, &policy(std::slice::from_ref(&key), &[]));
        assert_eq!(dkim(&report), one("dkim.atps_unauthorized.v1", 0, ERROR));
        let junk = ("signer.example._atps.example.com", "junk".to_string());
        let report = report_of(&signed(&o, &msg()), &policy(&[key, junk], &[]));
        assert_eq!(dkim(&report), one("dkim.atps_unauthorized.v1", 0, ERROR));
    }

    fn l_of(raw: &[u8]) -> u64 {
        parsed(raw).l
    }

    #[test]
    fn rule_body_length_exceeds() {
        let world = key_of(Kind::Ed25519);
        let o = Sign {
            body_length: true,
            ..Sign::default()
        };
        // Passing fixture: l= is the body length, so the whole body is covered.
        let raw = signed(&o, &msg());
        assert_eq!(verdicts(&raw, 0, &world), vec![]);
        // Failing fixture: l= claims ten more bytes than the body has, validly
        // signed. The crate hashes the whole body and passes it; Swarmail does not.
        let l = l_of(&raw);
        let forge = |o: &Sign, l: u64, message: &[u8]| forged(o, &format!("l={l}"), message);
        let longer = forge(&o, l + 10, &msg());
        assert_eq!(
            verdicts(&longer, 0, &world),
            one("dkim.body_length_exceeds.v1", 0, ERROR)
        );
        assert_eq!(verdicts(&forge(&o, l, &msg()), 0, &world), vec![]);
        // l= counts raw bytes: trailing blank lines make the raw body longer than
        // the canonical one, and the signer's own l= is clean.
        for cb in [Canonicalization::Simple, Canonicalization::Relaxed] {
            let o = Sign { cb, ..o.clone() };
            let blank = message("text\n\n\n");
            assert_eq!(verdicts(&signed(&o, &blank), 0, &world), vec![], "{cb:?}");
        }
        // A body with no final CRLF is five bytes canonical under simple, so an
        // l= of the canonical length (what RFC 6376 section 3.4.5 counts) is
        // fine and one byte more is not.
        let o = Sign {
            cb: Canonicalization::Simple,
            ..o
        };
        let bare = message("abc");
        assert_eq!(l_of(&signed(&o, &bare)), 3);
        assert_eq!(verdicts(&forge(&o, 5, &bare), 0, &world), vec![]);
        assert_eq!(
            verdicts(&forge(&o, 6, &bare), 0, &world),
            one("dkim.body_length_exceeds.v1", 0, ERROR)
        );
    }

    #[test]
    fn rule_body_length_partial() {
        let world = key_of(Kind::Ed25519);
        for cb in [Canonicalization::Relaxed, Canonicalization::Simple] {
            let o = Sign {
                body_length: true,
                cb,
                ..Sign::default()
            };
            // Passing fixture: l= covers the body. Failing fixture: text appended
            // after the signed length; the crate verifies, Swarmail warns (RFC 6376
            // section 8.2).
            let raw = signed(&o, &msg());
            assert_eq!(verdicts(&raw, 0, &world), vec![], "{cb:?}");
            let mut appended = raw;
            appended.extend_from_slice(b"Pay me now.\r\n");
            assert_eq!(
                verdicts(&appended, 0, &world),
                one("dkim.body_length_partial.v1", 0, Severity::Warning),
                "{cb:?}"
            );
        }
        // A warning qualifies at `error` and not at `warning`.
        let o = Sign {
            body_length: true,
            ..Sign::default()
        };
        let mut appended = signed(&o, &msg());
        appended.extend_from_slice(b"Pay me now.\r\n");
        // (Judged on the DKIM findings alone: the bulk checklist's own
        // not_evaluated errors disqualify every report until #255.)
        let report = only_dkim(report_of(&appended, &world));
        assert!(report.qualifies(Severity::Error));
        assert!(!report.qualifies(Severity::Warning));
    }

    #[test]
    fn precedence_and_occurrences() {
        let world = key_of(Kind::Ed25519);
        let good = header_of(&Sign::default(), &msg());
        let other_key = Sign {
            selector: "other",
            ..Sign::default()
        };
        let no_key = header_of(&other_key, &msg());
        // A field the crate rejects with a missing tag leaves no output of its own.
        let missing = String::from_utf8(good.clone())
            .unwrap()
            .replacen("bh=", "xh=", 1)
            .into_bytes();
        // A field it rejects with a DKIM error of its own leaves a neutral one.
        let unsupported = String::from_utf8(good.clone())
            .unwrap()
            .replacen("a=ed25519-sha256", "a=rsa-md5", 1)
            .into_bytes();
        let build = |fields: &[&[u8]]| {
            let mut raw = fields.concat();
            raw.extend_from_slice(&msg());
            raw
        };
        // A passing signature between a malformed one and a failing one.
        let raw = build(&[&missing, &good, &no_key]);
        assert_eq!(
            verdicts(&raw, 0, &world),
            vec![
                ("dkim.malformed.v1".to_string(), 0, ERROR),
                ("dkim.key_missing.v1".to_string(), 2, ERROR)
            ]
        );
        // Rejections between parsed fields do not shift the later occurrences.
        for rejected in [&missing, &unsupported] {
            let raw = build(&[&good, rejected, &no_key, rejected, &good, &no_key]);
            let expected = if rejected == &missing {
                "dkim.malformed.v1"
            } else {
                "dkim.unsupported.v1"
            };
            assert_eq!(
                verdicts(&raw, 0, &world),
                vec![
                    (expected.to_string(), 1, ERROR),
                    ("dkim.key_missing.v1".to_string(), 2, ERROR),
                    (expected.to_string(), 3, ERROR),
                    ("dkim.key_missing.v1".to_string(), 5, ERROR),
                ]
            );
        }
        // Every field gets its own verdict, and the crate's ignored output never
        // leaks into another: a duplicate-tag field that the crate would verify.
        let twice = String::from_utf8(good.clone())
            .unwrap()
            .trim_end()
            .to_string()
            + " d=example.com;\r\n";
        let raw = build(&[twice.as_bytes(), &good, twice.as_bytes()]);
        assert_eq!(
            verdicts(&raw, 0, &world),
            vec![
                ("dkim.malformed.v1".to_string(), 0, ERROR),
                ("dkim.malformed.v1".to_string(), 2, ERROR)
            ]
        );
        // The report lists findings sorted, once each, however the fields sit.
        let two = report(&build(&[&no_key, &no_key]), 0, &world);
        assert_eq!(ids(&two), ["dkim.key_missing.v1", "dkim.key_missing.v1"]);
        // Both fields ask for the same name: it is recorded once.
        assert_eq!(lookups(&two.dns).len(), 1);
    }

    // mail-auth 0.14.0 ends the headers at a CRLF line that holds no colon and
    // does not start with a space or tab; Swarmail skips such a line and goes on.
    // A whitespace-only line inside the block is a continuation in both, but a
    // whitespace-only first line ends the crate's headers and not Swarmail's.

    /// A signed message with a colon-less line right after the signature field:
    /// the crate ends the headers there, Swarmail ends them at the blank line.
    fn stray_line_after() -> Vec<u8> {
        let mut raw = header_of(&Sign::default(), &msg());
        raw.extend_from_slice(b"stray line\r\n");
        raw.extend_from_slice(&msg());
        raw
    }

    /// The same line before the signature: the crate sees the signature as body.
    fn stray_line_before() -> Vec<u8> {
        let mut raw = b"X-Pad: 1\r\nstray line\r\n".to_vec();
        raw.extend_from_slice(&base());
        raw
    }

    fn assert_ambiguous(raw: &[u8], world: &Arc<Effective>) {
        let report = report(raw, 0, world);
        assert_eq!(
            dkim(&report),
            one("dkim.header_boundary_ambiguous.v1", 0, ERROR)
        );
        assert!(report.dns.lookups.is_empty());
    }

    /// The message-level finding stands alone, so equality with it is exact.
    fn is_ambiguous(raw: &[u8], world: &Arc<Effective>) -> bool {
        verdicts(raw, 0, world) == one("dkim.header_boundary_ambiguous.v1", 0, ERROR)
    }

    fn offsets(raw: &[u8]) -> (Option<usize>, usize) {
        (
            AuthenticatedMessage::parse_with_opts(raw, None, true).map(|m| m.body_offset()),
            super::super::headers(raw).unwrap().1,
        )
    }

    #[test]
    fn boundary_agrees_for_crlf_lf_and_empty_body() {
        let world = key_of(Kind::Ed25519);
        let lf_raw = String::from_utf8(base()).unwrap().replace("\r\n", "\n");
        let empty = signed(&Sign::default(), &message(""));
        for raw in [base(), lf_raw.clone().into_bytes(), empty.clone()] {
            let (crate_offset, ours) = offsets(&raw);
            assert_eq!(crate_offset, Some(ours));
        }
        // The passing fixtures of the rule: no boundary finding, normal verdicts.
        assert_eq!(verdicts(&base(), 0, &world), vec![]);
        assert!(!is_ambiguous(lf_raw.as_bytes(), &world));
        let report = report(&empty, 0, &world);
        assert_eq!(dkim(&report), vec![]);
        assert_eq!(lookups(&report.dns).len(), 1);
    }

    #[test]
    fn boundary_whitespace_line_inside_the_block_is_no_difference() {
        // The first design's example: a whitespace-only line after, or among, the
        // fields is a continuation for the crate as well, so nothing is ambiguous.
        let world = key_of(Kind::Ed25519);
        let mut after = header_of(&Sign::default(), &msg());
        after.extend_from_slice(b" \r\n");
        after.extend_from_slice(&msg());
        let mut before = b"X-Pad: 1\r\n \r\n".to_vec();
        before.extend_from_slice(&base());
        for raw in [after, before] {
            let (crate_offset, ours) = offsets(&raw);
            assert_eq!(crate_offset, Some(ours));
            assert!(!is_ambiguous(&raw, &world));
        }
    }

    #[test]
    fn boundary_stray_line_after_signature() {
        let world = key_of(Kind::Ed25519);
        let raw = stray_line_after();
        // The precondition: the two parsers really end the headers apart.
        let (crate_offset, ours) = offsets(&raw);
        assert_ne!(crate_offset, Some(ours));
        let strict = AuthenticatedMessage::parse_with_opts(&raw, None, true).unwrap();
        assert!(!strict.dkim_signatures().is_empty());
        assert_ambiguous(&raw, &world);
    }

    #[test]
    fn boundary_stray_line_before_signature() {
        let world = key_of(Kind::Ed25519);
        let raw = stray_line_before();
        // The crate lists no signature at all, yet Swarmail sees the field.
        let strict = AuthenticatedMessage::parse_with_opts(&raw, None, true).unwrap();
        assert!(strict.dkim_signatures().is_empty());
        assert!(strict.errors().is_empty());
        assert_ambiguous(&raw, &world);
    }

    #[test]
    fn boundary_strict_parse_none_while_signed() {
        let world = key_of(Kind::Ed25519);
        let mut raw = b" \r\n".to_vec();
        raw.extend_from_slice(&base());
        assert!(AuthenticatedMessage::parse_with_opts(&raw, None, true).is_none());
        assert_ambiguous(&raw, &world);
    }

    /// A kit-signed field with a space before the colon: the crate's tag parser
    /// reads it as a DKIM-Signature, the shared scanner does not name it one.
    fn spaced_name() -> Vec<u8> {
        let header = String::from_utf8(header_of(&Sign::default(), &msg())).unwrap();
        header
            .replacen("DKIM-Signature:", "DKIM-Signature :", 1)
            .into_bytes()
    }

    #[test]
    fn boundary_dkim_field_name_with_space_is_ambiguous() {
        let world = key_of(Kind::Ed25519);
        let mut before = spaced_name();
        before.extend_from_slice(&base());
        let mut after = base();
        let split = after.len() - msg().len();
        after.splice(split..split, spaced_name());
        for raw in [before, after] {
            // The precondition: the offsets agree, only the field lists differ.
            let (crate_offset, ours) = offsets(&raw);
            assert_eq!(crate_offset, Some(ours));
            let (root, listed) = root_and_crate_values(&raw);
            assert_eq!((root.len(), listed.len()), (1, 2));
            assert_ambiguous(&raw, &world);
        }
    }

    /// The one `dkim.header_boundary_ambiguous.v1` finding, at `null`: the root list
    /// holds no DKIM-Signature field, so there is no occurrence to point at.
    fn assert_ambiguous_at_null(raw: &[u8], world: &Arc<Effective>) {
        let report = report(raw, 0, world);
        let found: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.rule_id.starts_with("dkim."))
            .collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rule_id, "dkim.header_boundary_ambiguous.v1");
        assert_eq!(found[0].severity, ERROR);
        assert_eq!(
            found[0].location,
            Location::Header {
                name: "dkim-signature".into(),
                occurrence: None,
            }
        );
        assert!(report.dns.lookups.is_empty());
    }

    #[test]
    fn boundary_lone_spaced_field_is_ambiguous_at_null() {
        // The only DKIM field is named with a space before the colon: the crate reads
        // a signature, the root list holds none (U2: the finding is located at null).
        let world = key_of(Kind::Ed25519);
        let mut raw = spaced_name();
        raw.extend_from_slice(&msg());
        let (root, listed) = root_and_crate_values(&raw);
        assert_eq!((root.len(), listed.len()), (0, 1));
        assert_ambiguous_at_null(&raw, &world);
    }

    #[test]
    fn unsigned_stray_and_whitespace_lines_have_no_dkim_finding() {
        // The guard is gated on "signed": mail with no signature gains nothing,
        // however its header block looks (AC1).
        let world = key_of(Kind::Ed25519);
        for (from, to) in [
            ("Subject: Hello\r\n", "Subject: Hello\r\nstray line\r\n"),
            ("Subject: Hello\r\n", "Subject: Hello\r\n \r\n"),
        ] {
            let raw = String::from_utf8(msg()).unwrap().replace(from, to);
            let report = report(raw.as_bytes(), 0, &world);
            assert_eq!(dkim(&report), vec![]);
            assert!(report.dns.lookups.is_empty());
        }
        let mut lead = b" \r\n".to_vec();
        lead.extend_from_slice(&msg());
        assert_eq!(verdicts(&lead, 0, &world), vec![]);
    }

    #[test]
    fn rule_header_boundary_ambiguous() {
        let world = key_of(Kind::Ed25519);
        // Passing fixture: a signed message whose boundaries agree.
        assert_eq!(verdicts(&base(), 0, &world), vec![]);
        // Failing fixtures: a stray line after and before the field.
        assert_ambiguous(&stray_line_after(), &world);
        assert_ambiguous(&stray_line_before(), &world);
        // It stands in for every verdict, even a field that would be malformed.
        let mut raw = b"DKIM-Signature: v=1\r\n".to_vec();
        raw.extend_from_slice(&stray_line_after());
        assert_ambiguous(&raw, &world);
        // A field name the crate reads as a signature and the root list does not.
        let mut spaced = spaced_name();
        spaced.extend_from_slice(&base());
        assert_ambiguous(&spaced, &world);
        // Failing fixture, located at null: the lone field the root list does not
        // hold, so the finding has no occurrence to point at.
        let mut lone = spaced_name();
        lone.extend_from_slice(&msg());
        assert_ambiguous_at_null(&lone, &world);
        // Passing: a whitespace-only line inside the block is no difference.
        let mut inside = header_of(&Sign::default(), &msg());
        inside.extend_from_slice(b" \r\n");
        inside.extend_from_slice(&msg());
        assert!(!is_ambiguous(&inside, &world));
    }

    /// The names of the root `dkim-signature` fields, as the shared scanner lists
    /// them, and the values the crate's strict parse lists, both in order.
    fn root_and_crate_values(raw: &[u8]) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
        let root = super::super::headers(raw)
            .unwrap()
            .0
            .into_iter()
            .filter(|(name, _)| name == b"dkim-signature")
            .map(|(_, value)| value)
            .collect();
        let strict = AuthenticatedMessage::parse_with_opts(raw, None, true).unwrap();
        let ours = fields(&strict)
            .into_iter()
            .map(|field| field.value.to_vec())
            .collect();
        (root, ours)
    }

    /// S1: the count conjunct's premise. For every spelling of a field the shared
    /// `headers()` scanner names `dkim-signature`, the crate's strict parse lists
    /// exactly that field, in the same order. The crate's values keep the folding
    /// whitespace the scanner collapses, so equality is on the unfolded text.
    #[test]
    fn crate_lists_every_root_dkim_field() {
        let good = header_of(&Sign::default(), &msg());
        let good = String::from_utf8(good).unwrap();
        let value = good.strip_prefix("DKIM-Signature:").unwrap().to_string();
        // The empty value sits between two fields that parse, so a list that put it
        // first or last, or dropped it, would differ from the root's.
        let cases: Vec<(&str, String)> = vec![
            ("DKIM-Signature", value.clone()),
            ("dkim-signature", value.clone()),
            ("DKIM-Signature", "\r\n".into()),
            ("DKIM-SIGNATURE", value.clone()),
            ("dKiM-sIgNaTuRe", value.clone()),
            // A value the parser rejects: no `bh=`.
            (
                "DKIM-Signature",
                " v=1; a=ed25519-sha256; d=example.com; s=sel; h=from; b=AAAA\r\n".into(),
            ),
            // A value folded across lines.
            ("DKIM-Signature", value.replacen("; ", ";\r\n\t", 3)),
        ];
        let unfold = |bytes: &Vec<u8>| -> String {
            String::from_utf8_lossy(bytes)
                .split(|c: char| c.is_ascii_whitespace())
                .filter(|word| !word.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        };
        for (name, value) in &cases {
            let mut raw = format!("{name}:{value}").into_bytes();
            raw.extend_from_slice(&msg());
            let (root, ours) = root_and_crate_values(&raw);
            assert_eq!(root.len(), 1, "{name}: the root list holds the field");
            assert_eq!(ours.len(), 1, "{name}: the crate lists the field");
            assert_eq!(unfold(&root[0]), unfold(&ours[0]), "{name}: same field");
        }
        // Several spellings in one message: the same fields, in the same order.
        let mut raw = Vec::new();
        for (name, value) in &cases {
            raw.extend_from_slice(format!("{name}:{value}").as_bytes());
        }
        raw.extend_from_slice(&msg());
        let (root, ours) = root_and_crate_values(&raw);
        assert_eq!(root.len(), cases.len());
        assert_eq!(
            root.iter().map(unfold).collect::<Vec<_>>(),
            ours.iter().map(unfold).collect::<Vec<_>>()
        );
    }

    /// The strict parse and the full one (used when a field carries `l=`) list the
    /// same fields at the same places, so the occurrence of a finding does not move
    /// between them.
    #[test]
    fn strict_and_full_parses_list_the_same_fields() {
        let with_l = header_of(
            &Sign {
                body_length: true,
                ..Sign::default()
            },
            &msg(),
        );
        let without_l = header_of(&Sign::default(), &msg());
        let no_bh =
            b"DKIM-Signature: v=1; a=ed25519-sha256; d=example.com; s=sel; h=from; b=AAAA\r\n";
        let mut raw = with_l.clone();
        raw.extend_from_slice(&without_l);
        raw.extend_from_slice(no_bh);
        raw.extend_from_slice(&msg());
        let strict = AuthenticatedMessage::parse_with_opts(&raw, None, true).unwrap();
        let full = AuthenticatedMessage::parse_with_opts(&raw, None, false).unwrap();
        let (strict, full) = (fields(&strict), fields(&full));
        assert_eq!(strict.len(), 3);
        assert_eq!(strict.iter().filter(|f| f.scan.has_l).count(), 1);
        assert_eq!(strict.len(), full.len());
        let base = raw.as_ptr() as usize;
        let at = |field: &Field<'_>| field.value.as_ptr() as usize - base;
        assert_eq!(
            strict.iter().map(at).collect::<Vec<_>>(),
            full.iter().map(at).collect::<Vec<_>>()
        );
    }

    /// S3: a finding's occurrence is the field's index among the root list's
    /// `dkim-signature` entries, computed here from that list and not by hand.
    #[test]
    fn finding_occurrence_is_the_root_occurrence() {
        let world = key_of(Kind::Ed25519);
        let first = header_of(&Sign::default(), &msg());
        // Signed over another body, so its body hash cannot match this message.
        let second = header_of(&Sign::default(), &message("another body\n"));
        let rejected =
            b"DKIM-Signature: v=1; a=ed25519-sha256; d=example.com; s=sel; h=from; b=AAAA\r\n";
        let mut raw = first.clone();
        raw.extend_from_slice(rejected);
        raw.extend_from_slice(&second);
        raw.extend_from_slice(&msg());
        let (root, _) = root_and_crate_values(&raw);
        let index_of = |field: &[u8]| {
            let wanted = String::from_utf8_lossy(field);
            let wanted = wanted
                .split_once(':')
                .unwrap()
                .1
                .split_whitespace()
                .last()
                .unwrap()
                .to_string();
            root.iter()
                .position(|value| String::from_utf8_lossy(value).trim().ends_with(&wanted))
                .unwrap() as u64
        };
        let (rejected_at, second_at) = (index_of(rejected), index_of(&second));
        assert_eq!((rejected_at, second_at), (1, 2));
        assert_eq!(
            verdicts(&raw, 0, &world),
            vec![
                ("dkim.malformed.v1".to_string(), rejected_at, ERROR),
                ("dkim.body_hash_mismatch.v1".to_string(), second_at, ERROR),
            ]
        );
    }

    /// Whether the crate alone, asked through the adapter, passes every signature.
    fn crate_passes(raw: &[u8], world: &Arc<Effective>) -> bool {
        let message = AuthenticatedMessage::parse_with_opts(raw, None, false).unwrap();
        let mut lookups = declared_lookups(world);
        let adapter = Adapter {
            lookups: RefCell::new(&mut lookups),
        };
        let outputs = AUTHENTICATOR
            .verify_dkim(Parameters::new(&message).with_cache(&adapter))
            .now_or_never()
            .unwrap();
        outputs.iter().all(|o| o.result() == &DkimResult::Pass)
    }

    #[test]
    fn missing_a_is_malformed() {
        let o = Sign {
            kind: Kind::Rsa2048,
            ..Sign::default()
        };
        let world = key_of(Kind::Rsa2048);
        let with = forged(&o, "t=1000", &msg());
        let without = forged_with(&o, "t=1000", false, &msg());
        assert!(!String::from_utf8_lossy(&without).contains("a=rsa"));
        // The crate defaults the algorithm to rsa-sha256 and verifies the field.
        assert!(crate_passes(&with, &world));
        assert!(crate_passes(&without, &world));
        // Swarmail does not: the tag is required (RFC 6376 section 3.5).
        assert_eq!(verdicts(&with, 0, &world), vec![]);
        assert_eq!(
            verdicts(&without, 0, &world),
            one("dkim.malformed.v1", 0, ERROR)
        );
    }

    #[test]
    fn non_numeric_tags_are_malformed() {
        let o = Sign::default();
        let world = key_of(Kind::Ed25519);
        let t = parsed(&base()).t;
        let later = t + 86_400;
        // Each pair: the same tag, non-numeric (malformed) and numeric (clean).
        for (bad, good) in [
            ("t=abc", format!("t={t}")),
            ("x=abc", format!("x={later}")),
            ("x=99999999999999999999", format!("x={later}")),
            ("t=1000; x=abc", format!("t=1000; x={later}")),
        ] {
            assert_eq!(
                verdicts(&forged(&o, bad, &msg()), 0, &world),
                one("dkim.malformed.v1", 0, ERROR),
                "{bad}"
            );
            assert_eq!(
                verdicts(&forged(&o, &good, &msg()), 0, &world),
                vec![],
                "{good}"
            );
        }
        // `l=abc` is malformed whatever the rest says.
        assert_eq!(
            verdicts(&forged(&o, "l=abc", &msg()), 0, &world),
            one("dkim.malformed.v1", 0, ERROR)
        );
    }

    #[test]
    fn folded_numeric_tag_is_clean() {
        let o = Sign::default();
        let world = key_of(Kind::Ed25519);
        let later = parsed(&base()).t + 86_400;
        let folded = format!("t=1000;\r\n x=\r\n {later}\r\n ");
        assert_eq!(verdicts(&forged(&o, &folded, &msg()), 0, &world), vec![]);
    }

    #[test]
    fn x_zero_without_t_is_expired() {
        let o = Sign::default();
        let world = key_of(Kind::Ed25519);
        // A literal `x=0` is an expiry in 1970, not an absent tag.
        let raw = forged(&o, "x=0", &msg());
        for receipt in [0, 1, u64::MAX] {
            assert_eq!(
                verdicts(&raw, receipt, &world),
                one("dkim.signature_expired.v1", 0, ERROR)
            );
        }
    }

    #[test]
    fn x_zero_with_t_is_malformed() {
        let world = key_of(Kind::Ed25519);
        let raw = forged(&Sign::default(), "t=1000; x=0", &msg());
        let report = report(&raw, 0, &world);
        assert_eq!(dkim(&report), one("dkim.malformed.v1", 0, ERROR));
        // Measured, and pinned: the crate still asks for the key of a field that
        // Swarmail judges malformed, so exactly that one lookup is recorded.
        assert_eq!(
            lookups(&report.dns),
            vec![("sel._domainkey.example.com".to_string(), Outcome::Records)]
        );
    }

    #[test]
    fn arc_and_rejected_fields_do_not_shift_pairing() {
        let world = key_of(Kind::Ed25519);
        let good = header_of(&Sign::default(), &msg());
        let other = Sign {
            selector: "other",
            ..Sign::default()
        };
        let failing = header_of(&other, &msg());
        let rejected = String::from_utf8(good.clone())
            .unwrap()
            .replacen("bh=", "xh=", 1)
            .into_bytes();
        let arc = b"ARC-Seal: i=1\r\nARC-Message-Signature: i=1; junk\r\nARC-Authentication-Results: i=1\r\n";
        let build = |fields: &[&[u8]]| {
            let mut raw = fields.concat();
            raw.extend_from_slice(&msg());
            raw
        };
        // The passing signature keeps its verdict and its occurrence, beside ARC
        // fields the crate rejects and a DKIM field it rejects with an error.
        let raw = build(&[arc, &rejected, &good, &failing]);
        assert_eq!(
            verdicts(&raw, 0, &world),
            vec![
                ("dkim.malformed.v1".to_string(), 0, ERROR),
                ("dkim.key_missing.v1".to_string(), 2, ERROR)
            ]
        );
        let raw = build(&[&failing, arc, &good, &rejected, arc]);
        assert_eq!(
            verdicts(&raw, 0, &world),
            vec![
                ("dkim.key_missing.v1".to_string(), 0, ERROR),
                ("dkim.malformed.v1".to_string(), 2, ERROR)
            ]
        );
    }

    #[test]
    fn trailing_dot_domain_is_selector_invalid() {
        // The crate asks for `sel._domainkey.example.com..`, which is no name: the
        // verdict judges that very name, not a rebuilt `sel._domainkey.example.com`.
        let o = Sign {
            domain: "example.com.",
            ..Sign::default()
        };
        let world = key_of(Kind::Ed25519);
        let report = report(&signed(&o, &msg()), 0, &world);
        assert_eq!(dkim(&report), one("dkim.selector_invalid.v1", 0, ERROR));
        assert!(report.dns.lookups.is_empty());
        // One judge: the adapter and the verdict agree on every name.
        for (name, valid) in [
            ("sel._domainkey.example.com.", true),
            ("sel._domainkey.example.com", true),
            ("sel._domainkey.example.com..", false),
            ("bad*sel._domainkey.example.com.", false),
            (".", false),
            ("", false),
        ] {
            assert_eq!(owner_of(name).is_some(), valid, "{name:?}");
        }
    }

    #[test]
    fn resolver_without_nameservers_answers_nothing() {
        // No cache at all: whatever the crate would ask, nothing answers. It must
        // finish on the first poll with an error, never a pass and never a hang.
        let raw = base();
        let message = AuthenticatedMessage::parse_with_opts(&raw, None, false).unwrap();
        let outputs = AUTHENTICATOR
            .verify_dkim(Parameters::new(&message))
            .now_or_never()
            .expect("a resolver with no name servers fails on the first poll");
        assert_eq!(outputs.len(), 1);
        assert!(matches!(
            outputs[0].result(),
            DkimResult::TempError(_) | DkimResult::PermError(_)
        ));
    }

    #[test]
    fn algorithm_canonicalization_matrix() {
        use Canonicalization::{Relaxed, Simple};
        for kind in Kind::ALL {
            for (ch, cb) in [
                (Simple, Simple),
                (Relaxed, Relaxed),
                (Relaxed, Simple),
                (Simple, Relaxed),
            ] {
                let o = Sign {
                    kind,
                    ch,
                    cb,
                    ..Sign::default()
                };
                let report = report(&signed(&o, &msg()), 0, &key_of(kind));
                assert_eq!(dkim(&report), vec![], "{kind:?} {ch:?}/{cb:?}");
                assert_eq!(
                    lookups(&report.dns),
                    vec![("sel._domainkey.example.com".to_string(), Outcome::Records)]
                );
            }
        }
    }

    #[test]
    fn whitespace_mutation_by_mode() {
        use Canonicalization::{Relaxed, Simple};
        let world = key_of(Kind::Ed25519);
        for (ch, cb) in [
            (Simple, Simple),
            (Relaxed, Relaxed),
            (Relaxed, Simple),
            (Simple, Relaxed),
        ] {
            let o = Sign {
                ch,
                cb,
                ..Sign::default()
            };
            let raw = String::from_utf8(signed(&o, &msg())).unwrap();
            // Folding a signed header changes it under simple only (RFC 6376 section 3.4.1).
            let folded = raw.replace("Subject: Hello\r\n", "Subject:\r\n Hello\r\n");
            assert_ne!(folded, raw);
            let expected = if ch == Simple {
                one("dkim.signature_mismatch.v1", 0, ERROR)
            } else {
                vec![]
            };
            assert_eq!(
                verdicts(folded.as_bytes(), 0, &world),
                expected,
                "fold {ch:?}/{cb:?}"
            );
            // Doubling inner body whitespace changes the body under simple only.
            let spaced = raw.replace("this  is", "this     is");
            assert_ne!(spaced, raw);
            let expected = if cb == Simple {
                one("dkim.body_hash_mismatch.v1", 0, ERROR)
            } else {
                vec![]
            };
            assert_eq!(
                verdicts(spaced.as_bytes(), 0, &world),
                expected,
                "space {ch:?}/{cb:?}"
            );
            // Trailing blank lines are ignored by both modes (RFC 6376 section 3.4.3).
            let blank = format!("{raw}\r\n\r\n");
            assert_eq!(
                verdicts(blank.as_bytes(), 0, &world),
                vec![],
                "blank {ch:?}/{cb:?}"
            );
        }
    }

    const RFC8463: &str = include_str!("dkim/rfc8463-appendix-a.txt");

    #[test]
    fn rfc8463_appendix_a_verifies_both_signatures() {
        // The published message and its two key records (RFC 8463 appendix A):
        // an ed25519 key, and a 1,024-bit RSA key that the crate's signer cannot
        // sign with but must verify.
        let world = policy(
            &[
                (
                    "brisbane._domainkey.football.example.com",
                    format!("v=DKIM1; k=ed25519; p={RFC8463_ED25519_PUBLIC}"),
                ),
                (
                    "test._domainkey.football.example.com",
                    format!("v=DKIM1; k=rsa; p={RFC8463_RSA1024_PUBLIC}"),
                ),
            ],
            &[],
        );
        let raw = crlf(RFC8463);
        let report = report(&raw, 0, &world);
        assert_eq!(dkim(&report), vec![]);
        assert_eq!(
            lookups(&report.dns),
            vec![
                (
                    "brisbane._domainkey.football.example.com".to_string(),
                    Outcome::Records
                ),
                (
                    "test._domainkey.football.example.com".to_string(),
                    Outcome::Records
                ),
            ]
        );
        // Both signatures are really checked: an altered body fails both.
        let altered = String::from_utf8(raw).unwrap().replace("Joe.", "Jon.");
        assert_eq!(
            verdicts(altered.as_bytes(), 0, &world),
            vec![
                ("dkim.body_hash_mismatch.v1".to_string(), 0, ERROR),
                ("dkim.body_hash_mismatch.v1".to_string(), 1, ERROR),
            ]
        );
    }

    #[test]
    fn failure_reports_ask_for_the_report_name() {
        // r=y on a failing signature makes the crate ask for the RFC 6651 record;
        // the name is answered by the declared map and recorded like any other.
        let o = Sign {
            reporting: true,
            ..Sign::default()
        };
        let altered = String::from_utf8(signed(&o, &msg()))
            .unwrap()
            .replace("test.", "tesT.");
        let report = report_of(altered.as_bytes(), &policy(&[], &[]));
        assert_eq!(dkim(&report), one("dkim.body_hash_mismatch.v1", 0, ERROR));
        assert_eq!(
            lookups(&report.dns),
            vec![(
                "_report._domainkey.example.com".to_string(),
                Outcome::NoRecords
            )]
        );
        // Declared, the answer is never typed as a report record (0125 section 8:
        // the crate's report sampling reads the clock, and is reached only through
        // one), so the verdict and the one lookup are the same whatever `rp=` says.
        for rp in ["100", "0"] {
            let declared = policy(
                &[(
                    "_report._domainkey.example.com",
                    format!("v=DKIM1; ra=postmaster; rp={rp}"),
                )],
                &[],
            );
            let report = report_of(altered.as_bytes(), &declared);
            assert_eq!(dkim(&report), one("dkim.body_hash_mismatch.v1", 0, ERROR));
            assert_eq!(
                lookups(&report.dns),
                vec![(
                    "_report._domainkey.example.com".to_string(),
                    Outcome::Records
                )],
                "rp={rp}"
            );
            // Two evaluations of the same message agree byte for byte.
            let first = serde_json::to_vec(&report).unwrap();
            let second = serde_json::to_vec(&report_of(altered.as_bytes(), &declared)).unwrap();
            assert!(first == second, "rp={rp}");
        }
        // A passing signature asks for no report record.
        let report = report_of(&signed(&o, &msg()), &key_of(Kind::Ed25519));
        assert_eq!(lookups(&report.dns).len(), 1);
    }

    #[test]
    fn report_selector_is_a_key() {
        // A selector of `_report` makes an ordinary key name: the answer is read as a
        // key record, so the signature passes. (Read as a report record, it would
        // fail closed as `key_invalid`.)
        let o = Sign {
            selector: "_report",
            ..Sign::default()
        };
        let raw = signed(&o, &msg());
        let world = policy(
            &[("_report._domainkey.example.com", Kind::Ed25519.record())],
            &[],
        );
        let report = report_of(&raw, &world);
        assert_eq!(dkim(&report), vec![]);
        assert_eq!(
            lookups(&report.dns),
            vec![(
                "_report._domainkey.example.com".to_string(),
                Outcome::Records
            )]
        );
    }

    #[test]
    fn signed_report_is_byte_identical_twice() {
        let raw = signed(&Sign::default(), &msg());
        let world = key_of(Kind::Ed25519);
        let first = serde_json::to_vec(&report(&raw, 0, &world)).unwrap();
        let second = serde_json::to_vec(&report(&raw, 0, &world)).unwrap();
        assert_eq!(first, second);
        // And for a failing one, whatever the order of its fields.
        let altered = String::from_utf8(raw).unwrap().replace("test.", "tesT.");
        let first = serde_json::to_vec(&report_of(altered.as_bytes(), &world)).unwrap();
        let second = serde_json::to_vec(&report_of(altered.as_bytes(), &world)).unwrap();
        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn service_judges_at_the_receipt_second() {
        let service = super::super::Service::default();
        let o = Sign {
            expiration: Some(86_400),
            ..Sign::default()
        };
        let raw = signed(&o, &msg());
        let x = parsed(&raw).x;
        let world = key_of(Kind::Ed25519);
        let at = |received_ms: i64| {
            let mut email = crate::store::email("dkim", "box", "a@example.org");
            email.raw = raw.clone();
            email.received_ms = received_ms;
            Arc::new(email)
        };
        let judge = |email: Arc<crate::model::Email>| {
            let (service, world) = (&service, world.clone());
            async move {
                let report = service.report_under(email, world, false).await.unwrap();
                dkim(&report)
            }
        };
        // The second is the floor of the millisecond receipt: x-1 s and 999 ms is
        // still second x-1, and x s exactly is the expiry.
        let before = (x - 1) as i64 * 1000 + 999;
        assert_eq!(judge(at(before)).await, vec![]);
        assert_eq!(
            judge(at(x as i64 * 1000)).await,
            one("dkim.signature_expired.v1", 0, ERROR)
        );
        // A negative receipt clamps to second zero, before any expiry.
        assert_eq!(judge(at(-5_000)).await, vec![]);
        assert_eq!(judge(at(i64::MIN)).await, vec![]);
    }

    #[test]
    fn guide_lists_every_dkim_rule() {
        let guide = include_str!("../../docs/guide/deliverability.md");
        let mut seen = BTreeSet::new();
        for rule in Rule::ALL {
            assert!(seen.insert(rule.id()), "{} twice", rule.id());
            assert!(rule.id().starts_with("dkim."), "{}", rule.id());
            assert!(rule.id().ends_with(".v1"), "{}", rule.id());
            let heading = format!("### `{}`\n", rule.id());
            let start = guide.find(&heading).expect("a heading per rule");
            let rest = &guide[start + heading.len()..];
            let section = &rest[..rest.find("\n##").unwrap()];
            let word = format!("{:?}", rule.severity()).to_lowercase();
            assert!(
                section.contains(&format!("**Severity:** {word}.")),
                "{}",
                rule.id()
            );
            assert!(section.contains(rule.hint()), "{}", rule.id());
            assert!(section.contains("**Passing:**"));
            assert!(section.contains("**Failing:**"));
            // A finding is the rule's id, severity and hint on the field's header.
            let finding = rule.finding(3);
            assert_eq!(finding.rule_id, rule.id());
            assert_eq!(finding.severity, rule.severity());
            assert_eq!(finding.fix_hint, rule.hint());
            assert_eq!(
                finding.location,
                Location::Header {
                    name: "dkim-signature".into(),
                    occurrence: Some(3)
                }
            );
            assert!(rule.hint().ends_with('.'));
            assert!(!rule.hint().contains('\n'));
        }
        assert_eq!(seen.len(), 18);
        // No rule is documented that the table lacks, and only one is a warning.
        assert_eq!(guide.matches("\n### `dkim.").count(), 18);
        let warnings = Rule::ALL
            .iter()
            .filter(|r| r.severity() == Severity::Warning);
        assert_eq!(warnings.count(), 1);
        for fact in [
            "## DKIM verification (`swarmail-html/6`)",
            "**Time is the receipt time.**",
            "`dkim.expired_since_receipt.v1`",
            "**The clock residual (its full radius).**",
            "**One body, one field list.**",
            "interop gap:**",
            "At most 16 `DKIM-Signature` fields",
            "at most 128 names",
            "`swarmail-html/6` (the eighteen DKIM rules of proposed 0125",
        ] {
            assert!(guide.contains(fact), "{fact}");
        }
        // The bounds the guide states are the bounds the code enforces.
        assert_eq!((MAX_DKIM_SIGNATURES, MAX_DKIM_SIGNED_HEADERS), (16, 128));
    }

    #[test]
    fn forged_signatures_verify_for_both_algorithms() {
        // The kit's own proof that explicit tags can be validly signed, so the
        // expiry fixtures above fail for their tags and not for a bad signature.
        for kind in Kind::ALL {
            let o = Sign {
                kind,
                ..Sign::default()
            };
            // `x` is derived from the signer's own `t`, and the receipt is before
            // it, so no calendar date decides the verdict.
            let t = parsed(&signed(&o, &msg())).t;
            let tags = format!("t={t}; x={}", t + 86_400);
            let raw = forged(&o, &tags, &msg());
            assert_eq!(verdicts(&raw, t, &key_of(kind)), vec![], "{kind:?}");
            // And the same bytes with a changed Subject do not verify.
            let altered = String::from_utf8(raw)
                .unwrap()
                .replace("Subject: Hello", "Subject: Hellp");
            assert_eq!(
                verdicts(altered.as_bytes(), t, &key_of(kind)),
                one("dkim.signature_mismatch.v1", 0, ERROR),
                "{kind:?}"
            );
        }
    }

    #[test]
    fn findings_past_the_report_cap_fail_the_analysis() {
        // The collector's cap is shared with every other collector: a full report
        // cannot take one more DKIM finding, and the analysis fails rather than
        // truncating (decision 0085).
        let full = || {
            Collector(vec![
                Finding {
                    rule_id: "x".into(),
                    location: Location::MimePart { path: vec![] },
                    severity: Severity::Info,
                    fix_hint: String::new(),
                };
                super::super::MAX_FINDINGS
            ])
        };
        let mut lookups = Lookups::new(policy(&[], &[]));
        let run = |raw: &[u8], lookups: &mut Lookups| {
            let (root, size) = super::super::headers(raw).unwrap();
            collect(raw, &root, size, 0, lookups, &mut full())
        };
        let no_key = signed(&Sign::default(), &msg());
        assert_eq!(run(&no_key, &mut lookups), Err(AnalysisError::Failed));
        // The bound finding and the boundary finding are pushed the same way.
        let seventeen = many_fields(MAX_DKIM_SIGNATURES + 1);
        assert_eq!(run(&seventeen, &mut lookups), Err(AnalysisError::Failed));
        assert_eq!(
            run(&stray_line_after(), &mut lookups),
            Err(AnalysisError::Failed)
        );
        // A message with no signature adds nothing, so a full report stays as is.
        assert_eq!(run(&msg(), &mut lookups), Ok(()));
    }
}
