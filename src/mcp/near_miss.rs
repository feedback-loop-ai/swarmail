//! The near misses of a timed-out wait (decision 0106): the retained mail in
//! the waiter's own inbox that its filter did not admit, the one whose
//! subject lies nearest the requested subject, and the inbox's last arrival.
//!
//! Everything is read from one [`Store::list`] snapshot of the wait's own
//! inbox, taken after the wait resolved and derived in a single pass. Nothing
//! is retained, indexed or counted for it, no other inbox is ever named, and
//! the path cannot fail: a timeout stays a timeout.

use super::focused::List;
use crate::model::Email;
use crate::store::{Filter, Store};
use serde_json::{Map, Value, json};
use std::collections::HashSet;
use std::sync::Arc;

/// The most arrivals presented.
pub(super) const NEAR_MISS_LIST_CAP: usize = 10;
/// The most recipient addresses one entry presents.
pub(super) const NEAR_MISS_ADDRESSES: usize = 5;
/// The longest `from`, `subject` or recipient string, in UTF-8 bytes.
pub(super) const NEAR_MISS_FIELD_BYTES: usize = 256;

/// The `near_misses` member of a timed-out answer, or `None` when the inbox
/// holds no mail at all (or does not exist), so that answer keeps today's
/// bytes. `skip_matching` is the report wait's rule: mail the filter admits
/// is already counted in `matched`, so it is no near miss; the focused waits
/// skip nothing and show such mail with an empty `missed`.
pub(super) async fn near_misses(
    store: Arc<Store>,
    inbox: String,
    filter: Filter,
    skip_matching: bool,
) -> Option<Value> {
    // An empty or missing inbox has nothing to say, and answers without
    // leaving the poll it is in: a zero-deadline wait over it still finishes
    // on the dispatch's first poll. The count is the cheap one `total`
    // already takes after every wait.
    if store.count(&inbox, &Filter::default()) == 0 {
        return None;
    }
    Store::offload(move || {
        derive(
            &store.list(&inbox, &Filter::default()),
            &filter,
            skip_matching,
        )
    })
    .await
}

/// `text` cut to [`NEAR_MISS_FIELD_BYTES`] bytes on a char boundary.
fn clip(text: &str) -> String {
    text[..text.floor_char_boundary(NEAR_MISS_FIELD_BYTES)].to_string()
}

/// The addresses the `to` filter already matches against — To, Cc, then the
/// envelope's RCPT list — without case-insensitive repeats: the first
/// [`NEAR_MISS_ADDRESSES`] clipped, and the exact distinct count.
fn recipients(email: &Email) -> (Vec<String>, usize) {
    let mut seen = HashSet::new();
    let mut shown = Vec::new();
    let all = email
        .to
        .iter()
        .map(|a| &a.address)
        .chain(email.cc.iter().map(|a| &a.address))
        .chain(email.recipients.iter());
    for address in all {
        if seen.insert(address.to_lowercase()) && shown.len() < NEAR_MISS_ADDRESSES {
            shown.push(clip(address));
        }
    }
    (shown, seen.len())
}

