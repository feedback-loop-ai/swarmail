# 0051 — Release prerequisites and decision provenance

- **Status:** proposed — acceptance is the operator's.
- **Date:** 2026-10-19.
- **Issues:** #141, #142, #144 under epic #120, batch 6.
- **Amends/clarifies:** [0019](0019-release-tag-integrity.md)'s independent
  publication/partial-release posture and
  [0045](0045-delivery-repairs.md) §§4–5, 8–10.
- **Preserves:** 0001–0005, 0019's exact-ref and absent-tag refusals,
  0020's test placement, 0026/0030's deployment knobs and 0050's pinned proofs.
- **Design:** [spec](../../specs/release-records/spec.md),
  [plan](../../specs/release-records/plan.md),
  [tasks](../../specs/release-records/tasks.md).

## Context and authority

Independent publishers permit chart 0.1.1 to publish at v0.1.1 while Cargo
0.1.0 prevents image publication. Installation examples leak anonymous
volumes, and documentation/decision links overstate delivered proof.

The operator ruling recorded in issue #144 on 2026-10-19 accepts existing
0034–0050 and 0055, including their dated amendments. Implementation records
that ruling; it does not retroactively infer acceptance from shipped code.
0055 reserves 0049–0054 for this epic; 0051 is its next free number. That
existing acceptance does not extend to 0051.

## Proposed ruling

1. **Image publication precedes chart publication.** Chart publication needs
   successful Docker publication explicitly, all existing ordinary gates and
   same-run tag rate proof. Docker also needs chart verification and a real
   image smoke. The actual chart preflight checks the Docker result and
   requires HTTP 200 for the bare appVersion image manifest, using the
   canonical publish guard. Absent or unknown image state refuses; existing
   chart/image absence probes retain their opposite 404-only admission and
   republish refusal. No rollback, retries, skip-on-existing or new credential
   is introduced. This strengthens the accepted release safety posture;
   it does not remove accepted successful-publication obligations.
2. **Preparation is distinct from release.** appVersion equals Cargo, and
   default render/Artifact Hub image references agree with appVersion. Chart
   version may differ while preparing a package. A successful joint tag run
   still satisfies both existing exact ref binds. The present v0.1.1 fixture
   publishes neither; the operator must align a release deliberately. Do not
   bump Cargo to hide the refusal or revert a prepared chart. Chart changes
   require a valid monotonic chart bump against the entire event baseline.
   Missing history refuses. History checks live with CI chart verification;
   their real-git selftests gate that job, not a new hermetic verify duty.
3. **Container invocation owns ephemeral cleanup.** Keep the image's
   non-root uid, owned /data and VOLUME contract. Ephemeral quickstarts use
   --rm and a 30-second stop grace; named storage has explicit operator
   cleanup. A real Docker smoke proves 250/readback across graceful and
   hard-kill restart, anonymous-volume removal and recursive target context
   exclusion. This is host/container persistence evidence, not a new
   power-loss guarantee. Main-tag Helm examples set Always explicitly.
4. **Correct the contract record without changing runtime behavior.** The
   spec's D01–D24 register is exhaustive for #142. OpenAPI pins compare
   defaults, domains and status maps with actual handler behavior; assert's
   inert limit is not a page. Dated errata identify historical corrections;
   CI assertions exclude ungated benchmark medians. SMTP second-AUTH, zero
   TLS timeout and other listed limitations are documented, not silently
   changed. Frozen guarantees and earlier epic repairs remain intact.
5. **Acceptance and amendment provenance are explicit.** Implement the
   operator's named status updates and bidirectional index membership checks,
   using existing status conventions. Accepted bodies stay intact; append
   dated linked amendment/correction markers, including the pointers promised
   by 0045. Clarify 0035's enumeration by dated mapping, not renumbered body
   text. A finite audited relationship list is tested in both directions;
   no parser purports to decide the truth of arbitrary prose.
6. **Proposed status supplies no authority to contradict an accepted ruling.**
   Restate accepted 0045 §8 in house rules: compatible implementation may
   land under a proposal; a contradiction waits for operator acceptance,
   including amendments to every one of 0001–0005. Weakening frozen
   guarantees remains refused regardless of green tests. The operator's
   acceptance of earlier proposals is not permanent permission for future
   ones. During implementation, append proposed amendment pointers from
   0019/0045 to this record, explicitly distinguishing proposed changes from
   accepted text; neither pointer nor this design self-accepts the change.

## Alternatives and losses

Adopt robustness's explicit Docker success and present-image check over
implicit scheduler gating alone. The extra bounded registry request costs
release availability during registry failures. Retain simplicity's existing
chart owner instead of a new pinned guard/shim family and gate inventory;
history proof remains CI-only. A static assertion of the ignore pattern loses
to a real Docker context proof, at build-time cost.

Adopt simplicity's independent preparation versions over permanent equality
of all manifests. A prepared tree can be unreleasable until the operator
cuts a joint release. Adopt finite documentation/decision tests over a new
record schema and generic prose crawler; humans still audit semantic claims.
Reject a proposed-decision expiry rule and any runtime fix to match old docs.

Image publication can succeed before chart failure. A rerun of all jobs may
then refuse the existing image under 0019; recovery stays an explicit
operator registry action. Presence probing is not digest attestation or
registry immutability, and does not prevent later deletion or concurrency
races. Cached smoke/publication rebuilds are not identical-byte proof.

## Proof and adoption

The linked spec's AC1–AC9 and tasks' paired tests are required. Exercise
actual guard scripts against real local HTTP and chart history against real
temporary Git repositories; preserve workflow wiring tests. Use actual
Docker/SMTP/REST for lifecycle proof, and existing API/listener/storage seams
under 0020 for documentation behavior. Run the full final-tree house gates
and chart/Docker CI proofs. An unavailable proof remains outstanding.

Implementation adds this record's index row as proposed and dated pointers
without rewriting accepted bodies. Only the operator accepts this decision.
No publication, external message, force push or unsigned commit is authorized.
