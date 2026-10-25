# Admit the published Brokkr engine

Date: 2026-10-24  
Status: draft design; implementation and verification remain outstanding  
Story: epic #192, issue #206; supplied rollup-a candidate (PR #217)

## Feature and rationale

CI and the offline verify seat must compile with the same published Brokkr
executable, identified by the reviewed binary SHA-256. A shared version string
is insufficient: different 0.12.0 engines have understood different bundle
grammars. Today CI builds the recorded patched source but never checks the
result against the executable that verify admits. Verify already hashes its
launched engine, yet describes that check as proving a build of the source SHA.
The record also lacks an acquisition URL and names an unavailable old receipt.

The operator settled acquisition on 2026-10-24: publish the pinned bytes.
Publication is recorded in the checksummed release packet and the candidate's
`.forge/bin/brokkr` matches the published digest. This design implements that
ruling. It does not require deterministic compilation or reconstruction of an
archive's historical source-composite hash.

The workflow acquires and admits the artifact. Operator setup provisions the
same artifact for offline verify, which checks the launched executable against
its dispatched pinned tooling record before running gates. Thus both consumers
admit the same bytes without giving the hermetic seat a network dependency.

## Authority and scope

The run-local intake is
`.brokkr/state/tasks/epic-192-issue-206-pin-one-repro-dcc89977.md`.
The chief verified `.forge/epic192/issue206/SHA256SUMS` from its own directory,
including `README.md`, `operator-rulings.md` and the release records, and
independently hashed `.forge/bin/brokkr`. These are supplied evidence, not a
fresh network observation. The committed identity below makes this design
usable without access to run-local positions or receipts.

This restores accepted [0050 §1](../../docs/decisions/0050-delivery-gates.md)
and preserves [0019 §1](../../docs/decisions/0019-release-tag-integrity.md),
[0045 §8](../../docs/decisions/0045-delivery-repairs.md),
[0056](../../docs/decisions/0056-signing-policy.md) and the existing routes and
candidate-local preflight of [0057](../../docs/decisions/0057-codex-delivery-recipes.md).
No new decision is declared: this is compatible enforcement of tooling identity,
not an amendment to a ruling or public runtime contract. Proposed 0072 is not
an exemption from accepted proof obligations.

Preserve synchronous accept-equals-stored, subscribe-before-check, oldest-only
pruning and ingest-time extraction (0001–0005), the exact coverage denominator,
protected review, security hold, and #171/#172's pinned proof ownership,
resource preflight and non-login execution. No runtime Rust, dependency,
license allowlist, recipe routing or journal change belongs to this feature.

## Published identity

| Field | Required value |
| --- | --- |
| `brokkr.sha` (source base) | `731189f9097308c01ae4f2b4c9caea5f41224844` |
| `brokkr.version` | `0.12.0` |
| `brokkr.binary_url` | `https://github.com/feedback-loop-ai/brokkr/releases/download/swarmail-engine-731189f-p3/brokkr-0.12.0-731189f-p3-x86_64-unknown-linux-gnu` |
| `brokkr.binary_sha256` (one scalar string) | `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f` |

Retain these ordered patches, paths relative to `bundles/self/`:

| Order | Path | SHA-256 |
| --- | --- | --- |
| 1 | `tooling-patches/0001-preserve-review-residual-facts.patch` | `5d4da2217135c5a2ca14354ae9907623cf67ff0076b078277cb77a09ad3020b5` |
| 2 | `tooling-patches/0002-preserve-review-scope-and-fixes.patch` | `361272eff344a5d5e05b832281407a99c3f1a77fc1fbc5216e19fb4ce7487173` |
| 3 | `tooling-patches/0003-bind-verified-candidate.patch` | `4098228e7649d1bda8961bb491aa29d982b462c420470e265f8b9acf0a148d96` |

The provenance note describes the published rustc 1.98.0 build of the source
base plus all three patches. The binary digest establishes byte identity
against the reviewed record; it does not independently prove source provenance,
reproducibility or publisher authenticity. Reference the published release,
not an unavailable historical receipt as the way to obtain the engine.

## Panel synthesis and rulings

Both positions are adopted on one existing tooling record, one artifact,
retained auxiliary source tests, offline verify, and behavioral refusal proof.
Their agreement on these points is substantive, not a compromise between two
engine authorities.

- Adopt simplicity's existing workflow/verifier/test owners and remove the
  release build used solely to supply CI's compile executable. Retain patched
  source policy, aggregation and candidate-binding tests for their independent
  semantic value. Cost: CI still checks out source and spends time on those
  tests, but their output cannot become a fallback engine.
- Where storage choices differ, adopt robustness's fresh private temporary
  directory with cleanup and bounded HTTPS transfer over simplicity's fixed
  runner path. Cost to the losing simpler choice: a few shell lifecycle lines
  and corresponding failure fixtures; benefit: no stale partial download or
  shared filename can be treated as the admitted file on rerun.
