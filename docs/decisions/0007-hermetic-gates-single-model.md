# 0007 — Hermetic gates and a single-model delivery bundle

**Status**: accepted (operator directive, 2026-10-07 — "GLM 5.3 flash on
spark only", mirroring xbox-inference)

## Ruling

The delivery constitution (bundle.json + policy.json) has five phases —
intake → implement → verify → review → ship — with two hard rules:

- **`verify` and `ship` are exec seats.** They run
  `scripts/verify-seat.sh` and `scripts/ship-seat.sh` — plain bash, no
  model in the loop. A shell proves what a model claims; a model must
  never be the thing that certifies its own work.
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
