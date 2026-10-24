# Implementation plan — published Brokkr pin

## Technical context

One Rust crate; this change concerns JSON tooling metadata, GitHub Actions
Bash/jq/curl and the canonical Bash verifier, with executable tooling fixtures
in the existing Rust integration suite. Use installed runner tools and existing
Rust test dependencies. No application build graph or dependency changes.
Target acquisition is the published Linux x86_64 GNU artifact. Verify remains
offline, uses cached dependencies, and installs nothing.

The repository has no plan template file under `.specify/templates/`; this
plan supplies the requested technical context, constitution check, project
structure, research, data model, contracts, quickstart and risks inline. The
single-repository Swarmail charter and this task govern the design.

## Constitution check

This implements 0050's pinned executable identity and preserves 0019 env
transport, 0045's proposal boundary and 0057's current recipes and preflight.
No semantic ADR is needed. Preserve 0001–0005, exact 100/100/100 coverage and
all gate ownership. Do not turn proposed 0072 or earlier incomplete #206 specs
into exemptions. Commit unsigned; never push, publish, change a journal or
accept a decision. The design commit contains only this spec/plan/tasks trio.

## Research resolved

The checksummed packet and both panel positions establish the exact release
asset and base-plus-patches record. The local binary has the authorized hash.
Current CI's final release build supplies all four compiles, so merely adding
a download step would leave the defect reachable. The verifier already checks
the digest before gates; its wording and regression assertion need correction.
The existing `VerifySeatFixture` provides pinned resources, result parsing and
gate markers. The seven-member shim manifest covers public wrappers, while
the independent reviewed table covers canonical script/tooling bytes.
Earlier reference patches predate seat changes: re-derive edits on this
candidate rather than apply them. No additional source-composite research or
remote lookup is required.

## Project structure and touched files

| File | Implementation responsibility |
| --- | --- |
| `bundles/self/tooling.json` | Add `binary_url`, retain exact identities, replace obsolete provenance |
| `.github/workflows/ci.yml` | Validate/export metadata, download/hash/admit, compile all recipes and refuse warning |
| `bundles/self/scripts/verify-seat.sh` | Correct identity comment and named refusal wording; preserve execution boundary |
| `tests/delivery_constitution.rs` | Record assertions, actual workflow/verifier fixtures, canonical digest updates |
| `bundles/self/scripts/shims.sha256` | Regenerate and verify existing seven entries; retain byte identity if wrappers unchanged |

No additional production helper script or test file is planned. Existing
`scripts/apply-brokkr-patches.py`, recipe definitions, public wrappers and
`tests/delivery_recipes.rs` remain owners/dependencies to verify, not targets
for unrelated edits. Keep research, model and contracts in this plan rather
than creating extra design files outside the required trio.

## Data model and contracts

The `brokkr` object adds one scalar string `binary_url`. Existing `sha`,
`version`, `binary_sha256`, `binary_provenance` and ordered `patches` remain.
The exact normative values are in spec.md. Do not broaden the published pin
to a digest list; existing verifier parser compatibility need not be redesigned.
Validate fields before any GitHub output is written, including strict whole
string checks that reject trailing newlines/control characters. Missing or
malformed digest and URL diagnostics must explicitly name those fields.

A named admission step receives URL, digest and version via `env:`. Download
and compile in that same shell step so a trap can clean a fresh private
`mktemp -d` directory at exit. Use `curl --fail --location` with HTTPS-only
initial/redirect protocols, finite connection/total timeouts (10/120 seconds),
and an explicit output file. No automatic fallback or retry is needed.
Hash the completed file with checked command status before chmod/execution.
Mismatch names `binary_sha256` and expected/observed values, then exits
nonzero. An optional version check follows admission, never precedes it.

Use that explicit absolute artifact path for each existing recipe. Capture
combined stdout/stderr in a new file per invocation; display the file and
preserve compiler failure status. On success, search for the same exact warning
substring as verify; fail naming the recipe and `policy-sweep warning`.
Avoid status-masking pipelines and unchecked output capture. Removing the
source release-build step leaves the source checkout, patch helper and core
policy/runtime aggregation/candidate-binding tests as auxiliary proofs.

The verifier still resolves the launched engine and consumes tooling beside
its pinned script. Wording uses `records no binary_sha256` and `does not match
the recorded binary_sha256`; source SHA may be contextual only. Preserve the
result envelope and its semantics: process exit zero may carry JSON `fail`.
Do not download inside verify or replace pinned resources with candidate files.