/// The caller's filter, one field at a time. Each projection is tested
/// through the one [`Filter::matches`], so "missed" can never disagree with
/// what the wait itself admitted.
struct Fields {
    projections: [(&'static str, Option<Filter>); 3],
}

impl Fields {
    fn new(filter: &Filter) -> Self {
        let only = |build: fn(String) -> Filter, value: &Option<String>| value.clone().map(build);
        Self {
            projections: [
                (
                    "to",
                    only(
                        |to| Filter {
                            to: Some(to),
                            ..Filter::default()
                        },
                        &filter.to,
                    ),
                ),
                (
                    "from",
                    only(
                        |from| Filter {
                            from: Some(from),
                            ..Filter::default()
                        },
                        &filter.from,
                    ),
                ),
                (
                    "subject",
                    only(
                        |subject| Filter {
                            subject: Some(subject),
                            ..Filter::default()
                        },
                        &filter.subject,
                    ),
                ),
            ],
        }
    }

    /// The supplied fields `email` fails, in the fixed order to, from,
    /// subject. `since_ms` never appears and unsupplied fields never do.
    fn missed(&self, email: &Email) -> Vec<&'static str> {
        self.projections
            .iter()
            .filter(|(_, projection)| {
                projection
                    .as_ref()
                    .is_some_and(|projection| !projection.matches(email))
            })
            .map(|(name, _)| *name)
            .collect()
    }
}

/// One mail as a near miss presents it.
fn entry(email: &Email, missed: &[&str]) -> Value {
    let (to, to_total) = recipients(email);
    json!({
        "email_id": email.id,
        "received_ms": email.received_ms,
        "from": email.from.as_ref().map(|from| clip(&from.address)),
        "to": to,
        "to_total": to_total,
        "subject": email.subject.as_deref().map(clip),
        "missed": missed,
    })
}

/// The distinct adjacent character pairs of the clipped, lowercased text.
fn bigrams(text: &str) -> HashSet<(char, char)> {
    let chars: Vec<char> = clip(text).to_lowercase().chars().collect();
    chars.windows(2).map(|pair| (pair[0], pair[1])).collect()
}

/// The object, derived from one newest-first snapshot of the inbox.
fn derive(snapshot: &[Arc<Email>], filter: &Filter, skip_matching: bool) -> Option<Value> {
    let last_arrival_ms = snapshot.first()?.received_ms;
    let fields = Fields::new(filter);
    let wanted = bigrams(filter.subject.as_deref().unwrap_or(""));
    let mut arrivals = List::with_cap(NEAR_MISS_LIST_CAP);
    // The strictly best overlap so far: the first seen is the newest, so a
    // tie never displaces it, and a zero overlap never starts one.
    let mut nearest: Option<(&Email, usize)> = None;
    for email in snapshot {
        if filter
            .since_ms
            .is_some_and(|since| email.received_ms < since)
        {
            continue;
        }
        let missed = fields.missed(email);
        if skip_matching && missed.is_empty() {
            continue;
        }
        arrivals.push(|| entry(email, &missed));
        if missed == ["subject"] {
            let score = bigrams(email.subject.as_deref().unwrap_or(""))
                .intersection(&wanted)
                .count();
            if score > nearest.map_or(0, |(_, best)| best) {
                nearest = Some((email, score));
            }
        }
    }
    let nearest = nearest.map(|(email, _)| entry(email, &["subject"]));
    let mut out = Map::new();
    arrivals.into(&mut out, "arrivals");
    out.insert("nearest_subject".into(), json!(nearest));
    out.insert("last_arrival_ms".into(), json!(last_arrival_ms));
    Some(Value::Object(out))
}

/// The closed modern projection of the object (decision 0106 §4): every
/// member required, `missed` an array of the three field names,
/// `nearest_subject` an entry or null. `maxLength` is left out because the
/// finite evaluator the output contract is proved with does not state it.
pub(super) fn schema() -> Value {
    let object = super::output_object;
    let count = json!({"type":"integer","minimum":0});
    let stamp = json!({"type":"integer","minimum":i64::MIN,"maximum":i64::MAX});
    let entry = object(
        json!({
            "email_id":{"type":"string"},
            "received_ms":stamp,
            "from":{"type":["string","null"]},
            "to":{"type":"array","items":{"type":"string"},"maxItems":NEAR_MISS_ADDRESSES},
            "to_total":count,
            "subject":{"type":["string","null"]},
            "missed":{"type":"array","items":{"type":"string","enum":["to","from","subject"]}}
        }),
        json!([
            "email_id",
            "received_ms",
            "from",
            "to",
            "to_total",
            "subject",
            "missed"
        ]),
    );
    object(
        json!({
            "arrivals":{"type":"array","items":entry,"maxItems":NEAR_MISS_LIST_CAP},
            "arrivals_total":count,
            "arrivals_truncated":{"type":"boolean"},
            "nearest_subject":{"anyOf":[entry,{"type":"null"}]},
            "last_arrival_ms":stamp
        }),
        json!([
            "arrivals",
            "arrivals_total",
            "arrivals_truncated",
            "nearest_subject",
            "last_arrival_ms"
        ]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EmailAddress;

    fn address(address: &str) -> EmailAddress {
        EmailAddress {
            name: None,
            address: address.into(),
        }
    }

    /// A stored-shaped mail for `to` with a sender, a subject and a stamp.
    fn mail(id: &str, to: &str, subject: Option<&str>, received_ms: i64) -> Arc<Email> {
        let mut mail = crate::store::email(id, "box", to);
        mail.from = Some(address("app@example.test"));
        mail.subject = subject.map(Into::into);
        mail.received_ms = received_ms;
        Arc::new(mail)
    }

    fn filter(to: Option<&str>, from: Option<&str>, subject: Option<&str>) -> Filter {
        Filter {
            to: to.map(Into::into),
            from: from.map(Into::into),
            subject: subject.map(Into::into),
            since_ms: None,
        }
    }

    fn ids(list: &Value) -> Vec<&str> {
        list.as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["email_id"].as_str().unwrap())
            .collect()
    }

    /// A cut lands on a char boundary at or below 256 bytes and never
    /// panics: ASCII at the limit, a two-byte char straddling it, a four-byte
    /// one, and a string already short enough.
    #[test]
    fn clipping_stops_on_a_char_boundary() {
        assert_eq!(clip(&"a".repeat(300)), "a".repeat(256));
        assert_eq!(clip(&"a".repeat(256)), "a".repeat(256));
        assert_eq!(clip("short"), "short");
        let straddling = format!("a{}", "é".repeat(200));
        assert!(!straddling.is_char_boundary(256));
        assert_eq!(clip(&straddling), format!("a{}", "é".repeat(127)));
        assert_eq!(clip(&straddling).len(), 255);
        let emoji = "😀".repeat(100);
        assert_eq!(clip(&emoji), "😀".repeat(64));
    }

    /// To, Cc, then the envelope list; repeats (in any case) count once;
    /// five are shown and the exact distinct count rides beside them.
    #[test]
    fn recipients_are_distinct_capped_and_counted() {
        let mut email = crate::store::email("e", "box", "a@x.test");
        email.to.push(address("b@x.test"));
        email.cc = vec![address("B@X.test"), address("c@x.test")];
        email.recipients = ["a@x.test", "d@x.test", "E@x.test", "f@x.test"]
            .map(String::from)
            .to_vec();
        let (shown, total) = recipients(&email);
        assert_eq!(
            shown,
            ["a@x.test", "b@x.test", "c@x.test", "d@x.test", "E@x.test"]
        );
        assert_eq!(total, 6);
        let long = format!("{}@x.test", "l".repeat(300));
        let mut email = crate::store::email("e", "box", &long);
        email.recipients.clear();
        let (shown, total) = recipients(&email);
        assert_eq!((shown[0].len(), total), (256, 1));
        email.to.clear();
        assert_eq!(recipients(&email), (vec![], 0));
    }

    /// `missed` names only supplied fields, in the order to, from, subject,
    /// each decided by the one `Filter::matches`; `since_ms` never appears.
    #[test]
    fn missed_names_supplied_fields_in_fixed_order() {
        let email = mail("e", "bob@example.test", Some("Welcome"), 5);
        let all = Fields::new(&Filter {
            since_ms: Some(1_000),
            ..filter(Some("alice"), Some("nobody"), Some("Confirm"))
        });
        assert_eq!(all.missed(&email), ["to", "from", "subject"]);
        let subject = Fields::new(&filter(Some("BOB"), Some("APP@"), Some("confirm")));
        assert_eq!(subject.missed(&email), ["subject"]);
        let to = Fields::new(&filter(Some("alice"), None, Some("welcome")));
        assert_eq!(to.missed(&email), ["to"]);
        assert!(Fields::new(&Filter::default()).missed(&email).is_empty());
        let mut blank = (*email).clone();
        blank.from = None;
        blank.subject = None;
        assert_eq!(all.missed(&blank), ["to", "from", "subject"]);
    }

    /// The entry carries exactly seven members, clipped, with the exact
    /// recipient count beside the shown ones.
    #[test]
    fn an_entry_has_exactly_its_seven_members() {
        let long = "s".repeat(300);
        let mail = mail("id-1", "alice@example.test", Some(&long), 42);
        let entry = entry(&mail, &["subject"]);
        assert_eq!(
            entry.as_object().unwrap().keys().collect::<Vec<_>>(),
            [
                "email_id",
                "from",
                "missed",
                "received_ms",
                "subject",
                "to",
                "to_total"
            ]
        );
        assert_eq!(
            entry,
            json!({"email_id":"id-1","received_ms":42,"from":"app@example.test",
                "to":["alice@example.test"],"to_total":1,
                "subject":"s".repeat(256),"missed":["subject"]})
        );
        let mut bare = crate::store::email("bare", "box", "a@x.test");
        bare.subject = None;
        let entry = entry_value(&bare);
        assert_eq!(
            (entry["from"].clone(), entry["subject"].clone()),
            (Value::Null, Value::Null)
        );
    }

    fn entry_value(email: &Email) -> Value {
        entry(email, &[])
    }

    /// The candidates are the mail the filter did not admit, newest first,
    /// each with what it missed; mail the report wait's filter admits is
    /// skipped, while the focused waits keep it with an empty `missed`.
    #[test]
    fn candidates_skip_admitted_mail_only_for_the_report_wait() {
        let snapshot = [
            mail(
                "c-sibling",
                "alice.team@example.test",
                Some("Confirm your account"),
                30,
            ),
            mail(
                "b-match",
                "alice@example.test",
                Some("Confirm your account"),
                20,
            ),
            mail("a-other", "alice@example.test", Some("Newsletter"), 10),
        ];
        let wanted = filter(Some("alice@"), None, Some("confirm your account"));
        let report = derive(&snapshot, &wanted, true).unwrap();
        assert_eq!(ids(&report["arrivals"]), ["c-sibling", "a-other"]);
        assert_eq!(report["arrivals"][0]["missed"], json!(["to"]));
        assert_eq!(report["arrivals"][1]["missed"], json!(["subject"]));
        assert_eq!(report["arrivals_total"], 2);
        assert_eq!(report["arrivals_truncated"], false);
        let focused = derive(&snapshot, &wanted, false).unwrap();
        assert_eq!(
            ids(&focused["arrivals"]),
            ["c-sibling", "b-match", "a-other"]
        );
        assert_eq!(focused["arrivals"][1]["missed"], json!([]));
        // The report wait's every mail may be admitted (a threshold wait
        // whose report failed): the object stays, with nothing to list.
        let admitted = derive(&snapshot, &Filter::default(), true).unwrap();
        assert_eq!(admitted["arrivals"], json!([]));
        assert_eq!(admitted["arrivals_total"], 0);
        assert_eq!(admitted["last_arrival_ms"], 30);
        assert_eq!(derive(&[], &wanted, true), None);
    }

    /// `since_ms` is the caller's own window: older mail leaves `arrivals`
    /// and `arrivals_total`, while `last_arrival_ms` is the whole inbox's.
    #[test]
    fn since_ms_bounds_candidates_but_not_the_last_arrival() {
        let snapshot = [
            mail("new", "x@example.test", Some("Recent"), 200),
            mail("edge", "x@example.test", Some("Edge"), 100),
            mail("old", "x@example.test", Some("Old"), 99),
        ];
        let window = Filter {
            since_ms: Some(100),
            ..filter(None, None, Some("nothing like it"))
        };
        let out = derive(&snapshot, &window, true).unwrap();
        assert_eq!(ids(&out["arrivals"]), ["new", "edge"]);
        assert_eq!(out["arrivals_total"], 2);
        assert_eq!(out["last_arrival_ms"], 200);
        let later = Filter {
            since_ms: Some(1_000),
            ..window
        };
        let out = derive(&snapshot, &later, false).unwrap();
        assert_eq!(out["arrivals"], json!([]));
        assert_eq!(out["arrivals_total"], 0);
        assert_eq!(out["last_arrival_ms"], 200);
        assert_eq!(out["nearest_subject"], Value::Null);
    }

    /// Acceptance order decides the last arrival, not the largest stamp.
    #[test]
    fn last_arrival_follows_acceptance_order() {
        let snapshot = [
            mail("accepted-last", "x@example.test", None, 10),
            mail("accepted-first", "x@example.test", None, 9_000),
        ];
        let out = derive(&snapshot, &filter(None, None, Some("s")), true).unwrap();
        assert_eq!(out["last_arrival_ms"], 10);
    }

    /// Eleven candidates show ten, count eleven and say so; entries past the
    /// cap are not built, but the nearest subject still scans all of them.
    #[test]
    fn arrivals_cap_at_ten_with_exact_totals() {
        let mut snapshot: Vec<Arc<Email>> = (0..11)
            .map(|i| mail(&format!("m{i:02}"), "x@example.test", Some("zzzz"), 100 - i))
            .collect();
        snapshot[10] = mail("m10", "x@example.test", Some("Confirm your accoun"), 0);
        let out = derive(
            &snapshot,
            &filter(None, None, Some("confirm your account")),
            true,
        )
        .unwrap();
        assert_eq!(out["arrivals"].as_array().unwrap().len(), 10);
        assert_eq!(out["arrivals_total"], 11);
        assert_eq!(out["arrivals_truncated"], true);
        assert!(!ids(&out["arrivals"]).contains(&"m10"));
        assert_eq!(out["nearest_subject"]["email_id"], "m10");
        let ten = derive(&snapshot[..10], &filter(None, None, Some("q")), true).unwrap();
        assert_eq!(ten["arrivals_total"], 10);
        assert_eq!(ten["arrivals_truncated"], false);
    }

    /// Only a mail that missed the subject alone is ranked; the highest
    /// distinct-bigram overlap wins, the newest wins a tie, and a zero score,
    /// a one-character filter or no subject filter leaves it null.
    #[test]
    fn nearest_subject_ranks_only_subject_only_misses() {
        let wanted = filter(Some("alice"), None, Some("Confirm your account"));
        let snapshot = [
            // Closer, but it also missed `to`: never the nearest.
            mail(
                "wrong-to",
                "bob@example.test",
                Some("Confirm your acount"),
                40,
            ),
            mail("far", "alice@example.test", Some("Welcome back"), 30),
            mail(
                "near",
                "alice@example.test",
                Some("CONFIRM you account"),
                20,
            ),
            mail("tie", "alice@example.test", Some("confirm you account"), 10),
        ];
        let out = derive(&snapshot, &wanted, true).unwrap();
        assert_eq!(out["nearest_subject"]["email_id"], "near");
        assert_eq!(out["nearest_subject"]["missed"], json!(["subject"]));
        assert_eq!(ids(&out["arrivals"]), ["wrong-to", "far", "near", "tie"]);
        let out = derive(&snapshot[..2], &wanted, true).unwrap();
        assert_eq!(out["nearest_subject"]["email_id"], "far");
        let out = derive(&snapshot[..1], &wanted, true).unwrap();
        assert_eq!(out["nearest_subject"], Value::Null);
        // No overlap at all, a mail without a subject, a one-character
        // filter and no subject filter.
        let silent = [
            mail("none", "x@example.test", Some("qqqq"), 2),
            mail("blank", "x@example.test", None, 1),
        ];
        for subject in [Some("zzzz"), Some("q"), Some(""), None] {
            let out = derive(&silent, &filter(None, None, subject), false).unwrap();
            assert_eq!(out["nearest_subject"], Value::Null, "{subject:?}");
        }
    }

    /// Characters, not bytes: a CJK subject ranks by its shared pairs.
    #[test]
    fn a_cjk_subject_ranks() {
        let snapshot = [
            mail("kana", "x@example.test", Some("ありがとう"), 3),
            mail("mail", "x@example.test", Some("確認メール"), 2),
        ];
        let out = derive(&snapshot, &filter(None, None, Some("確認コード")), true).unwrap();
        assert_eq!(out["nearest_subject"]["email_id"], "mail");
        assert_eq!(out["nearest_subject"]["subject"], "確認メール");
    }

    /// Hostile text is clipped, not trusted: a long subject, sender and
    /// recipient on a multibyte boundary come out under the byte cap.
    #[test]
    fn hostile_strings_are_clipped_in_the_object() {
        let mut email = crate::store::email("e", "box", &format!("a{}", "é".repeat(200)));
        email.from = Some(address(&format!("a{}", "é".repeat(200))));
        email.subject = Some(format!("a{}", "é".repeat(200)));
        let out = derive(&[Arc::new(email)], &filter(None, None, Some("x")), true).unwrap();
        let entry = &out["arrivals"][0];
        for text in [&entry["from"], &entry["subject"], &entry["to"][0]] {
            assert_eq!(text.as_str().unwrap().len(), 255);
        }
    }

    /// Against a real store: an empty or missing inbox answers `None`
    /// without leaving its first poll, a populated one answers the object,
    /// and the read changes nothing.
    #[tokio::test]
    async fn the_read_is_one_snapshot_of_the_waiters_inbox() {
        let store = Arc::new(Store::new(0));
        let ask = |inbox: &str, skip| {
            near_misses(
                store.clone(),
                inbox.into(),
                filter(Some("alice"), None, None),
                skip,
            )
        };
        let polled =
            std::pin::pin!(ask("box", true))
                .as_mut()
                .poll(&mut std::task::Context::from_waker(
                    futures_util::task::noop_waker_ref(),
                ));
        assert_eq!(polled, std::task::Poll::Ready(None));
        assert_eq!(ask("missing", false).await, None);
        for (id, to, ms) in [
            ("one", "bob@example.test", 1),
            ("two", "alice@example.test", 2),
        ] {
            let mut sent = crate::store::email(id, "box", to);
            sent.received_ms = ms;
            store.insert(sent).unwrap();
        }
        let mut elsewhere = crate::store::email("elsewhere", "other", "bob@example.test");
        elsewhere.subject = Some("Not yours".into());
        store.insert(elsewhere).unwrap();
        let out = ask("box", true).await.unwrap();
        assert_eq!(ids(&out["arrivals"]), ["one"]);
        assert_eq!(out["last_arrival_ms"], 2);
        let out = ask("box", false).await.unwrap();
        assert_eq!(ids(&out["arrivals"]), ["two", "one"]);
        assert!(!out.to_string().contains("elsewhere"));
        assert_eq!(store.count("box", &Filter::default()), 2);
        assert_eq!(store.count("other", &Filter::default()), 1);
    }

    /// The schema is closed: five required members, seven per entry.
    #[test]
    fn the_schema_is_closed_and_requires_every_member() {
        let schema = schema();
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["required"].as_array().unwrap().len(), 5);
        let entry = &schema["properties"]["arrivals"]["items"];
        assert_eq!(entry["additionalProperties"], false);
        assert_eq!(entry["required"].as_array().unwrap().len(), 7);
        assert_eq!(entry["properties"]["to"]["maxItems"], NEAR_MISS_ADDRESSES);
        assert_eq!(
            schema["properties"]["arrivals"]["maxItems"],
            NEAR_MISS_LIST_CAP
        );
    }
}
