# 0051 — Release prerequisites and decision provenance

- **Status:** accepted (operator, 2026-10-24).
- **Original 2026-10-19 proposal status:** proposed — acceptance is the operator's.
- **Corrected 2026-10-23 proposal status:** proposed — awaiting the operator.
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


## Proposed correction — 2026-10-23 (epic #192; awaiting the operator)

This review copy preserves the 2026-10-19 proposal above verbatim and adds
this dated amendment. It is prepared for the operator's review, remains
**proposed**, and does not record acceptance, authorize publication or close
#184. The tracked decision and its index have not been changed. Recipe
delivery remains the first landing; this release-policy choice is separate.

### Authority and the signing correction

Accepted decision 0056 already governs authoring: seats and the operator's
assistant commit unsigned, never push and never stop for signing. The operator
seals the branch before merge, separately from signed release tags. The
closing sentence above must therefore be read with its unsigned-commit
restriction superseded by 0056. This correction needs no new signing ruling
and grants no new push, merge, registry-delete, retag or release authority.

The release-version change below is different: accepting an earlier proposal,
shipping code or requesting this corrected draft does not accept 0051.
Accepted 0019 rejects the four-way equality assertion, and accepted 0045 §5
preserves independent preparation versions and the earlier binds. The current
workflow nevertheless couples successful chart publication to the same
image-release tag. That is the unresolved contradiction identified by
[#184 R4.1](https://github.com/feedback-loop-ai/swarmail/issues/184#issuecomment-6285338145),
rather than evidence that those accepted rulings already authorize it.

### Corrected proposed ruling for successful joint tag releases

The concrete option drafted here retains the current image-before-chart
design and makes its stronger release constraint explicit:

1. A successful **joint image-and-chart tag release** requires
   `tag version = Cargo package version = Chart appVersion = Chart version`.
   The chart's default rendered image and Artifact Hub image reference must
   agree with appVersion. This is a publication prerequisite for this joint
   workflow, **not a requirement that preparation trees always have equal
   application and chart package versions**.
2. This deliberately amends 0019's rejection of the four-way publication
   equality assertion and 0045 §5's independent-publication interpretation,
   in this limited joint-tag scope. Their accepted bodies remain intact;
   acceptance requires dated amendment pointers naming the actual operator
   ruling. Independent preparation versions, the exact-ref guards, the
   absent-tag refusals, the ordinary gates and the same-run tag rate proof
   remain. Main's continuous image publication keeps its separate accepted
   role under 0045 §4.
3. Image publication must succeed before chart publication. The chart checks
   the Docker job's explicit success, checks its appVersion image is present
   and checks its own chart tag is absent. The image absence guard still
   checks both bare and v-prefixed image tags. An unknown registry result
   refuses. This does not make the two publishers transactional or close the
   check-then-push race.
4. Image success followed by chart failure remains possible. A full rerun may
   then refuse the already-published image; recovery is an explicit operator
   decision, never automatic deletion, skip-on-exists or a republish override.
   Rerunning an old workflow invocation executes its recorded workflow; this
   proposal does not claim to retrofit current guards into historical runs.
5. Preparation does not select or publish a version. The operator separately
   chooses deliberate release versions and a tag after required work and
   proofs. Do not bump Cargo, revert a prepared chart or change an image
   reference merely to make a guard green. Retain the chart's event-baseline
   version-bump proof and its fail-closed missing-history behavior.
6. Accepting this corrected record would resolve the policy choice; it would
   not prove the implementation or a real release. The existing AC1–AC9 and
   paired tasks, all applicable house gates and CI-only proofs remain.
   Residuals in #169, #185 and #186 are not waived by accepting this record.

### Current versions and what the workflow actually permits

At this preparation snapshot, Cargo is `0.1.0`, Chart `version` is
`0.1.2`, `appVersion` is `0.1.0`, and the Artifact Hub image is
`ghcr.io/feedback-loop-ai/swarmail:0.1.0`. The earlier `0.1.1` wording
above records its original preparation state; it is not the current chart
version and is not a new release instruction.

The image job binds the tag to Cargo; chart coherence binds appVersion to
Cargo; the image-present preflight binds the same tag to appVersion; the
chart guard binds that tag to Chart version. Their composition gives the
equality above. With the observed preparation values, the tag-only flows are:

| Trigger, if ordinary prerequisites pass | Image job | Chart job |
|---|---|---|
| `v0.1.2` | Exact-ref refusal: Cargo says `0.1.0` | Does not publish because Docker did not succeed |
| `v0.1.0` | Can publish only if both image-tag absence probes admit it | Chart exact-ref refusal: Chart version says `0.1.2`; a partial image release is possible |
| A deliberately aligned future `vX` | Still needs both tag forms absent and every prerequisite | Still needs image-job success, present image and absent chart tag |

No registry probe or publication was performed to prepare this document.
The table describes the code's conditional behavior, not current registry
contents or proof that any named release is available.

### Adoption and outstanding evidence

- **Operator choice remains open.** The drafted option is the explicit
  joint-tag amendment above. The alternative is to retain independently
  versioned chart publication and redesign/restore the image dependency and
  preflight accordingly. That alternative requires a concrete reviewed
  workflow change; a presence probe that still binds the chart tag to
  appVersion does not implement it. Until an actual ruling, this draft does
  not legitimize the shipped contradiction or close #184 R4.1.
- **Signing is already settled.** Apply 0056 to author commits now; no part of
  this proposal creates an additional signing pause or permits seats to push.
- **Release values and the actual release are not selected.** A future release
  needs deliberate versions, a signed release tag and the separately
  authorized publication operation. Preparing or accepting this draft
  authorizes none of those external actions.
- **The real registry receipt remains outstanding.** #186 finding 18.5 needs a
  linked guarded-release run showing the real 404-then-publish path, exact tag
  and commit, relevant versions, prerequisite/rate results, guard results and
  image-before-chart ordering. Local HTTP/Git selftests, compilation and this
  draft cannot replace that receipt. Do not add a claim of digest attestation,
  registry immutability or identical smoke/build bytes.
- **Packaging attribution remains separate work.** The additional
  [#186 packaging finding](https://github.com/feedback-loop-ai/swarmail/issues/186#issuecomment-6285339357)
  names the packaged WHATWG CC-BY-4.0 component and asks for reviewed package,
  README and image attribution. Its notice-versus-metadata choice and
  package/image proofs remain unresolved here; no licensing conclusion or
  dependency-allowlist widening is made by this proposal.
- **Other record corrections remain separate work.** #184's gate inventory,
  release-note status/Unreleased entries and 0050 errata do not become proved
  by correcting 0051. Preserve accepted historical bodies and add dated
  corrections with actual evidence.

If the operator later accepts the drafted joint-release option, the adoption
change must record that explicit ruling and date, update the 0051 status and
index consistently, and append the scoped amendment pointers to 0019 and
0045. The present review copy makes none of those acceptance edits.

### Source-backed review register

The source snapshot and SHA-256 inventory are stored beside this draft in
`0051-release-records.sources.json`; the append-only candidate diff is
`0051-release-records.diff`. File/line references below are observations of
that snapshot, not permanently stable line numbers.

| Observation | Source |
|---|---|
| Current chart `0.1.2`, appVersion `0.1.0`, image annotation `0.1.0` | `charts/swarmail/Chart.yaml:7–8,26–28` |
| Current application `0.1.0` | `Cargo.toml:3` |
| Image exact-ref guard precedes the release-tag push | `.github/workflows/ci.yml:329–355` |
| Chart needs Docker success; appVersion equals Cargo; image and chart guards bind the same tag separately | `.github/workflows/ci.yml:430–483` |
| Preparation coherence does not itself require Chart version to equal Cargo | `scripts/chart-verify.sh:71–85` |
| Earlier equality rejection and independent preparation ruling | `docs/decisions/0019-release-tag-integrity.md:116–121`; `0045-delivery-repairs.md:52–57` |
| Accepted unsigned authoring and operator seal | `docs/decisions/0056-signing-policy.md:30–48` |
| Actual policy contradiction and record follow-ups | `issues/184-source.json`, including its 2026-10-20 review comment |
| Actual release receipt and additional packaging obligation | `issues/186-source.json`, including its 2026-10-20 review comment |

## Adoption — 2026-10-24

The operator's task instruction for run epic-192-issue-184-repair-the-fi-984472b4 accepts this exact corrected proposal, SHA-256 89fff17e5a160de79e837bb0e61b2f00bfeef85241d515d1a18e7679f9f257dd. This acceptance is recorded on 2026-10-24, the date supplied with the ruling; it is not backdated to the 2026-10-19 acceptance batch and is not attributed to this seat.

The corrected proposal dated 2026-10-23 and its original 2026-10-19 draft are retained as historical text. Their statements that the choice remains open, that 0051 is awaiting the operator, and that adoption is conditional describe the state before this ruling and are superseded by this acceptance. The accepted rule requires tag version = Cargo package version = Chart appVersion = Chart version only for a successful joint image-and-chart tag release; preparation versions may differ. Image publication still succeeds before chart publication. The exact-ref guards, absent-tag refusals, ordinary prerequisites, same-run tag rate proof, main's separate image role, partial-release limits and explicit operator recovery remain as stated.

This decision records the policy choice; it does not prove implementation or a real release, select release values, or authorize registry actions, publication, push or merge. Decision 0056 remains controlling: author commits are unsigned, the operator seals the branch before merge, and release tags are signed separately.
