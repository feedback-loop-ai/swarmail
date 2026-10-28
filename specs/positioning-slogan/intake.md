# Intake — Adopt Swarmail's positioning slogan

Run: adopt-swarmail-s-positioning-slo-2764001f
Date: 2026-10-26. Phase: intake only.
Baseline: 3534c755ea93dbf62dcef9959bac10aec2749cc6.

## Goal

Adopt the operator's exact positioning slogan across current documentation and package metadata, remove unmeasured product-wide "fastest" claims, and preserve the existing motto, measured claims and runtime behavior.

## Authority and evidence

The supplied issue-0 README and operator-rulings evidence files were read first and their SHA256 values matched SHA256SUMS: README 805ddb339cd258a2cd15ae24f5665fac0882d542a6b8edd5080f552cb65ee3a5; rulings 09bf99c1d435f29dae334103cc331278e8f92db93cf955d1ab0fc014030f2f05. The packet contains no prior-run outcomes. The operator's Positioning (2026-10-26) ruling is authority; this seat accepts no decision and requests no further ruling. No missing evidence prevents framing.

The exact full form is:

> Most mail mocks are built for a human looking at a web UI. Swarmail is built for a program waiting on mail.

The standalone short form is "Built for a program waiting on mail." The llms.txt summary starts "An SMTP mail mock built for a program waiting on mail, not a human at a web UI: …". Both package descriptions are exactly "An SMTP mail mock built for a program waiting on mail: lossless bursts, per-test inboxes, wait_for assertions, chaos injection, MCP built in."

Read the house rules, decision index, decisions 0001–0003, 0008, 0050, 0056 and current delivery-routing decisions 0057/0058/0081. Spot-checked the five named surfaces, llms_txt in src/api.rs, machine_docs_are_served_from_the_binary in tests/api.rs, tests/store_contract_docs.rs, scripts/chart-verify.sh and the pinned verify driver. Local history for these paths includes d2f7148, ca76e24 and 14dee3c; the current base is 3534c75. The candidate-local engine matches sha256 ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f.

## Scope and frozen guarantees

Primary changes belong to README.md, docs/index.html, src/static/llms.txt, Cargo.toml and charts/swarmail/Chart.yaml. Also remove current product-wide "fastest" from src/static/openapi.json, src/config.rs CLI about text, src/ui.rs page header, src/main.rs module summary and any other current description/summary/metadata found during implementation. This includes text literals under src/, without changing control flow or runtime contracts.

The positioning sits near 0001 (lossless acceptance) and 0002 (programmatic waits); 0003 is untouched. No semantic change is requested: synchronous accept == stored, subscribe-before-check and oldest-only pruning remain frozen. No decision amendment is needed. Preserve dependencies, licenses, extraction behavior, accepted decisions, historical specs and current bundle recipe routing. The operator has ruled that historical specs are records, not correction targets; restrict the superlative sweep to current product claims, not historical quotations or tests that reject the word.

## Checkable acceptance criteria

