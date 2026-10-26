//! Lazy analysis of immutable stored bytes. This is separate from ingest extraction.
//! No network, clock, persistence or Store guard enters the evaluator (0085).

use crate::{model::Email, store::Store};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Weak};
use tokio::sync::{Mutex as AsyncMutex, Semaphore};

pub const RULE_SET_ID: &str = "swarmail-demo/1";
pub const MAX_FINDINGS: usize = 256;
pub const MAX_REPORT_BYTES: usize = 65_536;
const MAX_HEADER_BYTES: usize = 65_536;
const CACHE_SLOTS: usize = 128;
const JOBS: usize = 4;

/// Declaration order is the inclusive threshold order, not lexical order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// Coordinates describe source bytes; link locators never contain a URL.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Location {
    Header {
        name: String,
        occurrence: Option<u64>,
    },
    MimePart {
        path: Vec<u64>,
    },
    Link {
        part_path: Vec<u64>,
        byte_offset: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub rule_id: String,
    pub location: Location,
    pub severity: Severity,
    pub fix_hint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub rule_set_id: String,
    pub findings: Vec<Finding>,
}

impl Report {
    pub fn qualifies(&self, threshold: Severity) -> bool {
        !self.findings.iter().any(|f| f.severity >= threshold)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisConfig {
    pub demo_enabled: bool,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self { demo_enabled: true }
    }
}

/// The complete effective input identity. #253 must add its immutable policy /
/// observation snapshot here before collecting findings from external facts.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Inputs {
    config: AnalysisConfig,
    rule_set_id: &'static str,
    #[cfg(test)]
    fixture: Vec<Finding>,
}

