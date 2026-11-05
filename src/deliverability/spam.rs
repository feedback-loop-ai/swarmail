//! Built-in deterministic spam signals (issue #260, proposed decision 0122).
//!
//! Six compile-time rules over the root Subject, the root From display name and
//! the sender's `text/plain` bodies. No Store, clock, network, locale, knob or
//! runtime catalogue: the same bytes always yield the same findings. Each rule
//! emits at most one finding, so the collector can never flood the report.
use super::{AnalysisError, Collector, Finding, Location, Severity};
use mail_parser::{Addr, Address, HeaderName, HeaderValue, MessageParser, MimeHeaders, PartType};
use regex::{Regex, bytes};
use std::sync::LazyLock;

/// Aggregate decoded eligible text, summed over every part before any match.
const SPAM_TEXT_BYTES: usize = 4_194_304;
const MIN_CAPITALS: usize = 8;
const MIN_ZERO_WIDTH: usize = 3;

/// The fixed phrase list; the guide reproduces it and a test binds the two.
const PHRASE_LIST: [&str; 14] = [
    "act now",
    "limited time offer",
    "100% free",
    "risk-free",
    "risk free",
    "you have won",
    "you are a winner",
    "claim your prize",
    "no credit check",
    "double your money",
    "make money fast",
    "once in a lifetime",
    "this is not spam",
    "cash bonus",
];

/// One linear-time ASCII-mode alternation, built once from the constant list.
/// `(?i-u)` makes case folding and `\b` ASCII-only; every phrase begins and ends
/// with an ASCII alphanumeric, so `\b` is exactly "no letter, digit or `_` next
/// to it". Each internal space matches a run of space, tab, CR or LF.
static PHRASES: LazyLock<bytes::Regex> = LazyLock::new(|| {
    let words = PHRASE_LIST.map(|phrase| {
        phrase
            .split(' ')
            .map(regex::escape)
            .collect::<Vec<_>>()
            .join("[ \\t\\r\\n]+")
    });
    bytes::Regex::new(&format!("(?i-u)\\b(?:{})\\b", words.join("|"))).unwrap()
});

/// An ASCII address-shaped token inside a display name.
static ADDRESS_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("(?-u)[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\\.[A-Za-z0-9-]+)+").unwrap()
});

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Signal {
    SubjectAllCaps,
    SubjectPunctuation,
    SubjectPhrases,
    FromDisplayNameAddress,
    BodyPhrases,
    BodyZeroWidth,
}

#[cfg(test)]
const SIGNALS: [Signal; 6] = [
    Signal::SubjectAllCaps,
    Signal::SubjectPunctuation,
    Signal::SubjectPhrases,
    Signal::FromDisplayNameAddress,
    Signal::BodyPhrases,
    Signal::BodyZeroWidth,
];

impl Signal {
    fn id(self) -> &'static str {
        match self {
            Self::SubjectAllCaps => "spam.subject.all_caps.v1",
            Self::SubjectPunctuation => "spam.subject.punctuation.v1",
            Self::SubjectPhrases => "spam.subject.phrases.v1",
            Self::FromDisplayNameAddress => "spam.from.display_name_address.v1",
            Self::BodyPhrases => "spam.body.phrases.v1",
            Self::BodyZeroWidth => "spam.body.zero_width.v1",
        }
    }

    /// Info or warning only: error stays the bulk checklist's requirement level.
    fn severity(self) -> Severity {
        match self {
            Self::SubjectPunctuation | Self::BodyZeroWidth => Severity::Info,
            Self::SubjectAllCaps
            | Self::SubjectPhrases
            | Self::FromDisplayNameAddress
            | Self::BodyPhrases => Severity::Warning,
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Self::SubjectAllCaps => {
                "Write the Subject in sentence case; an all-capital subject reads as shouting and is a common spam signal."
            }
            Self::SubjectPunctuation => "Remove repeated !, ? or $ characters from the Subject.",
            Self::SubjectPhrases => {
                "Remove high-pressure sales wording from the Subject; see the guide's phrase list."
            }
            Self::FromDisplayNameAddress => {
                "Do not put an email address in the From display name unless it is the sending address; a different address there looks like spoofing."
            }
            Self::BodyPhrases => {
                "Remove high-pressure sales wording from the text/plain body; see the guide's phrase list."
            }
            Self::BodyZeroWidth => {
                "Remove zero-width characters from the text/plain body; they are used to hide words from filters."
            }
        }
    }

    fn finding(self, location: Location) -> Finding {
        Finding {
            rule_id: self.id().into(),
            location,
            severity: self.severity(),
            fix_hint: self.hint().into(),
        }
    }
}

