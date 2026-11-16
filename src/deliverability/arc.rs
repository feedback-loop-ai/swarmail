//! ARC chain validation (issue #256, decision 0132).
//!
//! The chain of a stored message is validated by `mail-auth` against keys that
//! come from one place only: the declared DNS map, through `Lookups` (decision
//! 0120), by the adapter DKIM already uses (decision 0125). The crate parses the
//! three field families, sorts them by instance, checks the instance sequence and
//! the `cv=` values, hashes the body and checks every signature and seal. This
//! module owns what the crate cannot say or settles silently, and everything that
//! turns its result into one stable, versioned finding and the report's typed
//! `arc` member.
//!
//! The steps run in this order and the first that applies decides; the
//! specification's evaluation order is normative:
//!
//! 0. no ARC field anywhere: `none`;
//! 1. one body and one field list, on every path (mail with no `DKIM-Signature`
//!    included);
//! 2. the bound on `h=` names;
//! 3. each field judged in wire order: what the crate rejected, and what it would
//!    default (a missing `a=`, a repeated tag, a garbage number, an `i=` read as
//!    its low 32 bits), `rsa-sha1` and any `l=`;
//! 4. complete sets;
//! 5. distinct instances (the crate sorts with an unstable sort, so with two
//!    fields of one instance which it reads is not defined by the bytes);
//! 6. expiry, judged at the receipt second, never at the wall clock;
//! 7. the crate's verification, mapped by one `match`.
//!
//! Time. Swarmail reads no clock. `verify_arc` reads `SystemTime::now()` once,
//! after its count checks, and uses it in one place only: the expiry test of the
//! newest signature, `x == 0 || (x > t && x > now)`. Step 6 decides every chain
//! whose newest signature carries an `x=` tag, so the crate is called only when
//! its `x` is 0 and the left side of that test is true: the value it read is
//! never compared (decision 0132 section 5, held for the operator).
//!
//! The failing seal. The crate looks up the newest signature's key first, then
//! each seal's key from the newest down, once each, and stops at the first
//! failure. The number of names it asked for in the one call names the field.

use super::dkim::{AUTHENTICATOR, Adapter, crate_order};
use super::{AnalysisError, Collector, Finding, Headers, Location, Severity};
use futures_util::FutureExt;
use mail_auth::arc::{ArcError, ChainValidation};
use mail_auth::crypto::{Algorithm, CryptoError};
use mail_auth::{AuthenticatedMessage, DkimResult, DnsError, Error, Parameters};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// More `h=` names than this in one `ARC-Message-Signature` are a bound, not a
/// verdict. The crate's header selection is O(names x headers).
const MAX_SIGNED_HEADERS: usize = 128;
/// RFC 8617 section 4.2.1: instances run from 1 to 50.
const MAX_INSTANCE: u32 = 50;

/// The chain validation state. The first three are RFC 8617 section 5.2's and the
/// token `Authentication-Results` carries (`arc=none`, `arc=pass`, `arc=fail`);
/// `NotEvaluated` is Swarmail's, for a chain it declined to judge: no `arc=`
/// clause corresponds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArcState {
    #[default]
    None,
    Pass,
    Fail,
    NotEvaluated,
}

/// The report's `arc` member (decision 0132 section 2). The evaluator computes it
/// once; REST, MCP and the message page render that value and none derives it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArcChain {
    pub state: ArcState,
    /// The largest number of fields any one ARC family holds, in the root header
    /// list or in the crate's parse, whichever is larger.
    pub sets: u64,
    /// The instance `i=` that broke the chain; only with state `fail`.
    pub failing_instance: Option<u32>,
    /// The rule id of the report's one `arc.*` finding.
    pub reason: Option<String>,
}

/// The three field families, in the order ties are broken (decision 0132's rule
/// table): `arc-authentication-results`, `arc-message-signature`, `arc-seal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Family {
    Results,
    Signature,
    Seal,
}

impl Family {
    const ALL: [Family; 3] = [Self::Results, Self::Signature, Self::Seal];

    const fn name(self) -> &'static str {
        match self {
            Self::Results => "arc-authentication-results",
            Self::Signature => "arc-message-signature",
            Self::Seal => "arc-seal",
        }
    }

    /// The family of a root header name, which Swarmail's scanner has already
    /// lowercased and which holds no whitespace.
    fn of_root(name: &[u8]) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|family| family.name().as_bytes() == name)
    }

    /// The family the crate gave a field it rejected. The crate folds the name
    /// (ignoring spaces, tabs and line breaks inside it, and case) and takes
    /// `ARC-Seal` by its first eight characters; only fields the crate itself
    /// classified as authentication fields are ever asked about here, so this
    /// agrees with it on those.
    fn of_crate(name: &[u8]) -> Option<Self> {
        let folded = tag_name(name);
        match folded.as_slice() {
            b"arc-message-signature" => Some(Self::Signature),
            b"arc-authentication-results" => Some(Self::Results),
            other => other.starts_with(b"arc-seal").then_some(Self::Seal),
        }
    }
}

/// The closed rule table. The guide lists the same ids and a test binds the two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rule {
    HeaderBoundaryAmbiguous,
    LimitsExceeded,
    Malformed,
    Unsupported,
    SetIncomplete,
    SignatureExpired,
    InstanceInvalid,
    CvInvalid,
    BodyHashMismatch,
    KeyMissing,
    KeyInvalid,
    SignatureMismatch,
    NotEvaluated,
}

impl Rule {
    #[cfg(test)]
    const ALL: [Rule; 13] = [
        Self::HeaderBoundaryAmbiguous,
        Self::LimitsExceeded,
        Self::Malformed,
        Self::Unsupported,
        Self::SetIncomplete,
        Self::SignatureExpired,
        Self::InstanceInvalid,
        Self::CvInvalid,
        Self::BodyHashMismatch,
        Self::KeyMissing,
        Self::KeyInvalid,
        Self::SignatureMismatch,
        Self::NotEvaluated,
    ];

    const fn id(self) -> &'static str {
        match self {
            Self::HeaderBoundaryAmbiguous => "arc.header_boundary_ambiguous.v1",
            Self::LimitsExceeded => "arc.limits_exceeded.v1",
            Self::Malformed => "arc.malformed.v1",
            Self::Unsupported => "arc.unsupported.v1",
            Self::SetIncomplete => "arc.set_incomplete.v1",
            Self::SignatureExpired => "arc.signature_expired.v1",
            Self::InstanceInvalid => "arc.instance_invalid.v1",
            Self::CvInvalid => "arc.cv_invalid.v1",
            Self::BodyHashMismatch => "arc.body_hash_mismatch.v1",
            Self::KeyMissing => "arc.key_missing.v1",
            Self::KeyInvalid => "arc.key_invalid.v1",
            Self::SignatureMismatch => "arc.signature_mismatch.v1",
            Self::NotEvaluated => "arc.not_evaluated.v1",
        }
    }

    /// The chain state the rule gives: `not_evaluated` when Swarmail declined to
    /// judge, `fail` when the chain is observed broken.
    const fn state(self) -> ArcState {
        match self {
            Self::HeaderBoundaryAmbiguous | Self::LimitsExceeded | Self::NotEvaluated => {
                ArcState::NotEvaluated
            }
            _ => ArcState::Fail,
        }
    }

    /// The fix hints are constants: they never echo a domain or a value from the
    /// message, and hold no character that a JSON string or the page must escape.
    const fn hint(self) -> &'static str {
        match self {
            Self::HeaderBoundaryAmbiguous => {
                "A line in the header block that is not a header field (no colon, or only whitespace at the very top), or an ARC field name that not every parser accepts (such as a space before the colon), ends or splits the headers differently at different receivers, so the chain was not evaluated; remove it, fold it into the previous field, or write the name as ARC-Seal:, ARC-Message-Signature: or ARC-Authentication-Results: (RFC 5322 sections 2.2 and 2.2.3, RFC 8617 section 4.1)."
            }
            Self::LimitsExceeded => {
                "Sign at most 128 header names in the h= tag of each ARC-Message-Signature; the chain was not evaluated (RFC 8617 section 4.1.2)."
            }
            Self::Malformed => {
                "Fix the ARC field: each tag once; a= present; t= and x= plain decimal numbers; an i= instance from 1 to 50, written first in an ARC-Authentication-Results; valid base64, cv=, d=, s= and b= as RFC 8617 requires, and no h= in an ARC-Seal (RFC 8617 sections 4.1.1 to 4.1.3)."
            }
            Self::Unsupported => {
                "Use a= rsa-sha256 or ed25519-sha256 and no l= tag in an ARC-Message-Signature: a body length limit lets text be appended unsigned (RFC 8617 section 4.1.2, RFC 8301 section 3.1, RFC 6376 section 8.2)."
            }
            Self::SetIncomplete => {
                "Every instance needs exactly one ARC-Seal, one ARC-Message-Signature and one ARC-Authentication-Results; the three families hold different numbers of fields (RFC 8617 section 4.2)."
            }
            Self::SignatureExpired => {
                "The newest ARC-Message-Signature has expired: its x= is at or before the time the message was received, or at or before its t=; seal again with a later x= or none (RFC 8617 section 4.1.2, RFC 6376 section 3.5)."
            }
            Self::InstanceInvalid => {
                "The i= instances must be distinct and run 1, 2, 3 and so on, up to 50, in all three families (RFC 8617 sections 4.2.1 and 5.2)."
            }
            Self::CvInvalid => {
                "The ARC-Seal of instance 1 needs cv=none and every later seal needs cv=pass; a seal with cv=fail or any other value ends the chain (RFC 8617 sections 4.1.3 and 5.2)."
            }
            Self::BodyHashMismatch => {
                "The body differs from what the newest ARC-Message-Signature signed, so a hop changed it after sealing; seal after the last change, with the same c= (RFC 8617 section 5.2, RFC 6376 section 3.7)."
            }
            Self::KeyMissing => {
                "Declare a TXT record at the selector._domainkey.domain name of the field named in the finding in the DNS policy; none is declared, or the name is not a valid DNS name (RFC 8617 section 5.2, RFC 6376 section 3.6.2.1)."
            }
            Self::KeyInvalid => {
                "Publish a v=DKIM1 key record with a valid, non-empty p= and a k= that matches the a= of the field named in the finding (RFC 6376 section 3.6.1, RFC 8463)."
            }
            Self::SignatureMismatch => {
                "The field named in the finding is the newest signature or seal that does not verify against its declared key: a sealed header or an older ARC set changed after sealing, b= was made with another key, or p= is not the right public key (RFC 8617 section 5.2)."
            }
            Self::NotEvaluated => {
                "The chain was not evaluated: a key lookup failed temporarily, or the newest ARC-Message-Signature carries an x= later than the receipt time, which Swarmail does not verify; fix the DNS answer or seal without x=, and retry (RFC 8617 section 5.2)."
            }
        }
    }

    fn finding(self, family: Family, occurrence: Option<u64>) -> Finding {
        Finding {
            rule_id: self.id().into(),
            location: Location::Header {
                name: family.name().into(),
                occurrence,
            },
            severity: Severity::Error,
            fix_hint: self.hint().into(),
            weight: 0,
        }
    }
}

