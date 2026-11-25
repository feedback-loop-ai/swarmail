# 0138 — Bind nested driver usage to a committed paid attempt

- **Status:** proposed
- **Date:** 2026-11-23
- **Scope:** Part of #404, #391 and #164. [Specification](../../specs/nested-driver-usage/spec.md)

## Proposed evidence contract

0119 and 0136 at 64f4763e already authorize the nested driver as the token source. Accepted 0137 at 33be71de settles the profiles and the unit of score commitment. A separately written trial-id lookup could associate a bill from an interrupted attempt with its replacement.

Publish a versioned closed companion in the existing retained trial leaf. Its owner is the harness. Bind its digest in the same metadata commit that validates the score. Verify trial, slot/attempt, profile and source on replay. Expose only the six typed session fields specified in the linked design. Source-specific absence and ambiguity propagate to the reader. Retain independently valid fields. Keep schema-2 result rows and existing grade semantics.

This adds one small artifact and one digest per nested trial. Interruption can leave a paid execution unmeasured. Recovery follows 0137's cessation rule. A committed score consumes its slot even when usage is unavailable. The proposal adds no authority to spend, parse a direct CLI stream, or change mail behavior. Only the operator accepts this record.

## 2026-11-24 clarification of the proposal

A paid slot can run several agents. The three-agent inbox-isolation task in the accepted 0136 cohort exposed the ambiguity in “attempt”. Keep the slot assignment attempt as the owner of one companion. Within it list each expected agent and its distinct driver attempt, disposition and six typed fields in assignment order. Validate that roster against the harness assignment on replay. Derive the trial figure by checked addition per field. Missing or overflowing members make the affected figure unavailable. This preserves each session’s attribution through a sibling failure without changing the schema-2 row or introducing another commit marker.

## 2026-11-24 note: where the build differs

At a73ac17a the build differed from the proposal as follows.

- **Slot and assignment attempt.** The companion records the row's matrix
  coordinates as its slot and checks them on every read. It does not record
  the paid coordinator's attempt directory. A leaf is that attempt's own, so
  each leaf binds its companion. No trial-level field names the attempt
  independently of the row, and adding one would change the row. Duplicate
  owners are refused per driver attempt, across the leaves of one matrix
  replay and one `report --usage-from` read.
- **Where a missing companion refuses.** A scored nested row with no
  companion is refused at retention, at every leaf read (`check`, resume and
  matrix replay), and by `report`, which needs `--usage-from` for such rows.
  The runner reports it as a `harness_error` (`usage_companion_unretained`)
  instead of a scored row without usage. So a nested trial needs a retained
  leaf.
- **Superseded via profiles.** Only the three amended via-brokkr profiles
  need a companion. Rows of the superseded ones never launched the nested
  driver (`driver_contract_unknown`). They stay journal-route rows and read
  as before.

At that revision the release pin was enforced at the coordinator's admission and, on the
recipe route, by the wrapper. The offline stand-in driver is admitted only by
a test-equipment digest that the coordinator's own process reads. An agent's
environment is rebuilt, so the recipe route admits the release alone.

The second rework requires explicit assignment ownership because two independently valid leaves can claim the same driver attempt. Carry the paid assignment identifier outside the schema-2 row, compare it with the coordinator’s assignment on replay and claim driver attempts across the entire resume scan. Apply the same ownership checks when producing the baseline report and replaying it offline. The review also found that the production verifier itself reads the stand-in environment variable. The revised design requires a release-only trust set at each nested launch and confines fixture admission to inert builds barred from paid execution. These are requirements for the next implementation. The observations above describe a73ac17a.

After the second rework (run …-8cc2a397), the build differs as follows.

- **Trust set.** A release build admits the published driver digest alone
  at every nested launch, host and recipe, and reads no environment for it.
  A debug build also admits the stand-in built beside it and spends only
  synthetic credentials, so the stand-in seam cannot reach a real login.
- **Launch authority.** The host route's binding is granted by the live
  baseline coordinator on a private per-agent endpoint authenticated by
  kernel peer credentials, not by the argv string, which any caller can
  compute.
- **Ownership.** Driver attempts are claimed across the whole scan by resume,
  by the baseline's own report, by matrix replay and by `report
  --usage-from`. The companion is still version 1 and has no separate
  assignment field. Its slot is the row's matrix coordinates, whose
  `matrix_id` a paid chunk sets to its attempt id. Resume ties the slot to
  the attempt through the plan, which requires every cell's coordinates to
  carry the plan's `matrix_id`, equal to the attempt id. The cell must also
  match the row. A separate field would restate that binding. Adding one
  would need a companion version 2, which this rework did not build.

At 6074b83e the debug credential check searched for a fixture marker anywhere in the file. The third review found that an unrelated field could contain that marker. The build now differs from that revision. The debug check reads only the family's authorizing values: Claude's access token, Codex's access token and API key, and the selected dsh route's key. Each value present must be offline equipment, and at least one must be present. That means a `SYNTHETIC-` value, or a JWT whose signature segment decodes to one. The check does not compare against one exact fixture: the suites' fixtures differ by family and by test, and an authorizing value that a real issuer could never produce excludes a real login just as well. The release-build proof (`release_standin_is_not_admitted`) and the baseline-CLI proofs for the version-1 ownership chain now exist. Both are host-gated tests, and the evidence ledger records their runs.
