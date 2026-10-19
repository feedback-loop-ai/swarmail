# 0045 — Delivery proofs and documentation corrections

- **Status:** accepted (operator, 2026-10-19)
  Original draft status: proposed — acceptance is the operator's.
- **Date:** 2026-10-18
- **Amends/clarifies:** [0007](0007-hermetic-gates-single-model.md), [0009](0009-agent-council-hybrid.md), [0015](0015-boundary-failure-contract.md) §1, [0018](0018-shutdown-observed-waits-and-explicit-bounds.md) header/§6, and the process description in house rules.
- **Records existing amendments:** [0030](0030-loopback-plane-bounded-results.md) to 0018 and [0023](0023-off-worker-commits.md).
- **Design:** [spec](../../specs/delivery-repairs/spec.md), [plan](../../specs/delivery-repairs/plan.md), [tasks](../../specs/delivery-repairs/tasks.md).
- **Preserves:** 0001–0005, 0013's routes, 0019's guards, 0020's placement rules and 0035's by-effect coverage mechanism. No frozen guarantee is weakened.

## Context

Issues #103–#106 expose a mismatch between promised proof and actual delivery:
verify invokes mutable worktree gates, the design sequence has no final exec
check, release jobs do not depend on rate proof, and current install commands
refer to missing or obsolete artifacts. Several accepted decision sentences
also disagree with later accepted amendments or existing tested behavior.

This record proposes explicit ownership and corrections. It is neither a
release authorization nor operator acceptance of itself. The implementation
must distinguish restoring existing obligations from changing accepted policy.

## Proposed ruling

1. **Pinned verification is transitive over its executable local proof code.**
   The canonical coverage gate, guard and their proofs, scanner source and
   manifests, and executable prerequisite support reside under the pinned
   bundle and are executed from the dispatched pinned resources. Public
   script paths may remain compatibility shims, but verify refuses altered
   shims and never lets them choose its proof implementation. The application,
   chart and workflow being assessed remain candidate inputs. This restores
   0007; it does not claim compiler/cache/configuration provenance is solved.
2. **The deterministic design check owns the design verdict.** Chief returns
   `drafted`. A final exec sequence step, or an equivalent bounded explicit
   validation phase if required by the pinned grammar, validates the one spec
   committed by chief at HEAD and the dialect prerequisites, then returns
   `designed|fail`. There is no drafted-to-implement shortcut. Neither mtime nor
   a model's assertion substitutes for artifact identity or validation. A
   missing risks section fails, even beside many previously valid designs.
3. **Unknown residual safety does not authorize shipping.** Non-security
   residual admission explicitly requires security=false and severity at most
   medium; missing/malformed facts refuse. Security hold and above-medium
   residuals remain hard stops. Suppressing the policy sweep is not a repair.
4. **Release rate proof is permission, not advisory evidence.** Both release
   publishers require successful ordinary prerequisites and successful rate
   proof from the same invocation. Main's continuous image publication keeps
   its existing accepted role and may accept only rate=skipped while all
   ordinary prerequisites succeed. Actual Docker publishers may hold package
   write on main or tags; chart publication is tag-only, and chart/Docker
   verification jobs have no package-write grant. This expressly qualifies
   #104's tags-only wording to preserve 0018/0019's continuous path.
5. **Package preparation is distinct from application release.** Prepare
   chart 0.1.1, retain the application version, and document local chart
   installation with a current-image override. Do not republish 0.1.0 or add
   Cargo/Chart equality as a permanent invariant. 0019's independent binds
   remain; future joint releases must deliberately align manifests. The
   partial-publication and concurrent-probe races already named there remain.
6. **Correct 0015 §1's suffix wording without changing POP3.** On failed QUIT,
   committed prefix deletions remain committed and the failing/unprocessed
   suffix remains marked in TRANSACTION. Retrying QUIT applies the retained
   marks; RSET clears them. “Unmarked suffix” is erroneous prose. Preserve the
   accepted historical body and append a dated pointer to this correction.
7. **Reconcile later accepted lifecycle/placement amendments.** 0018's header
   addendum saying five seconds is stale: its body and current HTTP drain say
   ten seconds. Add a dated correction and the missing 0030 links in 0018 and
   0023. Describe current off-worker call sites, including later compat page
   construction, without inventing a guarantee that every list/await copy is
   offloaded. No mutation ordering, stop budget or protocol behavior changes.
