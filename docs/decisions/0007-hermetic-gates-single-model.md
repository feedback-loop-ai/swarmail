# 0007 — Hermetic gates and a single-model delivery bundle

**Status**: accepted (operator directive, 2026-10-07 — "GLM 5.3 flash on
spark only", mirroring xbox-inference)

## Ruling

The delivery constitution (bundle.json + policy.json) has five phases —
intake → implement → verify → review → ship — with two hard rules:

- **`verify` and `ship` are exec seats.** They run the bundle's own
  `bundles/self/scripts/verify-seat.sh` and `bundles/self/scripts/ship-seat.sh`
  — plain bash, no model in the loop. The scripts live inside the
  constitution so the hermetic box mounts the exact bytes the bundle pins
  (the exec dispatch resolves a seat script against the bundle root and
  re-walks its directory into the namespace), never whatever the worktree
  carries. A shell proves what a model claims; a model must never be the
  thing that certifies its own work.
- **One model route.** Every model seat runs `glm-flash` →
  `spark-glm/GLM-5.3-Flash-EXL3` via the `dsh` driver on the local spark
  route (`SPARK_API_KEY`). The adapter declares no other route, and the
  reviewer/judge is `glm-flash` too. Adding a route is a decision
  document, not an edit to `adapters/dsh.json`.

## Why

The engine's discipline is that proofs are cheap and deterministic; the
expensive, nondeterministic seat is the one that writes code. Pinning one
route keeps cost accounting trivial (`brokkr costs`), keeps the judge
capable of reading exactly what the implementer wrote, and keeps the
supply chain (one credential, one endpoint) auditable.

## Consequences

- `adapters/dsh.json` declares `tool_permissions: "unsupported"` and
  `mcp: "unsupported"` — the dsh driver cannot express a restricted tool
  surface, so agents declare no tool restrictions rather than pretend
  (compile refuses an unexpressable restriction rather than degrade
  silently).
- Spark routes refuse `--effort` (dsh 0.1.5-rc.1); `effortless_routes`
  records it and seats pin no effort — mirrors xbox-inference decision
  0035 addendum.
- The review gate is constitutionally protected (`protected_phase`:
  review); security-hold is a hard stop. The journal (`.forge/forge.db`)
  is the proof.

## Later-decision markers — 2026-10-19 (issue #144)

Appended 2026-10-19 by the issue #144 provenance audit, after the operator's
2026-10-19 ruling accepting 0034–0050 and 0055. The text above stays as the
accepted historical ruling; each marker names a later record with that
record's own verb and scope, and nothing above is rewritten by it.

- *(Amended/clarified by [0045](0045-delivery-repairs.md) (accepted
  2026-10-19): §1 — pinned verification is transitive over its executable
  local proof code; the canonical gates, their proofs and the scanner run
  from the dispatched pinned resources, and public script paths are checked
  compatibility shims that never select verify's proof implementation. 0045
  calls this a restoration of the hermetic-gates ruling, not new policy;
  [0050](0050-delivery-gates.md) §1 then implements it.)*

## Later-decision marker — 2026-10-23 (0057)

[0057](0057-codex-delivery-recipes.md), accepted by the operator's
2026-10-23 directive, amends the work-route restriction and adds the
Codex recipes' independent reviewer mapping. The shared design council
now uses Codex Sol/Astra. Hermetic exec verification/shipping, the four
review lenses, worst-verdict aggregation and the hard security hold remain
in force. The historical ruling above is not rewritten.
