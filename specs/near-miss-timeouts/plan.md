# Implementation plan — near misses on a timed-out wait

Companion to [spec.md](spec.md) (contract, rulings, AC1–AC11) and
[tasks.md](tasks.md) (ordered work with its witnesses). Decision
[0111](../../docs/decisions/0111-near-miss-timeouts.md) is proposed; the
work is compatible under 0045 §8.

## Technical context

- Base: main `d892acb` (merged #152 protocol revisions, #153
  structuredContent/outputSchema, #154 focused waits and link provenance).
- Rust 2024 edition, single crate; `evals/` is its own workspace and is
  gated separately with the same locked flags (`evals/README.md`).
- Dependencies: none added. Bigram overlap, char-boundary clipping and the
  JSON object are plain `std` + `serde_json` work already in the tree.
- Wire surfaces touched: MCP only (HTTP and stdio share the adapter). REST
  `await`, webhooks, feeds, compat shims and persistence are untouched.

## Constitution check

| Guarantee / rule | How the plan keeps it |
| --- | --- |
| 0001 accept == stored | Read-only; reads committed rows through `Store::list` after the wait; no write path touched. |
| 0002 subscribe before check | Zero diff in `wait_for`, `wait_for_report`, `wait_focused`. The near-miss read runs after the wait future resolved, in the adapter. |
| 0003 oldest-only pruning | Read-only; copies `Arc<Email>` for the walk and drops them; pins nothing. |
| 0004 AUTH is inbox | The read is keyed by the wait's `inbox`; no `list_all`, `inboxes()`, `has_inbox`. |
| 0005 extraction at ingest | Only stored fields (`subject`, `from`, `to`, `cc`, `recipients`, `received_ms`, `id`) are read; nothing is re-parsed. |
| 0018 observed totals | `total` and its one-insert residual unchanged; the near-miss object is derived from one snapshot so it cannot contradict itself. |
| 0020 test placement | Branch-stable derivation tests inline in `src/mcp/near_miss.rs`; wire shapes in `tests/`; no cold path, so no harness file. |
| 0082/0094/0105 output contract | Closed variants, every emitted member required, one captured Value for text and `structuredContent`; March descriptor bytes untouched; the March result-text departure is named in 0111. |
| 0071 catalogue | No change under `evals/tests/data/`; scripted controls only. |
| Exact coverage | Every new line executes in an inline or wire test; no `coverage(off)`. |
| Dependency policy | No new crate; `deny.toml` untouched. |

## Project structure and files touched

| File | Change |
| --- | --- |
| `src/mcp/near_miss.rs` (new) | `NEAR_MISS_LIST_CAP = 10`, `NEAR_MISS_ADDRESSES = 5`, `NEAR_MISS_FIELD_BYTES = 256`; `pub(super) async fn near_misses(store: Store, inbox: String, filter: Filter, skip_matching: bool) -> Option<Value>`; `missed`, `clip`, `recipients`, `bigrams`, `nearest`, `entry`; `pub(super) fn schema(address: &Value) -> Value` for the closed object; inline tests. |
| `src/mcp/focused.rs` | `List` becomes `pub(super)` with `List::with_cap(cap)` (default stays `LIST_CAP`); `call` attaches `near_misses` on the `None` outcome; `output_schema` adds the second `timed_out` variant. |
| `src/mcp.rs` | `mod near_miss;`; the `WaitOutcome::Timeout` arm attaches the object; `OUTPUT_SCHEMAS["swarmail_wait_for_email"]` gains the third variant. Descriptors untouched. |
| `README.md`, `src/static/llms.txt` | One paragraph each: the member, its presence rule, caps, `missed`, and the absent-key meaning. |
| `tests/mcp.rs` | Wire tests: exact populated shape, `since_ms`, skip-matched, cross-inbox no-leak over SMTP AUTH, store unchanged after timeout, empty inbox unchanged bytes. |
| `tests/focused_waits.rs` | Focused timeouts carry near misses with `missed: []` on filter-matching mail and `missed: [...]` on the rest; mistyped selector case. |
| `tests/mcp_revisions/structured.rs` | Schema variants on June/November for populated and empty timeouts, negatives, March text equality with modern `structuredContent`. |
| `tests/mcp_revisions/focused.rs` | Exact-shape cases for `timed_out` over the populated `links` and `history` inboxes gain their `near_misses`. |
| `evals/tests/common/mod.rs`, `evals/tests/proof.rs`, `evals/README.md` | Three helper modes, one proof test, control count and honest-limit prose. |
| `docs/decisions/0111-near-miss-timeouts.md`, `docs/decisions/README.md` | The proposed record and its index row (this design commit). |

## Data model and contracts

No stored type changes. The adapter-side value:

```text
near_misses = {
  arrivals:            [entry],   // newest first, ≤ NEAR_MISS_LIST_CAP
  arrivals_total:      usize,     // exact candidate count
  arrivals_truncated:  bool,
  nearest_subject:     entry | null,
  last_arrival_ms:     i64        // snapshot[0].received_ms
}
entry = {
  email_id: string, received_ms: i64,
  from: string | null,            // clipped
  to: [string],                   // distinct, ≤ NEAR_MISS_ADDRESSES, clipped
  to_total: usize,
  subject: string | null,         // clipped
  missed: ["to" | "from" | "subject"]   // fixed order, supplied fields only
}
```

Schema (modern projection only): `near_misses` object closed with the five
members required; `entry` closed with the seven members required;
`missed` is `{"type":"array","items":{"type":"string","enum":["to","from","subject"]}}`;
`nearest_subject` is `{"anyOf":[entry,{"type":"null"}]}`; counts carry
`minimum: 0`; `received_ms` and `last_arrival_ms` carry the i64 range the
focused schema already uses.

## Algorithm

1. The wait resolves `Timeout` (report wait) or `None` (focused wait).
2. `near_misses(store.clone(), inbox, filter, skip_matching).await` runs on
   `Store::offload`:
   1. `snapshot = store.list(&inbox, &Filter::default())`; if empty, return
      `None`.
   2. `last_arrival_ms = snapshot[0].received_ms`.
   3. For each email newest first: skip if `since_ms` excludes it; compute
      `missed` from the per-field projections (`Filter { to: filter.to.clone(), ..Default::default() }` etc.) through `Filter::matches`; for the report wait skip when `missed` is empty; otherwise `List::with_cap(NEAR_MISS_LIST_CAP).push(|| entry(...))` and, when `filter.subject` is some and `missed == ["subject"]`, score the clipped lowercase subject's bigram set against the filter's; keep the strictly greatest score (first seen wins ties, which is the newest).
   4. Build the object; `nearest_subject` is the kept entry or `null`.
3. The adapter inserts `near_misses` into the timed-out Value before
   `text_result`, so March text and modern `structuredContent` carry it from
   one Value.

Clipping walks `char_indices` and cuts before the first index above
`NEAR_MISS_FIELD_BYTES`; it never calls `String::truncate` on a raw byte
offset. Bigram sets are `HashSet<(char, char)>` over the clipped lowercase
string; a string shorter than two chars has an empty set.

## Quickstart and verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --locked --test mcp --test focused_waits --test mcp_revisions
(cd evals && cargo fmt --all -- --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked)
brokkr compile --bundle .brokkr/recipes/self
bash scripts/coverage-gate.sh && bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
```

The eval proof needs the coordinated trial scope its existing tests use
(`common::trial_scope()`); run it where `every_task_passes_with_the_oracle`
already runs.

## Risks

| Risk | Mitigation and accepted limit |
| --- | --- |
| The decoy is presented as `nearest_subject` and read as "the match" | The entry carries `to` and `missed`; the negative control proves acting on it still loses as `wrong_message`. Selection stays the agent's. |
| A hostile `Subject:`/recipient panics or bloats the response | Char-boundary clipping tested at a multibyte 256th byte; 10 entries × 7 strings × 256 bytes ≈ 18 KiB raw, far under the 10 MiB bridge cap; no trim loop needed. |
| Two reads contradict each other under a concurrent insert | One `list` snapshot derives every member; `total` keeps its separately documented residual (0018). |
| The near-miss path turns a timeout into an error | Infallible by construction: no `?`, no I/O; a removed inbox is an empty snapshot and omits the key. |
| March byte pins break | Only non-empty-inbox timeouts change; both captured baselines' timeouts are empty-inbox; AC6 reruns the comparisons untouched. |
| Modern schema admits a malformed object | Closed objects, required members, enum `missed`; negative controls in the finite evaluator. |
| Cost on every timeout at the 100 000 cap | One O(n) walk with bounded per-item work on the blocking pool, the same order as the `count()` already taken; no index. Accepted. |
| Cap eviction during a burst empties the block | Accepted and documented: the block describes what the store still retains; exactness would need retention. |
| Threshold waits hide filter-matching mail that failed the report | Documented; `total` counts it and the report tool explains it. |
| The focused exact-shape fixtures drift silently | `tests/mcp_revisions/focused.rs` cases over populated inboxes are updated deliberately with the full object, not loosened. |
| Harness proves nothing about live agents | Stated in the spec, decision and `evals/README.md`: scripted controls prove mechanical recovery; the live rate is not measured (no paid trials). |
| Decision number collision (0093 reserved, 0111 used) | Flagged in spec and 0111, whose number note records the path through 0106; nothing here renumbers another record. |
| Coverage denominator | New production lines all have inline or wire execution; no harness file is introduced because no branch is cold. |