1. README's first nonblank line immediately after "# Swarmail" is the full slogan in bold, exactly as quoted above, before badges or other material. The existing introduction beginning "**Email infrastructure for tests and agents.** One Rust binary:" remains byte-for-byte unchanged and follows in the introduction; the existing motto "*The swarm never drops a letter.*" remains. A placement assertion must fail if the slogan is moved below badges, omitted, unbolded or changed.
2. The landing page hero under header#top visibly presents the exact full slogan (normalize HTML markup/whitespace for comparison). It is hero copy rather than only a metadata string. Preserve measured throughput figures and their existing measurement qualifications.
3. The llms.txt summary begins with the exact ruled prefix "An SMTP mail mock built for a program waiting on mail, not a human at a web UI:" and continues with the existing single-binary/SMTP/API description and immediate-queryability guarantee. The real HTTP /llms.txt response is pinned to this summary and the source asset, not merely checked for the project name.
4. Cargo package.description and the YAML-decoded chart description equal the exact package sentence quoted above, including final punctuation. Their tests parse or inspect the effective metadata so folded YAML and Cargo metadata cannot conceal drift. The chart description change also carries a chart version greater than baseline 0.1.2, as required by the existing chart-history rule; appVersion, image annotation and Cargo package version retain coherence.
5. No current product-wide description, summary, CLI help, UI header or metadata presents Swarmail as "fastest" (case-insensitive). In particular the six spot-checked occurrences in Cargo.toml, llms.txt, OpenAPI info.description, src/config.rs, src/ui.rs and src/main.rs are removed or reworded. Existing benchmark-backed throughput numbers/qualifications are retained unchanged; no new comparative measurement or performance claim is introduced.
6. Existing text-pinning doc-claim, served llms.txt/OpenAPI, chart and package metadata proofs are updated or extended without weakening unrelated assertions. Real-server tests retain the complete served OpenAPI equality proof and assert the changed positioning/absence of the old superlative. Exact positive assertions and stale-claim refusals must fail on wrong slogan wording, wrong placement, old descriptions or a reintroduced product-wide superlative; tests must not derive their only expected wording from the asset under test.
7. The final diff changes documentation, metadata, display/help text and their proofs only; protocol, storage, waiting, pruning and extraction behavior do not change. No dependencies, allowlist widening, coverage exclusions, gate reductions, recipe edits or historical spec rewrites are included.
8. Every local house/pinned gate below passes on committed candidate work, including exact zero-missed lines, branches and functions plus coverage selftest. All phase work is committed unsigned. No push, merge, tag, publication, issue closure or decision acceptance occurs.

## Verify commands and ownership

Run offline and install nothing. Put the candidate-local engine on PATH, check its digest, and check fixed public shims before executing the pinned equivalents below (decision 0050). These implement every command in the house inventory; public scripts/ shims themselves are checked, not used to select verify's proof code.

```bash
export CARGO_NET_OFFLINE=true
export PATH="$PWD/.forge/bin:$PATH"
printf '%s  %s\n' ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f .forge/bin/brokkr | sha256sum -c -
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
cargo test --locked --test store_contract_docs
cargo test --locked --test api machine_docs_are_served_from_the_binary
cargo metadata --locked --offline --no-deps --format-version 1
```

The final full workspace test run must include any added slogan/package/chart metadata proofs. No cargo bench is needed: this work preserves measured numbers and introduces no performance claim. Preserve/report actual gate failures; never equate framing verification with implemented slogan acceptance.

Run the fail-closed native-tls absence proof too:

```bash
out=$(cargo tree -i native-tls 2>&1); code=$?
if [ "$code" -eq 0 ]; then
  printf '%s\n' "$out"; exit 1
elif printf '%s' "$out" | rg -q 'did not match any packages'; then
  :
else
  printf '%s\n' "$out"; exit 1
fi
```

CI owns Linux/macOS jobs, Helm lint/render, event-baseline chart history and its selftest, Docker smoke, network-backed RustSec, privileged port-80 exchange and tag-only release proofs named in the house inventory. The operator owns live repository settings, sealing and release acts. No external settings read, network or namespace proof is an intake acceptance criterion. Do not read GitHub, CI, the canonical journal or sibling worktrees.

This framing is the intake deliverable. Product implementation, its design and final feature verification remain outstanding; the engine alone selects subsequent phases.

## Intake baseline verification receipt

All local house gates passed on committed framing 8150d45e123fc189283f4d054b3ffe5db4abde92 using the digest-matched candidate-local engine and offline Cargo: format, locked workspace clippy/tests, self-bundle compile, exact coverage, coverage selftest, publish guard proof, keyless seal proof and license check. Fixed-shim integrity, design selftest, delivery tamper-boundary proof and fail-closed native-tls absence also passed. The compile reported 76 policy valuations over 14 groups, zero unruled, with no policy-sweep warning.

Coverage covered all 14,677 lines, 822 branches and 1,597 functions exactly. Per-command logs and exit codes are retained in the run-local proof directory .forge/proofs/positioning-intake-2764001f/; status.json records done: true and every exit code zero. The workspace transport timed out while the foreground batch continued; the final saved status and logs, not that timeout, establish these results.

This receipt adds documentation only. These are intake baseline proofs on unchanged product files, not proof of slogan implementation. The later implementation must satisfy AC1–AC8 and rerun final candidate verification. CI/operator-owned checks remain as assigned above.