/// What the crate parsed of one field that it accepted.
#[derive(Debug, Clone, Copy)]
struct Read {
    instance: u32,
    sha1: bool,
}

/// One ARC field as the crate lists it: parsed, or rejected with its error.
struct Field<'m> {
    family: Family,
    value: &'m [u8],
    outcome: Result<Read, &'m Error>,
}

/// The crate's own view of the three families, parsed and rejected, in wire
/// order, so that its lists and ours line up by construction.
fn fields<'m>(message: &'m AuthenticatedMessage<'m>) -> Vec<Field<'m>> {
    let parsed = |family, value, instance, algorithm: Algorithm| Field {
        family,
        value,
        outcome: Ok(Read {
            instance,
            sha1: algorithm == Algorithm::RsaSha1,
        }),
    };
    let mut fields: Vec<Field<'m>> = message
        .arc_seals()
        .iter()
        .map(|h| parsed(Family::Seal, h.value, h.header.i, h.header.a))
        .chain(
            message
                .arc_message_signatures()
                .iter()
                .map(|h| parsed(Family::Signature, h.value, h.header.i, h.header.a)),
        )
        .chain(
            message
                .arc_authentication_results()
                .iter()
                .map(|h| parsed(Family::Results, h.value, h.header.i, Algorithm::RsaSha256)),
        )
        .chain(message.errors().iter().filter_map(|h| {
            Some(Field {
                family: Family::of_crate(h.name)?,
                value: h.value,
                outcome: Err(&h.header),
            })
        }))
        .collect();
    fields.sort_by_key(|field| crate_order(field.value));
    fields
}

/// What the field's tag list says, read once more, narrowly (RFC 6376 section
/// 3.2, RFC 8617 section 4.1): only what the crate defaults silently.
#[derive(Debug, Default, PartialEq, Eq)]
struct Scan {
    duplicate: bool,
    has_a: bool,
    has_l: bool,
    /// Every `t=` and `x=` is one or more ASCII digits that fit 64 bits, once the
    /// folding whitespace around it is trimmed. Then the crate's value is the
    /// literal one.
    numbers_ok: bool,
    h_names: usize,
    /// The `i=` literal: ASCII digits with a value of 1 to 50. For an
    /// `ARC-Authentication-Results` it is the leading `i=` before the first `;`.
    instance: Option<u32>,
    x: Option<u64>,
    t: Option<u64>,
}

/// A plain decimal number that fits 64 bits, with the folding whitespace trimmed.
fn number(value: &[u8]) -> Option<u64> {
    let digits = value.trim_ascii();
    digits
        .iter()
        .all(u8::is_ascii_digit)
        .then(|| String::from_utf8_lossy(digits).parse().ok())
        .flatten()
}

/// A name as the crate reads it: whitespace ignored, case folded.
fn tag_name(name: &[u8]) -> Vec<u8> {
    name.iter()
        .filter(|b| !b.is_ascii_whitespace())
        .map(u8::to_ascii_lowercase)
        .collect()
}

fn instance_of(value: &[u8]) -> Option<u32> {
    number(value)
        .filter(|n| (1..=u64::from(MAX_INSTANCE)).contains(n))
        .and_then(|n| u32::try_from(n).ok())
}

/// Split a tag at its first `=`: its folded name and its value.
fn tag(tag: &[u8]) -> Option<(Vec<u8>, &[u8])> {
    let eq = tag.iter().position(|b| *b == b'=')?;
    Some((tag_name(&tag[..eq]), &tag[eq + 1..]))
}

fn scan(family: Family, value: &[u8]) -> Scan {
    let mut scan = Scan {
        numbers_ok: true,
        ..Scan::default()
    };
    // An ARC-Authentication-Results is `i=<n>;` and then free text: its instance
    // is its first tag, which is where the crate looks first.
    if family == Family::Results {
        let leading = value.split(|b| *b == b';').next().and_then(tag);
        scan.instance = leading
            .filter(|(name, _)| name == b"i")
            .and_then(|(_, tag_value)| instance_of(tag_value));
        return scan;
    }
    let mut seen = BTreeSet::new();
    for (name, tag_value) in value.split(|b| *b == b';').filter_map(tag) {
        scan.duplicate |= !name.is_empty() & !seen.insert(name.clone());
        match name.as_slice() {
            b"a" => scan.has_a = true,
            b"l" => scan.has_l = true,
            b"h" => scan.h_names += tag_value.iter().filter(|b| **b == b':').count() + 1,
            b"i" => scan.instance = scan.instance.or_else(|| instance_of(tag_value)),
            b"t" | b"x" => {
                let parsed = number(tag_value);
                scan.numbers_ok &= parsed.is_some();
                if name == b"t" {
                    scan.t = parsed;
                } else {
                    scan.x = parsed;
                }
            }
            _ => {}
        }
    }
    scan
}

/// How many `ARC-Message-Signature` fields of the strict parse carry an `l=` tag,
/// parsed or rejected: the ones DKIM's non-strict re-parse would hash again.
pub(super) fn message_signatures_with_l(message: &AuthenticatedMessage<'_>) -> usize {
    fields(message)
        .iter()
        .filter(|field| field.family == Family::Signature && scan(field.family, field.value).has_l)
        .count()
}

/// The verdict of step 3 for one field, or `None` when the field is sound: the
/// rule, and the field's own instance literal when it is 1 to 50.
fn field_verdict(field: &Field<'_>, scan: &Scan) -> Option<(Rule, Option<u32>)> {
    let own = scan.instance;
    // Everything the crate refused, except the body-length tag, which step 3
    // reports as unsupported whatever the crate did with the field.
    let rejected = field
        .outcome
        .is_err_and(|error| *error != Error::Arc(ArcError::BodyLengthTag));
    // What the crate would read as a default or as absent.
    let tags =
        (field.family != Family::Results) & (scan.duplicate | !scan.has_a | !scan.numbers_ok);
    // The instance literal must be a number from 1 to 50 that the crate read the
    // same way; it keeps only the low 32 bits of a longer one.
    let instance = own.is_none() | field.outcome.is_ok_and(|read| Some(read.instance) != own);
    if rejected | tags | instance {
        return Some((Rule::Malformed, own));
    }
    let length = (field.family == Family::Signature) & scan.has_l;
    let sha1 = field.outcome.is_ok_and(|read| read.sha1);
    (length | sha1).then_some((Rule::Unsupported, own))
}

/// One field that passed step 3, with what steps 4 to 7 need.
#[derive(Debug, Clone, Copy)]
struct Entry {
    family: Family,
    instance: u32,
    /// The index among the root list's fields of this family, in wire order.
    occurrence: u64,
    x: Option<u64>,
    t: Option<u64>,
}

/// Push the one finding and build the member. `failing` is dropped unless the
/// rule's state is `fail`: a chain that was not evaluated broke at no instance.
fn settle(
    out: &mut Collector,
    rule: Rule,
    at: (Family, Option<u64>),
    failing: Option<u32>,
    sets: u64,
) -> Result<ArcChain, AnalysisError> {
    out.push(rule.finding(at.0, at.1))?;
    Ok(ArcChain {
        state: rule.state(),
        sets,
        failing_instance: failing.filter(|_| rule.state() == ArcState::Fail),
        reason: Some(rule.id().into()),
    })
}

/// The field whose key the crate was asking for when it stopped, from the number
/// of names it asked for in the one call: the newest signature's first, then each
/// seal's from the newest down. `signatures` and `seals` are sorted by instance.
fn asked_for(asked: u64, signatures: &[Entry], seals: &[Entry]) -> Entry {
    let seals_asked = usize::try_from(asked).map_or(usize::MAX, |n| n.saturating_sub(1));
    if seals_asked == 0 {
        signatures[signatures.len() - 1]
    } else {
        seals[seals.len() - seals_asked]
    }
}