impl Default for Inputs {
    fn default() -> Self {
        Self {
            config: AnalysisConfig::default(),
            rule_set_id: RULE_SET_ID,
            #[cfg(test)]
            fixture: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisError {
    Unavailable,
    Failed,
}

impl AnalysisError {
    pub fn message(self) -> &'static str {
        match self {
            Self::Unavailable => "deliverability analysis unavailable",
            Self::Failed => "deliverability analysis failed",
        }
    }
}

/// All collectors share these bounds and the single canonicalizer. Contributions
/// are checked individually, before an unbounded temporary vector can accumulate.
struct Collector(Vec<Finding>);

impl Collector {
    fn push(&mut self, finding: Finding) -> Result<(), AnalysisError> {
        if self.0.len() == MAX_FINDINGS {
            return Err(AnalysisError::Failed);
        }
        self.0.push(finding);
        Ok(())
    }

    fn finish(mut self, inputs: &Inputs) -> Result<Report, AnalysisError> {
        self.0.sort();
        self.0.dedup();
        let report = Report {
            rule_set_id: inputs.rule_set_id.into(),
            findings: self.0,
        };
        // Report contains only infallibly serializable strings, enums and integers.
        if serde_json::to_vec(&report).unwrap().len() > MAX_REPORT_BYTES {
            return Err(AnalysisError::Failed);
        }
        Ok(report)
    }
}

/// Scan the entire bounded root header block even after finding the first marker
/// or when disabled. Values stay bytes: invalid UTF-8 never becomes a token.
fn demo_value(raw: &[u8]) -> Result<Option<Vec<u8>>, AnalysisError> {
    let mut first: Option<Vec<u8>> = None;
    let mut continuing = false;
    // Limit the input before searching for line endings: an oversized single
    // line must not force a scan of the whole stored body merely to refuse it.
    for chunk in raw[..raw.len().min(MAX_HEADER_BYTES)].split_inclusive(|b| *b == b'\n') {
        let line = chunk
            .strip_suffix(b"\r\n")
            .or_else(|| chunk.strip_suffix(b"\n"))
            .unwrap_or(chunk);
        if line.is_empty() {
            return Ok(first);
        }
        if matches!(line.first(), Some(b' ' | b'\t')) {
            if continuing {
                let value = first.as_mut().unwrap();
                value.push(b' ');
                value.extend_from_slice(
                    &line[line
                        .iter()
                        .position(|b| !matches!(b, b' ' | b'\t'))
                        .unwrap_or(line.len())..],
                );
            }
            continue;
        }
        continuing = false;
        if let Some(colon) = line.iter().position(|b| *b == b':') {
            let name = &line[..colon];
            let valid = !name.is_empty() && name.iter().all(|b| (33..=126).contains(b));
            if valid && name.eq_ignore_ascii_case(b"x-swarmail-demo") && first.is_none() {
                first = Some(line[colon + 1..].to_vec());
                continuing = true;
            }
        }
    }
    if raw.len() > MAX_HEADER_BYTES {
        Err(AnalysisError::Failed)
    } else {
        Ok(first)
    }
}

fn trim_value(mut value: &[u8]) -> &[u8] {
    while matches!(value.first(), Some(b' ' | b'\t')) {
        value = &value[1..];
    }
    while matches!(value.last(), Some(b' ' | b'\t')) {
        value = &value[..value.len() - 1];
    }
    value
}

fn collect(raw: &[u8], inputs: &Inputs, out: &mut Collector) -> Result<(), AnalysisError> {
    let value = demo_value(raw)?;
    if inputs.config.demo_enabled {
        let severity = match value.as_deref().map(trim_value) {
            Some(b"info") => Some(Severity::Info),
            Some(b"warning") => Some(Severity::Warning),
            Some(b"error") => Some(Severity::Error),
            _ => None,
        };
        if let Some(severity) = severity {
            out.push(Finding {
                rule_id: "demo.marker.v1".into(),
                location: Location::Header {
                    name: "x-swarmail-demo".into(),
                    occurrence: Some(0),
                },
                severity,
                fix_hint: "Remove the X-Swarmail-Demo header.".into(),
            })?;
        }
    }
    #[cfg(test)]
    for finding in &inputs.fixture {
        out.push(finding.clone())?;
    }
    Ok(())
}

fn evaluate(raw: &[u8], inputs: &Inputs) -> Result<Report, AnalysisError> {
    let mut out = Collector(Vec::new());
    collect(raw, inputs, &mut out)?;
    out.finish(inputs)
}

struct Slot {
    email: Weak<Email>,
    inputs: Inputs,
    report: Arc<AsyncMutex<Option<Arc<Report>>>>,
}

/// Short bookkeeping lock, async per-key lock and worker-owned reservations.
/// The deque's Arc is the only unpinned owner; Weak keys retain no mail bytes.
pub(crate) struct Service {
    slots: Mutex<VecDeque<Arc<Slot>>>,
    jobs: Arc<Semaphore>,
    #[cfg(test)]
    pub(crate) gate: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}

impl Default for Service {
    fn default() -> Self {
        Self {
            slots: Mutex::new(VecDeque::new()),
            jobs: Arc::new(Semaphore::new(JOBS)),
            #[cfg(test)]
            gate: Mutex::new(None),
        }
    }
}

impl Service {
    fn pin(&self, email: &Arc<Email>, inputs: &Inputs) -> Result<Arc<Slot>, AnalysisError> {
        let mut slots = self.slots.lock().unwrap();
        if let Some(slot) = slots.iter().find(|slot| {
            slot.inputs == *inputs
                && slot
                    .email
                    .upgrade()
                    .is_some_and(|old| Arc::ptr_eq(&old, email))
        }) {
            return Ok(slot.clone());
        }
        if slots.len() == CACHE_SLOTS {
            let index = slots
                .iter()
                .position(|s| Arc::strong_count(s) == 1)
                .ok_or(AnalysisError::Unavailable)?;
            slots.remove(index);
        }
        let slot = Arc::new(Slot {
            email: Arc::downgrade(email),
            inputs: inputs.clone(),
            report: Arc::new(AsyncMutex::new(None)),
        });
        slots.push_back(slot.clone());
        Ok(slot)
    }

    async fn with_inputs(
        &self,
        email: Arc<Email>,
        inputs: Inputs,
        ready_only: bool,
    ) -> Result<Arc<Report>, AnalysisError> {
        let slot = self.pin(&email, &inputs)?;
        let mut guard = if ready_only {
            slot.report
                .clone()
                .try_lock_owned()
                .map_err(|_| AnalysisError::Unavailable)?
        } else {
            slot.report.clone().lock_owned().await
        };
        if let Some(report) = &*guard {
            return Ok(report.clone());
        }
        if ready_only {
            return Err(AnalysisError::Unavailable);
        }
        let permit = self
            .jobs
            .clone()
            .try_acquire_owned()
            .map_err(|_| AnalysisError::Unavailable)?;
        #[cfg(test)]
        let gate = self.gate.lock().unwrap().clone();
        Store::offload(move || {
            // Keep every reservation in the actual worker through publication,
            // cancellation and unwind. Dropping the caller cannot refund a job.
            let _permit = permit;
            let _pin = slot;
            #[cfg(test)]
            if let Some(gate) = gate {
                gate();
            }
            let report = Arc::new(evaluate(&email.raw, &inputs)?);
            *guard = Some(report.clone());
            Ok(report)
        })
        .await
    }

    pub(crate) async fn report(
        &self,
        email: Arc<Email>,
        ready_only: bool,
    ) -> Result<Arc<Report>, AnalysisError> {
        self.with_inputs(email, Inputs::default(), ready_only).await
    }
}

#[cfg(test)]
mod concurrency_tests;

#[cfg(test)]
mod tests {
    use super::concurrency_tests::{config, get, mail, post, smtp};
    use super::*;
    use crate::store::Filter;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    fn finding(location: Location, severity: Severity) -> Finding {
        Finding {
            rule_id: "fixture.v1".into(),
            location,
            severity,
            fix_hint: "Fix fixture.".into(),
        }
    }

    #[test]
    fn report_contract_variants_and_order() {
        let locations = vec![
            Location::Header {
                name: "a".into(),
                occurrence: None,
            },
            Location::Header {
                name: "a".into(),
                occurrence: Some(0),
            },
            Location::Header {
                name: "a".into(),
                occurrence: Some(2),
            },
            Location::Header {
                name: "b".into(),
                occurrence: Some(0),
            },
            Location::MimePart { path: vec![] },
            Location::MimePart { path: vec![0, 2] },
            Location::MimePart { path: vec![0, 10] },
            Location::Link {
                part_path: vec![0],
                byte_offset: 2,
            },
            Location::Link {
                part_path: vec![0],
                byte_offset: 10,
            },
            Location::Link {
                part_path: vec![1],
                byte_offset: 0,
            },
        ];
        let mut expected = Vec::new();
        for location in locations {
            for severity in [Severity::Info, Severity::Warning, Severity::Error] {
                expected.push(finding(location.clone(), severity));
            }
        }
        let mut later_hint = expected.last().unwrap().clone();
        later_hint.fix_hint = "Z".into();
        expected.push(later_hint);
        let mut later_rule = expected[0].clone();
        later_rule.rule_id = "z.v1".into();
        expected.push(later_rule);
        let mut reversed = expected.clone();
        reversed.reverse();
        reversed.extend(expected.clone());
        let report = Collector(reversed).finish(&Inputs::default()).unwrap();
        assert_eq!(report.findings, expected);
        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(
            value["findings"][0],
            json!({"rule_id":"fixture.v1","severity":"info",
            "location":{"kind":"header","name":"a","occurrence":null},"fix_hint":"Fix fixture."})
        );
        assert_eq!(
            value["findings"][12]["location"],
            json!({"kind":"mime_part","path":[]})
        );
        assert_eq!(
            value["findings"][21]["location"],
            json!({"kind":"link","part_path":[0],"byte_offset":2})
        );
        assert_eq!(serde_json::from_value::<Report>(value).unwrap(), report);
        assert!(!report.qualifies(Severity::Error));
        let empty = Collector(Vec::new()).finish(&Inputs::default()).unwrap();
        assert_eq!(
            serde_json::to_value(&empty).unwrap(),
            json!({"rule_set_id":"swarmail-demo/1","findings":[]})
        );
        assert!(empty.qualifies(Severity::Info));
    }

    #[test]
    fn report_limits_refuse_without_truncation() {
        let input = Inputs::default();
        let f = finding(Location::MimePart { path: vec![] }, Severity::Info);
        let mut out = Collector(Vec::new());
        for _ in 0..MAX_FINDINGS {
            out.push(f.clone()).unwrap();
        }
        assert_eq!(out.push(f.clone()), Err(AnalysisError::Failed));
        assert_eq!(out.finish(&input).unwrap().findings.len(), 1);
        let mut f = f;
        let overhead = serde_json::to_vec(&Collector(vec![f.clone()]).finish(&input).unwrap())
            .unwrap()
            .len()
            - f.fix_hint.len();
        f.fix_hint = "x".repeat(MAX_REPORT_BYTES - overhead);
        let report = Collector(vec![f.clone()]).finish(&input).unwrap();
        assert_eq!(serde_json::to_vec(&report).unwrap().len(), MAX_REPORT_BYTES);
        f.fix_hint.push('x');
        assert_eq!(
            Collector(vec![f]).finish(&input),
            Err(AnalysisError::Failed)
        );
        let inputs = Inputs {
            fixture: vec![
                finding(Location::MimePart { path: vec![] }, Severity::Info);
                MAX_FINDINGS + 1
            ],
            ..input
        };
        assert_eq!(evaluate(b"", &inputs), Err(AnalysisError::Failed));
    }

    #[test]
    fn demo_marker_uses_first_raw_header() {
        for (raw, expected) in [
            (&b""[..], None),
            (b"\r\nX-Swarmail-Demo: error\r\n", None),
            (b"bad\n orphan\nX-Swarmail-Demo : error\n: error\n\n", None),
            (b"X-Swarmail-Demo: INFO\n", None),
            (b"X-Swarmail-Demo: info\r", None),
            (b"X-Swarmail-Demo:\n", None),
            (b"X-Swarmail-Demo: \xff\n", None),
            (b"X-\xff: error\n", None),
            (b"X-Swarmail-Demo: info\x0b\n", None),
            (b"X-Swarmail-Demo: info\x0c\n", None),
            (
                b"X-Swarmail-Demo: info\nX-Swarmail-Demo: error\n",
                Some(Severity::Info),
            ),
            (
                b"x-SWARMaiL-demo:\r\n \t warning \t\r\n\r\nbody",
                Some(Severity::Warning),
            ),
            (b"X-Swarmail-Demo: error", Some(Severity::Error)),
            (b"X-Swarmail-Demo: info\n malformed\n", None),
            (
                b"X-Swarmail-Demo: info\nbroken\n ignored\n",
                Some(Severity::Info),
            ),
            (
                b"X-Swarmail-Demo: info\nOther: x\n ignored\n",
                Some(Severity::Info),
            ),
            (b"X-Swarmail-Demo: info\n \t\n", Some(Severity::Info)),
            (
                b"\torphan\nX-Swarmail-Demo: \tinfo\t \n",
                Some(Severity::Info),
            ),
        ] {
            let report = evaluate(raw, &Inputs::default()).unwrap();
            assert_eq!(
                report.findings.first().map(|f| f.severity),
                expected,
                "{raw:?}"
            );
            if let Some(f) = report.findings.first() {
                assert_eq!(f.rule_id, "demo.marker.v1");
                assert_eq!(f.fix_hint, "Remove the X-Swarmail-Demo header.");
                assert_eq!(
                    f.location,
                    Location::Header {
                        name: "x-swarmail-demo".into(),
                        occurrence: Some(0)
                    }
                );
            }
        }
        assert!(
            evaluate(
                b"X-Swarmail-Demo: error\n",
                &Inputs {
                    config: AnalysisConfig {
                        demo_enabled: false
                    },
                    ..Inputs::default()
                }
            )
            .unwrap()
            .findings
            .is_empty()
        );
        let mut full = Collector(vec![
            finding(
                Location::MimePart { path: vec![] },
                Severity::Info
            );
            MAX_FINDINGS
        ]);
        assert_eq!(
            collect(b"X-Swarmail-Demo: error\n", &Inputs::default(), &mut full),
            Err(AnalysisError::Failed)
        );
    }

    #[test]
    fn demo_header_budget_is_exact() {
        let mut raw = b"X-Swarmail-Demo: warning\r\nOther: ".to_vec();
        raw.resize(MAX_HEADER_BYTES - 4, b'x');
        raw.extend_from_slice(b"\r\n\r\n");
        assert_eq!(raw.len(), MAX_HEADER_BYTES);
        assert_eq!(
            evaluate(&raw, &Inputs::default()).unwrap().findings.len(),
            1
        );
        raw.extend_from_slice(&vec![0xff; MAX_HEADER_BYTES + 1]);
        assert_eq!(
            evaluate(&raw, &Inputs::default()).unwrap().findings.len(),
            1
        );
        raw.insert(31, b'x');
        for demo_enabled in [true, false] {
            assert_eq!(
                evaluate(
                    &raw,
                    &Inputs {
                        config: AnalysisConfig { demo_enabled },
                        ..Inputs::default()
                    }
                ),
                Err(AnalysisError::Failed)
            );
        }
        assert!(evaluate(&vec![b'x'; MAX_HEADER_BYTES], &Inputs::default()).is_ok());
        assert_eq!(
            evaluate(&vec![b'x'; MAX_HEADER_BYTES + 1], &Inputs::default()),
            Err(AnalysisError::Failed)
        );
    }

    #[tokio::test]
    async fn cache_identity_covers_bytes_config_rules() {
        let service = Service::default();
        let mut email = crate::store::email("same", "box", "a@x.io");
        email.raw = b"X-Swarmail-Demo: warning\n\n".to_vec();
        let email = Arc::new(email);
        let first = service.report(email.clone(), false).await.unwrap();
        assert!(Arc::ptr_eq(
            &first,
            &service.report(email.clone(), true).await.unwrap()
        ));
        let disabled = Inputs {
            config: AnalysisConfig {
                demo_enabled: false,
            },
            ..Inputs::default()
        };
        assert!(
            service
                .with_inputs(email.clone(), disabled, false)
                .await
                .unwrap()
                .findings
                .is_empty()
        );
        let version = Inputs {
            rule_set_id: "fixture/2",
            ..Inputs::default()
        };
        assert_eq!(
            service
                .with_inputs(email.clone(), version, false)
                .await
                .unwrap()
                .rule_set_id,
            "fixture/2"
        );
        let mut independent = (*email).clone();
        independent.id = "different-arrival".into();
        independent.received_ms += 1000;
        independent.inbox = "elsewhere".into();
        let other = Service::default()
            .report(Arc::new(independent), false)
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&other).unwrap()
        );
        assert_eq!(service.slots.lock().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn cache_rejects_clone_reinsert_and_recovered_owner() {
        let store = Store::new(0);
        let mut e = crate::store::email("same", "box", "a@x.io");
        e.raw = b"X-Swarmail-Demo: error\n\n".to_vec();
        let selected = store.insert(e).unwrap();
        let dirty = store.report(selected.clone()).await.unwrap();
        store.clear("box").unwrap();
        let mut owned = Arc::try_unwrap(selected).unwrap();
        owned.raw = b"\n".to_vec();
        let clean = store.insert(owned).unwrap();
        assert!(
            store
                .report(clean.clone())
                .await
                .unwrap()
                .findings
                .is_empty()
        );
        let mut cloned = (*clean).clone();
        cloned.raw = b"X-Swarmail-Demo: info\n\n".to_vec();
        store.clear_all().unwrap();
        let replacement = store.insert(cloned).unwrap();
        assert_eq!(
            store.report(replacement).await.unwrap().findings[0].severity,
            Severity::Info
        );
        let mut recovered = clean;
        Arc::make_mut(&mut recovered).raw = b"X-Swarmail-Demo: warning\n".to_vec();
        assert_eq!(
            store.report(recovered).await.unwrap().findings[0].severity,
            Severity::Warning
        );
        assert_eq!(dirty.findings[0].severity, Severity::Error);
    }

    #[tokio::test]
    async fn cache_eviction_never_retrieves_removed_mail() {
        let service = Service::default();
        let mut pinned = Vec::new();
        let input = Inputs::default();
        for n in 0..CACHE_SLOTS {
            pinned.push(
                service
                    .pin(
                        &Arc::new(crate::store::email(&n.to_string(), "box", "a")),
                        &input,
                    )
                    .unwrap(),
            );
        }
        let mail = Arc::new(crate::store::email("next", "box", "a"));
        assert!(matches!(
            service.pin(&mail, &input),
            Err(AnalysisError::Unavailable)
        ));
        let oldest = Arc::downgrade(&pinned.remove(0));
        service.pin(&mail, &input).unwrap();
        assert!(oldest.upgrade().is_none());
        drop(pinned);
        assert_eq!(service.slots.lock().unwrap().len(), CACHE_SLOTS);
        let store = Store::new(1);
        let old = store
            .insert(crate::store::email("old", "box", "a"))
            .unwrap();
        store.report(old.clone()).await.unwrap();
        let weak = Arc::downgrade(&old);
        store
            .insert(crate::store::email("new", "box", "a"))
            .unwrap();
        assert!(store.get("old").is_none());
        store.report(old.clone()).await.unwrap();
        drop(old);
        assert!(weak.upgrade().is_none());
        store.delete("new").unwrap();
        assert!(store.get("new").is_none());
    }

    #[tokio::test]
    async fn report_provider_snapshot_is_deterministic() {
        let service = Service::default();
        let email = Arc::new(crate::store::email("seam", "box", "a"));
        let part = finding(Location::MimePart { path: vec![1, 0] }, Severity::Warning);
        let link = finding(
            Location::Link {
                part_path: vec![1, 0],
                byte_offset: 17,
            },
            Severity::Info,
        );
        let inputs = Inputs {
            rule_set_id: "fixture/1",
            fixture: vec![link.clone(), part.clone(), link],
            ..Inputs::default()
        };
        let report = service
            .with_inputs(email.clone(), inputs.clone(), false)
            .await
            .unwrap();
        assert_eq!(report.findings.len(), 2);
        assert_eq!(report.findings[0], part);
        assert!(!report.qualifies(Severity::Warning));
        assert!(report.qualifies(Severity::Error));
        assert!(Arc::ptr_eq(
            &report,
            &service
                .with_inputs(email.clone(), inputs, true)
                .await
                .unwrap()
        ));
        let changed = service
            .with_inputs(
                email,
                Inputs {
                    rule_set_id: "fixture/1",
                    ..Inputs::default()
                },
                false,
            )
            .await
            .unwrap();
        assert!(changed.findings.is_empty());
        assert_ne!(
            serde_json::to_vec(&report).unwrap(),
            serde_json::to_vec(&changed).unwrap()
        );
    }
    #[tokio::test]
    async fn report_panic_releases_owned_capacity() {
        let service = Arc::new(Service::default());
        *service.gate.lock().unwrap() = Some(Arc::new(|| panic!("fixture evaluation panic")));
        let s = service.clone();
        let email = mail("panic");
        let e = email.clone();
        assert!(
            tokio::spawn(async move { s.report(e, false).await })
                .await
                .unwrap_err()
                .is_panic()
        );
        assert_eq!(service.jobs.available_permits(), JOBS);
        *service.gate.lock().unwrap() = None;
        assert!(service.report(email, false).await.is_ok());
    }

    #[tokio::test]
    async fn report_failure_retries_without_clean_success() {
        let service = Service::default();
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        *service.gate.lock().unwrap() = Some(Arc::new(move || {
            c.fetch_add(1, Ordering::SeqCst);
        }));
        let mut e = (*mail("bad")).clone();
        e.raw = vec![b'x'; MAX_HEADER_BYTES + 1];
        let e = Arc::new(e);
        for _ in 0..2 {
            assert_eq!(
                service.report(e.clone(), false).await,
                Err(AnalysisError::Failed)
            );
        }
        assert_eq!(count.load(Ordering::SeqCst), 2);
        let slot = service.slots.lock().unwrap()[0].clone();
        assert!(slot.report.lock().await.is_none());
        assert_eq!(
            AnalysisError::Failed.message(),
            "deliverability analysis failed"
        );
        assert_eq!(
            AnalysisError::Unavailable.message(),
            "deliverability analysis unavailable"
        );
    }

    #[tokio::test]
    async fn report_failed_scan_is_not_exact_or_clean() {
        let store = Store::new(0);
        let mut bad = crate::store::email("bad", "box", "a");
        bad.raw = vec![b'x'; MAX_HEADER_BYTES + 1];
        store.insert(bad).unwrap();
        store
            .insert(crate::store::email("clean", "box", "a"))
            .unwrap();
        assert_eq!(
            store
                .qualify(
                    store.list("box", &Filter::default()),
                    Severity::Error,
                    usize::MAX,
                    false
                )
                .await,
            Err(AnalysisError::Failed)
        );
        assert_eq!(store.watcher_count(), 0, "exact assert never subscribes");
        assert!(matches!(
            store
                .wait_for_report(
                    "box",
                    &Filter::default(),
                    2,
                    Duration::from_secs(5),
                    Some(Severity::Error)
                )
                .await,
            Err(AnalysisError::Failed)
        ));
    }

    #[tokio::test]
    async fn report_is_lazy_and_ingest_fields_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("mail.db");
        let srv = crate::run_on(&config(Some(db.clone()))).await.unwrap();
        let id = smtp(&srv).await;
        post(
            &srv,
            "/api/v1/inboxes/seed/seed",
            serde_json::json!({"to":"a@x.io","text":"code 424242 https://x.io/"}),
        )
        .await;
        assert!(
            srv.store.reports.slots.lock().unwrap().is_empty(),
            "SMTP and seed must not evaluate reports"
        );
        get(&srv, "/").await;
        get(&srv, "/ui/inbox/default").await;
        assert!(
            srv.store.reports.slots.lock().unwrap().is_empty(),
            "list views must not warm analysis"
        );
        let original = srv.store.get(&id).unwrap();
        assert_eq!(
            get(&srv, &format!("/api/v1/messages/{id}/raw"))
                .await
                .1
                .as_bytes(),
            original.raw
        );
        let count = Arc::new(AtomicUsize::new(0));
        let n = count.clone();
        *srv.store.reports.gate.lock().unwrap() = Some(Arc::new(move || {
            n.fetch_add(1, Ordering::SeqCst);
        }));
        for path in [
            format!("/api/v1/messages/{id}/deliverability?unknown=x"),
            "/api/v1/inboxes/default/assert?no_findings_at_or_above=Warning".into(),
            "/api/v1/inboxes/default/await?no_findings_at_or_above=".into(),
        ] {
            assert_eq!(get(&srv, &path).await.0, 400);
        }
        assert_eq!(count.load(Ordering::SeqCst), 0);
        let first = get(&srv, &format!("/api/v1/messages/{id}/deliverability")).await;
        assert_eq!(first.0, 200);
        assert_eq!(
            get(&srv, &format!("/api/v1/messages/{id}/deliverability")).await,
            first
        );
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(original, srv.store.get(&id).unwrap());
        srv.stop().await;
        let srv = crate::run_on(&config(Some(db))).await.unwrap();
        assert!(
            srv.store.reports.slots.lock().unwrap().is_empty(),
            "restore must not evaluate"
        );
        assert_eq!(
            get(&srv, &format!("/api/v1/messages/{id}/deliverability")).await,
            first
        );
        assert_eq!(original, srv.store.get(&id).unwrap());
        srv.stop().await;
    }
}
