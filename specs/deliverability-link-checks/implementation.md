# Deliverability link checks — implementation record

Spec: [spec.md](spec.md) · Plan: [plan.md](plan.md) · Tasks: [tasks.md](tasks.md)
· Decision: [0121](../../docs/decisions/0121-deliverability-link-checks.md)
(proposed; only the operator accepts it).

Date: 2026-11-03, revised 2026-11-04. Run
`issue-258-deliverability-link-ch-d9779f5d`, from `main` `8732445`. Commits,
unsigned, in order: `8eb55aa` (static rules), `12a5066` (reachability),
`33efa4c` (the first records), `832c432` (the revised design), then the
revision's two commits (`4d13d6c`, the static corrections; and the
reachability corrections with these records), which the review's return (decision 0102, run
`issue-258-deliverability-link-ch-4052800f`) required.

## Disposition

The static slice may be approved by name under 0045 §8. The reachability
slice merges only after the operator accepts 0121.

- **Static link rules** (`8eb55aa`, and the revision's first commit): compatible
  implementation under 0045 §8. It stands alone and leaves every gate green.
- **Opt-in reachability** (`12a5066`, and the revision's second commit): built,
  but it must wait for the operator's acceptance of 0121. A broad reading of
  0085's "policy networking" rejection and 0106 §1's "no runtime network
  access" could cover an operator-enabled probe outside the evaluator
  (decision §7). This hold is not on 0056's mechanical hold list, so the PR
  body must carry it. 0121 stays `proposed`, and `docs/decisions/README.md` is
  unchanged by this revision (`git diff 33efa4c -- docs/decisions/README.md` is
  empty).

## What the revision changed (R1-R12)

| task | change | proof |
|---|---|---|
| R1 (HIGH) | `reach::permitted` is default-deny: IPv6 only in `2000::/3` minus `2001::/23`, `2001:db8::/32`, `2002::/16`, `3fff::/20`; IPv4 outside 13 registry blocks (adds `192.88.99/24`); mapped IPv6 unwrapped, every other embedded-IPv4 form refused, never decoded. `BLOCKED_V6` and its `64:ff9b::/96` entry are gone. `is_global` is not used. | `address_policy_table` rewritten to the spec's literal rows (it fails at `33efa4c`, where `fec0::1` was permitted) |
| R2 (HIGH) | Resolver-seam rows `nat64.test` and `sixtofour.test` (listener first, then `64:ff9b:1::a00:1` / `2002:a00:1::1`) give `Refused` with one accept in total; a loopback hop redirecting to `http://[fec0::1]/` gives `Refused` after exactly one accept | `probe_resolver_seam_pins_mixes_and_fails`, `probe_revalidates_each_hop` |
| R3 (HIGH) | The guide, `network-posture.md` and `llms.txt` state default-deny, the basis date, the embedded-IPv4 rule and the `2000::/3` residual | `rg 'fec0\|2000::/3'` shows the text in all three |
| R4 | Location wording is "prefix followed only by ASCII whitespace" in the guide and `llms.txt`; the code is unchanged | `link_offsets_index_transfer_decoded_source` now also runs a body with trailing ASCII whitespace and still gets `link` offsets |
| R5 | `token()` follows spec steps 1-7 in order (leading trim, scheme strip, cut, trailing trim, `www.` strip before qualification); `judge` strips `www.` only from the target. `key()` clears the fragment for `targets` and `observed` | `host_token_table`, `anchor_mismatch_pair`, `targets_are_distinct_...`, and the wire test `link_reachability_ignores_the_fragment` (one accept, a finding at both locations) |
| R6 | `link.*` is named in the OpenAPI route and `DeliverabilityReport` descriptions, the modern MCP report description, `README.md` and `docs/guide/README.md`; "per `Service`" replaces "per process" for the probe; the resolver residual is in `network-posture.md` and the guide | `report_served_docs_match_contract` (extended), `report_descriptions_name_link_rules` (new, `src/mcp.rs`) |
| R7 | The `reach.rs` test server has an open-connection gauge and a high-water mark; `observe_runs_four_at_a_time` counts instead of timing | mutation run below |
| R8 | `link_reachability_bounds_on_the_wire` asserts both held probes are `!is_finished()` after the cold default report | that test |
| R9 | `link_report_is_deterministic_cold_warm_and_across_services` is `link_evaluate_is_deterministic` | the renamed test |
| R10 | `probe_tests_use_loopback_only` names the "refused literal, never dialled" category (`10.0.0.1`, `[fec0::1]`), allows the two new seam names, and has a negative row (a public host built at run time is refused by the matcher) | that test |

**R7 mutation run (not committed).** With the production use site changed in
`reach.rs` (`.buffer_unordered(LINKS_AT_ONCE)`), `observe_runs_four_at_a_time`
turned red both ways: at **3** it never saw a fourth accept (the real 20 s
`accepted_at_least` deadline expired), and at **5** it failed
`high_water == 4` (left 5, right 4). The file was restored and the diff holds
no mutation. (A first attempt that changed the `LINKS_AT_ONCE` constant
instead also moved the test's own arithmetic, so it proved little and is not
counted.)

## Where the code differs from the plan and spec, and why

1. **The probe does not retain every occurrence.** `Service::probed` runs two
   bounded lexical scans around the network wait: `links::targets` (distinct
   targets, failing at the 33rd) and `links::observed` (findings for the
   targets that have outcomes, failing at the 257th). Memory is bounded by 32
   targets and 256 findings, not by the link count of a 4 MiB message. The
   composer is `links::compose(report, observed_findings)`, still pure and run
   through the one canonicalizer. Each scan takes one job permit on the worker
   and releases it; the probe holds none.
2. **`Outcome.final_origin` is always a `String`**, not an `Option`: a timeout
   reports the origin of the first URL asked.
3. **AC6's four-at-a-time proof does not use five timeouts.** The spec's
   wording has the server hold every connection and expects five `timeout`
   findings with a high-water mark of 4. I built that first and measured a
   high-water mark of **5**: the probe drops a timed-out request, and its
   socket closes a moment *after* the next target has connected, so the
   server counts five open connections for an instant. That is a property of
   the client's teardown order, not of the cap. The shipped test therefore
   holds each connection until the test releases it, and the server counts the
   connection closed *before* it answers, so a freed slot is visible on the
   gauge first. Outcomes are five `ok`, accepts 5, high-water exactly 4, and
   nothing compares a timestamp. The five-timeout class is still proven by
   `probe_timeout_on_silent_server` and the wire test of silent targets. The
   guide says a dropped probe's socket can outlive its slot briefly.
4. **No `_tests.rs` harness was needed.** Every branch is reachable by a
   deterministic local fixture. The only seams are `cfg(test)`: the resolver
   and the per-link budget, and each is installed by a test.
5. **The March baseline's seed row** moved from `swarmail-html/3` to `/4` as a
   hand edit of that one token, as #259 did; its provenance records the edit
   and the new digest.
6. **Offline guard.** `probe_tests_use_loopback_only` scans the URL literals of
   `reach.rs` and `tests/link_reachability.rs`: loopback, a `{}` placeholder
   bound to a local listener, refused literals that are never dialled, and
   names answered only by the resolver seam. One test resolves `localhost`
   through the system resolver under `public` mode to prove the real lookup
   path; it needs only `/etc/hosts` and contacts nothing.
7. **`R3`'s proof grep.** `rg -n 'Everything else is permitted' specs docs src`
   still matches `spec.md` (its revision note) and `tasks.md` (R3's own
   proof line), which quote the old sentence to say it is gone. No guide,
   `llms.txt`, decision or source file contains it.

Deviations that earlier revisions of this record listed and the revised spec
now absorbs (so they are no longer deviations): the `is_exact` rule, the
absence of a label cap, the AC11 scope (production files only) and the move of
the probe wire tests to `tests/link_reachability.rs`.

## Gates

All run `--locked` on the final tree, in this worktree (`nproc` is 128; no load
was applied beyond the gates themselves, and no load proof was needed or run).

| gate | result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean |
| `cargo test --workspace --locked` | 1,332 passed, 0 failed, 10 ignored (CI-owned or manual) |
| `cargo build --locked --bin swarmail` | built |
| `evals`: fmt, clippy, `SWARMAIL_BIN=<Cargo's artifact record> cargo test --locked` | clean, every suite green (14 min) |
| `bash scripts/coverage-gate.sh` | pass, exact 100 / 100 / 100 (lines 25059, branches 1870, functions 2534) |
| `bash scripts/coverage-gate.sh selftest` | pass |
| `bash scripts/publish-guard-test.sh` | OK |
| `bash scripts/seal-branch-test.sh` | 7 cases and 10 mutants passed |
| `cargo deny check licenses` | licenses ok |
| `git diff 8732445 -- Cargo.toml Cargo.lock evals/Cargo.lock deny.toml` | empty |
| `git diff 8732445 -- src/smtp.rs src/model.rs src/store/persist.rs .github scripts .brokkr` | empty |

**Not run here: the brokkr compile loop** (`brokkr compile --bundle` over
`.brokkr/recipes/*/`, reading each output for `lets the run go on without
reading`). The pinned brokkr binary (tooling.json, sha256 `29dc3309...`) is not
installed in this seat's box (no `brokkr` on `PATH`, none under
`~/.cargo/bin`), and fetching it would be installing tooling this seat does not
hold. This revision touches nothing under `.brokkr/` except this run's own
state, nor `scripts/` or `.github/`, so the compile inputs are unchanged, but
that is an argument and not a run. CI and the verify seat run it.

**CI-owned and not claimed:** macOS tests, the RustSec audit, Docker smoke,
Helm chart, the port-80 bridge exchange, and the tag-only rate proof.

## For the integrator

- Siblings #253 and #260 also bump `RULE_SET_ID`. Take the next free
  `swarmail-html/N` at merge and move the literals together: `RULE_SET_ID`,
  `html-rules.json`, the `html.rs` snapshot fixture, `tests/` literals, the
  OpenAPI and `llms.txt` text, the guide, and the March baseline seed row with
  its provenance digest. `shortener_list_is_canonical_and_pinned` also pins
  `RULE_SET_ID`.
- Adding to `Config` conflicts mechanically with other stories that add a
  field: the literals are one line each.