/// At least eight ASCII letters and none lowercase. Non-ASCII letters never count.
fn all_caps(subject: &str) -> bool {
    let mut capitals = 0;
    for byte in subject.bytes() {
        if byte.is_ascii_lowercase() {
            return false;
        }
        capitals += usize::from(byte.is_ascii_uppercase());
    }
    capitals >= MIN_CAPITALS
}

fn punctuation(subject: &str) -> bool {
    ["!!", "??", "$$"].iter().any(|run| subject.contains(run))
}

fn phrases(text: &str) -> bool {
    PHRASES.is_match(text.as_bytes())
}

/// ZWJ (U+200D) is excluded: emoji sequences use it legitimately.
fn zero_width(text: &str) -> bool {
    text.chars()
        .filter(|c| matches!(c, '\u{200B}' | '\u{200C}' | '\u{2060}' | '\u{FEFF}'))
        .nth(MIN_ZERO_WIDTH - 1)
        .is_some()
}

/// A display name that carries an address other than the mailbox's own.
fn foreign_address(name: &str, address: &str) -> bool {
    ADDRESS_TOKEN
        .find_iter(name)
        .any(|token| !token.as_str().eq_ignore_ascii_case(address))
}

/// The first field of `name` in wire order, as the report locates it
/// (`occurrence: 0`); a later occurrence never substitutes for it.
fn first<'a>(
    headers: &'a [mail_parser::Header<'a>],
    name: HeaderName<'a>,
) -> Option<&'a HeaderValue<'a>> {
    headers.iter().find(|h| h.name == name).map(|h| &h.value)
}

fn header(name: &str) -> Location {
    Location::Header {
        name: name.into(),
        occurrence: Some(0),
    }
}

