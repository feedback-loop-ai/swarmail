# Restore the STARTTLS constrained-peer fixture

Issue #189, epic #192. Design synthesis: 2026-10-24. Recovery baseline:
`548982ab252457b5f6e378517ae351574a372ea2`; stopped run:
`epic-192-next-189-luna-xhigh-del-8a156acb`; recovery run:
`epic-192-issue-189-recovery-on-c-e9b16dbc`.

## What and why

Restore the connection from the configured receive-buffer socket in
`smtp::tests::starttls_stalled_reply_releases_slot`. At the baseline,
`src/smtp.rs:3072–3074` creates a `TcpSocket`, requests a 1024-byte receive
buffer, and then connects an unrelated `TcpStream`. The stalled TLS peer
therefore uses default host buffering. The previous sibling fixture connected
the configured socket. A green flood test or exact coverage cannot prove that
socket configuration reaches the peer.

Change only that connection expression to `socket.connect(addr)` inside the
existing five-second timeout. Keep the later independent fresh client's
`TcpStream::connect(addr)`. The intended outcome is trustworthy evidence for
failed-session slot release, with existing production behavior preserved.

This is a test-fixture correction, not a semantic amendment. No new decision
is declared. Accepted authorities are [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md),
[0046](../../docs/decisions/0046-smtp-pop3-edges.md),
[0050](../../docs/decisions/0050-delivery-gates.md) and
[0057](../../docs/decisions/0057-codex-delivery-recipes.md).
The retained [SMTP/POP3 cleanup spec](../smtp-pop3-tidy/spec.md), including
AC1–AC8, remains normative; this recovery narrows its remaining fixture and
evidence obligations without rewriting its historical design.

## Evidence and review boundary

Both positions in `.brokkr/state/design/positions/{simplicity,robustness}.md`
and both intake records in `.brokkr/state/tasks/` were read. Source inspection
confirms their socket finding and the predecessor's intended connection owner.
All four retained receipts were inspected under
`.forge/results/286e5a51-9f0f-4f9a-952d-ad7a4398033e-`:

| Member | Recorded result | Consequence |
|---|---|---|
| correctness.json | clean | Historical production and gate evidence; it missed the socket defect. |
| security.json | residual, low; has_security_residual=true | Production described as safe; unused configured socket is a security-relevant proof defect. |
| guarantees.json | residual, low; has_security_residual=true | Independently confirms that defect; notes historical placement wording and receipt visibility limits. |
| tests.json | residual, low; has_security_residual=true | Confirms defect and reports independent behavioral mutation failures; implementation-owned evidence paths were missing. |

Every member records fixes_applied=false and spec_defect=false. No receipt
requires another production repair. The security member's prose permission
to ship does not override its typed residual or the task's reported
REVIEW-RESIDUAL-UNSAFE stop. This design grants no security clearance and
neither edits nor recreates journal rulings. Old green gates and mutation
claims remain historical evidence, not proof for the corrected candidate.

## Panel synthesis and rulings

The current positions agree on the repair and route; there is no genuine
opposing implementation proposal to average. Their different emphases are
resolved explicitly below.

| Question | Adopted ruling and rejected alternative | Cost of the ruling |
|---|---|---|
| Repair scope | Adopt simplicity's single connection-owner repair and robustness's structural dataflow requirement. Reject factories, transport traits, shared fixture frameworks, new seams and production changes. | The protocol-local fixture remains kernel-sensitive; no generalized backpressure simulator is delivered. |
| Proof burden | Adopt robustness's fresh behavioral mutation requirement and simplicity's bounded existing test. Neither structural inspection nor a green test substitutes for the other. Reject reliance on old mutation prose alone. | Implementation must collect inspectable red/restored-green receipts despite the one-expression diff. |
| Placement | Adopt both positions' inline, measured rule-1 test. Reject restoration of the old harness or moving code merely to avoid exact coverage. | A necessary new cold path would require diagnosis under 0020 and a scope return, not an automatic coverage escape. |
| Review size | Retain triage's engine classification and both panels' four independent lenses. Reject a chore downgrade justified by diff size or low severity: this is a security-relevant concurrency/resource proof under 0057 §7. | Full review and gate latency is accepted; the cheaper route would leave the prior unsafe admission facts unresolved. |
| Buffer guarantee | Require the successfully configured socket to become the TLS peer. Reject an assertion that the OS reports exactly 1024 bytes, sysctl edits and buffer polling. | Kernel rounding/autotuning remains possible; structural ownership plus behavioral mutation evidence establishes only this regression's non-vacuity. |
| Robustness failure ledger | Adopt its preservation boundaries, not new runtime work. Reject durable webhook replay, global memory budgets, altered DATA handling, listener redesign or new lifetime/rate promises. | Existing commit/reply ambiguity, retention limits and best-effort notification loss remain accepted limitations. |

