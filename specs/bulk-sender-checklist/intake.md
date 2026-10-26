# Intake — epic #251 issue #257

Run: `epic-251-issue-257-check-the-gma-b81e2125`; phase: intake; 2026-10-26.
Disposition: resolved framing; feature implementation and its proofs remain outstanding.

## Goal

Add a deterministic Gmail/Yahoo bulk-sender checklist to the shipped #252
deliverability report, with source-citing fixes, real-protocol paired fixtures,
and explicit not-evaluated findings wherever authentication evidence is absent.

## Authority and orientation

Both supplied issue257 packet files were verified from their directory against
SHA256SUMS: README.md `38109586c8b30405d02bb3401d56fe67a8c7a1cd8d509724e2581ca1aa63b488`
and operator-rulings.md `2bce9539c9c347732455db7192f2522db8775a85c6a2e6fc83681cc6e3fbe1c3`.
The packet has no prior outcomes. The operator's #251 ruling requires lazy,
cached analysis of stored bytes outside SMTP, future authentication through an
existing permissive crate, and no external scorer. This issue adds report rules;
it does not implement authentication verification or introduce a dependency.

Starting HEAD: `7f27fe6`. Local history includes `9f5f142` (report service) and
`a9d0681` (completed wait counts at expiry). Read house rules, decision index,
0001–0005, proposed 0085 and relevant 0045 §8/0072 proof boundaries. Spot-checked
`collect`, `evaluate`, `Collector::finish`, `Inputs`, `Service::report`,
`Store::insert`, `Store::qualify`, `Store::wait_for_report`, the existing REST
report handler, MCP/UI report consumers and `tests/deliverability.rs`.
The current report has only `rule_set_id` and typed `findings`; a finding has
`rule_id`, `location`, `severity`, `fix_hint`. No authentication observation
provider or result is present. That absence requires not-evaluated reporting;
it is not a reason to invent a passing result or block this additive work.

