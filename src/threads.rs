//! Thread grouping.
//!
//! Mails belong to one thread when an id chain connects them — `References`
//! and `In-Reply-To`, unioned over their `Message-ID`s (decision 0018: a mail
//! threads by its chain even when the parent is absent from the inbox —
//! absence never authorizes a subject merge). A mail with no chain at all
//! falls back to its normalized subject, and groups only with other
//! chain-less orphans of the same normalized subject: the fallback is
//! orphan-only and never bridges two chains. The grouping is a pure function
//! of the inbox contents, recomputed per request: no thread state exists
//! anywhere, so it can never disagree with the store.
//!
//! Node naming is load-bearing: members are prefixed `id:`, `subject:` and
//! `mail:`, and a component's representative is its lexicographically
//! smallest member, so the grouping itself never depends on arrival order.
//! The advertised identity is deliberately NOT that representative: the
//! URL-safe `key` the UI and the REST thread view address threads by is the
//! FNV-1a of the conversation's own first mail's node — the oldest member,
//! whose anchor a reply can never rewrite — so a later reply, even one whose
//! message-id sorts before the root's, cannot move an advertised URL.
//!
//! The key is derived per request, so mail stored before this derivation
//! (or before any upgrade) is re-keyed on the next request — there is no
//! thread state to migrate. A key still moves when the conversation's oldest
//! mail leaves the newest-`VIEW_CAP` window, is deleted or cap-pruned, or
//! when two mails share a receive millisecond and the id tie-break re-orders
//! them; those are the documented residuals of anchoring on the oldest mail.

use crate::model::Email;
use crate::store::{Filter, Store};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

/// How many emails a view covers: the newest of the inbox, the same window
/// the HTML views have always rendered. Keeps a per-insert push bounded for
/// huge inboxes; the store remains the complete truth.
pub const VIEW_CAP: usize = 200;

/// One thread: its display subject, its stable key, and its conversation in
/// order (oldest first).
#[derive(Debug, Clone, Serialize)]
pub struct Thread {
    /// URL-safe identity: FNV-1a of the conversation's first mail's node —
    /// append-stable as replies arrive (issue #9).
    pub key: String,
    /// The subject of the oldest mail in the thread that has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// How many emails the view covers.
    pub count: usize,
    /// Receive time of the newest mail in the thread (unix millis) — what
    /// threads are ordered by (newest thread first).
    pub last_received_ms: i64,
    /// The conversation, oldest first.
    pub emails: Vec<Arc<Email>>,
}

/// Every message-id in a `References`/`In-Reply-To` style header value, in
/// header order: the `<...>` spans. When the header carries no complete span
/// at all (a malformed header), the whitespace tokens with their angle
/// brackets stripped — junk beside a valid span is ignored, never a panic.
pub fn parse_ids(header: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut rest = header;
    while let Some(start) = rest.find('<') {
        let Some(end) = rest[start + 1..].find('>') else {
            break; // unterminated '<': nothing complete follows
        };
        let id = rest[start + 1..start + 1 + end].trim();
        if !id.is_empty() {
            ids.push(id.to_string());
        }
        rest = &rest[start + end + 2..];
    }
    if ids.is_empty() {
        ids = header
            .split_whitespace()
            .map(|token| token.trim_matches(['<', '>']))
            .filter(|token| !token.is_empty())
            .map(str::to_string)
            .collect();
    }
    ids
}

/// The subject two mails must share to fall into one thread: leading
/// reply/forward markers stripped, trimmed. `None` when nothing remains.
pub fn normalize_subject(subject: Option<&str>) -> Option<String> {
    let mut current = subject?.trim();
    while let Some(rest) = strip_marker(current) {
        current = rest;
    }
    let normalized = current.trim();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized.to_string())
    }
}