- Adopt robustness's executable proof of every admission/compile outcome and
  all four recipe invocations. Reconcile with simplicity by executing the
  actual named workflow bodies in `tests/delivery_constitution.rs`, without
  a YAML dependency, second admission algorithm or new generic wrapper.
  Cost: fixture extraction must fail on missing/ambiguous blocks and be kept
  aligned when workflow structure changes. Structural assertions alone lose
  because they cannot prove that mismatched bytes were never executed.
- Require a matched-digest successful verifier control as robustness requests;
  do not settle for simplicity's permitted later-gate failure as the only
  control. Reuse the existing fixture to keep this a small assertion. This
  costs one fixture execution and makes refusal-before-gates discriminating.
- Reject reproducible-build projects, mirrors/fallback engines, new receipt
  schemas, attestations, platform matrices and shared download frameworks.
  Their forgone benefit is independent provenance or extra availability; the
  operator chose publication plus byte identity and accepts visible outage
  failure. Robustness's runtime discussion is a preservation boundary, not
  new webhook, durability, SMTP lifetime or TLS feature work.

## Acceptance Criteria

1. **AC1 — obtainable, honest identity.** `bundles/self/tooling.json` contains
   exactly the base, version, URL, scalar digest and ordered patch identities
   above. Provenance names base-plus-patches and publication without claiming
   an unpatched source build or requiring the old composite receipt. A record
   regression fails for a changed URL, digest, source base or patch order.
2. **AC2 — CI admission precedes execution.** `brokkr-compile` validates record
   types and complete strings before emitting any GitHub outputs: digest is
   64 lowercase hexadecimal characters, URL is a single-line HTTPS release
   asset under `github.com/feedback-loop-ai/brokkr/releases/download/` without
   control characters. Existing source/version validation remains. Missing or
   malformed fields refuse by field name. Expression-derived values reach
   `run:` bodies only through `env:` and quoted shell expansion. Failed HTTP
   transfer, read or hash fails closed; wrong bytes report `binary_sha256`
   with expected and observed digests before chmod, `--version` or compile.
   Local executable fixtures prove these branches and no execution marker
   on mismatch; actual GitHub acquisition is CI-owned evidence.
3. **AC3 — one admitted compile path.** All four current recipes (`self`,
   `codex`, `codex-careful`, `codex-fast`) compile with the explicit path to the
   same checked artifact. A version assertion, if retained, occurs after
   hashing. Auxiliary source tests remain, but source-built and PATH engines
   cannot substitute. A matched-byte local control records exactly those
   four compiles through the admitted file; a mismatched control records none.
4. **AC4 — compile verdict parity.** For each recipe, both output streams are
   visible and inspected. A nonzero compiler exit remains failure. Exit zero
   containing the exact substring `lets the run go on without reading` fails
   with a named policy-sweep diagnostic and recipe; clean success proceeds.
   Execute clean, stdout-warning, stderr-warning and nonzero-exit fixtures
   against the actual workflow body. Fresh output per invocation must prevent
   stale diagnostics from deciding another recipe's verdict.
5. **AC5 — truthful verifier refusal.** The canonical verifier describes
   comparison of the launched binary with recorded `binary_sha256`, including
   missing-digest and mismatch messages, without claiming that hashing proves
   a source-SHA build. Its pinned record remains authoritative. Extend
   `verify_seat_refuses_missing_or_mismatched_engine_identity_before_gates`
   to parse the actual JSON result, assert failure, the named digest refusal
   and no gate marker; retain missing-digest coverage and add/retain a matched
   successful control. Removing comparison must make the mismatch test fail.
6. **AC6 — proof identity maintained.** Reconcile canonical tooling/verifier
   hashes in `tests/delivery_constitution.rs` and regenerate/check
   `bundles/self/scripts/shims.sha256` together in the implementation commit.
   The manifest still contains exactly the seven existing public shims;
   unchanged shim bytes mean unchanged manifest bytes. Canonical digest and
   public-shim assertions pass without weakening #171/#172 protections.
7. **AC7 — bounded verification claim.** Run every gate in the charter's
   [single inventory](../../docs/house-rules.md#gates--part-of-every-change),
   including exact coverage and its selftest, with candidate-local engine
   identity checked first. Run the delivery fixtures, design selftest,
   tamper-boundary proof, four warning-free recipe compiles and fail-closed
   native-tls absence proof. Record actual outcomes and distinguish design
   baseline evidence from implementation evidence. No local test stands in
   for the PR's live published-asset download or accepted 0050 dispatch proofs.

## Proof ownership and limits

Local execution proves admission/refusal control flow, record consistency and
ordinary gates. PR CI owns live artifact download/execution and the existing
CI-only chart/history/selftest, Docker, advisory, tag-rate and privileged
port-80 checks. Operator setup owns pre-provisioning, and the operator alone
owns sealing and decision acceptance. No new macOS, namespace, live settings,
network access or journal receipt is an offline acceptance criterion. Existing
obligations remain with their owners; unavailable evidence is reported as
outstanding, never fabricated or silently declared passed.
