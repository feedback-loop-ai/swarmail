# 0075 — Closed eval environments and safe diagnostics

- **Status:** proposed
- **Date:** 2026-10-24
- **Issue:** Epic #192 / #196, integrated onto #194
- **Number:** renumbered from 0062; 0062–0074 are reserved/taken and are not reused.
- **Specification:** [eval privacy port](../../specs/eval-privacy-port/spec.md).
- **Supplements:** accepted [0055](0055-agent-eval-harness.md) §§6–10 compatibly.
- **Preserves:** 0001–0005, 0020, 0050, 0056, accepted [0058](0058-codex-review-routing.md), and #194's implemented lifecycle described by proposed [0073](0073-matrix-cancellation.md).

## Context

Issue #196 closed caller-environment, raw-output, admin-detail and artifact
exposure paths in an earlier runner. #194 subsequently changed the same owners
to make matrix cancellation wait for owned cleanup and account for every slot.
Reapplying old source wholesale would discard that behavior. The verified
packet supplies privacy behavior and tests; this proposal carries them onto
the present owners. Historical gate passes are not current-candidate proof.

0055 requires eight agent inputs, sanitized settings, real execution, automatic
offline/locked preparation, draining and honest unconfined results. It does not
require ambient credentials or retained transcripts. This compatible contract
strengthening can be implemented while proposed under house rules/0045 §8;
an actual contradiction with an accepted ruling waits for the operator.
This document accepts neither itself nor 0073.

## Proposed ruling

1. Every child starts with env_clear. Common runtime map is fixed system PATH,
   C LANG/LC_ALL and fresh private HOME/TMPDIR/four XDG directories under exclusive
   0700 /tmp ownership. Server adds fixed logging; fixture alone adds its fresh
   admin secret; agents add the existing eight inputs and generated PWD equal
   to actual workdir; matrix workers add resolved SWARMAIL_BIN. No inherited
   caller HOME/PWD, credentials, prefixes, proxy/loader/startup or test controls.
2. Resolve programs at original cwd/PATH before clearing or changing cwd.
   Preserve native OS executable paths and literal later arguments through real
   CLI/worker boundaries. Explicit caller-selected wrapper/private config
   replaces ambient authentication. No secret values in argv/tracked evidence
   and no forwarding interface or inheritance fallback.
3. Preserve once-per-batch offline/locked preparation and custom-target Cargo
   JSON discovery. Preparation clears too: private defaults, generated tool PATH,
   resolved RUSTC/RUSTDOC, explicit/default CARGO_HOME/RUSTUP_HOME, optional
   nonempty RUSTUP_TOOLCHAIN/CARGO_TARGET_DIR. HOME defaults are lazy. Exact
   selector rules are in the spec; no general families/tokens/flags/wrappers,
   and no preparation selectors in runtime roles or metadata.
4. Drain streams concurrently and retain no arbitrary diagnostic bytes. Publish
   finite harness-authored facts and honest numeric statuses/deadlines/counts.
   Suppress raw bodies, URLs, errors, argv/paths and panic payloads at every
   publication sink, including CLI parse/hook and Admin failures. Every eval
   reqwest client uses no_proxy. Library callers own their external hooks/output.
5. Retain first typed loopback readiness only, nonblocking and bounded, with
   8192-byte complete-line limit. Cargo records cap at 1 MiB; matrix retains
   #194's stricter 64 KiB total capture and exactly-one matching row/exit
   validation. Discard overflow through newline without suffix parsing and
   continue drainage. Safe diagnostics cap at 64 KiB; metadata.swarmail_bin is
   null. Typed protocol exceptions never authorize echoing rejected bytes.
6. Keep #194's pre-spawn signal subscription, latched admission, once-only
   forwarding, independent Cargo ownership, body-abort/join and retained group/
   wait/drain identity. Private runtime ownership survives those fences.
   Concurrent five-second TERM grace and five-second post-KILL observation,
   actual cessation/reaping, platform identity rules and failure honesty remain.
   No second supervisor or shorter parent cleanup timer.
7. Preserve schema 1, full product accounting, committed prefix, post-cleanup
   active row, fresh zero-time refused suffix, cancellation-winning commit races,
   late-signal exit, actual statuses/lifetimes, snapshot/scoring precedence and
   report formulas. Preserve both changes' full test suites and assertion strength.
8. Publish artifacts through exclusive fresh leaf (0700 before payload),
   exclusive private temporary (0600 through descriptor before payload), complete
   write and no-clobber publication. Refuse existing leaves/targets without
   altering unowned bytes/modes; clean only owned names. Existing parents remain
   unchanged. No global umask mutation, post-write-only chmod or replacing rename.
   Per-file visibility is atomic, not a durable multi-file transaction.
9. Preserve explicit labels/schema values instead of scrubbing coincidental
   environment matches. Labels/argv/config selections are intentional inputs
   and must not contain secrets. Every result remains unconfined: deliberate
   child files, hostile same-user activity, trusted executables/Cargo config and
   filesystem/network access lie outside this privacy boundary.

## Synthesis and costs

Adopt both current panel positions' #194-first ownership and output omission.
Rule for robustness's complete publication/creation/lifetime proofs over an
agent-only patch or merely closing a startup receiver. Rule for simplicity's
existing owners over frameworks, extra supervisors and new dependencies.
Preparation cannot retain a trusted-build inheritance exception. The costs are
bounded typed parsers, explicit tool discovery and additional real-process proof.

Retain the stricter 64 KiB matrix limit over the former 1 MiB privacy wording.
Generate agent PWD rather than retaining the old no-PWD assertion. These reconcile
compatible requirements; neither admits ambient inheritance or weaker cancellation.
Any newly demonstrated contradiction stops that choice for the operator.

Accepted losses are transcripts, path-rich external errors and automatic key/
configuration discovery. Wrappers/prebuilt binaries provide deliberate setup.
No raw-debug escape hatch, broker, filesystem transaction/recovery system, root
server change, new dependency or expanded confinement claim is authorized.
Robustness's storage/webhook/chaos/TLS examples constrain regression preservation,
not new production scope.

## Proof and delivery

The linked spec preserves packet AC1–AC23 and adds explicit #194/integrated
preservation criteria. Its plan names every baseline/packet test counterpart,
real whole-map probes, independent emitted canaries, actual native worker argv,
lazy-home/custom-target builds, counted overflow controls, safe Admin failures,
pre-payload modes/collisions, causal cleanup bypass and real wire scoring.

All house gates, exact coverage/selftest, pinned candidate-local engine/proof
chain, independent eval checks and actual Linux/macOS preparation/artifact proof
remain mandatory. Five accepted-0058 careful results are required: Astra
correctness, official Flash security, Sonnet 5.5 tests, Astra guarantees and
independent Fable 5.1. Missing platform/CI evidence stays explicit; historical
receipts and a structural design pass cannot stand in for execution.

The exact four-file chief commit excludes the decision index. Implementation T1
adds one proposed 0075 row before documentation/full gates; until then that
inventory obligation is open. Commit unsigned and never push. Only the operator
accepts this proposal; the engine alone advances delivery.
