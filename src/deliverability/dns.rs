//! The declared DNS policy that the authentication stories read (decision 0120).
//!
//! A policy is a map of TXT records and a set of names that fail temporarily.
//! It is configuration, never derived from mail: the server file or a per-inbox
//! `PUT` supplies it, a request freezes it once, and every report records the
//! whole effective policy and each lookup a verifier made. Nothing here touches
//! the network, a clock, a global or a lock, so a report is a pure function of
//! the stored bytes and the configuration (decision 0085).

use serde::de::{Deserializer, Error as _, MapAccess, Visitor};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

/// One raw-size bound for the file, the API body and (through the canonical
/// form, which is never longer than its input plus 24 bytes) a report's policy.
pub(crate) const MAX_DNS_POLICY_BYTES: usize = 8_192;
/// Inboxes that may hold a declaration at once; the next is refused, none evicted.
pub(crate) const MAX_DECLARED_INBOXES: usize = 4_096;

/// A canonical DNS owner name: ASCII, lowercase, no trailing dot, dot-separated
/// labels of 1-63 bytes of `[a-z0-9_-]`, at most 253 bytes. A declared name
/// matches only itself: there is no wildcard expansion and no IDNA.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct OwnerName(String);

impl OwnerName {
    /// `None` for anything outside the grammar; never a panic, so a name taken
    /// from a header can be offered without a pre-check.
    pub(crate) fn parse(name: &str) -> Option<Self> {
        let name = name.strip_suffix('.').unwrap_or(name);
        let valid = (1..=253).contains(&name.len())
            && name.split('.').all(|label| {
                (1..=63).contains(&label.len())
                    && label
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            });
        valid.then(|| Self(name.to_ascii_lowercase()))
    }
}

/// The per-inbox declarations are at capacity (`MAX_DECLARED_INBOXES`).
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DeclarationsFull;

/// The canonical content of a declaration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Policy {
    txt: BTreeMap<OwnerName, BTreeSet<String>>,
    tempfail: BTreeSet<OwnerName>,
}

/// Where the effective policy came from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    #[default]
    Default,
    Server,
    Inbox,
}

/// A request's frozen policy: source and content together are part of the
/// report cache's exact key (decision 0085 and 0120 §6).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Effective {
    pub(crate) source: Source,
    pub(crate) policy: Policy,
}

impl Effective {
    pub(crate) fn new(source: Source, policy: Policy) -> Arc<Self> {
        Arc::new(Self { source, policy })
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "#254/#255 verifiers call it through Lookups")
    )]
    fn answer(&self, name: &OwnerName) -> Txt {
        if self.policy.tempfail.contains(name) {
            Txt::TempFail
        } else if let Some(values) = self.policy.txt.get(name) {
            Txt::Records(values.iter().cloned().collect())
        } else {
            Txt::NoRecords
        }
    }
}

/// The wire form of a policy: the declaration shape, canonical, with both
/// members always present. A report's `dns.policy` can be declared back verbatim.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyView {
    pub txt: Vec<TxtView>,
    pub tempfail: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TxtView {
    pub name: String,
    pub values: Vec<String>,
}

impl Policy {
    pub(crate) fn view(&self) -> PolicyView {
        PolicyView {
            txt: self
                .txt
                .iter()
                .map(|(name, values)| TxtView {
                    name: name.0.clone(),
                    values: values.iter().cloned().collect(),
                })
                .collect(),
            tempfail: self.tempfail.iter().map(|name| name.0.clone()).collect(),
        }
    }
}

/// The input shapes are read by hand, as JSON objects only. A derived struct
/// would also accept a JSON array as a tuple (`[]` as an empty declaration),
/// and a repeated member would be merged or lost; both are refusals here.
struct Declaration {
    txt: Vec<TxtDeclaration>,
    tempfail: Vec<String>,
}

struct TxtDeclaration {
    name: String,
    values: Vec<String>,
}

