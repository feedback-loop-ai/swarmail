//! The focused waits (decision 0094): `swarmail_wait_for_code` answers the
//! newest eligible mail's code with its provenance and every older one as
//! an explicit list, and `swarmail_wait_for_link` the one destination its
//! selectors leave — or, either way, the ambiguity, never a guess.
//!
//! Both read only what ingest stored (decision 0005): codes, the link
//! occurrences of 0094 §1, and for a historical row its legacy `links`
//! with unknown labels. Selection reads the whole filtered snapshot; the
//! 250-entry lists bound only the presentation, built while counting.

use super::near_miss;
use super::{
    McpContext, Revision, arg_i64, arg_inbox, arg_str, error_result, refusal, text_result,
};
use crate::model::{Email, LinkSource};
use crate::store::{Filter, FocusedEvaluation};
use serde_json::{Map, Value, json};
use std::collections::HashSet;
use std::sync::Arc;

pub(super) const CODE: &str = "swarmail_wait_for_code";
pub(super) const LINK: &str = "swarmail_wait_for_link";

/// The most entries any one candidate list presents.
const LIST_CAP: usize = 250;
/// A selector's source bound, in UTF-8 bytes.
const PATTERN_BYTES: usize = 1024;
/// A selector's compiled-size bound.
const COMPILED_BYTES: usize = 1 << 20;

/// The link selectors, compiled once per request before anything parks.
/// All supplied selectors must match the same occurrence.
struct Selectors {
    anchor_text: Option<regex::Regex>,
    host: Option<regex::bytes::Regex>,
    path: Option<regex::Regex>,
}

/// A supplied selector's source, bounded before it is compiled.
fn pattern(args: &Value, key: &str) -> Result<Option<String>, (i64, String)> {
    match arg_str(args, key)? {
        Some(pattern) if pattern.len() > PATTERN_BYTES => {
            Err(refusal(key, "a regex of at most 1024 UTF-8 bytes"))
        }
        pattern => Ok(pattern),
    }
}

fn uncompiled(key: &str) -> (i64, String) {
    refusal(key, "a valid regex within the 1 MiB compiled-size limit")
}

/// A case-sensitive Unicode search, as written.
fn text_selector(args: &Value, key: &str) -> Result<Option<regex::Regex>, (i64, String)> {
    pattern(args, key)?
        .map(|source| {
            regex::RegexBuilder::new(&source)
                .size_limit(COMPILED_BYTES)
                .build()
                .map_err(|_| uncompiled(key))
        })
        .transpose()
}

impl Selectors {
    /// Every selector refuses by name and never echoes the pattern.
    fn parse(args: &Value) -> Result<Self, (i64, String)> {
        let anchor_text = text_selector(args, "anchor_text")?;
        // A parsed host is ASCII (IDNA included): bytes mode with Unicode
        // off keeps `.` and classes byte-wise, and the ASCII-insensitive
        // default is a flag the caller's own `(?-i)` can turn off.
        let host = pattern(args, "host")?
            .map(|source| {
                regex::bytes::RegexBuilder::new(&source)
                    .unicode(false)
                    .case_insensitive(true)
                    .size_limit(COMPILED_BYTES)
                    .build()
                    .map_err(|_| uncompiled("host"))
            })
            .transpose()?;
        let path = text_selector(args, "path")?;
        Ok(Self {
            anchor_text,
            host,
            path,
        })
    }

    /// One occurrence against every supplied selector. Host and path come
    /// from a local parse of the stored URL, only when one is asked for:
    /// the parser's host spelling without credentials or port, and its
    /// serialized path without query or fragment. No network is touched.
    fn matches(&self, link: &Occurrence<'_>) -> bool {
        if let Some(anchor) = &self.anchor_text
            && !link.anchor_text.is_some_and(|text| anchor.is_match(text))
        {
            return false;
        }
        if self.host.is_none() && self.path.is_none() {
            return true;
        }
        let Ok(url) = reqwest::Url::parse(link.url) else {
            return false;
        };
        self.host.as_ref().is_none_or(|host| {
            url.host_str()
                .is_some_and(|name| host.is_match(name.as_bytes()))
        }) && self
            .path
            .as_ref()
            .is_none_or(|path| path.is_match(url.path()))
    }
}

/// One link occurrence as the link wait presents it.
struct Occurrence<'a> {
    index: usize,
    url: &'a str,
    anchor_text: Option<&'a str>,
    rel: Option<&'a str>,
    source: Option<LinkSource>,
}