/// Validate the chain of one stored message. `strict` is the crate's strict parse
/// (`None` when its boundary guard refused it or it could not read the message),
/// `root` and `size` the header list and body offset every other collector uses.
pub(super) fn evaluate(
    strict: Option<&AuthenticatedMessage<'_>>,
    root: &Headers,
    size: usize,
    adapter: &Adapter<'_, '_>,
    receipt: u64,
    out: &mut Collector,
) -> Result<ArcChain, AnalysisError> {
    let fields = strict.map_or_else(Vec::new, fields);
    let counts = |of: &dyn Fn(Family) -> usize| Family::ALL.map(of);
    let crate_counts = counts(&|f| fields.iter().filter(|field| field.family == f).count());
    let root_counts = counts(&|f| {
        root.iter()
            .filter(|(name, _)| Family::of_root(name) == Some(f))
            .count()
    });
    let sets = crate_counts
        .iter()
        .chain(&root_counts)
        .copied()
        .max()
        .map_or(0, |most| most as u64);
    // Step 0: no ARC field in either list is no chain.
    if sets == 0 {
        return Ok(ArcChain::default());
    }
    // Step 1: one body and one field list.
    let Some(message) =
        strict.filter(|message| message.body_offset() == size && crate_counts == root_counts)
    else {
        let first = root.iter().find_map(|(name, _)| Family::of_root(name));
        let at = (first.unwrap_or(Family::Seal), first.map(|_| 0));
        return settle(out, Rule::HeaderBoundaryAmbiguous, at, None, sets);
    };
    let scans: Vec<Scan> = fields.iter().map(|f| scan(f.family, f.value)).collect();
    let mut seen = [0u64; 3];
    let occurrences: Vec<u64> = fields
        .iter()
        .map(|field| {
            let slot = &mut seen[field.family as usize];
            *slot += 1;
            *slot - 1
        })
        .collect();
    let located = |index: usize| (fields[index].family, Some(occurrences[index]));
    // Step 2: the bound.
    let over = fields.iter().zip(&scans).position(|(field, scan)| {
        field.family == Family::Signature && scan.h_names > MAX_SIGNED_HEADERS
    });
    if let Some(index) = over {
        return settle(out, Rule::LimitsExceeded, located(index), None, sets);
    }
    // Step 3: the first field in wire order with a verdict decides.
    let verdict = fields
        .iter()
        .zip(&scans)
        .enumerate()
        .find_map(|(index, (field, scan))| field_verdict(field, scan).map(|v| (index, v)));
    if let Some((index, (rule, instance))) = verdict {
        return settle(out, rule, located(index), instance, sets);
    }
    let entries: Vec<Entry> = fields
        .iter()
        .zip(&scans)
        .zip(&occurrences)
        .filter_map(|((field, scan), occurrence)| {
            field.outcome.ok().map(|read| Entry {
                family: field.family,
                instance: read.instance,
                occurrence: *occurrence,
                x: scan.x,
                t: scan.t,
            })
        })
        .collect();
    // Step 4: complete sets. The lowest instance the families disagree on is the
    // one named; the family with the fewest fields is where it is missing.
    if !root_counts.iter().all(|n| *n == root_counts[0]) {
        let family = Family::ALL
            .into_iter()
            .min_by_key(|f| root_counts[*f as usize])
            .unwrap_or(Family::Seal);
        let uneven = |instance: &u32| {
            let held = Family::ALL.map(|f| {
                entries
                    .iter()
                    .filter(|e| e.family == f && e.instance == *instance)
                    .count()
            });
            !held.iter().all(|n| *n == held[0])
        };
        let instance = (1..=MAX_INSTANCE).find(uneven);
        return settle(out, Rule::SetIncomplete, (family, None), instance, sets);
    }
    // Step 5: distinct instances, in every family.
    let mut instances: BTreeMap<(u32, Family), Vec<u64>> = BTreeMap::new();
    for entry in &entries {
        instances
            .entry((entry.instance, entry.family))
            .or_default()
            .push(entry.occurrence);
    }
    if let Some(((instance, family), occurrences)) = instances.iter().find(|(_, o)| o.len() > 1) {
        let at = (*family, Some(occurrences[1]));
        return settle(out, Rule::InstanceInvalid, at, Some(*instance), sets);
    }
    let sorted = |family: Family| {
        let mut list: Vec<Entry> = entries
            .iter()
            .copied()
            .filter(|e| e.family == family)
            .collect();
        list.sort_by_key(|entry| entry.instance);
        list
    };
    let (results, signatures, seals) = (
        sorted(Family::Results),
        sorted(Family::Signature),
        sorted(Family::Seal),
    );
    let newest = signatures[signatures.len() - 1];
    let at_newest = (Family::Signature, Some(newest.occurrence));
    // Step 6: expiry, at the receipt second. Whether the crate would call the
    // signature expired is its wall clock's business; Swarmail never asks.
    if let Some(x) = newest.x {
        let expired = x <= receipt || newest.t.is_some_and(|t| x <= t);
        let rule = if expired {
            Rule::SignatureExpired
        } else {
            Rule::NotEvaluated
        };
        return settle(out, rule, at_newest, Some(newest.instance), sets);
    }
    // Step 7: the crate. The newest signature has no `x=`, so its expiry test
    // is `0 == 0` and the clock it read is never compared.
    let before = adapter.calls();
    let output = AUTHENTICATOR
        .verify_arc(Parameters::new(message).with_cache(adapter))
        .now_or_never()
        .expect("a declared map answers on the first poll");
    let key = asked_for(adapter.calls() - before, &signatures, &seals);
    let (rule, entry, failing) = match output.result() {
        DkimResult::Pass => {
            return Ok(ArcChain {
                state: ArcState::Pass,
                sets,
                failing_instance: None,
                reason: None,
            });
        }
        // The first of seal, signature and results at sorted position `p` whose
        // instance is not `p`.
        DkimResult::Fail(Error::Arc(ArcError::InvalidInstance(p))) => {
            let at = (*p as usize).saturating_sub(1);
            let entry = [&seals, &signatures, &results]
                .map(|list| list[at])
                .into_iter()
                .find(|entry| entry.instance != *p)
                .unwrap_or(seals[at]);
            (Rule::InstanceInvalid, entry, *p)
        }
        // The first seal, by instance, whose `cv=` the crate refuses.
        DkimResult::Fail(Error::Arc(ArcError::InvalidChainValidation)) => {
            let bad = message
                .arc_seals()
                .iter()
                .enumerate()
                .position(|(pos, seal)| {
                    seal.header.cv
                        != if pos == 0 {
                            ChainValidation::None
                        } else {
                            ChainValidation::Pass
                        }
                });
            let entry = seals[bad.unwrap_or(0)];
            (Rule::CvInvalid, entry, entry.instance)
        }
        DkimResult::Neutral(Error::Arc(ArcError::BodyHashMismatch)) => {
            (Rule::BodyHashMismatch, newest, newest.instance)
        }
        DkimResult::TempError(_) => (Rule::NotEvaluated, key, key.instance),
        DkimResult::PermError(Error::Dns(DnsError::RecordNotFound(_))) => {
            (Rule::KeyMissing, key, key.instance)
        }
        DkimResult::PermError(_)
        | DkimResult::Fail(Error::Crypto(CryptoError::IncompatibleAlgorithms)) => {
            (Rule::KeyInvalid, key, key.instance)
        }
        // The crate does not check key bytes before use, and the ed25519 backend
        // words a failed check as a library error, not `FailedVerification`: any
        // other failure is a signature that does not verify against the key.
        _ => (Rule::SignatureMismatch, key, key.instance),
    };
    settle(
        out,
        rule,
        (entry.family, Some(entry.occurrence)),
        Some(failing),
        sets,
    )
}

#[cfg(test)]
mod tests {
    use super::super::auth::{self, Envelope};
    use super::super::dkim::tests::{
        BODY, Kind, RFC8463_ED25519_PUBLIC, RFC8463_RSA1024_PUBLIC, RSA2048_PRIVATE, Sign, ed25519,
        message, policy, rsa, signed,
    };
    use super::super::dns::{Effective, Lookups, Outcome};
    use super::super::headers;
    use super::super::{Inputs, Report, evaluate_at};
    use super::*;
    use mail_auth::AuthenticationResults;
    use mail_auth::arc::ArcSealer;
    use mail_auth::crypto::{Ed25519Key, RsaKey, Sha256, SigningKey};
    use mail_auth::headers::HeaderWriter;
    use std::cell::RefCell;
    use std::sync::Arc;

    /// The receipt second every fixture is judged at, unless a test says another.
    const RECEIPT: u64 = 1_800_000_000;

    /// One forwarder: its own selector and domain, and the kind of key it seals
    /// with. The two private keys are the DKIM kit's, committed test material.
    #[derive(Clone, Copy)]
    struct Hop {
        kind: Kind,
        domain: &'static str,
        selector: &'static str,
    }

    const HOPS: [Hop; 4] = [
        Hop {
            kind: Kind::Ed25519,
            domain: "fwd1.example",
            selector: "arc1",
        },
        Hop {
            kind: Kind::Ed25519,
            domain: "fwd2.example",
            selector: "arc2",
        },
        Hop {
            kind: Kind::Rsa2048,
            domain: "fwd3.example",
            selector: "arc3",
        },
        Hop {
            kind: Kind::Ed25519,
            domain: "fwd4.example",
            selector: "arc4",
        },
    ];

    impl Hop {
        fn name(self) -> String {
            format!("{}._domainkey.{}", self.selector, self.domain)
        }
    }

    /// A policy declaring each hop's own key.
    fn keys(hops: &[Hop]) -> Arc<Effective> {
        let records: Vec<(String, String)> = hops
            .iter()
            .map(|hop| (hop.name(), hop.kind.record()))
            .collect();
        let borrowed: Vec<(&str, String)> = records
            .iter()
            .map(|(name, value)| (name.as_str(), value.clone()))
            .collect();
        policy(&borrowed, &[])
    }

    /// The policy of `hops` with one name overridden or added.
    fn keys_with(hops: &[Hop], name: &str, value: &str) -> Arc<Effective> {
        let mut records: Vec<(String, String)> = hops
            .iter()
            .filter(|hop| hop.name() != name)
            .map(|hop| (hop.name(), hop.kind.record()))
            .collect();
        records.push((name.into(), value.into()));
        let borrowed: Vec<(&str, String)> = records
            .iter()
            .map(|(name, value)| (name.as_str(), value.clone()))
            .collect();
        policy(&borrowed, &[])
    }

    /// The policy of `hops` without the key of one.
    fn keys_without(hops: &[Hop], missing: Hop) -> Arc<Effective> {
        let rest: Vec<Hop> = hops
            .iter()
            .filter(|hop| hop.name() != missing.name())
            .copied()
            .collect();
        keys(&rest)
    }

    /// `ArcSealer` over either key, sealed from the previous hop's own passing
    /// `verify_arc` output, obtained through the same adapter as production.
    fn seal(raw: &[u8], hop: Hop) -> Vec<u8> {
        fn set<K: SigningKey<Hasher = Sha256>>(
            key: K,
            hop: Hop,
            message: &AuthenticatedMessage<'_>,
            previous: &mail_auth::ArcOutput<'_>,
        ) -> String {
            let results = AuthenticationResults::new("mx.example.net");
            // No expiration is ever configured: no kit fixture carries `x=`.
            let set = ArcSealer::from_key(key)
                .domain(hop.domain)
                .selector(hop.selector)
                .headers(["From", "To", "Subject", "Date"])
                .seal(message, &results, previous)
                .unwrap();
            set.to_header()
        }
        let world = keys(&HOPS);
        let mut lookups = Lookups::new(world);
        let shared = RefCell::new(&mut lookups);
        let adapter = Adapter::over(&shared);
        let parsed = AuthenticatedMessage::parse(raw).unwrap();
        let previous = AUTHENTICATOR
            .verify_arc(Parameters::new(&parsed).with_cache(&adapter))
            .now_or_never()
            .unwrap();
        let header = match hop.kind {
            Kind::Ed25519 => set::<Ed25519Key>(ed25519(), hop, &parsed, &previous),
            Kind::Rsa2048 => set::<RsaKey<Sha256>>(rsa(RSA2048_PRIVATE), hop, &parsed, &previous),
        };
        let mut sealed = header.into_bytes();
        sealed.extend_from_slice(raw);
        sealed
    }

    /// The fixture message sealed by the first `n` hops, oldest first. The newest
    /// set is the first in the header block, as a forwarder writes it.
    fn chain(n: usize) -> Vec<u8> {
        HOPS[..n]
            .iter()
            .fold(message(BODY), |raw, hop| seal(&raw, *hop))
    }