impl<'de> Deserialize<'de> for Declaration {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Members;
        impl<'de> Visitor<'de> for Members {
            type Value = Declaration;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an object with optional txt and tempfail members")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Declaration, A::Error> {
                let (mut txt, mut tempfail) = (None, None);
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "txt" if txt.is_none() => txt = Some(map.next_value()?),
                        "tempfail" if tempfail.is_none() => tempfail = Some(map.next_value()?),
                        "txt" | "tempfail" => {
                            return Err(A::Error::custom(format!("duplicate field `{key}`")));
                        }
                        other => return Err(A::Error::unknown_field(other, &["txt", "tempfail"])),
                    }
                }
                Ok(Declaration {
                    txt: txt.unwrap_or_default(),
                    tempfail: tempfail.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_map(Members)
    }
}

impl<'de> Deserialize<'de> for TxtDeclaration {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Members;
        impl<'de> Visitor<'de> for Members {
            type Value = TxtDeclaration;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("an object with name and values members")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<TxtDeclaration, A::Error> {
                let (mut name, mut values) = (None, None);
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "name" if name.is_none() => name = Some(map.next_value()?),
                        "values" if values.is_none() => values = Some(map.next_value()?),
                        "name" | "values" => {
                            return Err(A::Error::custom(format!("duplicate field `{key}`")));
                        }
                        other => return Err(A::Error::unknown_field(other, &["name", "values"])),
                    }
                }
                Ok(TxtDeclaration {
                    name: name.ok_or_else(|| A::Error::missing_field("name"))?,
                    values: values.ok_or_else(|| A::Error::missing_field("values"))?,
                })
            }
        }
        deserializer.deserialize_map(Members)
    }
}

/// Validate a declaration into its canonical form. Nothing is repaired: every
/// duplicate, overlap or out-of-grammar spelling is a refusal. The messages
/// name the position, never the offending text, so they stay small.
pub(crate) fn parse(bytes: &[u8]) -> Result<Policy, String> {
    if bytes.len() > MAX_DNS_POLICY_BYTES {
        return Err(format!(
            "declaration is larger than {MAX_DNS_POLICY_BYTES} bytes"
        ));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "declaration is not valid UTF-8")?;
    let declaration: Declaration =
        serde_json::from_str(text).map_err(|e| format!("not a DNS policy declaration: {e}"))?;
    let mut txt = BTreeMap::new();
    for (i, entry) in declaration.txt.into_iter().enumerate() {
        let name = OwnerName::parse(&entry.name)
            .ok_or_else(|| format!("txt[{i}].name is not a valid owner name"))?;
        if entry.values.is_empty() {
            return Err(format!("txt[{i}].values is empty"));
        }
        let mut values = BTreeSet::new();
        for (j, value) in entry.values.into_iter().enumerate() {
            if !value.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
                return Err(format!("txt[{i}].values[{j}] is not printable ASCII"));
            }
            if !values.insert(value) {
                return Err(format!("txt[{i}].values[{j}] repeats a value"));
            }
        }
        if txt.insert(name, values).is_some() {
            return Err(format!("txt[{i}].name repeats an owner name"));
        }
    }
    let mut tempfail = BTreeSet::new();
    for (i, name) in declaration.tempfail.iter().enumerate() {
        let name = OwnerName::parse(name)
            .ok_or_else(|| format!("tempfail[{i}] is not a valid owner name"))?;
        if txt.contains_key(&name) {
            return Err(format!("tempfail[{i}] is also declared under txt"));
        }
        if !tempfail.insert(name) {
            return Err(format!("tempfail[{i}] repeats an owner name"));
        }
    }
    Ok(Policy { txt, tempfail })
}