/// Every occurrence of one mail: the ingest-time metadata, or for a row
/// stored before it existed the legacy `links` with unknown label, rel and
/// source. Fresh empty metadata is known empty and never falls back.
fn occurrences<'a>(email: &'a Email, mut visit: impl FnMut(Occurrence<'a>)) {
    match &email.link_metadata {
        Some(metadata) => {
            for (index, link) in metadata.iter().enumerate() {
                visit(Occurrence {
                    index,
                    url: &link.url,
                    anchor_text: link.anchor_text.as_deref(),
                    rel: link.rel.as_deref(),
                    source: Some(link.source),
                });
            }
        }
        None => {
            for (index, url) in email.links.iter().enumerate() {
                visit(Occurrence {
                    index,
                    url,
                    anchor_text: None,
                    rel: None,
                    source: None,
                });
            }
        }
    }
}

/// A presented list: the first `cap` entries ([`LIST_CAP`] unless built
/// with [`List::with_cap`]), built only while there is room, and the exact
/// total over all of them.
pub(super) struct List {
    entries: Vec<Value>,
    total: usize,
    cap: usize,
}

impl Default for List {
    fn default() -> Self {
        Self::with_cap(LIST_CAP)
    }
}

impl List {
    pub(super) fn with_cap(cap: usize) -> Self {
        Self {
            entries: Vec::new(),
            total: 0,
            cap,
        }
    }

    pub(super) fn push(&mut self, entry: impl FnOnce() -> Value) {
        self.total += 1;
        if self.entries.len() < self.cap {
            self.entries.push(entry());
        }
    }

    /// `name`, `name_total` and `name_truncated` into `out`.
    pub(super) fn into(self, out: &mut Map<String, Value>, name: &str) {
        out.insert(
            format!("{name}_truncated"),
            json!(self.total > self.entries.len()),
        );
        out.insert(format!("{name}_total"), json!(self.total));
        out.insert(name.to_string(), Value::Array(self.entries));
    }
}

/// The provenance every candidate carries. `age_ms` is the parse-start
/// age at the one sampled response time, never negative; the widened
/// difference of two i64 values is at most u64::MAX, so the cast is exact.
/// `sender` is the stored From — which can be the envelope fallback — or
/// null; nothing is reparsed or inferred.
fn provenance(email: &Email, now: i64) -> Map<String, Value> {
    let age = (i128::from(now) - i128::from(email.received_ms)).max(0) as u64;
    let mut entry = Map::new();
    entry.insert("email_id".into(), json!(email.id));
    entry.insert("received_ms".into(), json!(email.received_ms));
    entry.insert("age_ms".into(), json!(age));
    entry.insert("sender".into(), json!(email.from));
    entry
}

fn code_entry(email: &Email, code: &str, now: i64) -> Value {
    let mut entry = provenance(email, now);
    entry.insert("code".into(), json!(code));
    Value::Object(entry)
}

/// Codes: the newest eligible mail in acceptance order decides. One
/// distinct code there selects; several are ambiguous, with no fallback to
/// older mail. Older eligible mail contributes its distinct codes in
/// acceptance order, then stored order; equal codes in different mail
/// keep separate provenance.
fn code_result(emails: &[Arc<Email>]) -> Option<Value> {
    let mut eligible = emails.iter().filter(|email| !email.codes.is_empty());
    let newest = eligible.next()?;
    let now = chrono::Utc::now().timestamp_millis();
    let mut candidates = List::default();
    let mut older = List::default();
    for code in distinct(&newest.codes) {
        candidates.push(|| code_entry(newest, code, now));
    }
    let mut matched = 1usize;
    for email in eligible {
        matched += 1;
        for code in distinct(&email.codes) {
            older.push(|| code_entry(email, code, now));
        }
    }
    let mut out = Map::new();
    if candidates.total == 1 {
        out.insert("status".into(), json!("selected"));
        out.insert("selected".into(), candidates.entries[0].clone());
    } else {
        out.insert("status".into(), json!("ambiguous"));
    }
    out.insert("matched".into(), json!(matched));
    candidates.into(&mut out, "candidates");
    older.into(&mut out, "older_candidates");
    Some(Value::Object(out))
}

/// One mail's codes without repeats, in stored order, for this response.
fn distinct(codes: &[String]) -> impl Iterator<Item = &str> {
    let mut seen = HashSet::new();
    codes
        .iter()
        .map(String::as_str)
        .filter(move |code| seen.insert(*code))
}