Candidate-local engine SHA-256 matches
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`.
Bundle routing and pinned proof scripts were inspected and remain unchanged.
No GitHub, CI service, journal, sibling worktree or network source was read.
The source URLs below are references for rule hints, not claims of a live fetch.

## Checkable acceptance criteria

1. **One report owner.** Checklist results flow through #252's existing typed
   findings, collector, cache, REST endpoint, MCP text result, UI and report
   predicates. Paired expected reports pin stable rule IDs, severity, header or
   MIME location, source-citing nonempty fix hints, canonical order/deduplication
   and a changed rule-set version. Cold and warm results agree byte-for-byte.
   There is no separate checklist endpoint or second evaluator in an adapter.
2. **One-click unsubscribe.** A fixture containing a syntactically valid HTTPS
   List-Unsubscribe target and `List-Unsubscribe-Post: List-Unsubscribe=One-Click`
   satisfies the structural rule. Missing/invalid target, mailto-only target,
   missing post header and an incorrect post value fail it. RFC 8058 is cited
   in the fix hint, together with the applicable provider guidance. The rule
   does not claim a successful unsubscribe transaction or verified DKIM header
   coverage from header syntax alone; unavailable authentication stays explicit.
3. **Message identity and date.** Separate assertions prove valid Message-ID
   and RFC 5322 Date accepted, and missing or malformed values flagged with the
   correct root-header location. Design fixes duplicate/ambiguous-header and
   obsolete-syntax handling in proposed 0089 and pins those cases in fixtures.
   Date validity is syntax/calendar validity, with no ambient-clock freshness
   verdict. Each failing rule's hint cites its published format/provider source.
4. **Plain-text alternative.** A genuine sender-supplied text/plain MIME body
   satisfies the rule; HTML-only mail fails. Paired fixtures distinguish a body
   alternative from a text/plain attachment, including an encoded/nested MIME
   example. HTML display rendering cannot manufacture a passing alternative.
   The hint cites published guidance and identifies this as checklist advice
   where a provider does not mandate the exact MIME recommendation.
5. **Authentication is evidence-dependent.** Cover the packet's DMARC presence,
   From-domain alignment and authenticated sending (DKIM or SPF passing and
   aligned) checks through #255-compatible explicit immutable results. With no
   such results, each dependent check produces a finding explicitly saying
   `not evaluated`, identifying the missing evidence and its published source;
   no passing verdict may be inferred from missing findings, From syntax,
   DKIM-Signature presence or sender-supplied Authentication-Results headers.
   Design pins the exact existing-findings encoding, severities, alignment
   semantics and threshold behavior in proposed 0089. Trusted fixture results
   must distinguish aligned pass, unaligned/authentication failure and unavailable
   evidence without an ambient DNS/network request. This candidate's production
   path uses not evaluated until genuine results can be supplied by #255.
6. **Every rule has paired wire evidence.** Extend `tests/deliverability.rs`
   using real SMTP submission followed by existing report reads. For every
   checklist rule assert the stable ID's passing and failing behavior, exact
   severity/location/hint source and absence of a false passing verdict. For
   authentication rules use a test-only immutable observation seam on a real
   server where necessary (owner-local harness if crate-private under 0020),
   plus a real-protocol absent-evidence case. Such fixtures are trusted test
   inputs, never a new public way to assert authentication results. Additional
   cases pin malformed/folded/duplicate headers, MIME parsing and bounded failure.
7. **Shared surfaces and predicates stay coherent.** SMTP-delivered fixtures
   yield identical typed report content through REST and current MCP; the UI
   renders/escapes the new rules and source hints. Assert/await behavior agrees
   with the chosen severities, including not-evaluated cases. Update demo-only
   descriptions and expected reports affected by the new rule set. Preserve
   existing sanitized unavailable/failed errors; analysis failure cannot become
   an empty successful report. No #153 protocol dependency is added.
8. **Lazy and bounded, with frozen invariants preserved.** Existing owner tests
   and real-server regressions prove zero analysis on SMTP/seed/restore, raw and
   extracted-field immutability, synchronous queryability after DATA 250, exact
   cache input/version identity, bounded worker/cache/report behavior and failure
   handling. MIME analysis must have explicit bounded behavior and fail loudly
   rather than return a partial clean report. Subscribe-before-scan and oldest
   eviction tests stay green. Any future authentication observation identity
   must participate in cache identity; no stale not-evaluated report is reusable
   as an evaluated result under different inputs.
9. **Decision and proof discipline.** New rule semantics and the future #255
   input boundary are recorded in proposed 0089 and indexed as proposed. Keep
   0085 proposed; accepted decisions and historical specs are not rewritten.
   All local gates below must pass on committed implementation, including exact
   zero missed lines, branches and functions and the coverage selftest. Do not
   lower coverage, widen license allowances or alter recipe/reviewer routing.

## Frozen guarantees and scope

Nearest: 0001 (report work never enters synchronous acceptance) and 0002
(existing report-aware waits retain subscribe-before-check). 0003 remains
covered by oldest-only pruning/cache ownership regressions. Ingest links/codes
remain under accepted 0005; this is separate lazy analysis of stored bytes under
0085 and the operator's direction, not extraction moved into queries. Compatible
work may proceed with a proposed record under accepted 0045 §8. No weakening
or accepted semantic contradiction is requested.

Published references for hints and design: [Gmail sender guidelines](https://support.google.com/mail/answer/81126),
[Yahoo sender best practices](https://senders.yahooinc.com/best-practices/),
[RFC 8058 §§3–4](https://www.rfc-editor.org/rfc/rfc8058),
[RFC 5322 §§3.3/3.6.1/3.6.4](https://www.rfc-editor.org/rfc/rfc5322),
[RFC 2046 §5.1.4](https://www.rfc-editor.org/rfc/rfc2046),
and [RFC 7489 alignment](https://www.rfc-editor.org/rfc/rfc7489).
Pin each rule's exact hint in tests; distinguish structural checks, advice and
unavailable authentication rather than claiming provider acceptance.

## Verify commands

Focused commands after implementation (new fixture names are design-owned):

```bash
cargo test --locked --test deliverability
cargo test --locked --lib deliverability::
cargo test --locked --test stdio report
cargo test --locked --test burst
cargo test --locked --test p2
cargo test --locked --test rate
```

No new performance claim or benchmark is required. All house gates plus the
existing verify driver's proofs run offline, installing nothing. Admit only the
candidate-local engine, check the seven fixed public shims, then execute the
canonical bundle scripts rather than unchecked wrappers (0050 §1):

```bash
export CARGO_NET_OFFLINE=true
export PATH="$PWD/.forge/bin:$PATH"
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum --check --strict
sha256sum --check --strict bundles/self/scripts/shims.sha256
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
.forge/bin/brokkr compile --bundle bundles/self
bash bundles/self/scripts/coverage-gate.sh
bash bundles/self/scripts/coverage-gate.sh selftest
bash bundles/self/scripts/publish-guard-test.sh
bash bundles/self/scripts/seal-branch-test.sh
cargo deny check licenses
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
out=$(cargo tree --locked -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
  printf '%s\n' "$out" >&2; exit 1
elif printf '%s' "$out" | grep -q 'did not match any packages'; then
  :
else
  printf '%s\n' "$out" >&2; exit 1
fi
```

Capture compile output and reject `lets the run go on without reading` even
on exit zero. Run exact coverage and its selftest separately, retaining their
logs and report counts. The intake baseline proves only this framing candidate;
verification must rerun after implementation. Commit unsigned under 0056.

CI owns macOS, network RustSec, chart/Helm/history selftests, Docker smoke,
privileged port-80 exchange and tag-only release-rate proofs. The operator owns
live GitHub settings, setup, sealing and publication. These are not additional
local acceptance criteria; namespace/network access is not requested. No push,
merge, tag, publication, issue closure or decision acceptance is authorized.

## Intake baseline evidence

All 14 local checks passed on framing commit
`80eefbedbecf5516e15efbc2f0e5f5895d6f3166`, tree
`4813f46abe4ac98754066bebce1b8645f6363d6a`. The
[committed receipt](../../docs/delivery/issue257-intake-gates.json) records
commands, exit statuses, engine identity and checked log hashes. Exact coverage
is **15,520/15,520 lines, 876/876 branches and 1,656/1,656 functions**; its
selftest passed. Full report evidence is under `target/coverage/run.LPZ37E`;
logs are under the ignored candidate-local intake evidence directory.

The workspace client timed out after 300 seconds; the continuing runner's
durable final receipt and all 14 log hashes were checked after completion.
Cargo ran offline and nothing was installed. The receipt commit adds only
documentation/evidence; tested production, tests, gates and bundle bytes are
unchanged. This is resolved intake, not feature completion. Implementation
and fresh verification remain required.
