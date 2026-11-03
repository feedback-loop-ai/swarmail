# 0122 — Deterministic spam-signal rules, and a proposed weight and total

- **Status:** proposed
- **Date:** 2026-11-03
- **Scope:** Epic #251 issue #260 (phase 3, P2), delivered from `main` 8732445 on
  the `claude-careful` recipe, run `issue-260-built-in-deterministic-9e9f4283`.
  The number was assigned by the dispatcher.
- **Specification:** [built-in deterministic spam signals](../../specs/spam-signal-rules/spec.md).
- **Authority:** the operator's #251 ruling of 2026-10-26, as quoted in accepted
  [0085](0085-deliverability-report.md): built-in deterministic spam in later
  stories, no external scorer, analysis lazy from stored bytes and never on SMTP.
  Operator direction of 2026-11-03: start the remaining deliverability stories.
- **Preserves:** 0001–0003 and ingest extraction under 0005; accepted 0085's
  report shape, ordering, bounds, cache and failure semantics; accepted
  [0089](0089-bulk-sender-checklist.md) (bulk checklist) and
  [0106](0106-html-client-compatibility.md)/[0107](0107-html-review-repair.md)
  (HTML heuristics) unchanged in meaning.
- **Amends, only if accepted:** §2 of this record amends 0085's report contract
  sentence "no … scores … enter the report" and its exact four-key finding. §1
  amends nothing and is compatible implementation under 0045 §8.

## Decision proposed

### 1. The rule set (compatible; implemented by this run)

1. A third built-in collector, `src/deliverability/spam.rs`, joins the demo
   marker, the bulk checklist and the HTML heuristics inside the existing lazy
   evaluation. It is a pure function of the stored bytes and the compile-time
   rule set: no Store, clock, network, locale, configuration knob or dependency.
2. Six rules, each emitting at most one finding per report, severity info or
   warning only: `spam.subject.all_caps.v1` (warning),
   `spam.subject.punctuation.v1` (info), `spam.subject.phrases.v1` (warning),
   `spam.from.display_name_address.v1` (warning), `spam.body.phrases.v1`
   (warning) and `spam.body.zero_width.v1` (info). Their exact matching, fixed
   phrase list, locations and fix hints are normative in the specification.
   Header rules read the first root Subject/From in wire order, RFC 2047-decoded;
   body rules read sender-supplied `text/plain` parts only and report the first
   matching part.
3. Bounds: the existing 8 MiB raw, 65,536-byte header block, 256-part and depth-32
   admission, plus a new 4,194,304-byte cap on aggregate decoded eligible text,
   summed before any match. Exceeding it fails the whole report; truncated
   scanning is refused. Matching is linear (one ASCII-mode `regex::bytes`
   alternation compiled once, one zero-width pass).
4. The single `rule_set_id` becomes `swarmail-spam/1`, recorded in every report
   and in the cache key. A rule-meaning, hint, location or order change requires a
   new id (0085 §1). A parallel story that also bumps the id must, when it
   integrates second, mint a fresh, never-used id; two rule sets never share one.

### 2. Weight and total (amends 0085; waits for the operator)

1. Every finding gains `weight` (a non-negative integer) as its **last** key; the
   report gains `total` as its **last** key. The existing key prefixes stay
   byte-identical.
2. Weight is derived from severity, the same for every rule family:
   `info = 1`, `warning = 3`, `error = 10`. Two exceptions, stated once:
   any rule id containing `.not_evaluated.` weighs 0 (missing trusted evidence
   is not a sender signal; a fresh deployment must not score every message), and
   `demo.marker.v1` weighs 0 (synthetic). This is how bulk (#257) and HTML (#259)
   findings count: by their existing severities, with no per-id table to drift
   when a sibling adds a rule.
3. Weight is assigned in `Collector::push`; `total` is the `checked_add` sum
   computed in `Collector::finish` after sort and deduplication and inside the
   `MAX_REPORT_BYTES` check, so a cache hit and a miss are byte-identical. It is
   bounded by `MAX_FINDINGS × 10 = 2,560`; overflow, unreachable by the bounds,
   fails the report rather than wrapping. The edge (REST, MCP, UI) never computes
   a score.
4. Weight and total never participate in `no_findings_at_or_above` or any other
   predicate; severity alone qualifies.
5. The flip changes report bytes and so bumps the id again (`swarmail-spam/2`).
   The closed OpenAPI and MCP output schemas gain the two required properties,
   and the guide's "proposed weight" labels become the emitted weights.

Proposed weights of the spam rules under §2: all_caps 3, punctuation 1,
subject phrases 3, display name 3, body phrases 3, zero width 1.

## Alternatives ruled and costs

- **Emit weight and total now** (the issue's literal ask). Refused: it contradicts
  accepted 0085, so under 0045 §8 it waits. Cost: #260 stays open for §2.
- **Dormant weight code** (unserialized field, cfg, feature flag). Refused: dead
  under clippy `-D warnings` and the exact coverage gate, and a contradiction
  riding in a compatible change.
- **Per-rule weights** (robustness position: 10–30 per spam rule, severity for
  foreign findings). Rejected for one severity table: no evidence calibrates finer
  numbers, and each number is a version-bump trigger. Cost: two warnings weigh the
  same. Per-rule overrides remain an additive later change.
- **`swarmail/4` family-neutral id.** Rejected for the established family-named
  sequence; identical neutral bumps on parallel branches would merge silently.
- **Scanning `text/html` for phrases.** Rejected: attributes, CSS and tag-split
  phrases make it noisy and evadable, and HTML belongs to #259; HTML-only mail
  already receives `bulk.plain_text.v1`.
- **A finding per matching part.** Rejected: 256 parts × 2 rules can exceed
  `MAX_FINDINGS` and let a sender force an analysis failure.
- **Error severity for spoof-shaped display names.** Rejected: error is the
  checklist's requirement level, and production reports already carry error.
- **A JSON catalogue like `html-rules.json`.** Rejected: a loader adds a failure
  path that can only produce a failed report; typed Rust data cannot drift.
- **Reply-To, organizational-domain, URL or shortener rules.** Rejected here:
  correct domain logic needs a Public Suffix List (a dependency or wrong
  answers); links belong to #258 and DNS to #253.
- **Truncating the body scan at the cap.** Refused by 0085: oversized analysis is
  a failure. Cost: a message with over 4 MiB of decoded `text/plain`, which gets a
  report today, gets an analysis failure.

## Disposition

The specification's AC1–AC15 are this run's scope and prove §1 with inline and
real-SMTP fixtures, a passing and a failing one per rule. AC16 (§2) is held for
the operator. Accepting §1 alone is possible; accepting §2 authorizes the
follow-up described in §2.5. This record does not change any other decision's
status.