/// Load the server file: a regular file, read through a bound so a device or an
/// endless stream cannot exhaust memory, then parsed. A FIFO is refused before
/// it is opened, so it cannot block startup.
pub(crate) fn load_file(path: &Path) -> Result<Policy, String> {
    fn read(path: &Path) -> std::io::Result<Vec<u8>> {
        if !std::fs::metadata(path)?.is_file() {
            return Err(std::io::Error::other("not a regular file"));
        }
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(MAX_DNS_POLICY_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    }
    parse(&read(path).map_err(|e| e.to_string())?)
}

/// What a declared map answers for a TXT query.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "#254/#255 verifiers receive these answers")
)]
pub(crate) enum Txt {
    /// The record's canonical values, sorted.
    Records(Vec<String>),
    /// A definite negative (NXDOMAIN and NODATA are not distinguished).
    NoRecords,
    /// An operational failure. Never a pass and never a clean result: a
    /// verifier that meets it owes a `not_evaluated` finding or an analysis
    /// failure (decision 0120 §4).
    TempFail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Records,
    NoRecords,
    TempFail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordType {
    Txt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LookupView {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: RecordType,
    pub outcome: Outcome,
    pub values: Vec<String>,
}

/// The report's `dns` member: which policy and which records an evaluation used.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DnsRecord {
    pub source: Source,
    pub policy: PolicyView,
    pub lookups: Vec<LookupView>,
}

/// The one lookup interface the verifiers call: built per evaluation over the
/// frozen snapshot, synchronous and pure. Every answered name is recorded once,
/// so a verifier cannot read DNS without the report saying so.
#[derive(Debug, Default)]
pub(crate) struct Lookups {
    effective: Arc<Effective>,
    used: BTreeMap<OwnerName, Txt>,
}

impl Lookups {
    pub(crate) fn new(effective: Arc<Effective>) -> Self {
        Self {
            effective,
            used: BTreeMap::new(),
        }
    }

    #[cfg_attr(not(test), expect(dead_code, reason = "#254/#255 verifiers call it"))]
    pub(crate) fn resolve_txt(&mut self, name: &OwnerName) -> Txt {
        self.used
            .entry(name.clone())
            .or_insert_with(|| self.effective.answer(name))
            .clone()
    }