8. **Proposed is a status, not operator acceptance.** Compatible work that
   preserves all accepted rulings may be implemented while a record is
   proposed. A proposal cannot authorize contradicting an accepted ruling;
   such a change waits for operator acceptance. This applies to amendments to
   all of 0001–0005, not only the frozen first three. Weakening a frozen
   guarantee remains refused even if a proposal and green tests accompany it.
   Documentation corrections may record observed behavior while clearly
   preserving the distinction between observation and accepted normative text.
9. **Signing statements distinguish authorship and landing.** Branch author
   commits use the configured operator key (or the operator-authorized
   fallback); GitHub squash merges carry GitHub's signature. A disclosed
   inability to sign is an exception requiring the applicable operator
   instruction, not a standing permission inferred from history. This design
   does not remove or disable signing.
10. **Document existing runtime limitations honestly.** Zero TLS handshake
    timeout stays accepted with immediate timeout behavior; non-UTF-8 SMTP
    commands close without a reply. OpenAPI/help changes correct current
    defaults, bounds and error envelopes and carry executable examples; they
    do not redefine the server to fit old prose.

## Alternatives and costs

A pinned hash manifest followed by executing mutable worktree proof code was
rejected in favor of the explicit pinned-execution requirement. Relocation and
workspace-root plumbing cost more initially; canonical implementations avoid
duplicating whole scripts. Wrapper templates are deliberately small.

Selecting the newest directory by mtime was rejected because fresh checkouts
can certify an unrelated old spec. Requiring chief's HEAD commit is stricter:
an unrelated intervening commit forces re-entry rather than a guess.

A TLS parse-time refusal and a mandatory joint version bump were rejected as
new runtime/release changes without the framed need. Their costs remain
visible: zero can still disable successful handshakes, and a future release
must prepare both manifests deliberately. Docker's real persistence smoke is
accepted despite build time because rendering/static checks cannot establish
that uid 65532 can write and recover a named volume.

## Proof and adoption

The spec's AC1–AC12 and each task's paired test are the proof obligations.
Required boundary fixtures include changed gate/helper refusal, invalid design
through actual dispatch, missing residual facts, skipped/failed release needs,
hostile tooling fields, real Docker restart/readback, concurrent chart renders,
and documentation-to-wire pins. Exact coverage and all existing gates stand.

Implementation indexes this record as proposed and appends dated pointers to
the relevant accepted decisions. It must not silently label this record
accepted. No publication, tag push, dependency expansion or issue closure is
authorized by this record.

## Later-decision markers — 2026-10-19 (issue #144)

Appended 2026-10-19 by the issue #144 provenance audit. The operator accepted
this record on 2026-10-19; the text above stays as written. Each marker uses
the later record's own verb and scope.

**Dated correction of the pointer claims (§§6–7 and the paragraph above).**
Until 2026-10-19 the dated pointers this record says it appends did not
exist: no decision but the index linked 0045. They now exist —
[0015](0015-boundary-failure-contract.md) carries the §6 suffix-wording
correction; [0018](0018-shutdown-observed-waits-and-explicit-bounds.md)
carries the §7 ten-second correction and 0030's §3 extension marker;
[0023](0023-off-worker-commits.md) carries 0030's §1 extension marker;
[0007](0007-hermetic-gates-single-model.md) and
[0009](0009-agent-council-hybrid.md) carry markers for §§1–3. None of those
pointers was wrong to add, so no claim above is withdrawn.

- *(Refined by [0050](0050-delivery-gates.md) (accepted 2026-10-19):
  §§1–3 — one canonical pinned proof chain with checked public shims, the
  validator's identity bound to chief's HEAD commit, and a defined
  destination for unknown residual facts, including 0050 §4's dated
  in-place rule order.)*
- *(Amended/clarified by [0051](0051-release-records.md) (proposed
  2026-10-19 — not accepted): this is a pointer to a **proposal**, not an
  accepted amendment, and §§4–5 and 8–10 above govern until the operator
  rules on it. 0051 would order image publication before chart publication
  and restate §8's proposal rule in house rules; it does not extend the
  2026-10-19 acceptance of this record to itself.)*