/// Links: every matching occurrence of the whole snapshot, newest mail
/// first, then occurrence order. Exactly one distinct exact URL selects,
/// however often it repeats; two spellings are two destinations even when
/// a parser would normalize them alike.
fn link_result(emails: &[Arc<Email>], selectors: &Selectors) -> Option<Value> {
    let now = chrono::Utc::now().timestamp_millis();
    let mut candidates = List::default();
    let mut urls = HashSet::new();
    let mut matched = 0usize;
    for email in emails {
        let mut eligible = false;
        occurrences(email, |link| {
            if selectors.matches(&link) {
                eligible = true;
                urls.insert(link.url);
                candidates.push(|| {
                    let mut entry = provenance(email, now);
                    entry.insert("url".into(), json!(link.url));
                    entry.insert("link_index".into(), json!(link.index));
                    entry.insert("anchor_text".into(), json!(link.anchor_text));
                    entry.insert("rel".into(), json!(link.rel));
                    entry.insert("source".into(), json!(link.source));
                    Value::Object(entry)
                });
            }
        });
        matched += usize::from(eligible);
    }
    if matched == 0 {
        return None;
    }
    let mut out = Map::new();
    if urls.len() == 1 {
        out.insert("status".into(), json!("selected"));
        out.insert("selected_url".into(), json!(urls.iter().next()));
    } else {
        out.insert("status".into(), json!("ambiguous"));
    }
    out.insert("matched".into(), json!(matched));
    out.insert("distinct_urls".into(), json!(urls.len()));
    candidates.into(&mut out, "candidates");
    Some(Value::Object(out))
}

/// The timed-out answer: nothing eligible, nothing presented.
fn timed_out(link: bool) -> Value {
    let mut out = Map::new();
    out.insert("status".into(), json!("timed_out"));
    out.insert("matched".into(), json!(0));
    List::default().into(&mut out, "candidates");
    if link {
        out.insert("distinct_urls".into(), json!(0));
    } else {
        List::default().into(&mut out, "older_candidates");
    }
    Value::Object(out)
}

/// Stage and run one focused wait. Every refusal — closed shape, types,
/// inbox, filters, selectors, then the timeout's isError — answers before
/// the readiness park and before any watcher is registered.
pub(super) async fn call(
    ctx: &McpContext,
    name: &str,
    a: &Value,
    revision: Revision,
) -> Result<Value, (i64, String)> {
    let inbox = arg_inbox(a)?;
    let filter = Filter {
        to: arg_str(a, "to")?,
        from: arg_str(a, "from")?,
        subject: arg_str(a, "subject")?,
        since_ms: arg_i64(a, "since_ms")?,
    };
    let link = name == LINK;
    let evaluate: FocusedEvaluation = if link {
        let selectors = Selectors::parse(a)?;
        Arc::new(move |emails| link_result(&emails, &selectors))
    } else {
        Arc::new(|emails| code_result(&emails))
    };
    let timeout_ms = match a.get("timeout_ms") {
        None => crate::api::DEFAULT_WAIT_TIMEOUT_MS,
        Some(v) => match v.as_u64() {
            Some(ms) => ms,
            None => return Ok(error_result(crate::api::WAIT_TIMEOUT_REFUSAL.to_string())),
        },
    };
    let timeout = match crate::api::checked_wait_timeout_ms(timeout_ms) {
        Ok(timeout) => timeout,
        Err(msg) => return Ok(error_result(msg)),
    };
    // The same readiness gauge and shutdown race as `swarmail_wait_for_email`.
    let _park = crate::api::park_await(&inbox);
    let outcome = tokio::select! {
        outcome = ctx.store.wait_focused(&inbox, &filter, timeout, evaluate) => Some(outcome),
        _ = crate::api::shutdown_signal(ctx.shutdown.clone()) => None,
    };
    match outcome {
        Some(Some(found)) => Ok(text_result(found, revision)),
        Some(None) => {
            // A timeout names what the inbox holds instead (decision 0106):
            // read after the wait resolved, never inside it, and nothing
            // attached when the inbox holds nothing. The focused waits skip
            // no mail: what their filter admitted but carried no code or
            // selected link shows with an empty `missed`.
            let mut out = timed_out(link);
            let near = near_miss::near_misses(ctx.store.clone(), inbox, filter, false).await;
            if let Some(near) = near {
                out["near_misses"] = near;
            }
            Ok(text_result(out, revision))
        }
        None => Ok(error_result("shutting down".into())),
    }
}