    /// The report member: the complete effective policy and the lookups, sorted.
    pub(crate) fn finish(self) -> DnsRecord {
        DnsRecord {
            source: self.effective.source,
            policy: self.effective.policy.view(),
            lookups: self
                .used
                .into_iter()
                .map(|(name, txt)| {
                    let (outcome, values) = match txt {
                        Txt::Records(values) => (Outcome::Records, values),
                        Txt::NoRecords => (Outcome::NoRecords, Vec::new()),
                        Txt::TempFail => (Outcome::TempFail, Vec::new()),
                    };
                    LookupView {
                        name: name.0,
                        kind: RecordType::Txt,
                        outcome,
                        values,
                    }
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn name(s: &str) -> OwnerName {
        OwnerName::parse(s).unwrap()
    }

    #[test]
    fn owner_name_grammar() {
        assert_eq!(
            name("S1._DomainKey.Example.COM.").0,
            "s1._domainkey.example.com"
        );
        assert_eq!(name("a").0, "a");
        assert_eq!(name("_dmarc.a-b.example").0, "_dmarc.a-b.example");
        let long = |last: usize| {
            let mut parts = vec!["a".repeat(63); 3];
            parts.push("b".repeat(last));
            parts.join(".")
        };
        // 63 + 1 + 63 + 1 + 63 + 1 + 61 = 253 bytes; one more is refused.
        assert_eq!(long(61).len(), 253);
        assert!(OwnerName::parse(&long(61)).is_some());
        assert!(OwnerName::parse(&long(62)).is_none());
        assert!(OwnerName::parse(&format!("{}.com", "a".repeat(63))).is_some());
        assert!(OwnerName::parse(&format!("{}.com", "a".repeat(64))).is_none());
        for bad in [
            "",
            ".",
            "a..b",
            ".a",
            "a..",
            "a\0b",
            "a\r\nb",
            "a b",
            "é.com",
            "*.example",
            "a\u{3000}b",
            "a/b",
            "a:b",
        ] {
            assert!(OwnerName::parse(bad).is_none(), "{bad:?}");
        }
    }

    fn accept(declaration: serde_json::Value) -> Policy {
        parse(declaration.to_string().as_bytes()).unwrap()
    }

    #[test]
    fn parse_accepts_and_canonicalizes() {
        assert_eq!(parse(b"{}").unwrap(), Policy::default());
        let policy = accept(json!({
            "tempfail": ["B.Example.", "c.example"],
            "txt": [
                {"name": "Z.Example.", "values": ["v=spf1 -all", "  Keep Case  "]},
                {"name": "a.example", "values": ["x"]},
            ],
        }));
        assert_eq!(
            serde_json::to_string(&policy.view()).unwrap(),
            r#"{"txt":[{"name":"a.example","values":["x"]},{"name":"z.example","values":["  Keep Case  ","v=spf1 -all"]}],"tempfail":["b.example","c.example"]}"#
        );
        // Only `tempfail` is declared: `txt` is empty, not absent, on output.
        assert!(
            accept(json!({"tempfail": ["x.example"]}))
                .view()
                .txt
                .is_empty()
        );
        // The empty string is a legal TXT value.
        assert_eq!(
            accept(json!({"txt":[{"name":"e","values":[""]}]}))
                .txt
                .len(),
            1
        );
    }

    #[test]
    fn parse_refusals() {
        let pad = |n: usize| format!(r#"{{"txt":[],"tempfail":[]}}{}"#, " ".repeat(n));
        let (at, over) = (pad(8_168), pad(8_169));
        assert_eq!((at.len(), over.len()), (8_192, 8_193));
        assert!(parse(at.as_bytes()).is_ok());
        let cases: Vec<(Vec<u8>, &str)> = vec![
            (b"".to_vec(), "not a DNS policy declaration"),
            (b"{".to_vec(), "not a DNS policy declaration"),
            (b"[]".to_vec(), "not a DNS policy declaration"),
            (br#"{"extra":1}"#.to_vec(), "unknown field"),
            (br#"{"txt":[],"txt":[]}"#.to_vec(), "duplicate field `txt`"),
            (
                br#"{"txt":[{"name":"a","values":["x"],"ttl":1}]}"#.to_vec(),
                "unknown field",
            ),
            (br#"{"txt":null}"#.to_vec(), "not a DNS policy declaration"),
            (b"[[],[]]".to_vec(), "expected an object"),
            (b"null".to_vec(), "expected an object"),
            (
                br#"{"tempfail":[],"tempfail":[]}"#.to_vec(),
                "duplicate field `tempfail`",
            ),
            (
                br#"{"txt":[["a",["x"]]]}"#.to_vec(),
                "expected an object with name and values",
            ),
            (
                br#"{"txt":[{"name":"a","values":["x"],"values":["y"]}]}"#.to_vec(),
                "duplicate field `values`",
            ),
            (
                br#"{"txt":[{"name":"a","name":"b","values":["x"]}]}"#.to_vec(),
                "duplicate field `name`",
            ),
            (
                br#"{"txt":[{"values":["x"]}]}"#.to_vec(),
                "missing field `name`",
            ),
            (
                br#"{"txt":[{"name":"a"}]}"#.to_vec(),
                "missing field `values`",
            ),
            (br#"{"tempfail":[1]}"#.to_vec(), "invalid type"),
            (b"{\"txt\":[],\"x\":\"\xff\"}".to_vec(), "not valid UTF-8"),
            (over.into_bytes(), "larger than 8192 bytes"),
            (
                br#"{"txt":[{"name":"a b","values":["x"]}]}"#.to_vec(),
                "txt[0].name is not a valid owner name",
            ),
            (
                br#"{"txt":[{"name":"a","values":["x\ty"]}]}"#.to_vec(),
                "txt[0].values[0] is not printable ASCII",
            ),
            (
                br#"{"txt":[{"name":"a","values":["ok","x\u007f"]}]}"#.to_vec(),
                "txt[0].values[1] is not printable ASCII",
            ),
            (
                "{\"txt\":[{\"name\":\"a\",\"values\":[\"é\"]}]}"
                    .as_bytes()
                    .to_vec(),
                "is not printable ASCII",
            ),
            (
                br#"{"txt":[{"name":"a","values":[]}]}"#.to_vec(),
                "txt[0].values is empty",
            ),
            (
                br#"{"txt":[{"name":"a","values":["x","x"]}]}"#.to_vec(),
                "txt[0].values[1] repeats a value",
            ),
            (
                br#"{"txt":[{"name":"a","values":["x"]},{"name":"A.","values":["y"]}]}"#.to_vec(),
                "txt[1].name repeats an owner name",
            ),
            (
                br#"{"tempfail":["a","b c"]}"#.to_vec(),
                "tempfail[1] is not a valid owner name",
            ),
            (
                br#"{"tempfail":["a","A."]}"#.to_vec(),
                "tempfail[1] repeats an owner name",
            ),
            (
                br#"{"txt":[{"name":"a","values":["x"]}],"tempfail":["a."]}"#.to_vec(),
                "tempfail[0] is also declared under txt",
            ),
        ];
        for (input, message) in cases {
            let error =
                parse(&input).expect_err(&String::from_utf8_lossy(&input[..input.len().min(80)]));
            assert!(error.contains(message), "{error:?} lacks {message:?}");
        }
    }

    fn three_orders() -> [serde_json::Value; 3] {
        [
            json!({"txt":[{"name":"b.example","values":["2","1"]},{"name":"a.example","values":["z","y"]}],"tempfail":["d.example","c.example"]}),
            json!({"tempfail":["c.example","d.example"],"txt":[{"values":["y","z"],"name":"A.EXAMPLE"},{"name":"b.example.","values":["1","2"]}]}),
            json!({"txt":[{"name":"a.example","values":["z","y"]},{"name":"b.example","values":["1","2"]}],"tempfail":["c.example","d.example"]}),
        ]
    }

    #[test]
    fn canonical_view_is_order_independent() {
        let views: Vec<String> = three_orders()
            .into_iter()
            .map(|d| serde_json::to_string(&accept(d).view()).unwrap())
            .collect();
        assert_eq!(views[0], views[1]);
        assert_eq!(views[1], views[2]);
        // A report's `dns.policy` can be declared back verbatim.
        let back = parse(views[0].as_bytes()).unwrap();
        assert_eq!(back, accept(three_orders()[0].clone()));
        assert_eq!(serde_json::to_string(&back.view()).unwrap(), views[0]);
    }

    #[test]
    fn canonical_view_worst_case() {
        // Exactly 8,192 bytes, omitting `tempfail`; the escape `\"` is as long
        // in the canonical form as in the input.
        let head = r#"{"txt":[{"name":"A.example.","values":[""#;
        let tail = r#""]}]}"#;
        let fill = MAX_DNS_POLICY_BYTES - head.len() - tail.len();
        let mut value = "\\\"".repeat(fill / 2);
        value.push_str(&"x".repeat(fill - value.len()));
        let input = format!("{head}{value}{tail}");
        assert_eq!(input.len(), MAX_DNS_POLICY_BYTES);
        let canonical = serde_json::to_string(&parse(input.as_bytes()).unwrap().view()).unwrap();
        // The omitted `tempfail` costs `,"tempfail":[]` and the trailing-dot
        // strip saves one byte, so the canonical form is the input plus 13.
        assert_eq!(canonical.len(), MAX_DNS_POLICY_BYTES + 13);
        assert!(canonical.len() <= MAX_DNS_POLICY_BYTES + 24);
        // The input that omits both members pays for them exactly.
        let empty = serde_json::to_string(&parse(b"{}").unwrap().view()).unwrap();
        assert_eq!(empty, r#"{"txt":[],"tempfail":[]}"#);
        assert_eq!(empty.len(), 24);
    }

    #[test]
    fn lookup_outcomes() {
        let policy = accept(json!({
            "txt": [
                {"name": "s1._domainkey.example.com", "values": ["v=DKIM1; k=rsa; p=AAAA"]},
                {"name": "example.com", "values": ["v=spf1 -all", "other"]},
                {"name": "_dmarc.example.com", "values": ["v=DMARC1; p=reject"]},
            ],
            "tempfail": ["_dmarc.example.org"],
        }));
        let mut lookups = Lookups::new(Effective::new(Source::Server, policy));
        assert_eq!(
            lookups.resolve_txt(&name("S1._DomainKey.example.com")),
            Txt::Records(vec!["v=DKIM1; k=rsa; p=AAAA".into()])
        );
        assert_eq!(
            lookups.resolve_txt(&name("example.com")),
            Txt::Records(vec!["other".into(), "v=spf1 -all".into()])
        );
        assert_eq!(
            lookups.resolve_txt(&name("_dmarc.example.com")),
            Txt::Records(vec!["v=DMARC1; p=reject".into()])
        );
        assert_eq!(
            lookups.resolve_txt(&name("_dmarc.example.org")),
            Txt::TempFail
        );
        assert_eq!(
            lookups.resolve_txt(&name("undeclared.example")),
            Txt::NoRecords
        );
        // A repeat answers the same and records nothing new.
        assert_eq!(
            lookups.resolve_txt(&name("undeclared.example")),
            Txt::NoRecords
        );
        assert_eq!(lookups.used.len(), 5);
        let record = serde_json::to_value(lookups.finish()).unwrap();
        assert_eq!(
            record["lookups"],
            json!([
                {"name":"_dmarc.example.com","type":"txt","outcome":"records","values":["v=DMARC1; p=reject"]},
                {"name":"_dmarc.example.org","type":"txt","outcome":"temp_fail","values":[]},
                {"name":"example.com","type":"txt","outcome":"records","values":["other","v=spf1 -all"]},
                {"name":"s1._domainkey.example.com","type":"txt","outcome":"records","values":["v=DKIM1; k=rsa; p=AAAA"]},
                {"name":"undeclared.example","type":"txt","outcome":"no_records","values":[]},
            ])
        );
        assert_eq!(record["source"], "server");
        assert_eq!(record["policy"]["tempfail"], json!(["_dmarc.example.org"]));
        // Nothing resolved, nothing recorded; the default is the empty world.
        assert_eq!(
            serde_json::to_string(&Lookups::default().finish()).unwrap(),
            r#"{"source":"default","policy":{"txt":[],"tempfail":[]},"lookups":[]}"#
        );
    }

    #[cfg(unix)]
    #[test]
    fn load_file_refusals() {
        let dir = tempfile::tempdir().unwrap();
        let write = |file: &str, bytes: &[u8]| {
            let path = dir.path().join(file);
            std::fs::write(&path, bytes).unwrap();
            path
        };
        assert_eq!(
            load_file(&write("empty-map.json", b"{}")).unwrap(),
            Policy::default()
        );
        let good = write("good.json", br#"{"tempfail":["x.example"]}"#);
        assert_eq!(load_file(&good).unwrap().tempfail.len(), 1);
        let big = write("big.json", &vec![b' '; MAX_DNS_POLICY_BYTES + 1]);
        assert!(
            load_file(&big)
                .unwrap_err()
                .contains("larger than 8192 bytes")
        );
        let bad = write("bad.json", b"{");
        assert!(load_file(&bad).unwrap_err().contains("not a DNS policy"));
        let empty = write("zero.json", b"");
        assert!(load_file(&empty).unwrap_err().contains("not a DNS policy"));
        let missing = load_file(&dir.path().join("missing.json")).unwrap_err();
        assert!(missing.contains("No such file"), "{missing}");
        assert_eq!(load_file(dir.path()).unwrap_err(), "not a regular file");
        // A FIFO is refused without being opened: it must return, not hang.
        let fifo = dir.path().join("fifo");
        // No child process: a fork inside the test binary can hold a copy of
        // another test's lock descriptor and fail that test's `try_lock`.
        let path = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
        // SAFETY: `path` is a valid NUL-terminated string that outlives the call.
        assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
        assert_eq!(load_file(&fifo).unwrap_err(), "not a regular file");
        // An endless device is refused as not regular; it is never read.
        let zero = Path::new("/dev/zero");
        assert_eq!(load_file(zero).unwrap_err(), "not a regular file");
    }
}