    /// The header block as fields (each with its line ending), and the rest from
    /// the blank line on.
    fn split(raw: &[u8]) -> (Vec<String>, String) {
        let text = String::from_utf8(raw.to_vec()).unwrap();
        let (head, rest) = text.split_once("\r\n\r\n").unwrap();
        let mut fields: Vec<String> = Vec::new();
        for line in head.split("\r\n") {
            match fields.last_mut() {
                Some(last) if line.starts_with([' ', '\t']) => {
                    last.push_str(line);
                    last.push_str("\r\n");
                }
                _ => fields.push(format!("{line}\r\n")),
            }
        }
        (fields, format!("\r\n{rest}"))
    }

    fn join(fields: &[String], rest: &str) -> Vec<u8> {
        let mut text: String = fields.concat();
        text.push_str(rest);
        text.into_bytes()
    }

    /// Rewrite the fields of a message.
    fn edit(raw: &[u8], change: impl FnOnce(&mut Vec<String>)) -> Vec<u8> {
        let (mut fields, rest) = split(raw);
        change(&mut fields);
        join(&fields, &rest)
    }

    /// Where the ARC field `name` of instance `i` stands in the field list.
    fn at(fields: &[String], name: &str, i: u32) -> usize {
        fields
            .iter()
            .position(|field| {
                field
                    .to_ascii_lowercase()
                    .starts_with(&format!("{name}: i={i};"))
            })
            .expect("the fixture holds the field")
    }

    /// Rewrite one tag text of one ARC field.
    fn change(raw: &[u8], name: &str, i: u32, from: &str, to: &str) -> Vec<u8> {
        edit(raw, |fields| {
            let index = at(fields, name, i);
            assert!(
                fields[index].contains(from),
                "{from} not in {}",
                fields[index]
            );
            fields[index] = fields[index].replacen(from, to, 1);
        })
    }

    /// Drop every field of one set.
    fn without_set(raw: &[u8], i: u32) -> Vec<u8> {
        edit(raw, |fields| {
            let doomed: Vec<usize> = [
                "arc-seal",
                "arc-message-signature",
                "arc-authentication-results",
            ]
            .iter()
            .map(|name| at(fields, name, i))
            .collect();
            let mut at = 0;
            fields.retain(|_| {
                at += 1;
                !doomed.contains(&(at - 1))
            });
        })
    }

    /// Drop one field.
    fn without(raw: &[u8], name: &str, i: u32) -> Vec<u8> {
        edit(raw, |fields| {
            let index = at(fields, name, i);
            fields.remove(index);
        })
    }

    /// The evaluated report. Every fixture passes through the member's invariants.
    fn report(raw: &[u8], receipt: u64, world: &Arc<Effective>) -> Report {
        let inputs = Inputs {
            dns: world.clone(),
            auth: None,
            ..Inputs::default()
        };
        let mut report = evaluate_at(raw, receipt, &Envelope::default(), &inputs).unwrap();
        // DMARC's own walk (0126) asks for `_dmarc` names of the From domain.
        report
            .dns
            .lookups
            .retain(|lookup| !lookup.name.starts_with("_dmarc."));
        members_hold(&report);
        report
    }

    /// The five invariants of the `arc` member (specification, "The arc report
    /// member"), checked on every report the tests build.
    fn members_hold(report: &Report) {
        let found: Vec<&Finding> = report
            .findings
            .iter()
            .filter(|f| f.rule_id.starts_with("arc."))
            .collect();
        let member = &report.arc;
        assert!(found.len() <= 1, "{found:?}");
        assert_eq!(
            member.state == ArcState::None,
            member.sets == 0,
            "{member:?}"
        );
        // Only a broken chain names the instance that broke it.
        assert!((member.state == ArcState::Fail) | member.failing_instance.is_none());
        match member.state {
            ArcState::None | ArcState::Pass => {
                assert!(found.is_empty() & member.reason.is_none(), "{report:?}");
                assert!(member.failing_instance.is_none());
                assert_eq!(member.state == ArcState::Pass, member.sets > 0);
            }
            ArcState::Fail | ArcState::NotEvaluated => {
                assert_eq!(found.len(), 1, "{report:?}");
                assert_eq!(member.reason.as_deref(), Some(found[0].rule_id.as_str()));
                let rule = Rule::ALL
                    .iter()
                    .find(|r| r.id() == found[0].rule_id)
                    .unwrap();
                assert_eq!(member.state, rule.state());
                assert_eq!(found[0].severity, Severity::Error);
            }
        }
    }

    /// What a test asserts of a failing report: the rule, the field and its root
    /// occurrence, and the instance.
    #[track_caller]
    fn expect(report: &Report, rule: Rule, at: (Family, Option<u64>), instance: Option<u32>) {
        let found: Vec<&Finding> = report
            .findings
            .iter()
            .filter(|f| f.rule_id.starts_with("arc."))
            .collect();
        assert_eq!(found.len(), 1, "{report:?}");
        assert_eq!(*found[0], {
            let mut finding = rule.finding(at.0, at.1);
            finding.weight = if rule == Rule::NotEvaluated { 0 } else { 10 };
            finding
        });
        assert_eq!(report.arc.state, rule.state());
        assert_eq!(report.arc.reason.as_deref(), Some(rule.id()));
        assert_eq!(report.arc.failing_instance, instance, "{:?}", report.arc);
    }

    fn lookups(report: &Report) -> Vec<String> {
        report.dns.lookups.iter().map(|l| l.name.clone()).collect()
    }

    fn passes(raw: &[u8], world: &Arc<Effective>, sets: u64) {
        let report = report(raw, RECEIPT, world);
        assert_eq!(
            report.arc,
            ArcChain {
                state: ArcState::Pass,
                sets,
                failing_instance: None,
                reason: None,
            }
        );
        assert!(
            !report
                .findings
                .iter()
                .any(|f| f.rule_id.starts_with("arc."))
        );
    }

    #[test]
    fn rule_table_is_closed() {
        let expected = [
            ("arc.header_boundary_ambiguous.v1", "not_evaluated"),
            ("arc.limits_exceeded.v1", "not_evaluated"),
            ("arc.malformed.v1", "fail"),
            ("arc.unsupported.v1", "fail"),
            ("arc.set_incomplete.v1", "fail"),
            ("arc.signature_expired.v1", "fail"),
            ("arc.instance_invalid.v1", "fail"),
            ("arc.cv_invalid.v1", "fail"),
            ("arc.body_hash_mismatch.v1", "fail"),
            ("arc.key_missing.v1", "fail"),
            ("arc.key_invalid.v1", "fail"),
            ("arc.signature_mismatch.v1", "fail"),
            ("arc.not_evaluated.v1", "not_evaluated"),
        ];
        assert_eq!(Rule::ALL.len(), expected.len());
        let mut hints = BTreeSet::new();
        for (rule, (id, state)) in Rule::ALL.into_iter().zip(expected) {
            assert_eq!(rule.id(), id);
            assert_eq!(serde_json::to_value(rule.state()).unwrap(), state);
            let name = id
                .strip_prefix("arc.")
                .and_then(|rest| rest.strip_suffix(".v1"));
            assert!(name.is_some_and(|n| n.bytes().all(|b| b.is_ascii_lowercase() | (b == b'_'))));
            let hint = rule.hint();
            // Nothing that a JSON string or the page must escape.
            assert!(!hint.is_empty() & hint.ends_with('.'));
            assert!(!hint.contains(['"', '<', '>', '&', '\'', '\\']));
            assert!(hint.chars().all(|c| !c.is_control()));
            assert!(hints.insert(hint), "{id} repeats a hint");
            let finding = rule.finding(Family::Seal, Some(3));
            assert_eq!(finding.severity, Severity::Error);
            assert_eq!(
                serde_json::to_value(&finding.location).unwrap(),
                serde_json::json!({"kind":"header","name":"arc-seal","occurrence":3})
            );
            // Weights follow 0122 section 2 with no exception of their own.
            let mut out = Collector(Vec::new());
            out.push(finding).unwrap();
            assert_eq!(
                out.0[0].weight,
                [10, 0][(rule == Rule::NotEvaluated) as usize]
            );
        }
    }

    #[test]
    fn member_serializes_in_key_order() {
        let json = |chain: ArcChain| serde_json::to_string(&chain).unwrap();
        assert_eq!(
            json(ArcChain::default()),
            r#"{"state":"none","sets":0,"failing_instance":null,"reason":null}"#
        );
        let member = |state, sets, failing_instance, reason: Option<&str>| ArcChain {
            state,
            sets,
            failing_instance,
            reason: reason.map(Into::into),
        };
        assert_eq!(
            json(member(ArcState::Pass, 2, None, None)),
            r#"{"state":"pass","sets":2,"failing_instance":null,"reason":null}"#
        );
        assert_eq!(
            json(member(
                ArcState::Fail,
                3,
                Some(2),
                Some("arc.key_missing.v1")
            )),
            r#"{"state":"fail","sets":3,"failing_instance":2,"reason":"arc.key_missing.v1"}"#
        );
        assert_eq!(
            json(member(
                ArcState::NotEvaluated,
                1,
                None,
                Some("arc.not_evaluated.v1")
            )),
            r#"{"state":"not_evaluated","sets":1,"failing_instance":null,"reason":"arc.not_evaluated.v1"}"#
        );
        // The member is closed: an unknown key is refused.
        assert!(
            serde_json::from_str::<ArcChain>(
                r#"{"state":"none","sets":0,"failing_instance":null,"reason":null,"x":1}"#
            )
            .is_err()
        );
    }

    #[test]
    fn kit_seals_verifiably() {
        for n in 1..=4 {
            let raw = chain(n);
            let world = keys(&HOPS[..n]);
            passes(&raw, &world, n as u64);
            // The newest signature's key and each seal's key: one name per hop.
            let report = report(&raw, RECEIPT, &world);
            let mut names: Vec<String> = HOPS[..n].iter().map(|hop| hop.name()).collect();
            names.sort();
            assert_eq!(lookups(&report), names);
            assert!(
                report
                    .dns
                    .lookups
                    .iter()
                    .all(|l| l.outcome == Outcome::Records)
            );
        }
    }

    #[test]
    fn no_arc_field_is_none() {
        let world = keys(&HOPS);
        let none = |raw: &[u8], world: &Arc<Effective>| {
            let report = report(raw, RECEIPT, world);
            assert_eq!(report.arc, ArcChain::default());
            assert!(
                !report
                    .findings
                    .iter()
                    .any(|f| f.rule_id.starts_with("arc."))
            );
            report
        };
        none(&message(BODY), &world);
        // Mail with a DKIM signature and no ARC field.
        let dkim = policy(
            &[("sel._domainkey.example.com", Kind::Ed25519.record())],
            &[],
        );
        let signed = signed(&Sign::default(), &message(BODY));
        none(&signed, &dkim);
        // Bytes the crate cannot parse as a message at all.
        none(b"garbage\r\n\r\nbody\r\n", &world);
        // A DKIM-guard stop with no ARC field anywhere is no chain either.
        let stray = edit(&signed, |fields| fields.push("stray line\r\n".into()));
        none(&stray, &dkim);
    }