## Test design

Extend the existing fixture/test owners. Read the named workflow literal
`run: |` bodies with a small deterministic extractor that checks the job,
unique step identity and indentation, and fails on absent/ambiguous structure.
Execute the extracted bodies as Bash, with fixture env/output files. Separate
structural checks assert env wiring and no expression interpolation in run
bodies. Do not implement a second parser/admission decision in Rust.

A fixture-local curl replacement copies fixture bytes or fails acquisition;
it only controls transport in an isolated PATH. The admitted executable writes
its private execution marker, validates requested recipe paths and emits
selected compile outputs/status. These are tooling control-flow tests, not
application protocol substitutes or network-acquisition receipts.

Cover missing/wrong-type/invalid digest and URL, newline-bearing metadata,
download failure, mismatched bytes (including no `--version` marker), matched
bytes and all four recipes. For compile, exercise stdout/stderr warning,
nonzero exit and clean success. Test mutated control flow by disabling the
comparison in a scratch copy and ensure the mismatch assertion detects it;
no committed bypass or production test switch. Reuse the actual verifier
fixture for JSON failure/no-gates and a matching successful control.

After edits, regenerate shim checksums from the existing seven path entries;
do not add the canonical verifier to this manifest. Update the reviewed
canonical verifier/tooling hashes in the same implementation commit and
check manifest/table consistency. Unchanged shim bytes produce no manifest
diff, which is an honest regeneration outcome.

## Quickstart and verification

Run from the candidate through workspace hands. First verify
`.forge/bin/brokkr` against
`ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`, set
`CARGO_NET_OFFLINE=true` and prepend the candidate's absolute `.forge/bin`
to PATH so indirect engine calls cannot select a host/sibling binary.

During implementation run:

```bash
cargo test --locked --test delivery_constitution
cargo test --locked --test delivery_recipes
sha256sum --check --strict bundles/self/scripts/shims.sha256
bash bundles/self/scripts/design-check.sh selftest
bash bundles/self/scripts/delivery-proof-test.sh
```

Run every command in [house-rules Gates](../../docs/house-rules.md#gates--part-of-every-change)
without weakening `--locked`, coverage or selftests. Use the candidate-local
engine explicitly for compilation. Compile `self`, `codex`, `codex-careful`
and `codex-fast`, capture combined diagnostics and reject the exact sweep
warning on every invocation. Prove native-tls absent fail-closed: `cargo tree
--locked -i native-tls` returning success means present; only a nonzero result
with `did not match any packages` proves absence; any other error refuses.
Retain logs and statuses under ignored `target/issue206-design/` for this
seat and a distinct implementation evidence directory for later work.

Commit implementation before its final verification; commit only the design
trio at this phase and keep that commit HEAD for the deterministic design
check. CI supplies real download evidence on the PR. No local workflow fixture
or design gate run should be reported as feature implementation completion.

## Risks

- Release asset outage/deletion prevents clean acquisition. Fail clearly and
  retain no automatic alternate engine. CI owns availability evidence; setup
  may use already provisioned, digest-checked bytes.
- The digest binds bytes to the reviewed record, not source provenance or
  cryptographic publisher authenticity. Preserve base and patch records and
  state that limit; reproduction/attestation is outside the operator's ruling.
- Workflow extraction can become brittle. Keep helpers local and small,
  reject missing/duplicate blocks and test env wiring separately; never fall
  back to a test-only copy of the algorithm.
- Output capture can hide compiler failures or reuse stale warnings. Preserve
  status explicitly, allocate per-invocation logs, and execute failure and
  both-stream warning fixtures.
- Legitimate canonical edits can leave stale pins or erase #171/#172 changes.
  Review narrow diffs, regenerate the seven-member manifest and canonical hash
  expectations together, then run delivery fixtures and tamper proof.
- Temporary-directory cleanup adds lifecycle code. Keep admission and all
  compiles in one shell with one EXIT trap; refuse transfer/hash failures.
  This does not promise protection from a privileged local file-replacing actor.
- Prior designs imposed unavailable reproduction/live-journal prerequisites.
  Use this issue's operator ruling and explicit ownership; record genuinely
  missing evidence without inventing receipts or broadening local acceptance.