/// One leading reply/forward marker, ASCII case-insensitive: `Re:`, `Fw:`,
/// `Fwd:`, `Aw:` — tolerating whitespace between the word and the colon
/// (`RE : Plan`, a mailer convention). A bare word or a numbered `Re[2]:`
/// is left alone: anything non-whitespace between word and colon ends the
/// marker.
fn strip_marker(subject: &str) -> Option<&str> {
    let bytes = subject.as_bytes();
    let word_end = bytes.iter().position(|b| !b.is_ascii_alphabetic())?;
    let colon = bytes[word_end..]
        .iter()
        .position(|b| *b == b':')
        .map(|i| word_end + i)?;
    if !bytes[word_end..colon].iter().all(u8::is_ascii_whitespace) {
        return None;
    }
    match subject[..word_end].to_ascii_lowercase().as_str() {
        "re" | "fw" | "fwd" | "aw" => Some(subject[colon + 1..].trim_start()),
        _ => None,
    }
}

/// Union-find over thread nodes. The root of a component is always its
/// lexicographically smallest member (`union` points the larger root at the
/// smaller one), so the representative does not depend on arrival order.
#[derive(Default)]
struct Components {
    parent: HashMap<String, String>,
}

impl Components {
    /// The component's representative for `x`, with the path compressed.
    /// `union` points the larger root at the smaller and the compression
    /// below points every visited node at the root, so no entry ever
    /// self-parents or cycles: the walk ends on the first parentless node.
    fn find(&mut self, x: &str) -> String {
        let mut root = self.parent.get(x).cloned().unwrap_or_else(|| x.to_string());
        while let Some(parent) = self.parent.get(&root) {
            root = parent.clone();
        }
        let mut cursor = x.to_string();
        while cursor != root {
            let next = self
                .parent
                .get(&cursor)
                .expect("every non-root node has a parent")
                .clone();
            self.parent.insert(cursor, root.clone());
            cursor = next;
        }
        root
    }

    fn union(&mut self, a: &str, b: &str) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return;
        }
        let (small, large) = if ra < rb { (ra, rb) } else { (rb, ra) };
        self.parent.insert(large, small);
    }
}

/// The node an email itself hangs from: its own message id when it has one,
/// else its In-Reply-To, else its first reference, else its
/// (Unicode-lowercased) normalized subject, else the mail's own id (a
/// thread of exactly one). The fold is `str::to_lowercase()` — one-thread
/// grouping across scripts (`Über`/`über`); the named residual is that
/// lowercase mapping is not full case folding, so `ß` and `ss` do not
/// merge (no ICU, no folding crate).
fn thread_node(email: &Email) -> String {
    if let Some(id) = &email.message_id {
        return format!("id:{id}");
    }
    if let Some(id) = &email.in_reply_to {
        return format!("id:{id}");
    }
    if let Some(first) = email.references.first() {
        return format!("id:{first}");
    }
    if let Some(subject) = normalize_subject(email.subject.as_deref()) {
        return format!("subject:{}", subject.to_lowercase());
    }
    format!("mail:{}", email.id)
}