pub(super) fn collect(raw: &[u8], out: &mut Collector) -> Result<(), AnalysisError> {
    // Raw size, part count and nesting depth are bounded by the engine's own
    // admission before any collector runs.
    if raw.is_empty() {
        return Ok(());
    }
    let message = MessageParser::default()
        .parse(raw)
        .ok_or(AnalysisError::Failed)?;
    let mut found = Vec::new();
    let headers = message.headers();
    if let Some(subject) = first(headers, HeaderName::Subject).and_then(HeaderValue::as_text) {
        for (signal, hit) in [
            (Signal::SubjectAllCaps, all_caps(subject)),
            (Signal::SubjectPunctuation, punctuation(subject)),
            (Signal::SubjectPhrases, phrases(subject)),
        ] {
            if hit {
                found.push(signal.finding(header("subject")));
            }
        }
    }
    if let Some(HeaderValue::Address(Address::List(list))) = first(headers, HeaderName::From)
        && let [
            Addr {
                name: Some(name),
                address: Some(address),
            },
            ..,
        ] = list.as_slice()
        && foreign_address(name, address)
    {
        found.push(Signal::FromDisplayNameAddress.finding(header("from")));
    }

    // Eligible text parts, in wire order, with their structural child-index path.
    let mut stack = vec![(0usize, Vec::new(), false)];
    let mut eligible = Vec::new();
    let mut bytes = 0usize;
    while let Some((id, path, excluded)) = stack.pop() {
        let part = &message.parts[id];
        let excluded = excluded
            || part.attachment_name().is_some()
            || part
                .content_disposition()
                .is_some_and(|d| d.ctype().eq_ignore_ascii_case("attachment"));
        if let PartType::Multipart(children) = &part.body {
            for (n, child) in children.iter().enumerate().rev() {
                let mut path = path.clone();
                path.push(n as u64);
                stack.push((*child as usize, path, excluded));
            }
        } else if let PartType::Text(text) = &part.body
            && !excluded
            && (part.content_type().is_none() || part.is_content_type("text", "plain"))
        {
            if part.is_encoding_problem {
                return Err(AnalysisError::Failed);
            }
            bytes += text.len();
            if bytes > SPAM_TEXT_BYTES {
                return Err(AnalysisError::Failed);
            }
            eligible.push((path, text));
        }
    }
    // The aggregate size is settled above, before any lexical work.
    for (signal, test) in [
        (Signal::BodyPhrases, phrases as fn(&str) -> bool),
        (Signal::BodyZeroWidth, zero_width),
    ] {
        if let Some((path, _)) = eligible.iter().find(|(_, text)| test(text)) {
            found.push(signal.finding(Location::MimePart { path: path.clone() }));
        }
    }
    for finding in found {
        out.push(finding)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{Inputs, MAX_RAW_BYTES, evaluate};
    use super::*;
    use base64::{Engine, engine::general_purpose::STANDARD};

    fn scan(raw: &[u8]) -> Result<Vec<Finding>, AnalysisError> {
        let mut out = Collector(Vec::new());
        collect(raw, &mut out)?;
        Ok(out.0)
    }

    fn ids(raw: &str) -> Vec<String> {
        scan(raw.as_bytes())
            .unwrap()
            .into_iter()
            .map(|f| f.rule_id)
            .collect()
    }

    fn fires(raw: &str, signal: Signal) -> bool {
        ids(raw).iter().any(|id| id == signal.id())
    }

    fn subject(value: &str, signal: Signal) -> bool {
        fires(&format!("Subject: {value}\r\n\r\nbody"), signal)
    }

    fn body(value: &str, signal: Signal) -> bool {
        fires(&format!("Subject: s\r\n\r\n{value}"), signal)
    }

    fn from(value: &str) -> bool {
        fires(
            &format!("From: {value}\r\n\r\nbody"),
            Signal::FromDisplayNameAddress,
        )
    }

    fn multipart(parts: &[&str]) -> String {
        let mut raw = String::from("Content-Type: multipart/mixed; boundary=b\r\n\r\n");
        for part in parts {
            raw.push_str(&format!("--b\r\n{part}\r\n"));
        }
        raw.push_str("--b--\r\n");
        raw
    }

    #[test]
    fn catalogue_is_closed_and_disjoint() {
        let shape = Regex::new(r"^spam\.[a-z]+\.[a-z_]+\.v1$").unwrap();
        let mut seen = std::collections::HashSet::new();
        for signal in SIGNALS {
            let id = signal.id();
            assert!(shape.is_match(id), "{id}");
            assert!(seen.insert(id), "duplicate {id}");
            assert!(!id.starts_with("bulk."));
            assert!(!id.starts_with("html."));
            assert_ne!(id, "demo.marker.v1");
            assert!(signal.severity() < Severity::Error, "{id}");
            assert!(!signal.hint().trim().is_empty(), "{id}");
            let finding = signal.finding(header("subject"));
            assert_eq!(finding.rule_id, id);
            assert_eq!(finding.severity, signal.severity());
            assert_eq!(finding.fix_hint, signal.hint());
        }
        assert_eq!(seen.len(), 6);
        assert_eq!(PHRASE_LIST.len(), 14);
    }

    #[test]
    fn subject_rules_boundaries() {
        use Signal::*;
        assert!(!subject("ABCDEFG", SubjectAllCaps), "seven capitals");
        assert!(subject("ABCDEFGH", SubjectAllCaps), "eight capitals");
        assert!(subject("ABCD 1234 EFGH!", SubjectAllCaps));
        assert!(
            !subject("ABCDEFGHi", SubjectAllCaps),
            "one lowercase letter"
        );
        assert!(!subject("1234 5678 9012", SubjectAllCaps), "no letters");
        // Non-ASCII letters are neither capitals nor lowercase: no Unicode folding.
        assert!(!subject("ÄÖÜÄÖÜÄÖÜ", SubjectAllCaps));
        assert!(subject("ÄÖÜ ABCDEFGH", SubjectAllCaps));
        assert!(!subject("ÄÖÜ ABCDEFGé h", SubjectAllCaps));
        for run in ["!!", "??", "$$", "Hi!!!", "a?? b"] {
            assert!(subject(run, SubjectPunctuation), "{run}");
        }
        for single in ["!", "?", "$", "! ? $", "!?$"] {
            assert!(!subject(single, SubjectPunctuation), "{single}");
        }
        // RFC 2047 words are decoded before the rules read them.
        assert!(subject("=?UTF-8?B?QUJDREVGR0g=?=", SubjectAllCaps));
        assert!(subject("=?UTF-8?Q?Hi=21=21?=", SubjectPunctuation));
        assert!(subject("=?UTF-8?Q?Act_now?=", SubjectPhrases));
        // A folded subject is unfolded.
        assert!(fires("Subject: Act\r\n now\r\n\r\nbody", SubjectPhrases));
    }

    #[test]
    fn subject_findings_carry_the_subject_location() {
        let found = scan(b"Subject: ACT NOW!! FREE\r\n\r\nbody").unwrap();
        let ids: Vec<_> = found.iter().map(|f| f.rule_id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "spam.subject.all_caps.v1",
                "spam.subject.punctuation.v1",
                "spam.subject.phrases.v1"
            ]
        );
        assert!(found.iter().all(|f| f.location == header("subject")));
        assert_eq!(found[1].severity, Severity::Info);
        assert_eq!(found[0].severity, Severity::Warning);
    }

    #[test]
    fn display_name_rule_boundaries() {
        assert!(from("\"ceo@bank.example\" <billing@example.test>"));
        assert!(from("\"a@b.co and ceo@bank.example\" <a@b.co>"));
        assert!(!from("\"Billing@Example.test\" <billing@example.test>"));
        assert!(!from("\"billing@example.test\" <billing@example.test>"));
        assert!(!from("\"Billing\" <billing@example.test>"));
        assert!(!from("<billing@example.test>"));
        assert!(!from("billing@example.test"));
        assert!(!from("\"not an address: ceo@\" <a@b.co>"));
        assert!(
            !from("\"user@localhost\" <a@b.co>"),
            "needs a dotted domain"
        );
        assert!(!from("Name Only"));
        assert!(!from("G: ceo@bank.example;"), "a group yields nothing");
        assert!(!from(""), "an empty From yields nothing");
        // The first mailbox only, RFC 2047-decoded.
        assert!(from("=?UTF-8?B?eEB5LmNv?= <a@b.co>"));
        assert!(!from("\"Alice\" <a@b.co>, \"ceo@bank.example\" <c@d.co>"));
        let found = scan(b"From: \"x@y.co\" <a@b.co>\r\n\r\nbody").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].location, header("from"));
    }

    #[test]
    fn first_occurrence_only() {
        // Only the second field would fire: the first, in wire order, decides.
        let raw = "Subject: Hello\r\nSubject: ACT NOW!!\r\n\r\nbody";
        assert!(ids(raw).is_empty());
        let raw = "From: \"A\" <a@b.co>\r\nFrom: \"x@y.co\" <a@b.co>\r\n\r\nbody";
        assert!(ids(raw).is_empty());
        // And the first fires even if a later one is clean.
        let raw = "Subject: ACT NOW!!\r\nSubject: Hello\r\n\r\nbody";
        assert!(fires(raw, Signal::SubjectPhrases));
        // Missing, empty or non-text fields yield nothing.
        for raw in [
            "X-A: b\r\n\r\nbody",
            "Subject:\r\n\r\nbody",
            "Subject: \r\nFrom: \r\n\r\nbody",
        ] {
            assert!(ids(raw).is_empty(), "{raw:?}");
        }
    }

    #[test]
    fn phrases_match_exactly() {
        for phrase in PHRASE_LIST {
            for text in [
                phrase.to_string(),
                phrase.to_uppercase(),
                format!("Hello, {phrase}."),
                format!("{phrase}!"),
                phrase.replace(' ', "\r\n "),
                phrase.replace(' ', "\t  \t"),
                format!("é{phrase}é"),
            ] {
                assert!(phrases(&text), "{text:?}");
            }
            for text in [
                format!("x{phrase}"),
                format!("{phrase}x"),
                format!("7{phrase}"),
                format!("{phrase}9"),
                format!("_{phrase}"),
                format!("{phrase}_"),
            ] {
                assert!(!phrases(&text), "{text:?}");
            }
        }
        // ASCII-only case folding: KELVIN SIGN and LONG S are not k and s.
        assert!(!phrases("no credit chec\u{212A}"));
        assert!(!phrases("thi\u{17F} is not spam"));
        // Transactional wording stays clear.
        for text in [
            "verify your account",
            "urgent",
            "congratulations",
            "act  nowhere",
            "actnow",
            "",
        ] {
            assert!(!phrases(text), "{text:?}");
        }
        assert!(phrases("act   now"));
    }

    #[test]
    fn body_phrases_and_zero_width_boundaries() {
        use Signal::*;
        assert!(body("Please ACT NOW", BodyPhrases));
        assert!(!body("Please act soon", BodyPhrases));
        for ch in ['\u{200B}', '\u{200C}', '\u{2060}', '\u{FEFF}'] {
            assert!(body(&format!("a{ch}b{ch}c{ch}d"), BodyZeroWidth), "{ch:?}");
            assert!(!body(&format!("a{ch}b{ch}c"), BodyZeroWidth), "{ch:?}");
        }
        assert!(body("\u{200B}\u{200C}x\u{2060}", BodyZeroWidth), "mixed");
        let zwj = "a\u{200D}b\u{200D}c\u{200D}d\u{200D}e";
        assert!(!body(zwj, BodyZeroWidth), "ZWJ is excluded");
    }

    #[test]
    fn body_eligibility() {
        use Signal::BodyPhrases as P;
        let hit = "Act now";
        let part = |headers: &str| format!("{headers}\r\n\r\n{hit}");
        // No Content-Type at all is text/plain (RFC 2045).
        assert!(fires(&format!("From: a@b.co\r\n\r\n{hit}"), P));
        assert!(fires(&format!("Content-Type: text/plain\r\n\r\n{hit}"), P));
        let upper = format!("Content-Type: TEXT/Plain; charset=utf-8\r\n\r\n{hit}");
        assert!(fires(&upper, P));
        assert!(fires(&multipart(&[&part("Content-Type: text/plain")]), P));
        assert!(fires(&multipart(&[&part("X-A: b")]), P), "defaulted child");
        // Ineligible: html, other text types, other media, attachments.
        for headers in [
            "Content-Type: text/html",
            "Content-Type: text/csv",
            "Content-Type: application/octet-stream",
            "Content-Type: text/plain\r\nContent-Disposition: attachment",
            "Content-Type: text/plain; name=a.txt",
            "Content-Type: text/plain\r\nContent-Disposition: inline; filename=a.txt",
        ] {
            assert!(!fires(&multipart(&[&part(headers)]), P), "{headers}");
        }
        // An attachment ancestor excludes its whole subtree.
        let inner = multipart(&[&part("Content-Type: text/plain")]);
        let nested = multipart(&[&format!(
            "Content-Type: multipart/mixed; boundary=c\r\nContent-Disposition: attachment\r\n\r\n--c\r\n{}\r\n--c--",
            part("Content-Type: text/plain")
        )]);
        assert!(fires(&inner, P));
        assert!(!fires(&nested, P));
        // Embedded messages and digest defaults are opaque.
        let embedded = multipart(&[&format!(
            "Content-Type: message/rfc822\r\n\r\nSubject: x\r\n\r\n{hit}"
        )]);
        assert!(!fires(&embedded, P));
        let digest = format!(
            "Content-Type: multipart/digest; boundary=b\r\n\r\n--b\r\n\r\nSubject: x\r\n\r\n{hit}\r\n--b--\r\n"
        );
        assert!(!fires(&digest, P));
        // A multipart without a boundary is not text/plain.
        assert!(!fires(
            &format!("Content-Type: multipart/mixed\r\n\r\n{hit}"),
            P
        ));
        // Transfer encodings are decoded before matching.
        let b64 = STANDARD.encode(hit);
        let encoded = format!("Content-Transfer-Encoding: base64\r\n\r\n{b64}");
        assert!(fires(&encoded, P));
        let qp = "Content-Transfer-Encoding: quoted-printable\r\n\r\n";
        assert!(fires(&format!("{qp}Act=20now"), P));
        assert!(!fires(&format!("{qp}Act=20nowhere"), P));
    }

    #[test]
    fn undecodable_eligible_text_fails() {
        for encoding in ["base64", "quoted-printable"] {
            let raw = format!(
                "Content-Type: text/plain\r\nContent-Transfer-Encoding: {encoding}\r\n\r\n====%z"
            );
            assert_eq!(
                scan(raw.as_bytes()),
                Err(AnalysisError::Failed),
                "{encoding}"
            );
        }
    }

    #[test]
    fn first_part_wins_and_paths_are_structural() {
        let hit = "Content-Type: text/plain\r\n\r\nAct now \u{200B}\u{200B}\u{200B}";
        let miss = "Content-Type: text/plain\r\n\r\nhello";
        let at = |path: Vec<u64>| Location::MimePart { path };
        let found = scan(multipart(&[hit, hit]).as_bytes()).unwrap();
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|f| f.location == at(vec![0])));
        let found = scan(multipart(&[miss, hit]).as_bytes()).unwrap();
        assert!(found.iter().all(|f| f.location == at(vec![1])));
        // A nested part is addressed by its child-index path from the root.
        let nested = format!(
            "Content-Type: multipart/alternative; boundary=c\r\n\r\n--c\r\n{miss}\r\n--c\r\n{hit}\r\n--c--"
        );
        let found = scan(multipart(&[miss, &nested]).as_bytes()).unwrap();
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|f| f.location == at(vec![1, 1])));
        // The two body rules may land on different parts.
        let split = multipart(&[
            "Content-Type: text/plain\r\n\r\nact now",
            "Content-Type: text/plain\r\n\r\n\u{200B}\u{200B}\u{200B}",
        ]);
        let found = scan(split.as_bytes()).unwrap();
        let by_id = |id: &str| found.iter().find(|f| f.rule_id == id).unwrap();
        assert_eq!(by_id("spam.body.phrases.v1").location, at(vec![0]));
        assert_eq!(by_id("spam.body.zero_width.v1").location, at(vec![1]));
        // A single-part message is the root: the empty path.
        let root = scan(b"\r\nact now").unwrap();
        assert_eq!(root[0].location, at(vec![]));
    }

    #[test]
    fn many_parts_yield_one_finding_per_rule() {
        let leaf = "Content-Type: text/plain\r\n\r\nAct now \u{200B}\u{200B}\u{200B}";
        let raw = multipart(&[leaf; 255]);
        let report = evaluate(raw.as_bytes(), &Inputs::default()).unwrap();
        let spam: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.rule_id.starts_with("spam."))
            .collect();
        assert_eq!(spam.len(), 2);
        assert!(
            spam.iter()
                .all(|f| f.location == Location::MimePart { path: vec![0] })
        );
        // One more part exceeds the engine's own admission, not ours.
        let raw = multipart(&[leaf; 256]);
        assert_eq!(
            evaluate(raw.as_bytes(), &Inputs::default()),
            Err(AnalysisError::Failed)
        );
    }

    #[test]
    fn blank_and_unparseable_input() {
        assert_eq!(scan(b""), Ok(Vec::new()));
        assert_eq!(scan(b" "), Err(AnalysisError::Failed));
        assert_eq!(scan(b"\r\n"), Ok(Vec::new()));
    }

    /// Adversarial near-miss text of exactly `n` bytes: no phrase matches.
    fn near_miss(n: usize) -> String {
        let unit = "act nowx ";
        let mut text = unit.repeat(n / unit.len());
        text.push_str(&"x".repeat(n % unit.len()));
        assert_eq!(text.len(), n);
        assert!(!phrases(&text));
        text
    }

    fn plain(text: &str) -> String {
        format!("Content-Type: text/plain\r\n\r\n{text}")
    }

    #[test]
    fn text_cap_is_exact() {
        assert_eq!(SPAM_TEXT_BYTES, 4_194_304);
        let at = plain(&near_miss(SPAM_TEXT_BYTES));
        assert_eq!(scan(at.as_bytes()), Ok(Vec::new()));
        let over = plain(&near_miss(SPAM_TEXT_BYTES + 1));
        assert_eq!(scan(over.as_bytes()), Err(AnalysisError::Failed));
        // The sum over parts is what counts.
        let half = SPAM_TEXT_BYTES / 2;
        let a = plain(&near_miss(half));
        let c = plain(&near_miss(half + 1));
        assert_eq!(scan(multipart(&[&a, &a]).as_bytes()), Ok(Vec::new()));
        assert_eq!(
            scan(multipart(&[&a, &c]).as_bytes()),
            Err(AnalysisError::Failed)
        );
        // An over-cap total fails even when an earlier part would have matched.
        let early = plain("act now");
        let big = plain(&near_miss(SPAM_TEXT_BYTES));
        assert_eq!(
            scan(multipart(&[&early, &big]).as_bytes()),
            Err(AnalysisError::Failed)
        );
        // An over-cap attachment is not eligible text, so it does not count.
        let attachment = format!(
            "Content-Type: text/plain\r\nContent-Disposition: attachment; filename=a.txt\r\n\r\n{}",
            near_miss(SPAM_TEXT_BYTES + 1)
        );
        let found = scan(multipart(&[&early, &attachment]).as_bytes()).unwrap();
        assert_eq!(found.len(), 1);
        // The engine's raw bound still precedes every collector.
        let mut raw = b"Content-Type: application/octet-stream\r\n\r\n".to_vec();
        raw.resize(MAX_RAW_BYTES, b'x');
        assert!(evaluate(&raw, &Inputs::default()).is_ok());
        raw.push(b'x');
        assert_eq!(
            evaluate(&raw, &Inputs::default()),
            Err(AnalysisError::Failed)
        );
    }

    #[test]
    fn evaluate_is_deterministic() {
        let raw = "From: \"x@y.co\" <a@b.co>\r\nSubject: ACT NOW!! FREE\r\n\r\nrisk-free \u{200B}\u{200B}\u{200B}";
        let first = evaluate(raw.as_bytes(), &Inputs::default()).unwrap();
        let again = evaluate(raw.as_bytes(), &Inputs::default()).unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&again).unwrap()
        );
        assert_eq!(first.rule_set_id, "swarmail-html/5");
        let spam = first
            .findings
            .iter()
            .filter(|f| f.rule_id.starts_with("spam."))
            .count();
        assert_eq!(spam, 6);
    }

    #[test]
    fn guide_documents_every_rule() {
        let guide = include_str!("../../docs/guide/deliverability.md");
        for signal in SIGNALS {
            let heading = format!("### `{}`\n", signal.id());
            let start = guide.find(&heading).expect("a heading per rule");
            let rest = &guide[start + heading.len()..];
            let section = &rest[..rest.find("\n###").unwrap()];
            let word = format!("{:?}", signal.severity()).to_lowercase();
            let severity = format!("**Severity:** {word}");
            assert!(section.contains(&severity), "{} {severity}", signal.id());
            assert!(section.contains(signal.hint()), "{}", signal.id());
            assert!(section.contains("proposed in 0122 — not in the report"));
            assert!(section.contains("**Passing:**"));
            assert!(section.contains("**Failing:**"));
        }
        for phrase in PHRASE_LIST {
            assert!(guide.contains(&format!("`{phrase}`")), "{phrase}");
        }
        for bound in ["8,388,608", "65,536", "256 including root", "4,194,304"] {
            assert!(guide.contains(bound), "{bound}");
        }
        assert!(guide.contains("## Spam signals (`swarmail-spam/1`)"));
    }
}
