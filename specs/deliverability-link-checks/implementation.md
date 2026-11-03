# Deliverability link checks — implementation record

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md) · Tasks: [tasks.md](tasks.md)
· Decision: [0121](../../docs/decisions/0121-deliverability-link-checks.md)
(proposed; only the operator accepts it).

Date: 2026-11-03. Run `issue-258-deliverability-link-ch-d9779f5d`, from `main`
`8732445`. Commits, unsigned, in slice order: `8eb55aa` (static rules),
`12a5066` (reachability), then the records commit that adds this file.

## Disposition

- **Slice 1, static link rules** (`8eb55aa`): compatible implementation under
  0045 §8. It stands alone and leaves every gate green.
- **Slice 2, opt-in reachability** (`12a5066`): built, but it must wait for the
  operator's acceptance of 0121. A broad reading of 0085's "policy networking"
  rejection and 0106 §1's "no runtime network access" could cover an
  operator-enabled probe outside the evaluator (decision §7). The operator may
  approve slice 1 by name.

## Where the code differs from the plan, and why

1. **`exact` is not `part.contents() == body.as_bytes()`.** For an HTML part
   mail-parser's `contents()` returns the charset-decoded text, so that
   comparison is always true and would emit offsets for Latin-1 mail that are
   wrong. `html::payload` decodes the transfer encoding itself, through
   mail-parser's own body decoders (`MessageStream::decode_base64_mime` and
   `decode_quoted_printable_mime`, or the raw body slice for 7bit/8bit), and
   `is_exact` requires the character-decoded text to be a prefix of that
   payload with only ASCII whitespace after it. A refused decode is an empty
   payload, which is never a prefix of a non-empty body. The `with_parts`
   walk keeps the 0106 budgets and order. No new dependency.
2. **The label has no character cap.** The spec says "capped at #154's
   character cap", but ingest has none (`LinkRegions` collapses and breaks
   only; `link_metadata_scanned` stores whatever it produced, and its own test
   pins a 3000-character label). The `Anchors` consumer mirrors ingest, and
   `anchor_labels_match_ingest_anchor_text` pins parity, so no cap was invented.
   Labels are disjoint text runs, so total label work is bounded by the 4 MiB
   body.
3. **The probe does not retain every occurrence.** `Service::probed` runs two
   bounded lexical scans around the network wait: `links::targets` (distinct
   targets, failing at the 33rd) and `links::observed` (findings for the
   targets that have outcomes, failing at the 257th). Memory is bounded by 32
   targets and 256 findings, not by the link count of a 4 MiB message. The
   composer is `links::compose(report, observed_findings)`, still pure and
   run through the one canonicalizer. Each scan takes one job permit on the
   worker and releases it; the probe holds none.
4. **`Outcome.final_origin` is always a `String`**, not an `Option`: a timeout
   reports the origin of the first URL asked.
5. **The address table adds `64:ff9b::/96` (well-known NAT64)**, which can
   embed an IPv4 address. It is stricter than the spec's list and is covered
   by `address_policy_table` and the guide.
6. **Tests in a new file.** The wire tests for the probe live in
   `tests/link_reachability.rs`, not in `tests/deliverability.rs`, so the
   sibling stories that edit that file merge cleanly. The static rule wire
   test is in `tests/deliverability.rs`. `docs/guide/testing.md` lists the new
   suite (a docs test requires it). `report_served_docs_match_contract` now
   expects the second OpenAPI parameter.
7. **`Config` gained `link_reachability`**, so every `Config` literal names
   it (26 sites, one line each, including `benches/read_stall.rs` and
   `src/store/persist_tests.rs`). AC11's `src/store/persist*.rs` therefore
   carries that one test-harness line; `persist.rs` and `persist_tests.rs`
   production behaviour is untouched, and `smtp.rs`, `model.rs` and the
   dependency files carry no diff.
8. **No `_tests.rs` harness was needed.** Every branch is reachable by a
   deterministic local fixture. The test servers hold connections until the
   client closes them. The only seams are `cfg(test)`: the resolver and the
   per-link budget, and each is installed by a test. Timeout tests use a
   short budget through the seam; the wire tests pay the real 5 s.
9. **The March baseline's seed row** moves from `swarmail-html/3` to `/4` as a
   hand edit of that one token, as #259 did; its provenance records the edit
   and the new digest.
10. **Offline guard.** `probe_tests_use_loopback_only` scans the URL literals
    of `reach.rs` and `tests/link_reachability.rs`. It allows loopback, a
    `{}` placeholder bound to a local listener, `10.0.0.1` (never dialled) and
    names answered only by the resolver seam. One test resolves `localhost`
    through the system resolver under `public` mode to prove the real lookup
    path; it needs only `/etc/hosts` and contacts nothing.

## Gates

All run `--locked` on the final tree, in this worktree.

| gate | result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean |
| `cargo test --workspace --locked` | 1,330 passed, 0 failed, 10 ignored (the ignored ones are CI-owned or manual) |
| `cargo build --locked --bin swarmail` | built |
| `evals`: fmt, clippy, `SWARMAIL_BIN=<candidate> cargo test --locked` | clean, all suites green |
| `bash scripts/coverage-gate.sh` | pass, exact 100 / 100 / 100 (lines / branches / functions) |
| `bash scripts/coverage-gate.sh selftest` | pass |
| `bash scripts/publish-guard-test.sh` | OK |
| `bash scripts/seal-branch-test.sh` | 7 cases and 10 mutants passed |
| `cargo deny check licenses` | licenses ok |
| `git diff 8732445 -- Cargo.toml Cargo.lock evals/Cargo.lock deny.toml` | empty |

**Not run here: the brokkr compile loop** (`brokkr compile --bundle` over
`.brokkr/recipes/*/`). The pinned brokkr binary (tooling.json, sha256
`29dc3309…`) is not installed in this seat's box, and fetching it would be
installing tooling this seat does not hold. This change touches nothing under
`.brokkr/`, `scripts/` or `.github/` (`git diff 8732445 --stat` over those
paths is empty), so the compile inputs are unchanged, but that is an argument
and not a run. CI and the verify seat run it.

**CI-owned and not claimed:** macOS tests, the RustSec audit, Docker smoke,
Helm chart, the port-80 bridge exchange, and the tag-only rate proof.

**Load:** none applied. The tests are single-process, bounded, and use at most
8 local sockets; no load proof was needed or run.

## For the integrator

- Siblings #253 and #260 also bump `RULE_SET_ID`. Take the next free
  `swarmail-html/N` at merge and move the literals together: `RULE_SET_ID`,
  `html-rules.json`, the `html.rs` snapshot fixture, `tests/` literals, the
  OpenAPI and `llms.txt` text, the guide, and the March baseline seed row with
  its provenance digest. `shortener_list_is_canonical_and_pinned` also pins
  `RULE_SET_ID`.
- Adding to `Config` conflicts mechanically with other stories that add a
  field: the literals are one line each.
