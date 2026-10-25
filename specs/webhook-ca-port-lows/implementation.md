# Webhook CA port and LOW repairs — implementation evidence

2026-10-25. This records the implementer's port, not security clearance or
protected review. The current [spec](spec.md) governs the restored historical
artifacts. The known low shared-blocking-pool availability residual is tracked
by #220; accepted 0069's implementation is outside this change.

## Patch dispositions

The supplied packet's 21 SHA256SUMS entries and the exact candidate-local engine
digest were verified before implementation. All 17 patches contributed to this
port; old whole-file snapshots were not substituted for current main.

| Patch | Final disposition |
| --- | --- |
| 01 | Restored safety design history through its later supplied revisions; final decision name is 0067. |
| 02 | Ported canonical validated projection, owned API preparation, delivery offload/publication, construction harness, wire tests and documentation. Merged current GET query admission, SMTP test seams and TLS-helper return shape. |
| 03 | Restored recovery spec/plan/tasks as dated history. |
| 04 | Retained recovery provenance corrections; new receipts identify this candidate and engine. |
| 05 | Restored GET-failure decision at final name 0068 and pool-review history. |
| 06 | Ported served GET 500 proof and API/shared-pool coupling/cancellation tests; retained main's GET 400. |
| 07 | Restored bounded-execution proposal history without its runtime implementation or obsolete #195 stop. |
| 08 | Restored final 0067/0068/0069 names and links; merged the index additively, preserving current 0062/0064 and other rows. |
| 09 | Retained bounded-CA refinement history; current scope cites the later #220 ruling. |
| 10 | Restored accepted 0067/0068 and supplied historical 0069 byte-for-byte. The index citation records later acceptance for #220 while matching the preserved record's historical status header. |
| 11 | Restored dated completion intake with the current-design pointer. |
| 12 | Restored final safety spec/plan/tasks, with an explicit dated notice assigning this port's obligations here. |
| 13 | Ported full certificate/privacy, actual-handler cancellation, accounting and scheduling proofs. Preserved current query/schema/body contracts. |
| 14 | Retained private cfg(test) harness declarations, cancellation re-export and lexical cache guard scopes. |
| 15 | Retained duplicate-waiter reuse proof through the winner's existing TLS connection. |
| 16 | Retained boolean comparisons and private-body sentinel; extended both to the unsafe refusal proof. |
| 17 | Retained complete raw prior-state comparisons and extended the API failure proof with keyed injection, unrelated handlers, one-shot recovery and owner cleanup. |

The restored record Git blob IDs match patch 10's resulting IDs:

- 0067: `df7aacbc689d5741e67b6643b98b00f3d6120a43`
- 0068: `a512becdf5f5059b821b59bc634ed9323e5b4708`
- 0069: `c3c44bd0f58322be7011b45b31ea3c8350dd65d8`

No accepted body was rewritten or newly accepted. Historical specs carry a
leading notice explaining their dated receipts/stops and the current authority.
0077 remains proposed. Dependencies, license allowlist, lockfile, bundles,
routing, pins and gates are unchanged.

## Behavior and proof owners

`WebhookTarget::redacted` is the common public projection. It canonicalizes
certificate DER in order with duplicates, validates the entire reconstruction
through the delivery builder, and yields empty supplied output on any failure.
Installed raw PEM and mixed-input admission remain intact. Real GET/PUT tests
cover private-key labels, unrelated sections/prose, noncanonical wrapping,
repeated roots, four response fields and raw installed state. Direct-install
cases include malformed and relabeled private DER and later-invalid bundles.

GET retains main's query refusal before snapshot access. GET/PUT preparation
runs outside config/cache guards on blocking workers. Only the awaiting PUT
owner installs, once with no intervening await before its prepared response.
Real-handler cancellation, complete state, fixed join-failure 500s, cache/LRU
neutrality and the served 200/400/403/500 GET contract are exercised.

Delivery builds outside async workers and cache guards, rechecks for a winner,
and preserves raw-text keys and the 32-entry LRU. Source inspection confirms
losing/evicted clients drop outside guards; no destructor-observation test is
claimed. Real SMTP/TLS/metrics tests cover exact actual-build/eviction deltas,
winner connection reuse, retries/exhaustion, cancellation and shutdown. Finite
spare-capacity tests establish DATA 250 followed by immediate queryability,
including 33-root/eight-session fan-out. They do not establish a global work or
latency bound.

The LOW diagnostic repair uses `private_webhook_json`, a 64..96 base64 body
slice, fixed-message boolean assertions and a common HTTP helper that reports
method/path/status without body or parser text. Isolated child failures are
captured in memory for status, prefix, private-body-only leakage, installed-state
comparison and private/common JSON parsing. Only sanitized verdicts leave the
parent; malformed JSON still fails and empty 204/304 stays absence.

The API panic seam is keyed by a generated raw PEM. Complete unrelated GET and
PUT requests cannot consume it. Matching GET/PUT alone receive their fixed
500, the failed PUT preserves every prior raw field and counter/LRU observation,
subsequent requests succeed, and cleanup cannot erase another armed key. The
mutex is released before panicking. The README, module and llms.txt use the
supported SMTP wording and identify the #220 residual.

Main's #197 strict-query admission, #202 decoded metric identities/escaping and
#181/0070 object-only seed/chaos behavior remain in place and in the workspace
regressions. The only SMTP diff is a cfg(test) DATA-submission observer;
production SMTP, Store, await, pruning, extraction and POP3 are unchanged.

## Fresh evidence and handoff

Run-local receipts are under `.forge/results/implement-195-port/`:

- `negative-controls.json` records nine red/restored-positive controls for raw
  echo, unvalidated projection, construction placement, served GET 500,
  private refusal diagnostics, header-only sentinel, common JSON diagnostics,
  README's old claim and unkeyed API injection. Each red is the intended test
  failure and each restored run passes. Raw captured output is never retained;
  receipt fields include command, exit, output/source hashes and engine digest.
- `gates.json` binds actual final gate exits and log hashes to committed HEAD,
  tree and engine digest. It is the gate authority; this document does not
  substitute for it. Scripts verify shim integrity and fail-closed locked
  native-tls absence, as well as every house command and the design/delivery
  selftests. The exact-coverage preflight covered 14,229/14,229 lines,
  808/808 branches and 1,556/1,556 functions; the final receipt records its own
  fresh result. No denominator exclusion or floor change was introduced.

An initial full-run refusal found an unsupported index status spelling; the
index now uses the preserved historical `proposed` header status with the
later acceptance in its citation. All 25 documentation-contract tests pass.
The final gate receipt, produced after this evidence commit, owns the full
committed-candidate verdict. Earlier attempts are not final-candidate proof.

The unchanged recipe owns hermetic verification and protected five-office
review of the entire ported disclosure fix, API/cache lifecycle and LOW repairs.
No panel verdict or security clearance is supplied by the author. The known
low availability/security residual is tracked by #220, neither new nor cleared.
CI owns macOS, network RustSec, Helm/chart, Docker smoke, privileged port-80 and
tag-only release proofs. The operator owns sealing, live GitHub settings and
merge disposition. No push, merge, tag, publication or issue closure occurred.