/// The two descriptors, appended after the existing catalogue so every
/// earlier entry keeps its position and bytes. Inputs are closed and carry
/// the wait's filters and timeout domain; the link wait adds its selectors.
pub(super) fn descriptors() -> [Value; 2] {
    let filters = |selectors: bool| {
        let mut properties = json!({
            "inbox": { "type": "string" },
            "to": { "type": "string", "description": super::TO_FILTER_DESCRIPTION },
            "from": { "type": "string", "description": "Case-insensitive substring of the header From address" },
            "subject": { "type": "string" },
            "since_ms": { "type": "integer", "description": "Inclusive receive-time lower bound in ms since the epoch (parse start, not the Date header); negative values are legal. Use it, or a private inbox, to exclude mail older than the action" },
            "timeout_ms": { "type": "integer", "default": 5000, "minimum": 0, "maximum": 60000 }
        });
        if selectors {
            for (key, description) in [
                (
                    "anchor_text",
                    "Rust regex search over the lexical anchor text, case-sensitive; unknown labels (text-part, historical or unattributable links) never match",
                ),
                (
                    "host",
                    "Rust regex search over the parsed host (no credentials or port; IDNA as punycode, IPv6 bracketed), ASCII case-insensitive by default; exact host: ^example[.]test$",
                ),
                (
                    "path",
                    "Rust regex search over the parsed path, without query or fragment and with no extra decoding, case-sensitive; prefix: ^/confirm/",
                ),
            ] {
                properties[key] = json!({ "type": "string", "description": description });
            }
        }
        json!({ "type": "object", "properties": properties, "additionalProperties": false })
    };
    [
        json!({ "name": CODE, "description": "Wait for mail carrying codes and answer the newest: one distinct code in the newest eligible email (acceptance order) is selected, with its email id, sender and parse-start age; several codes there are ambiguous, never resolved from older mail. Older eligible emails' codes are listed as older_candidates. Each list shows at most 250 entries with exact totals and truncation flags; nothing eligible by timeout_ms (default 5000, at most 60000) is status timed_out.",
            "inputSchema": filters(false) }),
        json!({ "name": LINK, "description": "Wait for mail with a link matching every supplied selector on the same occurrence — anchor_text, host and path are Rust regex searches of at most 1024 UTF-8 bytes and 1 MiB compiled each — and answer the destination: exactly one distinct URL across all matching occurrences is selected_url; several are ambiguous, with every candidate's anchor text, rel, text/html source and provenance, never a guess. Candidates show at most 250 entries with exact totals; distinct_urls counts every destination. Nothing eligible by timeout_ms (default 5000, at most 60000) is status timed_out.",
            "inputSchema": filters(true) }),
    ]
}

