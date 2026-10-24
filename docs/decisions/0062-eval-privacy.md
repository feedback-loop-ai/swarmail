# 0062 — Closed eval environments and safe diagnostics

- **Status:** proposed
- **Date:** 2026-10-24
- **Issue:** Epic #192 / #196
- **Specification:** [eval privacy](../../specs/eval-privacy/spec.md).
- **Supplements:** accepted [0055](0055-agent-eval-harness.md) §§6–9.
- **Preserves:** 0001–0005, 0020, 0050, 0056 and 0057.
- **Number:** next after existing 0061; earlier gaps are not reused.

## Context

Eval commands inherit arbitrary caller environment; raw output tails, build/
matrix stderr and path-bearing errors enter diagnostics/artifacts. README
promises inheritance. Closing these paths changes that public contract while
preserving real command execution, app scoring and truthful unconfined results.

This proposal is not operator acceptance. 0055 requires eight agent inputs,
sanitized settings, draining, offline/locked build and lifecycle/result honesty,
not API-key inheritance or transcripts. Compatible strengthening may proceed
while proposed under house rules/0045 §8. Any discovered contradiction waits
for specific operator acceptance before conflicting implementation.

## Proposed ruling

1. Every harness child starts with env_clear and the exact spec map: system
   PATH, C locale, private HOME/TMPDIR/four XDG directories under a 0700 /tmp
   owner. Swarmail adds fixed logging; fixture alone adds its fresh admin
   secret; agents add exactly eight inputs; matrix workers add only resolved
   SWARMAIL_BIN. No inherited prefixes or test-control exceptions.
2. Resolve executable programs against original caller cwd/PATH before clearing
   environment/changing cwd; preserve subsequent literal arguments. Matrix
   carries resolved programs to workers. Explicit wrapper/configuration access
   replaces ambient authentication/runtime setup, with no key values in argv
   or tracked material. No inheritance fallback or forwarding interface.
3. Preserve once-per-batch offline/locked build and custom-target JSON discovery.
   Clear preparation too: private runtime defaults plus resolved RUSTC/RUSTDOC,
   generated tool-directory PATH, explicit/default CARGO_HOME/RUSTUP_HOME and
   optional RUSTUP_TOOLCHAIN/CARGO_TARGET_DIR. CARGO selects the executable
   before launch. No general families, tokens, flags, proxies or wrappers.
   Named locators never flow into runtime roles or metadata. Local config,
   build scripts and executables remain trusted unconfined file inputs.
4. Drain both streams concurrently and retain zero arbitrary diagnostic bytes.
   Publish only finite role/stage/category facts and numeric statuses/deadlines.
   Suppress response/error text, paths, argv and panic payloads at every sink,
   including CLI parse errors and executable panic hook. Library callers' own
   output/hooks are outside this CLI guarantee.
5. Readiness retains only its first typed loopback announcement through bounded
   nonblocking producer storage, with 8192-byte line limit. Cargo records and
   matrix rows cap at 1 MiB. Discard overflow safely; continue drains and fail
   when required protocol data is invalid/missing. No raw queue/output capture.
   Keep bounded safe diagnostics.log and metadata.swarmail_bin as null;
   temporary artifacts obey the same policy.
6. Preserve schema 1, explicit labels, actual exits, matrix cardinality, snapshot/
   scoring precedence and lifecycle. Do not scrub structured values for
   coincidental matches. Explicit labels/config/argv must not contain secrets;
   their intentional use, local filenames and deliberate child writes are not
   covered by caller-only environment exclusion.
7. This is not confinement against hostile processes or filesystem/network
   access. Every result remains unconfined. No new dependency, provider,
   secret store, root-code change or raw-debug escape hatch is authorized.

## Panel resolution and consequences

Adopt both positions' omission, literal argv and authentication migration.
Rule for robustness over simplicity's optional trusted-build exception:
preparation also clears the environment. Rule for robustness over merely
closing the existing readiness receiver after startup: pre-ready floods also
need a bound. Cost is toolchain discovery/parser proof and reduced ambient
build compatibility.

Rule for simplicity on implementation shape: existing runner/main/test owners,
no framework, secret dictionary or new crate. Adopt robustness's exhaustive
publication audit and panic/CLI handling. Preserve explicit attribution rather
than redact arbitrary structured values. Accepted losses are transcripts,
automatic credential discovery and ambient runtime convenience; safe failure
facts and secret-free state remain usable evidence.

Robustness's storage-crash, webhook-hang, cap-burst, chaos, hostile DATA, TLS and
waiter examples constrain regression behavior. They grant no production edit,
new eval coverage or durability claim; frozen guarantees remain intact.

## Proof and delivery

Spec AC1–AC10 and paired tasks require complete actual-role observations,
independent output canaries, lookup/runtime/auth tests, real offline preparation,
bounded-buffer/lifecycle proof, separate mutation controls and full real oracle/
null/overlapping-isolation suites. All fixtures/evidence use synthetic values.

Full pinned house gates, separate eval proof, independent protected reviews and
security hold remain mandatory. CI-only evidence stays with CI. Chief's required
four-file commit excludes the index; implementation T1 adds the proposed 0062
row before root gates. No clean-root or implementation-completion claim follows
from this draft. Deterministic design validation alone rules these artifacts;
only the operator accepts this proposal.