    #[test]
    fn rule_header_boundary_ambiguous() {
        let world = keys(&HOPS);
        let first = chain(1);
        let ambiguous = |raw: &[u8], at: (Family, Option<u64>)| {
            let report = report(raw, RECEIPT, &world);
            expect(&report, Rule::HeaderBoundaryAmbiguous, at, None);
            assert_eq!(report.arc.sets, 1);
            assert!(lookups(&report).is_empty());
        };
        // The newest field of a forwarder is first: that is the first root field.
        let first_field = (Family::Seal, Some(0));
        // A colon-less line before the ARC fields ends the crate's headers early.
        let mut before = b"stray line\r\n".to_vec();
        before.extend_from_slice(&first);
        ambiguous(&before, first_field);
        // ...and one after them, between the fields and the rest.
        let after = edit(&first, |fields| fields.insert(3, "stray line\r\n".into()));
        ambiguous(&after, first_field);
        // A leading whitespace-only line.
        let mut blank = b" \r\n".to_vec();
        blank.extend_from_slice(&first);
        ambiguous(&blank, first_field);
        // A name with a space before the colon is an ARC-Seal to the crate and
        // no field to Swarmail's scan: beside a valid chain, the first root
        // field is the signature; alone, no root field exists to point at.
        let spaced = change(&first, "arc-seal", 1, "ARC-Seal:", "ARC-Seal :");
        ambiguous(&spaced, (Family::Signature, Some(0)));
        let alone = edit(&spaced, |fields| {
            fields.retain(|f| f.starts_with("ARC-Seal :") | !f.starts_with("ARC-"))
        });
        ambiguous(&alone, (Family::Seal, None));
        // A name the crate takes for a seal by its first eight characters.
        let prefixed = change(&first, "arc-seal", 1, "ARC-Seal:", "ARC-Sealed:");
        ambiguous(&prefixed, (Family::Signature, Some(0)));
        // The DKIM guard stopped first: ARC still names its own finding, and the
        // DKIM finding is DKIM's.
        let dkim_world = keys_with(&HOPS, "sel._domainkey.example.com", &Kind::Ed25519.record());
        let signed = signed(&Sign::default(), &first);
        let both = edit(&signed, |fields| fields.insert(4, "stray line\r\n".into()));
        let report = report(&both, RECEIPT, &dkim_world);
        expect(
            &report,
            Rule::HeaderBoundaryAmbiguous,
            first_field_after_dkim(),
            None,
        );
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.rule_id == "dkim.header_boundary_ambiguous.v1")
        );
    }

    /// The DKIM-Signature field is the first in `signed`, so the ARC-Seal is the
    /// first of its own family all the same.
    fn first_field_after_dkim() -> (Family, Option<u64>) {
        (Family::Seal, Some(0))
    }

    #[test]
    fn crate_lists_every_root_arc_field() {
        // Every spelling the root scanner names an ARC family: the crate lists
        // exactly those fields, in the same order, with the same unfolded values.
        let spellings = [
            [
                "ARC-Seal",
                "ARC-Message-Signature",
                "ARC-Authentication-Results",
            ],
            [
                "arc-seal",
                "arc-message-signature",
                "arc-authentication-results",
            ],
            [
                "ARC-SEAL",
                "ARC-MESSAGE-SIGNATURE",
                "ARC-AUTHENTICATION-RESULTS",
            ],
            [
                "Arc-Seal",
                "Arc-Message-Signature",
                "Arc-Authentication-Results",
            ],
        ];
        let unfolded = |value: &[u8]| {
            String::from_utf8_lossy(value)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        };
        for names in spellings {
            let raw = edit(&chain(2), |fields| {
                for field in fields.iter_mut() {
                    for (canon, spelled) in [
                        "ARC-Seal",
                        "ARC-Message-Signature",
                        "ARC-Authentication-Results",
                    ]
                    .iter()
                    .zip(names)
                    {
                        if field.starts_with(canon) {
                            field.replace_range(..canon.len(), spelled);
                        }
                    }
                }
            });
            let (root, size) = headers(&raw).unwrap();
            let parsed = AuthenticatedMessage::parse_with_opts(&raw, None, true).unwrap();
            assert_eq!(parsed.body_offset(), size);
            let listed = fields(&parsed);
            assert_eq!(listed.len(), 6);
            let ours: Vec<(Family, String)> = root
                .iter()
                .filter_map(|(name, value)| Family::of_root(name).map(|f| (f, unfolded(value))))
                .collect();
            let theirs: Vec<(Family, String)> = listed
                .iter()
                .map(|field| (field.family, unfolded(field.value)))
                .collect();
            assert_eq!(ours, theirs, "{names:?}");
        }
    }

    #[test]
    fn rule_limits_exceeded() {
        let world = keys(&HOPS);
        let names = |n: usize| {
            (0..n)
                .map(|i| format!("x-h{i}"))
                .collect::<Vec<_>>()
                .join(":")
        };
        let raw = chain(2);
        let with = |n: usize| {
            change(
                &raw,
                "arc-message-signature",
                2,
                "h=Date:Subject:To:From",
                &format!("h={}", names(n)),
            )
        };
        // 128 names are not a bound (the signature then fails on its own terms)...
        let at_bound = report(&with(128), RECEIPT, &world);
        expect(
            &at_bound,
            Rule::SignatureMismatch,
            (Family::Signature, Some(0)),
            Some(2),
        );
        // ...129 are, at the field that carries them.
        let over = report(&with(129), RECEIPT, &world);
        expect(
            &over,
            Rule::LimitsExceeded,
            (Family::Signature, Some(0)),
            None,
        );
        assert_eq!(over.arc.sets, 2);
        assert!(lookups(&over).is_empty());
        // An older signature's names count too, and the first field in wire order
        // is the one named.
        let older = change(
            &raw,
            "arc-message-signature",
            1,
            "h=Date:Subject:To:From",
            &format!("h={}", names(129)),
        );
        expect(
            &report(&older, RECEIPT, &world),
            Rule::LimitsExceeded,
            (Family::Signature, Some(1)),
            None,
        );
    }

    /// Replace one text of the message, once.
    fn retext(raw: &[u8], from: &str, to: &str) -> Vec<u8> {
        let text = String::from_utf8(raw.to_vec()).unwrap();
        assert!(text.contains(from), "{from} not in the fixture");
        text.replacen(from, to, 1).into_bytes()
    }

    /// Rewrite `tag=value;` of one ARC field: `to` replaces the whole tag text
    /// (the value runs to the next semicolon), and `""` removes it.
    fn retag(raw: &[u8], name: &str, i: u32, tag: &str, to: &str) -> Vec<u8> {
        edit(raw, |fields| {
            let index = at(fields, name, i);
            let field = &fields[index];
            let start = [format!(" {tag}="), format!("\t{tag}=")]
                .iter()
                .filter_map(|needle| field.find(needle.as_str()))
                .min()
                .expect("the fixture holds the tag")
                + 1;
            let end = start
                + field[start..]
                    .find(';')
                    .map_or(field.len() - start, |n| n + 1);
            fields[index] = format!("{}{to}{}", &field[..start], &field[end..]);
        })
    }

    /// Insert one tag text after the `h=` tag of a signature.
    fn tagged(raw: &[u8], name: &str, i: u32, tag: &str) -> Vec<u8> {
        change(
            raw,
            name,
            i,
            "h=Date:Subject:To:From;",
            &format!("h=Date:Subject:To:From; {tag};"),
        )
    }

    const SEAL: &str = "arc-seal";
    const MESSAGE_SIGNATURE: &str = "arc-message-signature";
    const RESULTS: &str = "arc-authentication-results";

    /// A label, the fixture, the field it names and the instance it reports.
    type Case = (&'static str, Vec<u8>, (Family, Option<u64>), Option<u32>);

    #[test]
    fn rule_malformed() {
        let world = keys(&HOPS);
        let one = chain(1);
        let sig = (Family::Signature, Some(0));
        let seal = (Family::Seal, Some(0));
        let results = (Family::Results, Some(0));
        let cases: Vec<Case> = vec![
            (
                "repeated tag",
                change(
                    &one,
                    MESSAGE_SIGNATURE,
                    1,
                    "c=relaxed/relaxed;",
                    "c=relaxed/relaxed; s=arc1;",
                ),
                sig,
                Some(1),
            ),
            (
                "repeated tag in a seal",
                change(&one, SEAL, 1, "cv=none;", "cv=none; d=fwd1.example;"),
                seal,
                Some(1),
            ),
            (
                "no a= in a signature",
                retag(&one, MESSAGE_SIGNATURE, 1, "a", ""),
                sig,
                Some(1),
            ),
            (
                "no a= in a seal",
                retag(&one, SEAL, 1, "a", ""),
                seal,
                Some(1),
            ),
            (
                "x=abc",
                tagged(&one, MESSAGE_SIGNATURE, 1, "x=abc"),
                sig,
                Some(1),
            ),
            (
                "t=abc",
                retag(&one, MESSAGE_SIGNATURE, 1, "t", "t=abc;"),
                sig,
                Some(1),
            ),
            (
                "t= past 64 bits",
                retag(&one, SEAL, 1, "i", "i=1; t=99999999999999999999;"),
                seal,
                Some(1),
            ),
            (
                "no i= in a signature",
                retag(&one, MESSAGE_SIGNATURE, 1, "i", ""),
                sig,
                None,
            ),
            (
                "i=0",
                retag(&one, MESSAGE_SIGNATURE, 1, "i", "i=0;"),
                sig,
                None,
            ),
            ("i=51", retag(&one, SEAL, 1, "i", "i=51;"), seal, None),
            (
                "i=4294967297 is 1 to the crate",
                retag(&one, MESSAGE_SIGNATURE, 1, "i", "i=4294967297;"),
                sig,
                None,
            ),
            (
                "i= not digits",
                retag(&one, SEAL, 1, "i", "i=1x;"),
                seal,
                None,
            ),
            (
                "i not first in results",
                retext(
                    &one,
                    "ARC-Authentication-Results: i=1; mx.example.net",
                    "ARC-Authentication-Results: mx.example.net; i=1",
                ),
                results,
                None,
            ),
            (
                "no i= in results",
                retext(
                    &one,
                    "ARC-Authentication-Results: i=1; mx.example.net",
                    "ARC-Authentication-Results: mx.example.net",
                ),
                results,
                None,
            ),
            (
                "a seal with h=",
                change(&one, SEAL, 1, "cv=none;", "cv=none; h=from;"),
                seal,
                Some(1),
            ),
            (
                "a seal with no cv=",
                retag(&one, SEAL, 1, "cv", ""),
                seal,
                Some(1),
            ),
            (
                "an unknown algorithm",
                retag(&one, MESSAGE_SIGNATURE, 1, "a", "a=nope;"),
                sig,
                Some(1),
            ),
        ];
        for (label, raw, at, instance) in cases {
            let report = report(&raw, RECEIPT, &world);
            expect(&report, Rule::Malformed, at, instance);
            assert_eq!(report.arc.sets, 1, "{label}");
            // The crate is not asked: nothing was looked up.
            assert!(lookups(&report).is_empty(), "{label}");
        }
    }

    #[test]
    fn rule_unsupported() {
        let world = keys(&HOPS);
        let one = chain(1);
        let sig = (Family::Signature, Some(0));
        let seal = (Family::Seal, Some(0));
        let cases = [
            ("l=5", tagged(&one, MESSAGE_SIGNATURE, 1, "l=5"), sig),
            ("l=0", tagged(&one, MESSAGE_SIGNATURE, 1, "l=0"), sig),
            (
                "l=garbage",
                tagged(&one, MESSAGE_SIGNATURE, 1, "l=abc"),
                sig,
            ),
            (
                "sha1 signature",
                retag(&one, MESSAGE_SIGNATURE, 1, "a", "a=rsa-sha1;"),
                sig,
            ),
            ("sha1 seal", retag(&one, SEAL, 1, "a", "a=rsa-sha1;"), seal),
        ];
        for (label, raw, at) in cases {
            let report = report(&raw, RECEIPT, &world);
            expect(&report, Rule::Unsupported, at, Some(1));
            assert!(lookups(&report).is_empty(), "{label}");
        }
        // An `l=` on a seal is no signature's body length: the crate ignores the
        // tag and so does Swarmail (the seal then fails on its own terms).
        let on_seal = change(&one, SEAL, 1, "cv=none;", "cv=none; l=5;");
        let report = report(&on_seal, RECEIPT, &world);
        expect(&report, Rule::SignatureMismatch, seal, Some(1));
    }

    #[test]
    fn first_field_in_wire_order_decides() {
        let world = keys(&HOPS);
        let two = chain(2);
        // Wire order: seal 2, signature 2, results 2, seal 1, signature 1, results 1.
        let unsupported_newer = tagged(&two, MESSAGE_SIGNATURE, 2, "l=5");
        let malformed_older = retag(&unsupported_newer, SEAL, 1, "cv", "");
        expect(
            &report(&malformed_older, RECEIPT, &world),
            Rule::Unsupported,
            (Family::Signature, Some(0)),
            Some(2),
        );
        let malformed_newer = retag(&two, SEAL, 2, "cv", "");
        let unsupported_older = tagged(&malformed_newer, MESSAGE_SIGNATURE, 1, "l=5");
        expect(
            &report(&unsupported_older, RECEIPT, &world),
            Rule::Malformed,
            (Family::Seal, Some(0)),
            Some(2),
        );
        // Inside one field a malformed tag outranks an unsupported one.
        let both = tagged(
            &tagged(&two, MESSAGE_SIGNATURE, 2, "l=5"),
            MESSAGE_SIGNATURE,
            2,
            "x=abc",
        );
        expect(
            &report(&both, RECEIPT, &world),
            Rule::Malformed,
            (Family::Signature, Some(0)),
            Some(2),
        );
    }

    #[test]
    fn rule_set_incomplete() {
        let world = keys(&HOPS);
        let two = chain(2);
        let cases = [
            (
                "no results for 2",
                without(&two, RESULTS, 2),
                Family::Results,
                2,
            ),
            (
                "no results for 1",
                without(&two, RESULTS, 1),
                Family::Results,
                1,
            ),
            (
                "no signature for 1",
                without(&two, MESSAGE_SIGNATURE, 1),
                Family::Signature,
                1,
            ),
            ("no seal for 2", without(&two, SEAL, 2), Family::Seal, 2),
        ];
        for (label, raw, family, instance) in cases {
            let report = report(&raw, RECEIPT, &world);
            expect(&report, Rule::SetIncomplete, (family, None), Some(instance));
            assert_eq!(report.arc.sets, 2, "{label}");
            assert!(lookups(&report).is_empty(), "{label}");
        }
        // ARC fields with no signature at all are a broken chain, never none.
        let one = chain(1);
        let no_signature = without(&one, MESSAGE_SIGNATURE, 1);
        let report = report(&no_signature, RECEIPT, &world);
        expect(
            &report,
            Rule::SetIncomplete,
            (Family::Signature, None),
            Some(1),
        );
        assert_eq!(report.arc.sets, 1);
        // Ties between the shortest families go in the order results, signature,
        // seal: the results alone are short of the other two, which tie at zero.
        let only_results = without(&without(&one, MESSAGE_SIGNATURE, 1), SEAL, 1);
        expect(
            &report_of(&only_results, &world),
            Rule::SetIncomplete,
            (Family::Signature, None),
            Some(1),
        );
        let only_seal = without(&without(&one, MESSAGE_SIGNATURE, 1), RESULTS, 1);
        expect(
            &report_of(&only_seal, &world),
            Rule::SetIncomplete,
            (Family::Results, None),
            Some(1),
        );
    }

    fn report_of(raw: &[u8], world: &Arc<Effective>) -> Report {
        report(raw, RECEIPT, world)
    }

    /// One forged set whose fields parse and mean nothing else.
    fn forged_set(i: u32) -> [String; 3] {
        [
            format!(
                "ARC-Seal: i={i}; a=ed25519-sha256; s=s; d=forged.example; cv=pass; b=AAAA\r\n"
            ),
            format!(
                "ARC-Message-Signature: i={i}; a=ed25519-sha256; s=s; d=forged.example; h=from; bh=AAAA; b=AAAA\r\n"
            ),
            format!("ARC-Authentication-Results: i={i}; mx.example.net\r\n"),
        ]
    }

    #[test]
    fn duplicated_instances_are_refused() {
        let world = keys(&HOPS);
        let two = chain(2);
        // Two complete sets that both say i=1, written in both orders.
        let both = |raw: &[u8]| {
            let mut raw = raw.to_vec();
            for name in [SEAL, MESSAGE_SIGNATURE, RESULTS] {
                raw = change(&raw, name, 2, "i=2;", "i=1;");
            }
            raw
        };
        let newest_first = both(&two);
        let oldest_first = edit(&newest_first, |fields| {
            let newer: Vec<String> = fields.drain(..3).collect();
            fields.splice(3..3, newer);
        });
        for raw in [&newest_first, &oldest_first] {
            let report = report(raw, RECEIPT, &world);
            // The results family is the first to duplicate, and its second field
            // in wire order is occurrence 1.
            expect(
                &report,
                Rule::InstanceInvalid,
                (Family::Results, Some(1)),
                Some(1),
            );
            assert_eq!(report.arc.sets, 2);
            assert!(lookups(&report).is_empty());
        }
        // Duplicates only in the signatures and the seals: the signatures come
        // first in the order of families.
        let three = chain(3);
        let signatures_and_seals = change(
            &change(&three, SEAL, 3, "i=3;", "i=2;"),
            MESSAGE_SIGNATURE,
            2,
            "i=2;",
            "i=1;",
        );
        // The seals duplicate instance 2 and the signatures instance 1: the lowest
        // instance wins, and its second field in wire order is named.
        expect(
            &report_of(&signatures_and_seals, &world),
            Rule::InstanceInvalid,
            (Family::Signature, Some(2)),
            Some(1),
        );
        let seals_only = change(&three, SEAL, 3, "i=3;", "i=2;");
        expect(
            &report_of(&seals_only, &world),
            Rule::InstanceInvalid,
            (Family::Seal, Some(1)),
            Some(2),
        );
        // Fifty-one sets cannot all differ: no lookup, and the same rule.
        let mut fields: Vec<String> = (1..=50).chain([1]).flat_map(forged_set).collect();
        let (rest, _) = split(&message(BODY));
        fields.extend(rest);
        let raw = join(&fields, &split(&message(BODY)).1);
        let report = report_of(&raw, &world);
        expect(
            &report,
            Rule::InstanceInvalid,
            (Family::Results, Some(50)),
            Some(1),
        );
        assert_eq!(report.arc.sets, 51);
        assert!(lookups(&report).is_empty());
    }

    #[test]
    fn rule_signature_expired_and_not_evaluated() {
        let world = keys(&HOPS);
        let two = chain(2);
        let sig = (Family::Signature, Some(0));
        // The sealer wrote the wall clock into t=; take it out so that x= is the
        // only time on the field, and no assertion reads the clock.
        let untimed = retag(&two, MESSAGE_SIGNATURE, 2, "t", "");
        let at_x = |x: &str| tagged(&untimed, MESSAGE_SIGNATURE, 2, &format!("x={x}"));
        let raw = at_x("1000");
        let before = report(&raw, 999, &world);
        expect(&before, Rule::NotEvaluated, sig, None);
        assert!(lookups(&before).is_empty());
        let at = report(&raw, 1000, &world);
        expect(&at, Rule::SignatureExpired, sig, Some(2));
        assert!(lookups(&at).is_empty());
        expect(
            &report(&raw, 5000, &world),
            Rule::SignatureExpired,
            sig,
            Some(2),
        );
        // A literal x=0 is expired at every receipt.
        expect(
            &report(&at_x("0"), 0, &world),
            Rule::SignatureExpired,
            sig,
            Some(2),
        );
        // x= at or before its own t= is expired, whatever the receipt.
        let timed = tagged(&at_x("2000"), MESSAGE_SIGNATURE, 2, "t=2000");
        expect(
            &report(&timed, 1, &world),
            Rule::SignatureExpired,
            sig,
            Some(2),
        );
        let later = tagged(&at_x("2000"), MESSAGE_SIGNATURE, 2, "t=1500");
        expect(&report(&later, 1, &world), Rule::NotEvaluated, sig, None);
        // x= on an older signature is nobody's expiry: the chain fails where the
        // edit broke the seal that covers it.
        let older = tagged(&two, MESSAGE_SIGNATURE, 1, "x=1000");
        expect(
            &report(&older, RECEIPT, &world),
            Rule::SignatureMismatch,
            (Family::Seal, Some(0)),
            Some(2),
        );
        // A chain that is also structurally wrong is decided by its x= first.
        let wrong = retag(&at_x("2000"), SEAL, 2, "cv", "cv=fail;");
        expect(&report(&wrong, 1, &world), Rule::NotEvaluated, sig, None);
    }

    /// The key record of another key of the same type, from RFC 8463.
    fn other_key(kind: Kind) -> String {
        match kind {
            Kind::Ed25519 => format!("v=DKIM1; k=ed25519; p={RFC8463_ED25519_PUBLIC}"),
            Kind::Rsa2048 => format!("v=DKIM1; k=rsa; p={RFC8463_RSA1024_PUBLIC}"),
        }
    }

    /// Alter the first character of one field's `b=`: still base64, no longer the
    /// signature the sealer made.
    fn flipped(raw: &[u8], name: &str, i: u32) -> Vec<u8> {
        edit(raw, |fields| {
            let index = at(fields, name, i);
            let start = fields[index].find("\tb=").expect("the field holds b=") + 3;
            // The next character of the base64 alphabet: still base64, and no
            // longer the signature the sealer made, whatever it began with.
            const ALPHABET: &str =
                "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let first = char::from(fields[index].as_bytes()[start]);
            let at = ALPHABET.find(first).expect("base64 starts the signature");
            fields[index].replace_range(start..=start, &ALPHABET[(at + 1) % 64..][..1]);
        })
    }

    #[test]
    fn rule_instance_invalid() {
        let world = keys(&HOPS);
        // A gap: sets 1 and 3 name instance 2, at the seal that stands there.
        let gap = without_set(&chain(3), 2);
        let report = report(&gap, RECEIPT, &world);
        expect(
            &report,
            Rule::InstanceInvalid,
            (Family::Seal, Some(0)),
            Some(2),
        );
        assert_eq!(report.arc.sets, 2);
        assert!(lookups(&report).is_empty());
        // A chain that starts at 2 names instance 1.
        let late = without_set(&chain(2), 1);
        expect(
            &report_of(&late, &world),
            Rule::InstanceInvalid,
            (Family::Seal, Some(0)),
            Some(1),
        );
        // The first of seal, signature and results whose instance is not the
        // position: here the seal is right and the signature is not, then the
        // results.
        let two = chain(2);
        expect(
            &report_of(&change(&two, MESSAGE_SIGNATURE, 2, "i=2;", "i=3;"), &world),
            Rule::InstanceInvalid,
            (Family::Signature, Some(0)),
            Some(2),
        );
        expect(
            &report_of(&change(&two, RESULTS, 2, "i=2;", "i=3;"), &world),
            Rule::InstanceInvalid,
            (Family::Results, Some(0)),
            Some(2),
        );
    }

    #[test]
    fn rule_cv_invalid() {
        let world = keys(&HOPS);
        let two = chain(2);
        let seal_1 = (Family::Seal, Some(1));
        let seal_2 = (Family::Seal, Some(0));
        expect(
            &report_of(&change(&two, SEAL, 1, "cv=none;", "cv=pass;"), &world),
            Rule::CvInvalid,
            seal_1,
            Some(1),
        );
        expect(
            &report_of(&change(&two, SEAL, 2, "cv=pass;", "cv=none;"), &world),
            Rule::CvInvalid,
            seal_2,
            Some(2),
        );
        expect(
            &report_of(&change(&two, SEAL, 2, "cv=pass;", "cv=fail;"), &world),
            Rule::CvInvalid,
            seal_2,
            Some(2),
        );
        // The first seal by instance that is wrong is the one named.
        let three = chain(3);
        let bad = change(
            &change(&three, SEAL, 3, "cv=pass;", "cv=fail;"),
            SEAL,
            2,
            "cv=pass;",
            "cv=none;",
        );
        let report = report_of(&bad, &world);
        expect(&report, Rule::CvInvalid, (Family::Seal, Some(1)), Some(2));
        assert!(lookups(&report).is_empty());
    }

    #[test]
    fn rule_body_hash_mismatch() {
        let world = keys(&HOPS);
        let raw = retext(&chain(2), "this  is a test", "this  is a tesT");
        let report = report_of(&raw, &world);
        expect(
            &report,
            Rule::BodyHashMismatch,
            (Family::Signature, Some(0)),
            Some(2),
        );
        assert!(lookups(&report).is_empty());
        // Appended text is a changed body too.
        let appended = [chain(1), b"more\r\n".to_vec()].concat();
        expect(
            &report_of(&appended, &world),
            Rule::BodyHashMismatch,
            (Family::Signature, Some(0)),
            Some(1),
        );
    }

    #[test]
    fn rule_key_missing() {
        let two = chain(2);
        // The newest signature's key.
        let report = report_of(&two, &keys_without(&HOPS[..2], HOPS[1]));
        expect(
            &report,
            Rule::KeyMissing,
            (Family::Signature, Some(0)),
            Some(2),
        );
        assert_eq!(lookups(&report), vec![HOPS[1].name()]);
        assert_eq!(report.dns.lookups[0].outcome, Outcome::NoRecords);
        // An older seal's key: the lookups before it were answered.
        let report = report_of(&two, &keys_without(&HOPS[..2], HOPS[0]));
        expect(&report, Rule::KeyMissing, (Family::Seal, Some(1)), Some(1));
        assert_eq!(lookups(&report), vec![HOPS[0].name(), HOPS[1].name()]);
        // The newest seal's key, through a selector that no key answers to.
        let seal_selector = change(&two, SEAL, 2, "s=arc2;", "s=nokey;");
        let report = report_of(&seal_selector, &keys(&HOPS));
        expect(&report, Rule::KeyMissing, (Family::Seal, Some(0)), Some(2));
        // A selector outside the DNS grammar is never looked up, and still a
        // definite absence under 0120.
        let outside = change(&two, MESSAGE_SIGNATURE, 2, "s=arc2;", "s=a..b;");
        let report = report_of(&outside, &keys(&HOPS));
        expect(
            &report,
            Rule::KeyMissing,
            (Family::Signature, Some(0)),
            Some(2),
        );
        assert!(lookups(&report).is_empty());
    }

    #[test]
    fn rule_key_invalid() {
        let one = chain(1);
        let name = HOPS[0].name();
        let sig = (Family::Signature, Some(0));
        for (label, record) in [
            ("no key record", "v=spf1 -all".to_string()),
            ("revoked", "v=DKIM1; k=ed25519; p=".to_string()),
            ("the other type", Kind::Rsa2048.record()),
        ] {
            let world = keys_with(&HOPS[..1], &name, &record);
            let report = report_of(&one, &world);
            expect(&report, Rule::KeyInvalid, sig, Some(1));
            assert_eq!(report.dns.lookups[0].outcome, Outcome::Records, "{label}");
        }
        // An older seal's key.
        let world = keys_with(&HOPS[..2], &name, "v=DKIM1; k=ed25519; p=");
        expect(
            &report_of(&chain(2), &world),
            Rule::KeyInvalid,
            (Family::Seal, Some(1)),
            Some(1),
        );
    }

    #[test]
    fn rule_signature_mismatch() {
        let name = |hop: Hop| hop.name();
        // A wrong key of the right type: the seal at i=1 of two, with the newer
        // set verifying; and the seal at i=2 of three.
        let wrong = |hops: &[Hop], at: Hop| keys_with(hops, &name(at), &other_key(at.kind));
        expect(
            &report_of(&chain(2), &wrong(&HOPS[..2], HOPS[0])),
            Rule::SignatureMismatch,
            (Family::Seal, Some(1)),
            Some(1),
        );
        let three = report_of(&chain(3), &wrong(&HOPS[..3], HOPS[1]));
        expect(
            &three,
            Rule::SignatureMismatch,
            (Family::Seal, Some(1)),
            Some(2),
        );
        // One name per hop that was asked about: the newest signature and seal
        // share a key, so that is two names for three asks.
        assert_eq!(lookups(&three), vec![HOPS[1].name(), HOPS[2].name()]);
        // The newest signature's own key, of either type.
        expect(
            &report_of(&chain(1), &wrong(&HOPS[..1], HOPS[0])),
            Rule::SignatureMismatch,
            (Family::Signature, Some(0)),
            Some(1),
        );
        expect(
            &report_of(&chain(3), &wrong(&HOPS[..3], HOPS[2])),
            Rule::SignatureMismatch,
            (Family::Signature, Some(0)),
            Some(3),
        );
        // The oldest seal altered after the second hop sealed: the newest seal
        // that covers it is the one that fails.
        let altered = flipped(&chain(2), SEAL, 1);
        expect(
            &report_of(&altered, &keys(&HOPS)),
            Rule::SignatureMismatch,
            (Family::Seal, Some(0)),
            Some(2),
        );
        // A header the newest signature covers, changed after sealing.
        let from = retext(&chain(2), "From: Alice", "From: Mallory");
        expect(
            &report_of(&from, &keys(&HOPS)),
            Rule::SignatureMismatch,
            (Family::Signature, Some(0)),
            Some(2),
        );
        // A record that is a key record with bytes that are no key.
        let junk = keys_with(&HOPS[..1], &name(HOPS[0]), "v=DKIM1; k=ed25519; p=AAAA");
        expect(
            &report_of(&chain(1), &junk),
            Rule::SignatureMismatch,
            (Family::Signature, Some(0)),
            Some(1),
        );
    }

    #[test]
    fn rule_not_evaluated_is_never_clean() {
        let two = chain(2);
        let temp = |name: &str| {
            let records: Vec<(String, String)> = HOPS[..2]
                .iter()
                .filter(|hop| hop.name() != name)
                .map(|hop| (hop.name(), hop.kind.record()))
                .collect();
            let borrowed: Vec<(&str, String)> = records
                .iter()
                .map(|(n, v)| (n.as_str(), v.clone()))
                .collect();
            policy(&borrowed, &[name])
        };
        for (hop, at) in [
            (HOPS[1], (Family::Signature, Some(0))),
            (HOPS[0], (Family::Seal, Some(1))),
        ] {
            let report = report_of(&two, &temp(&hop.name()));
            expect(&report, Rule::NotEvaluated, at, None);
            for threshold in [Severity::Info, Severity::Warning, Severity::Error] {
                assert!(!report.qualifies(threshold));
            }
            assert!(
                report
                    .dns
                    .lookups
                    .iter()
                    .any(|l| l.outcome == Outcome::TempFail)
            );
        }
    }

    #[test]
    fn occurrences_are_root_occurrences() {
        // A chain written oldest-first: the crate sorts by instance, the finding
        // names the field's place in the message.
        let oldest_first = edit(&chain(3), |fields| {
            let sets: Vec<Vec<String>> = fields.chunks(3).map(<[String]>::to_vec).collect();
            let rest: Vec<String> = fields.drain(9..).collect();
            *fields = sets.into_iter().rev().flatten().chain(rest).collect();
        });
        passes(&oldest_first, &keys(&HOPS[..3]), 3);
        let wrong = keys_with(&HOPS[..3], &HOPS[0].name(), &other_key(HOPS[0].kind));
        expect(
            &report_of(&oldest_first, &wrong),
            Rule::SignatureMismatch,
            (Family::Seal, Some(0)),
            Some(1),
        );
        // The same fault in the newest-first message is the last seal.
        expect(
            &report_of(&chain(3), &wrong),
            Rule::SignatureMismatch,
            (Family::Seal, Some(2)),
            Some(1),
        );
    }

    /// A DKIM-signed message and its own key beside a hop's.
    fn dkim_world(hops: &[Hop]) -> Arc<Effective> {
        keys_with(hops, "sel._domainkey.example.com", &Kind::Ed25519.record())
    }

    fn dkim_signed() -> Vec<u8> {
        signed(&Sign::default(), &message(BODY))
    }

    /// Every fixture of the suite, with the policy it is judged under.
    fn fixtures() -> Vec<(&'static str, Vec<u8>, Arc<Effective>)> {
        let all = keys(&HOPS);
        let two = chain(2);
        vec![
            ("none", message(BODY), all.clone()),
            ("pass 1", chain(1), all.clone()),
            ("pass 2", two.clone(), all.clone()),
            ("pass 3", chain(3), all.clone()),
            ("gap", without_set(&chain(3), 2), all.clone()),
            ("incomplete", without(&two, RESULTS, 2), all.clone()),
            (
                "body",
                retext(&two, "this  is a test", "this  is a tesT"),
                all.clone(),
            ),
            (
                "missing key",
                two.clone(),
                keys_without(&HOPS[..2], HOPS[0]),
            ),
            (
                "wrong key",
                two.clone(),
                keys_with(&HOPS[..2], &HOPS[0].name(), &other_key(HOPS[0].kind)),
            ),
            ("malformed", retag(&two, SEAL, 1, "cv", ""), all.clone()),
            (
                "expired",
                tagged(
                    &retag(&two, MESSAGE_SIGNATURE, 2, "t", ""),
                    MESSAGE_SIGNATURE,
                    2,
                    "x=1000",
                ),
                all.clone(),
            ),
            ("boundary", [b"stray\r\n".to_vec(), two].concat(), all),
        ]
    }

    #[test]
    fn arc_member_invariants() {
        // Every fixture passes the five invariants inside `report`; together they
        // reach all four states, and `none` only where no ARC field exists.
        let mut states = BTreeSet::new();
        for (label, raw, world) in fixtures() {
            let report = report(&raw, RECEIPT, &world);
            states.insert(report.arc.state as u8);
            let text = String::from_utf8_lossy(&raw).to_ascii_lowercase();
            assert_eq!(
                report.arc.state == ArcState::None,
                !text.contains("arc-seal"),
                "{label}"
            );
        }
        assert_eq!(states.len(), 4);
    }

    #[test]
    fn arc_reports_are_byte_identical() {
        for (label, raw, world) in fixtures() {
            let first = serde_json::to_vec(&report(&raw, RECEIPT, &world)).unwrap();
            let second = serde_json::to_vec(&report(&raw, RECEIPT, &world)).unwrap();
            assert_eq!(first, second, "{label}");
        }
    }

    /// The six facts and the findings of every other family, for one message.
    fn others(raw: &[u8], world: &Arc<Effective>) -> (super::super::AuthEvidence, Vec<Finding>) {
        let (fields, size) = headers(raw).unwrap();
        let mut lookups = Lookups::new(world.clone());
        let mut out = Collector(Vec::new());
        let (facts, _) = auth::collect(
            raw,
            &fields,
            size,
            RECEIPT,
            &Envelope::default(),
            &mut lookups,
            &mut out,
        )
        .unwrap();
        let mut findings: Vec<Finding> = out
            .0
            .into_iter()
            .filter(|f| !f.rule_id.starts_with("arc."))
            .collect();
        findings.sort();
        (facts, findings)
    }

    #[test]
    fn arc_moves_no_fact() {
        // The same DKIM-signed message with no chain, a passing chain and a
        // failing one: the facts and every non-ARC finding are identical.
        let plain = dkim_signed();
        let sealed = seal(&plain, HOPS[0]);
        let absent = others(&plain, &dkim_world(&[]));
        let passing = others(&sealed, &dkim_world(&HOPS[..1]));
        let failing = others(&sealed, &dkim_world(&[]));
        assert_eq!(absent, passing);
        assert_eq!(absent, failing);
        assert_eq!(
            report(&sealed, RECEIPT, &dkim_world(&HOPS[..1])).arc.state,
            ArcState::Pass
        );
        assert_eq!(
            report(&sealed, RECEIPT, &dkim_world(&[])).arc.state,
            ArcState::Fail
        );
        assert_eq!(
            report(&plain, RECEIPT, &dkim_world(&[])).arc.state,
            ArcState::None
        );
    }

    #[test]
    fn arc_verdict_ignores_dkim_l() {
        let world = dkim_world(&HOPS[..1]);
        let raw = chain(1);
        let with_l = |extra: usize| {
            let signature = Sign {
                body_length: true,
                ..Sign::default()
            };
            let mut text = String::from_utf8(signed(&signature, &raw)).unwrap();
            for _ in 0..extra {
                text.insert_str(
                    0,
                    "ARC-Message-Signature: i=9; a=rsa-sha256; s=s; d=x.example; h=from; l=5; bh=AAAA; b=AAAA\r\n",
                );
            }
            text.into_bytes()
        };
        // An unrelated DKIM-Signature with l= makes DKIM re-parse the message
        // without strict; the chain's verdict is the one the strict parse gives.
        let plain = report(&raw, RECEIPT, &world).arc;
        let beside = report(&with_l(0), RECEIPT, &world).arc;
        assert_eq!(plain.state, ArcState::Pass);
        assert_eq!(beside, plain);
        // So it is with the DKIM bound already fired: ARC reads the strict parse
        // that DKIM bounded. (Nothing is signed but the one l= field here.)
        let many = report(&with_l(16), RECEIPT, &world);
        assert!(
            many.findings
                .iter()
                .any(|f| f.rule_id == "dkim.limits_exceeded.v1")
        );
        // The forged fields are rejected by the crate and judged as such.
        assert_eq!(many.arc.reason.as_deref(), Some("arc.unsupported.v1"));
    }

    #[test]
    fn message_signatures_with_l_count_parsed_and_rejected_fields() {
        let l = |tag: &str| {
            format!(
                "ARC-Message-Signature: i=1; a=rsa-sha256; s=s; d=x.example; h=from; {tag}; bh=AAAA; b=AAAA\r\n"
            )
        };
        let raw = [
            l("l=5"),
            l("l=0"),
            l("t=1"),
            "ARC-Seal: l=5; i=1\r\n".to_string(),
            "From: a@example.com\r\n\r\nbody\r\n".to_string(),
        ]
        .concat();
        let parsed = AuthenticatedMessage::parse_with_opts(raw.as_bytes(), None, true).unwrap();
        // `l=5` is rejected by the strict parse, `l=0` is parsed, `t=1` has none.
        assert_eq!(message_signatures_with_l(&parsed), 2);
    }

    #[test]
    fn findings_past_the_report_cap_fail_the_analysis() {
        // The collector's cap is shared with every collector: a full report takes
        // no more findings, and the analysis fails rather than truncating.
        let full = || {
            Collector(vec![
                Finding {
                    rule_id: "x".into(),
                    location: Location::MimePart { path: vec![] },
                    severity: Severity::Info,
                    fix_hint: String::new(),
                    weight: 0,
                };
                super::super::MAX_FINDINGS
            ])
        };
        let run = |raw: &[u8]| {
            let (root, size) = headers(raw).unwrap();
            let parsed = AuthenticatedMessage::parse_with_opts(raw, None, true);
            let mut lookups = Lookups::new(keys(&HOPS));
            let shared = RefCell::new(&mut lookups);
            let adapter = Adapter::over(&shared);
            let before = adapter.calls();
            let result = evaluate(parsed.as_ref(), &root, size, &adapter, RECEIPT, &mut full());
            (result, adapter.calls() - before)
        };
        // A broken chain pushes its one finding, the bound refuses it...
        assert_eq!(
            run(&retext(&chain(1), "this  is a test", "this  is a tesT")),
            (Err(AnalysisError::Failed), 0)
        );
        // ...a boundary finding is pushed the same way...
        assert_eq!(
            run(&[b"x\r\n".to_vec(), chain(1)].concat()).0,
            Err(AnalysisError::Failed)
        );
        // ...while a passing chain, and mail with none, add nothing to a full report.
        let (pass, asked) = run(&chain(2));
        assert_eq!(pass.unwrap().state, ArcState::Pass);
        // The crate asks once for the newest signature and once per seal.
        assert_eq!(asked, 3);
        assert_eq!(run(&message(BODY)).0.unwrap(), ArcChain::default());
    }

    #[test]
    fn guide_lists_every_arc_rule() {
        let guide = include_str!("../../docs/guide/deliverability.md");
        let llms = include_str!("../static/llms.txt");
        for rule in Rule::ALL {
            let heading = format!("### `{}`\n", rule.id());
            let start = guide.find(&heading).expect("a heading per rule");
            let rest = &guide[start + heading.len()..];
            let section = &rest[..rest.find("\n##").unwrap()];
            assert!(section.contains("**Severity:** error"), "{}", rule.id());
            let state = serde_json::to_value(rule.state()).unwrap();
            assert!(
                section.contains(&format!("The state is `{}`.", state.as_str().unwrap())),
                "{}",
                rule.id()
            );
            assert!(section.contains(rule.hint()), "{}", rule.id());
            assert!(section.contains("**Passing:**"));
            assert!(section.contains("**Failing:**"));
            assert!(llms.contains(&format!("`{}`", rule.id())), "{}", rule.id());
        }
        // No rule is documented that the table lacks.
        assert_eq!(guide.matches("\n### `arc.").count(), Rule::ALL.len());
        for fact in [
            "## ARC (`swarmail-html/9`)",
            "**The `arc` member.**",
            "**Which seal failed.**",
            "**Time is the receipt time.**",
            "**Authentication-Results correspondence.** `arc=none`",
            "`arc=pass` does not say the sender is authentic",
            "`swarmail-html/9` (the thirteen ARC rules",
        ] {
            assert!(guide.contains(fact), "{fact}");
        }
        for fact in [
            "## ARC (`swarmail-html/9`)",
            "arc=pass says the chain validated",
        ] {
            assert!(llms.contains(fact), "{fact}");
        }
        // The bounds the guide states are the bounds the code enforces.
        assert_eq!((MAX_SIGNED_HEADERS, MAX_INSTANCE), (128, 50));
    }
}