The smallest justified implementation is one expression; the smallest
policy-approved review for this recorded classification remains the full
protected route. The engine alone selects advancement. Where selected,
reviewers remain Qwen Max correctness, Opus security, official DeepSeek Flash
guarantees and DashScope GLM 5.3 tests; never DeepSeek Pro or provider fallback.

## Acceptance Criteria

1. **R1 — Configured connection.** The actual stalled peer is created by the
   same `TcpSocket` on which `set_recv_buffer_size(1024)` succeeds. Its connect
   remains inside the existing five-second timeout with unchanged await/error
   handling. The subsequent fresh client retains its independent connection.
   Source/diff inspection refuses the old expression even if tests pass.
2. **R2 — Preserved bounded proof.** The targeted locked test passes with real
   generated, trusted TLS; max_sessions=1; 200 ms idle timeout; 100,000 EHLOs;
   unread open TLS half; `finished_while_parked` completion observation; fresh
   220 before dropping that half; ten-second observation/admission bounds;
   thirty-second scenario fence; and existing task abort/join before final
   result assertions. Keep setup bounds and failure ownership. No added sleep,
   retry, enlarged deadline/flood or early peer closure may manufacture success.
3. **R3 — Non-vacuity.** Reversibly restoring unbounded shutdown on the failed
   TLS-session path makes this corrected test fail behaviorally within its
   existing independent bound, identifying failure to release the slot while
   the stalled peer remains open. Compile failure, zero matched tests or
   admission only after releasing the peer is not evidence. Record candidate,
   exact mutation, command, exit, diagnostic, restoration and passing rerun
   in inspectable run-local receipts. Restore all temporary mutations.
4. **R4 — Six findings remain satisfied.** Preserve retained AC1–AC8 and all
   six #189 criteria: (21.1) measured inline STARTTLS regression; (21.2) strict
   borrowed AUTH username bytes, no PLAIN temporary Vec, owned validated
   identity; (21.3) one SMTP cap-refusal write/flush timeout owner and exact
   421; (21.4) original SMTP envelope and authenticated POP3 snapshot survive
   observed accept-error backoff; (21.5) scoped thread-affine injection guards
   restore prior counts, including the API caller and supported exit modes;
   (21.6) exactly one cfg(test) on unchanged persist_tests. Existing exact
   inbox/envelope/body/ID and refusal-no-effect assertions remain. Run the
   relevant SMTP, POP3, API, persistence and wire suites. Preserve both
   session-abort mutation proofs with inspectable receipts; reproduce them
   if the historical implementation evidence cannot be located and verified.
5. **R5 — Contract and scope.** The source delta from the recovery baseline
   is only R1 in `src/smtp.rs`; design artifacts and ignored evidence are
   separate. Preserve 0046's failed-session error return without further peer
   I/O, successful bounded cleanup, dedicated STARTTLS handshake deadline,
   verbatim validated identities and POP3 positive-write progress policy.
   Keep admission reap, retained sessions, observed health recovery, backoff
   and shutdown ownership. Preserve synchronous accept==stored,
   subscribe-before-check, oldest-only pruning and ingest extraction.
   No dependency, feature flag, coverage exclusion, gate/policy edit, accepted
   decision edit or new availability promise is authorized.
6. **R6 — Fresh gates and measured placement.** The inline body remains
   measured at zero missed lines, branches and functions. All gates in the
   authoritative [house inventory](../../docs/house-rules.md#gates--part-of-every-change)
   pass on the restored final implementation, with locked commands, pinned
   candidate-local engine identity, coverage selftest, publish/seal proofs,
   licenses, shim checks and fail-closed native-tls absence. Retain configured
   pinned auxiliary proofs. Record actual exits and report CI-only evidence
   separately; historical reports do not satisfy this requirement.
7. **R7 — Protected review.** Fresh independent review explicitly resolves
   the retained security residual on the corrected final candidate through
   the existing protected policy. No safe facts may be inferred from severity,
   production-safe prose or the absence of a security-hold verdict. Keep
   unsigned author commits, recipe ancestry and no push/merge/tag/publish.
   Do not claim issue completion while required evidence or review remains.

## Design handoff

These are future implementation obligations. This commit contains only the
three recovery design artifacts. The sequence's pinned deterministic check
owns the design verdict; this author returns drafted. Design validation or
baseline gates cannot demonstrate the still-unapplied fixture repair.