/// FNV-1a 64-bit, hex-encoded: a stable, dependency-free way to turn a
/// thread's anchor — its first mail's node — into a short URL-safe key.
/// Distinct components never share a node, so distinct threads never share
/// a key.
fn thread_key(canonical: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in canonical.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Group `emails` into threads. Within a thread the conversation is ordered
/// oldest first (receive time, then id); the threads themselves are ordered
/// newest first (most recent activity, then key, so equal timestamps still
/// render deterministically).
pub fn group_threads(emails: &[Arc<Email>]) -> Vec<Thread> {
    // Pass 1: connect the id graph — every mail's own message id, its
    // In-Reply-To and its References are one component. The mail's subject
    // node is deliberately NOT unioned in (decision 0018): a subject hook
    // here would let any id-bearing mail bridge two unrelated chains whose
    // mails merely share a subject. The normalized subject groups chain-less
    // orphans only — pass 2 lands a mail with no ids on its subject node,
    // where other orphans of the same normalized subject already wait.
    let mut components = Components::default();
    for email in emails {
        let mut ids: Vec<String> = email
            .references
            .iter()
            .map(|id| format!("id:{id}"))
            .collect();
        if let Some(id) = &email.in_reply_to {
            ids.push(format!("id:{id}"));
        }
        if let Some(id) = &email.message_id {
            ids.push(format!("id:{id}"));
        }
        if let Some(first) = ids.first() {
            for other in &ids[1..] {
                components.union(first, other);
            }
        }
    }

    // Pass 2: every email lands in its component's bucket.
    let mut grouped: HashMap<String, Vec<Arc<Email>>> = HashMap::new();
    for email in emails {
        let representative = components.find(&thread_node(email));
        grouped
            .entry(representative)
            .or_default()
            .push(email.clone());
    }

    let mut threads: Vec<Thread> = grouped
        .into_values()
        .map(|mut emails| {
            emails.sort_by(|a, b| {
                a.received_ms
                    .cmp(&b.received_ms)
                    .then_with(|| a.id.cmp(&b.id))
            });
            Thread {
                // The anchor is the conversation's own first message (the
                // bucket is sorted oldest first), not the union-find
                // representative: appending replies — even one whose
                // message-id sorts before the root's — cannot change it.
                key: thread_key(&thread_node(&emails[0])),
                subject: emails.iter().find_map(|e| e.subject.clone()),
                count: emails.len(),
                last_received_ms: emails.last().map_or(0, |e| e.received_ms),
                emails,
            }
        })
        .collect();
    threads.sort_by(|a, b| {
        b.last_received_ms
            .cmp(&a.last_received_ms)
            .then_with(|| a.key.cmp(&b.key))
    });
    threads
}

/// The thread view of one inbox: the newest `VIEW_CAP` emails, grouped and
/// ordered. Both the REST endpoints and the SSE feed are built on this, so
/// the two surfaces cannot drift. The window is taken in the store
/// (`list_bounded`) — the view never walks the inbox deeper than its cap.
pub fn view(store: &Store, inbox: &str) -> Vec<Thread> {
    let recent: Vec<Arc<Email>> = store.list_bounded(inbox, &Filter::default(), VIEW_CAP);
    group_threads(&recent)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mail with only the fields threading looks at.
    fn mail(id: &str, subject: Option<&str>, ms: i64) -> Email {
        let mut email = crate::store::email(id, "box", "to@x.io");
        email.subject = subject.map(str::to_string);
        email.received_ms = ms;
        email
    }

    /// A mail with an explicit `References` chain.
    fn chained(id: &str, subject: Option<&str>, ms: i64, refs: &[&str]) -> Email {
        let mut email = mail(id, subject, ms);
        email.references = refs.iter().map(|r| r.to_string()).collect();
        email
    }

    fn group(emails: Vec<Email>) -> Vec<Thread> {
        let arcs: Vec<Arc<Email>> = emails.into_iter().map(Arc::new).collect();
        group_threads(&arcs)
    }

    fn keys(threads: &[Thread]) -> Vec<String> {
        threads.iter().map(|t| t.key.clone()).collect()
    }

    fn ids_of(thread: &Thread) -> Vec<String> {
        thread.emails.iter().map(|e| e.id.clone()).collect()
    }

    #[test]
    fn ids_are_parsed_from_bracket_spans() {
        assert_eq!(
            parse_ids("<a@x.io> <b@x.io>"),
            vec!["a@x.io".to_string(), "b@x.io".to_string()]
        );
        assert_eq!(parse_ids("junk before <a@x.io> and after"), vec!["a@x.io"]);
        assert_eq!(parse_ids(""), Vec::<String>::new());
    }

    #[test]
    fn malformed_headers_degrade_without_panicking() {
        // Unterminated bracket: the complete ids survive, the junk is dropped.
        assert_eq!(parse_ids("<a@x.io> trailing <oops"), vec!["a@x.io"]);
        // Empty span: ignored, nothing invented in its place.
        assert_eq!(parse_ids("<> <b@x.io>"), vec!["b@x.io"]);
        // No brackets at all: the tokens are the ids.
        assert_eq!(parse_ids("a@x.io b@x.io"), vec!["a@x.io", "b@x.io"]);
        // Unterminated bracket with no other content: brackets stripped.
        assert_eq!(parse_ids("<lonely@x.io"), vec!["lonely@x.io"]);
        // Angle-bracket soup with nothing inside.
        assert_eq!(parse_ids("<>"), Vec::<String>::new());
    }

    #[test]
    fn subjects_normalize_by_stripping_reply_markers() {
        assert_eq!(
            normalize_subject(Some("Re: Re: FWD:  Hello World ")),
            Some("Hello World".to_string())
        );
        assert_eq!(normalize_subject(Some("fwd: hello")), Some("hello".into()));
        assert_eq!(normalize_subject(Some("Re:")), None);
        assert_eq!(normalize_subject(Some("   ")), None);
        assert_eq!(normalize_subject(None), None);
        // Not a marker: unchanged (case preserved).
        assert_eq!(normalize_subject(Some("Re: hello")), Some("hello".into()));
        assert_eq!(
            normalize_subject(Some("Re[2]: hello")),
            Some("Re[2]: hello".into())
        );
        assert_eq!(
            normalize_subject(Some("Forwarding: hello")),
            Some("Forwarding: hello".into())
        );
        // A bare word with no colon is not a marker.
        assert_eq!(normalize_subject(Some("Rewrite")), Some("Rewrite".into()));
    }

    #[test]
    fn the_aw_marker_and_a_padded_colon_strip() {
        // The German convention `Aw:` and a whitespace-padded `RE :` are
        // markers too; the numbered variant and a foreign word are not.
        assert_eq!(normalize_subject(Some("AW: Plan")), Some("Plan".into()));
        assert_eq!(normalize_subject(Some("RE : Plan")), Some("Plan".into()));
        assert_eq!(normalize_subject(Some("aw :\tX")), Some("X".into()));
        assert_eq!(
            normalize_subject(Some("Aw[2]: Plan")),
            Some("Aw[2]: Plan".into())
        );
        assert_eq!(
            normalize_subject(Some("Auto: Plan")),
            Some("Auto: Plan".into())
        );
    }

    #[test]
    fn the_subject_fallback_folds_unicode_case() {
        // `Über`/`über` land in one thread. The doc comment names the
        // residual: lowercase mapping is not full folding, so `ß` vs `ss`
        // do not merge.
        let threads = group(vec![
            mail("u1", Some("Über Plan"), 100),
            mail("u2", Some("aw: über plan"), 200),
        ]);
        assert_eq!(threads.len(), 1, "Unicode case folds: {threads:?}");
        assert_eq!(ids_of(&threads[0]), ["u1", "u2"]);
    }

    #[test]
    fn a_reference_chain_is_one_thread_in_conversation_order() {
        // The chain's mails publish the ids their replies cite — the wire
        // evidence is what links them (decision 0018).
        let mut root = chained("root", Some("Plan"), 300, &[]);
        root.message_id = Some("root@x.io".into());
        let mut mid = chained("mid", Some("Re: Plan"), 200, &["root@x.io"]);
        mid.message_id = Some("mid@x.io".into());
        let leaf = chained("leaf", Some("Re: Plan"), 100, &["root@x.io", "mid@x.io"]);
        let threads = group(vec![root, mid, leaf]);
        assert_eq!(threads.len(), 1, "one chain, one thread: {threads:?}");
        assert_eq!(ids_of(&threads[0]), ["leaf", "mid", "root"]);
        assert_eq!(threads[0].count, 3);
        assert_eq!(threads[0].last_received_ms, 300);
        // The oldest mail in the thread names it.
        assert_eq!(threads[0].subject.as_deref(), Some("Re: Plan"));
    }

    #[test]
    fn an_unpublished_parent_leaves_the_chain_on_its_dangling_id() {
        // The reply cites an id no inbox mail publishes: it still threads by
        // that reference — with any other mail citing the same id — while
        // the chain-less parent stays an orphan on its subject. Absence does
        // not authorize a subject merge (decision 0018).
        let threads = group(vec![
            chained("root", Some("Plan"), 300, &[]),
            chained("mid", Some("Re: Plan"), 200, &["root"]),
            chained("leaf", Some("Re: Plan"), 100, &["root", "mid"]),
        ]);
        assert_eq!(threads.len(), 2, "{threads:?}");
        let chain = threads.iter().find(|t| t.count == 2).unwrap();
        assert_eq!(ids_of(chain), ["leaf", "mid"]);
        assert_eq!(chain.key, thread_key("id:root"), "the dangling id anchors");
        let orphan = threads.iter().find(|t| t.count == 1).unwrap();
        assert_eq!(ids_of(orphan), ["root"]);
    }

    #[test]
    fn in_reply_to_alone_links_a_reply() {
        let mut reply = mail("r2", Some("Re: Hello"), 200);
        reply.in_reply_to = Some("r1".to_string());
        let mut root = mail("r1", Some("Hello"), 100);
        root.message_id = Some("r1".into());
        let threads = group(vec![reply, root]);
        assert_eq!(threads.len(), 1, "{threads:?}");
        assert_eq!(ids_of(&threads[0]), ["r1", "r2"]);
    }

    #[test]
    fn subject_fallback_groups_chainless_mail() {
        let threads = group(vec![
            mail("a", Some("Re: Fwd: STATUS"), 100),
            mail("b", Some("status"), 200),
            mail("c", Some("Other"), 300),
        ]);
        assert_eq!(threads.len(), 2, "{threads:?}");
        let status = threads.iter().find(|t| t.count == 2).unwrap();
        assert_eq!(ids_of(status), ["a", "b"]);
        // The display subject keeps its original case.
        assert_eq!(status.subject.as_deref(), Some("Re: Fwd: STATUS"));
    }

    #[test]
    fn a_reference_only_links_when_the_parent_publishes_its_id() {
        // The parent has no Message-ID on record and the subject drifted:
        // the dangling reference cannot be resolved, so nothing links.
        let orphaned = group(vec![
            chained("m1", Some("Kickoff"), 100, &[]),
            chained("m2", Some("Re: Kickoff (notes attached)"), 200, &["m1"]),
        ]);
        assert_eq!(orphaned.len(), 2, "{orphaned:?}");
        // With the parent's Message-ID on record, the same chain links.
        let mut parent = chained("m1", Some("Kickoff"), 100, &[]);
        parent.message_id = Some("m1@x.io".into());
        let linked = group(vec![
            parent,
            chained(
                "m2",
                Some("Re: Kickoff (notes attached)"),
                200,
                &["m1@x.io"],
            ),
        ]);
        assert_eq!(linked.len(), 1, "{linked:?}");
    }

    #[test]
    fn a_chainless_reply_on_the_same_subject_stays_out_of_the_chain() {
        // Decision 0018: the subject fallback is orphan-only. A mail with no
        // chain never joins a chain merely because their subjects match —
        // "Re: Re: Kickoff" carries no ids, so it threads alone.
        let mut root = chained("m1", Some("Kickoff"), 100, &[]);
        root.message_id = Some("m1@x.io".into());
        let threads = group(vec![
            root,
            chained(
                "m2",
                Some("Re: Kickoff (notes attached)"),
                200,
                &["m1@x.io"],
            ),
            mail("m3", Some("Re: Re: Kickoff"), 300),
        ]);
        assert_eq!(threads.len(), 2, "{threads:?}");
        let chain = threads.iter().find(|t| t.count == 2).unwrap();
        assert_eq!(ids_of(chain), ["m1", "m2"]);
        let orphan = threads.iter().find(|t| t.count == 1).unwrap();
        assert_eq!(ids_of(orphan), ["m3"]);
    }

    #[test]
    fn two_chains_sharing_a_subject_stay_apart() {
        // Two explicit chains whose mails share a normalized subject are two
        // conversations; subject equality alone never welds them (red on the
        // deleted pass-1 subject-union, which merged them for the wrong
        // reason).
        let mut first = chained("a1", Some("Verify your email"), 100, &[]);
        first.message_id = Some("a1@x.io".into());
        let mut first_reply = chained("a2", Some("Re: Verify your email"), 150, &["a1@x.io"]);
        first_reply.message_id = Some("a2@x.io".into());
        let mut second = chained("b1", Some("verify your email"), 200, &[]);
        second.message_id = Some("b1@x.io".into());
        let threads = group(vec![first, first_reply, second]);
        assert_eq!(threads.len(), 2, "{threads:?}");
        // Newest thread first: b1 (received at 200) leads.
        assert_eq!(threads.iter().map(|t| t.count).collect::<Vec<_>>(), [1, 2]);
    }

    #[test]
    fn a_chain_and_a_subject_only_mail_with_equal_subjects_stay_apart() {
        let mut root = chained("r1", Some("Plan"), 100, &[]);
        root.message_id = Some("r1@x.io".into());
        let threads = group(vec![
            root,
            chained("r2", Some("Re: Plan"), 200, &["r1@x.io"]),
            mail("lonely", Some("Re: Fwd: Plan"), 300),
        ]);
        assert_eq!(threads.len(), 2, "{threads:?}");
        assert!(
            threads
                .iter()
                .all(|t| t.count == 1 || ids_of(t) == ["r1", "r2"])
        );
    }

    #[test]
    fn a_bridge_mail_merges_two_chains_and_the_retired_key_answers_nothing() {
        // A mail referencing two existing threads is a genuine bridge: one
        // conversation. The surviving key is the merged thread's anchor — the
        // oldest first mail — and every other pre-merge key is simply gone
        // from the view: a held client re-lists rather than silently reading
        // the wrong thread (decision 0018: merges pay in docs, not aliases).
        let mut a = chained("a", Some("Thread A"), 100, &[]);
        a.message_id = Some("a@x.io".into());
        let mut b = chained("b", Some("Thread B"), 200, &[]);
        b.message_id = Some("b@x.io".into());
        let separate = group(vec![a.clone(), b.clone()]);
        assert_eq!(separate.len(), 2, "{separate:?}");
        let key_a = separate
            .iter()
            .find(|t| ids_of(t) == ["a"])
            .unwrap()
            .key
            .clone();
        let key_b = separate
            .iter()
            .find(|t| ids_of(t) == ["b"])
            .unwrap()
            .key
            .clone();
        assert_ne!(key_a, key_b);

        let mut bridge = chained("bridge", Some("Both"), 300, &["a@x.io", "b@x.io"]);
        bridge.message_id = Some("c@x.io".into());
        let merged = group(vec![a, b, bridge]);
        assert_eq!(
            merged.len(),
            1,
            "the bridge welds the two chains: {merged:?}"
        );
        assert_eq!(ids_of(&merged[0]), ["a", "b", "bridge"]);
        // The surviving key is the merged thread's own anchor: the oldest
        // first mail's node — not the union-find representative.
        let expected = thread_key(&thread_node(&merged[0].emails[0]));
        assert_eq!(merged[0].key, expected);
        // The other pre-merge key is not an alias for the merged thread:
        // whichever one the merged thread does not carry is retired.
        let retired = [key_a, key_b]
            .into_iter()
            .find(|k| merged[0].key != *k)
            .expect("the merged thread keeps exactly one pre-merge key");
        assert!(
            !merged.iter().any(|t| t.key == *retired),
            "the retired key answers nothing: {retired}"
        );
    }

    #[test]
    fn malformed_reference_headers_do_not_authorize_a_subject_merge() {
        // "<bogus" parses (token fallback) to a dangling id: harmless. The
        // mail owns its chain, so it threads by it even though the parent is
        // absent — and the chain-less mail on the same subject stays apart
        // (decision 0018: absence does not authorize subject merging).
        let threads = group(vec![
            chained("a", Some("Plan"), 100, &["<bogus"]),
            mail("b", Some("Plan"), 200),
        ]);
        assert_eq!(threads.len(), 2, "{threads:?}");
        for thread in &threads {
            assert_eq!(thread.count, 1);
        }
    }

    #[test]
    fn headers_missing_entirely_leave_singletons() {
        let threads = group(vec![mail("a", None, 100), mail("b", None, 200)]);
        assert_eq!(threads.len(), 2, "no headers, no grouping: {threads:?}");
        for thread in &threads {
            assert!(thread.subject.is_none());
            assert_eq!(thread.count, 1);
        }
        // Newest thread first: b (received at 200) leads.
        assert_eq!(threads[0].last_received_ms, 200);
        // Two distinct singletons must not share a key.
        assert_ne!(keys(&threads)[0], keys(&threads)[1]);
    }

    #[test]
    fn duplicate_message_ids_still_land_in_one_thread() {
        let mut first = chained("dup-1", Some("Hi"), 100, &[]);
        first.message_id = Some("same@x.io".to_string());
        let mut second = chained("dup-2", Some("Re: Hi"), 200, &[]);
        second.message_id = Some("same@x.io".to_string());
        let threads = group(vec![first, second]);
        assert_eq!(threads.len(), 1, "{threads:?}");
        assert_eq!(threads[0].count, 2);
    }

    #[test]
    fn cyclic_references_collapse_into_one_thread() {
        // A genuine cycle — each mail cites the id the other publishes — is
        // one id component (decision 0018: the id graph alone links them).
        let mut a = chained("a", Some("Loop"), 100, &["b@x.io"]);
        a.message_id = Some("a@x.io".into());
        let mut b = chained("b", Some("Re: Loop"), 200, &["a@x.io"]);
        b.message_id = Some("b@x.io".into());
        let threads = group(vec![a, b]);
        assert_eq!(threads.len(), 1, "a cycle is one component: {threads:?}");
    }

    #[test]
    fn threads_are_ordered_newest_first_and_keys_are_deterministic() {
        let emails = vec![
            chained("old", Some("Old thread"), 100, &[]),
            chained("new", Some("New thread"), 200, &[]),
        ];
        let first = group(emails.clone());
        let second = group(emails);
        assert_eq!(keys(&first), keys(&second), "grouping is deterministic");
        assert_eq!(first[0].subject.as_deref(), Some("New thread"));
        assert_eq!(first[1].subject.as_deref(), Some("Old thread"));
        // Equal timestamps break the tie on the key, so the order is stable.
        let tied = group(vec![
            mail("t1", Some("Tied"), 100),
            mail("t2", Some("Tied"), 100),
        ]);
        assert_eq!(tied.len(), 1, "equal timestamps still group");
        assert_eq!(tied[0].count, 2);
    }

    #[test]
    fn keys_are_url_safe_and_distinct() {
        let key = thread_key("id:a@x.io");
        assert_eq!(key.len(), 16);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(key, thread_key("id:a@x.io"), "stable");
        assert_ne!(key, thread_key("id:b@x.io"), "distinct per thread");
    }

    #[test]
    fn the_thread_key_survives_a_reply_with_a_smaller_message_id() {
        // The issue's exact scenario: the reply's id sorts before the root's,
        // so the lexicographic representative would flip — but the advertised
        // key is anchored on the conversation's own first mail, and must not
        // move. (Red on the representative scheme, which re-keyed here.)
        let mut root = chained("root", Some("Stable"), 100, &[]);
        root.message_id = Some("z@x.io".into());
        let threads = group(vec![root.clone()]);
        assert_eq!(threads.len(), 1, "{threads:?}");
        let advertised = threads[0].key.clone();

        let mut reply = chained("reply", Some("Re: Stable"), 200, &[]);
        reply.message_id = Some("a@x.io".into());
        reply.in_reply_to = Some("z@x.io".into());
        let grown = group(vec![root.clone(), reply.clone()]);
        assert_eq!(grown.len(), 1, "{grown:?}");
        assert_eq!(
            grown[0].key, advertised,
            "a reply with a smaller message-id must not move the thread URL"
        );
        assert_eq!(grown[0].count, 2);

        // And it stays put under further growth: the anchor is the oldest
        // mail, which appends can never replace.
        let mut third = chained("third", Some("Re: Stable"), 300, &[]);
        third.message_id = Some("0@x.io".into());
        third.in_reply_to = Some("a@x.io".into());
        let grown = group(vec![root, reply, third]);
        assert_eq!(grown[0].key, advertised, "the key is append-stable");
        assert_eq!(grown[0].count, 3);
    }

    #[test]
    fn the_view_caps_at_the_newest_view_cap_emails() {
        let store = Store::new(0);
        for i in 0..(VIEW_CAP as i64 + 25) {
            let email = mail(&format!("id-{i}"), Some("s"), i);
            let _ = store.insert(email);
        }
        let threads = view(&store, "box");
        let covered: usize = threads.iter().map(|t| t.count).sum();
        assert_eq!(covered, VIEW_CAP, "the view covers the newest only");
        assert!(
            !threads
                .iter()
                .any(|t| t.emails.iter().any(|e| e.id == "id-0")),
            "the oldest mail is outside the view window"
        );
    }

    #[test]
    fn the_view_of_an_unknown_inbox_is_empty() {
        let store = Store::new(0);
        assert!(view(&store, "ghost").is_empty());
    }
}