/// The closed output variants (decision 0094 §7). Every member a variant
/// emits is required, a winner exists only in `selected`, and the
/// count-shaped invariants a schema can state are stated; the rest are the
/// producer's, proven by its tests.
pub(super) fn output_schema(link: bool, address: &Value) -> Value {
    let object = super::output_object;
    let count = json!({"type":"integer","minimum":0,"maximum":u64::MAX});
    let mut candidate = json!({
        "email_id":{"type":"string"},
        "received_ms":{"type":"integer","minimum":i64::MIN,"maximum":i64::MAX},
        "age_ms":count,
        "sender":{"anyOf":[address,{"type":"null"}]}
    });
    if link {
        for (key, schema) in [
            ("url", json!({"type":"string"})),
            ("link_index", count.clone()),
            ("anchor_text", json!({"type":["string","null"]})),
            ("rel", json!({"type":["string","null"]})),
            (
                "source",
                json!({"type":["string","null"],"enum":["text","html",null]}),
            ),
        ] {
            candidate[key] = schema;
        }
    } else {
        candidate["code"] = json!({"type":"string"});
    }
    let required: Vec<&String> = candidate.as_object().unwrap().keys().collect();
    let candidate = object(candidate.clone(), json!(required));
    let list = |min: u64, max: usize| json!({"type":"array","items":candidate,"minItems":min,"maxItems":max});
    let at_least = |min: u64| json!({"type":"integer","minimum":min,"maximum":u64::MAX});
    let exactly = |value: u64| json!({"type":"integer","const":value});
    let flag = json!({"type":"boolean"});
    let never = json!({"type":"boolean","const":false});
    let variant = |status: &str, mut properties: Value| {
        properties["status"] = json!({"type":"string","const":status});
        let required: Vec<&String> = properties.as_object().unwrap().keys().collect();
        object(properties.clone(), json!(required))
    };
    let (selected, ambiguous, timed_out) = if link {
        (
            json!({"matched":at_least(1),"candidates":list(1, LIST_CAP),"candidates_total":at_least(1),
                "candidates_truncated":flag,"distinct_urls":exactly(1),"selected_url":{"type":"string"}}),
            json!({"matched":at_least(1),"candidates":list(2, LIST_CAP),"candidates_total":at_least(2),
                "candidates_truncated":flag,"distinct_urls":at_least(2)}),
            json!({"matched":exactly(0),"candidates":list(0, 0),"candidates_total":exactly(0),
                "candidates_truncated":never,"distinct_urls":exactly(0)}),
        )
    } else {
        let older = list(0, LIST_CAP);
        (
            json!({"matched":at_least(1),"candidates":list(1, 1),"candidates_total":exactly(1),
                "candidates_truncated":never,"older_candidates":older,"older_candidates_total":count,
                "older_candidates_truncated":flag,"selected":candidate}),
            json!({"matched":at_least(1),"candidates":list(2, LIST_CAP),"candidates_total":at_least(2),
                "candidates_truncated":flag,"older_candidates":older,"older_candidates_total":count,
                "older_candidates_truncated":flag}),
            json!({"matched":exactly(0),"candidates":list(0, 0),"candidates_total":exactly(0),
                "candidates_truncated":never,"older_candidates":list(0, 0),
                "older_candidates_total":exactly(0),"older_candidates_truncated":never}),
        )
    };
    // The same timeout with the inbox's near misses (decision 0106): a
    // second closed variant, so exactly one of the two ever validates.
    let mut near = timed_out.clone();
    near["near_misses"] = near_miss::schema();
    json!({"type":"object","oneOf":[
        variant("selected", selected),
        variant("ambiguous", ambiguous),
        variant("timed_out", timed_out),
        variant("timed_out", near)
    ]})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EmailAddress, LinkMetadata};
    use std::sync::atomic::{AtomicU64, Ordering};

    fn context() -> (tokio::sync::watch::Sender<bool>, McpContext) {
        let (stop, shutdown) = tokio::sync::watch::channel(false);
        let ctx = McpContext {
            tool_rate: Arc::new(super::super::ToolRate::default()),
            store: Arc::new(crate::store::Store::new(0)),
            chaos: Arc::new(crate::chaos::Chaos::default()),
            shutdown,
        };
        (stop, ctx)
    }

    async fn answer(ctx: &McpContext, tool: &str, arguments: Value) -> Value {
        let req = json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
                         "params":{"name": tool, "arguments": arguments}});
        super::super::handle(ctx, &req).await.unwrap()
    }

    fn payload(reply: &Value) -> Value {
        serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
    }

    fn mail(id: &str, received_ms: i64, codes: &[&str]) -> Arc<Email> {
        let mut mail = crate::store::email(id, "box", "a@x.io");
        mail.received_ms = received_ms;
        mail.codes = codes.iter().map(|c| c.to_string()).collect();
        Arc::new(mail)
    }

    fn link(
        url: &str,
        anchor: Option<&str>,
        rel: Option<&str>,
        source: LinkSource,
    ) -> LinkMetadata {
        LinkMetadata {
            url: url.into(),
            anchor_text: anchor.map(Into::into),
            rel: rel.map(Into::into),
            source,
        }
    }

    fn ids(list: &Value) -> Vec<(String, String)> {
        list.as_array()
            .unwrap()
            .iter()
            .map(|e| {
                (
                    e["email_id"].as_str().unwrap().into(),
                    e["code"].as_str().unwrap().into(),
                )
            })
            .collect()
    }

    /// Decision 0094 §5: acceptance order decides, not the stamps or ids —
    /// the newest acceptance carries the smallest stamp and id here. Its one
    /// distinct code selects; older mail is listed with separate provenance
    /// for an equal code; ineligible mail is skipped; ages never go
    /// negative or overflow at either end of the i64 stamp range.
    #[test]
    fn codes_follow_acceptance_order_with_explicit_older_candidates() {
        let mut newest = (*mail("a-newest", i64::MIN, &["3333", "3333"])).clone();
        newest.from = Some(EmailAddress {
            name: Some("Sender".into()),
            address: "s@x.io".into(),
        });
        let snapshot = vec![
            Arc::new(newest),
            mail("b-uncoded", 0, &[]),
            mail("c-older", i64::MAX, &["1111", "1111", "3333"]),
        ];
        let before = chrono::Utc::now().timestamp_millis();
        let result = code_result(&snapshot).unwrap();
        let after = chrono::Utc::now().timestamp_millis();
        assert_eq!(result["status"], "selected");
        assert_eq!(result["matched"], 2);
        assert_eq!(result["selected"], result["candidates"][0]);
        let selected = result["selected"].as_object().unwrap();
        assert_eq!(
            selected.keys().collect::<Vec<_>>(),
            ["age_ms", "code", "email_id", "received_ms", "sender"]
        );
        assert_eq!(selected["code"], "3333");
        assert_eq!(
            selected["sender"],
            json!({"name":"Sender","address":"s@x.io"})
        );
        let age = selected["age_ms"].as_u64().unwrap();
        let widened = |now: i64| (i128::from(now) - i128::from(i64::MIN)) as u64;
        assert!((widened(before)..=widened(after)).contains(&age));
        assert!(age > i64::MAX as u64);
        assert_eq!(
            ids(&result["older_candidates"]),
            [
                ("c-older".into(), "1111".into()),
                ("c-older".into(), "3333".into())
            ]
        );
        assert_eq!(result["older_candidates"][0]["age_ms"], 0);
        assert_eq!(result["older_candidates"][0]["sender"], Value::Null);
        assert_eq!(
            (
                result["candidates_total"].clone(),
                result["candidates_truncated"].clone()
            ),
            (json!(1), json!(false))
        );
        assert_eq!(
            (
                result["older_candidates_total"].clone(),
                result["older_candidates_truncated"].clone()
            ),
            (json!(2), json!(false))
        );
        assert_eq!(code_result(&[mail("none", 0, &[])]), None);
    }

    /// Several distinct codes in the newest mail are ambiguous with no
    /// fallback to an older single code; the lists cap at 250 entries only
    /// after every code is counted, historical arrays above 64 included.
    #[test]
    fn newest_multicode_is_ambiguous_and_lists_cap_after_counting() {
        let many = |n: usize| -> Vec<String> { (0..n).map(|i| format!("{i:04}")).collect() };
        let (c250, c251) = (many(250), many(251));
        let result = code_result(&[mail("new", 0, &["1", "2"]), mail("old", 0, &["9"])]).unwrap();
        assert_eq!(result["status"], "ambiguous");
        assert!(result.get("selected").is_none());
        assert_eq!(
            ids(&result["candidates"]),
            [("new".into(), "1".into()), ("new".into(), "2".into())]
        );
        assert_eq!(result["older_candidates_total"], 1);
        let result = code_result(&[mail(
            "new",
            0,
            &c250.iter().map(String::as_str).collect::<Vec<_>>(),
        )])
        .unwrap();
        assert_eq!(result["candidates_total"], 250);
        assert_eq!(result["candidates"].as_array().unwrap().len(), 250);
        assert_eq!(result["candidates_truncated"], false);
        let result = code_result(&[mail(
            "new",
            0,
            &c251.iter().map(String::as_str).collect::<Vec<_>>(),
        )])
        .unwrap();
        assert_eq!(result["candidates_total"], 251);
        assert_eq!(result["candidates"].as_array().unwrap().len(), 250);
        assert_eq!(result["candidates_truncated"], true);
        assert_eq!(result["candidates"][249]["code"], "0249");
        let result = code_result(&[
            mail("new", 0, &["7"]),
            mail("a", 0, &c250.iter().map(String::as_str).collect::<Vec<_>>()),
            mail("b", 0, &["1"]),
        ])
        .unwrap();
        assert_eq!(result["status"], "selected");
        assert_eq!(result["older_candidates_total"], 251);
        assert_eq!(result["older_candidates_truncated"], true);
        assert_eq!(result["older_candidates"].as_array().unwrap().len(), 250);
        assert_eq!(result["matched"], 3);
    }

    fn selectors(args: Value) -> Selectors {
        Selectors::parse(&args).unwrap()
    }

    fn count(email: &Email, selectors: &Selectors) -> usize {
        let mut n = 0;
        occurrences(email, |link| n += usize::from(selectors.matches(&link)));
        n
    }

    /// Host and path come from a local parse of the stored URL: no
    /// credentials, port, query or fragment, no extra decoding; IDNA and
    /// bracketed IPv6 as the parser spells them; an exact host needs its
    /// anchors. Anchor patterns never match an unknown label.
    #[test]
    fn link_selectors_read_parsed_components() {
        let mut email = (*mail("links", 0, &[])).clone();
        email.links = vec![
            "invalid".into(),
            "https://user:pw@EXAMPLE.test:8080/a%20b?q=decoy#fragment".into(),
            "https://example.test.evil.io/a%20b".into(),
        ];
        for (args, n) in [
            (json!({}), 3),
            (json!({"anchor_text":""}), 0),
            (json!({"host":""}), 2),
            (json!({"path":""}), 2),
            (json!({"host":"example[.]test"}), 2),
            (json!({"host":"^example[.]test$","path":"^/a%20b$"}), 1),
            (json!({"host":"user|pw|8080|decoy"}), 0),
            (json!({"path":"decoy|fragment|a b"}), 0),
            (json!({"host":"(?-i)^EXAMPLE[.]test$"}), 0),
            (json!({"host":r"\Aexample[.]test\z"}), 1),
        ] {
            assert_eq!(count(&email, &selectors(args.clone())), n, "{args}");
        }
        for (url, host, path) in [
            ("http://[::1]:80/x", r"^\[::1\]$", "^/x$"),
            ("https://bücher.test/a", "^xn--bcher-kva[.]test$", "^/a$"),
        ] {
            email.links = vec![url.into()];
            let both = selectors(json!({"host":host,"path":path}));
            assert_eq!(count(&email, &both), 1, "{url}");
        }
        // A parsed URL without a host fails a host selector; an unparsable
        // one fails every component selector.
        email.links = vec!["data:text,x".into(), "http://[::1".into()];
        assert_eq!(count(&email, &selectors(json!({"host":".*"}))), 0);
        assert_eq!(count(&email, &selectors(json!({"path":".*"}))), 1);
    }

    /// Decision 0094 §6: all selectors match the same occurrence; one exact
    /// URL, however repeated, selects; a second destination — even past
    /// the 250th entry — is ambiguous; historical rows adapt their legacy
    /// links with unknown provenance; fresh empty metadata never does.
    #[test]
    fn link_selection_reads_the_whole_snapshot() {
        let mut fresh = (*mail("fresh", 5, &[])).clone();
        fresh.links = vec!["https://x.io/".into(), "https://y.io/".into()];
        fresh.link_metadata = Some(vec![
            link("https://x.io/", Some("Confirm"), None, LinkSource::Html),
            link(
                "https://x.io/",
                Some("Other"),
                Some("nofollow"),
                LinkSource::Html,
            ),
            link("https://y.io/", None, None, LinkSource::Text),
        ]);
        let none = selectors(json!({"anchor_text":"^Confirm$","host":"^y[.]io$"}));
        assert_eq!(link_result(&[Arc::new(fresh.clone())], &none), None);
        let labels = selectors(json!({"anchor_text":"(?i)confirm|Other"}));
        let result = link_result(&[Arc::new(fresh.clone())], &labels).unwrap();
        assert_eq!(result["status"], "selected");
        assert_eq!(result["selected_url"], "https://x.io/");
        assert_eq!(
            (result["matched"].clone(), result["distinct_urls"].clone()),
            (json!(1), json!(1))
        );
        assert_eq!(result["candidates_total"], 2);
        let second = result["candidates"][1].as_object().unwrap();
        assert_eq!(
            second.keys().collect::<Vec<_>>(),
            [
                "age_ms",
                "anchor_text",
                "email_id",
                "link_index",
                "received_ms",
                "rel",
                "sender",
                "source",
                "url"
            ]
        );
        assert_eq!(
            Value::Object(second.clone()),
            json!({"age_ms":second["age_ms"],"anchor_text":"Other","email_id":"fresh","link_index":1,
                "received_ms":5,"rel":"nofollow","sender":null,"source":"html","url":"https://x.io/"})
        );
        let mut historical = (*mail("historical", 4, &[])).clone();
        historical.links = vec!["https://x.io/".into(); 250];
        historical.links.push("https://x.io/other".into());
        let all = selectors(json!({}));
        let result = link_result(
            &[Arc::new(historical.clone())],
            &selectors(json!({"path":"^/$"})),
        )
        .unwrap();
        assert_eq!(result["status"], "selected");
        assert_eq!(result["candidates_total"], 250);
        assert_eq!(result["candidates_truncated"], false);
        assert_eq!(result["candidates"][0]["source"], Value::Null);
        assert_eq!(result["candidates"][0]["anchor_text"], Value::Null);
        let result = link_result(&[Arc::new(historical.clone())], &all).unwrap();
        assert_eq!(result["status"], "ambiguous");
        assert!(result.get("selected_url").is_none());
        assert_eq!(
            (
                result["distinct_urls"].clone(),
                result["candidates_total"].clone()
            ),
            (json!(2), json!(251))
        );
        assert_eq!(result["candidates"].as_array().unwrap().len(), 250);
        assert_eq!(result["candidates_truncated"], true);
        assert!(
            result["candidates"]
                .as_array()
                .unwrap()
                .iter()
                .all(|c| c["url"] == "https://x.io/")
        );
        // Newest first across mail; anchor selectors skip unknown labels.
        let snapshot = [Arc::new(fresh.clone()), Arc::new(historical.clone())];
        let result = link_result(&snapshot, &selectors(json!({"host":"^x[.]io$"}))).unwrap();
        assert_eq!(result["matched"], 2);
        assert_eq!(result["candidates"][0]["email_id"], "fresh");
        assert_eq!(result["candidates"][2]["email_id"], "historical");
        let result = link_result(&snapshot, &selectors(json!({"anchor_text":""}))).unwrap();
        assert_eq!(result["matched"], 1);
        // Known-empty metadata never falls back to the legacy array.
        fresh.link_metadata = Some(vec![]);
        assert_eq!(link_result(&[Arc::new(fresh)], &all), None);
        // Two spellings of one destination are two destinations.
        historical.links = vec!["https://x.io/".into(), "https://X.io/".into()];
        let result = link_result(&[Arc::new(historical)], &all).unwrap();
        assert_eq!(result["distinct_urls"], 2);
    }

    #[test]
    fn timed_out_answers_are_empty_and_closed() {
        assert_eq!(
            timed_out(false),
            json!({"status":"timed_out","matched":0,"candidates":[],"candidates_total":0,
                "candidates_truncated":false,"older_candidates":[],"older_candidates_total":0,
                "older_candidates_truncated":false})
        );
        assert_eq!(
            timed_out(true),
            json!({"status":"timed_out","matched":0,"candidates":[],"candidates_total":0,
                "candidates_truncated":false,"distinct_urls":0})
        );
    }

    /// Every refusal answers before the readiness park and registers no
    /// watcher: wrong types and explicit nulls, an unknown key, a bad
    /// inbox and over-long, invalid or over-compiled selectors are -32602
    /// naming the field without echoing the pattern; the timeout keeps its
    /// isError text refusal. The 1024-byte bound is inclusive.
    #[tokio::test]
    async fn admission_refuses_before_any_registration() {
        let (_stop, ctx) = context();
        for tool in [CODE, LINK] {
            for args in [
                json!({"timeout_ms":null}),
                json!({"timeout_ms":60001}),
                json!({"timeout_ms":-1}),
            ] {
                let reply = answer(&ctx, tool, args).await;
                assert_eq!(reply["result"]["isError"], true, "{tool} {reply}");
            }
            for (args, message) in [
                (json!({"to":null}), "to must be a string"),
                (json!({"from":1}), "from must be a string"),
                (json!({"subject":[]}), "subject must be a string"),
                (json!({"since_ms":1.5}), "since_ms must be an integer"),
                (json!({"inbox":""}), crate::model::BLANK_INBOX_REFUSAL),
                (json!({"count":1}), "unknown argument: count"),
            ] {
                let reply = answer(&ctx, tool, args).await;
                assert_eq!(reply["error"]["code"], -32602, "{tool} {reply}");
                assert_eq!(reply["error"]["message"], message);
            }
        }
        let reply = answer(&ctx, CODE, json!({"anchor_text":"x"})).await;
        assert_eq!(reply["error"]["message"], "unknown argument: anchor_text");
        let secret = "s3cret".repeat(200);
        for key in ["anchor_text", "host", "path"] {
            for (pattern, domain) in [
                (json!(null), "a string"),
                (
                    json!("[s3cret"),
                    "a valid regex within the 1 MiB compiled-size limit",
                ),
                (json!(secret), "a regex of at most 1024 UTF-8 bytes"),
                (
                    json!("s3cret{1000000}"),
                    "a valid regex within the 1 MiB compiled-size limit",
                ),
            ] {
                let mut args = json!({});
                args[key] = pattern;
                let reply = answer(&ctx, LINK, args).await;
                assert_eq!(reply["error"]["code"], -32602);
                assert_eq!(reply["error"]["message"], format!("{key} must be {domain}"));
            }
        }
        let bound = "é".repeat(512);
        assert!(Selectors::parse(&json!({"anchor_text":bound,"host":bound,"path":bound})).is_ok());
        let over = format!("{bound}a");
        assert!(Selectors::parse(&json!({"path":over})).is_err());
        assert_eq!(ctx.store.watcher_count(), 0, "no refusal subscribed");
        for tool in [CODE, LINK] {
            let reply = answer(&ctx, tool, json!({"timeout_ms":0})).await;
            assert_eq!(payload(&reply), timed_out(tool == LINK));
            assert_eq!(reply["result"]["isError"], false);
        }
    }

    /// Omitted inbox is `default`, the default timeout applies, and an
    /// eligible mail already stored answers at once.
    #[tokio::test]
    async fn defaults_select_the_default_inbox() {
        let (_stop, ctx) = context();
        let mut seeded = crate::store::email("seeded", "default", "a@x.io");
        seeded.codes = vec!["4242".into()];
        seeded.links = vec!["https://x.io/go".into()];
        ctx.store.insert(seeded).unwrap();
        let code = payload(&answer(&ctx, CODE, json!({})).await);
        assert_eq!(code["selected"]["code"], "4242");
        let link = payload(&answer(&ctx, LINK, json!({"path":"^/go$"})).await);
        assert_eq!(link["selected_url"], "https://x.io/go");
    }

    /// A stop after the wait has actually parked — its third clock read is
    /// the remaining time, computed right before `recv` with no await
    /// between — answers the shutdown error; a stopped server refuses a
    /// new wait the same way.
    #[tokio::test]
    async fn shutdown_after_park_answers_an_error() {
        for tool in [CODE, LINK] {
            let (stop, ctx) = context();
            let origin = std::time::Instant::now();
            let reads = Arc::new(AtomicU64::new(0));
            let parked = Arc::new(tokio::sync::Notify::new());
            let (observed, notify) = (reads.clone(), parked.clone());
            ctx.store.set_wait_clock(Some(Arc::new(move || {
                observed.fetch_add(1, Ordering::SeqCst);
                notify.notify_one();
                origin
            })));
            let ctx = Arc::new(ctx);
            let waiting = ctx.clone();
            let task = tokio::spawn(async move {
                answer(&waiting, tool, json!({"inbox":"parked","timeout_ms":60000})).await
            });
            while reads.load(Ordering::SeqCst) < 3 {
                parked.notified().await;
            }
            stop.send(true).unwrap();
            let stopped = tokio::time::timeout(std::time::Duration::from_secs(30), task)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(stopped["result"]["isError"], true);
            assert_eq!(stopped["result"]["content"][0]["text"], "shutting down");
            let again = answer(&ctx, tool, json!({"inbox":"parked"})).await;
            assert_eq!(again["result"]["content"][0]["text"], "shutting down");
        }
    }
}
