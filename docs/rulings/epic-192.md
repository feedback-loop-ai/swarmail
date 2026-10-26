# Operator rulings — epic #192, 2026-10-24

Given by the operator in a session with the operator's assistant, which recorded them here. Each ruling is the operator's acceptance or direction. Seats implement them; seats do not re-litigate them or accept anything further.

| Item | Ruling |
|---|---|
| #175 | Narrowed to findings 7.1–7.4 and their two acceptance criteria. The pre-existing coverage-denominator gap (sources reached through CARGO_HOME / RUSTUP_HOME / /rustc, HIGH, gate integrity) is split into #219 and is out of #175's scope. Review panels should record it as pre-existing and tracked in #219, not as a residual of a #175 change, unless the change makes it worse. |
| #198 | **0071 accepted** (named eval traps invalidate a trial; catalogue, snapshot and trial-result versions move to 2; amends 0055 §§1, 2, 9). |
| #181 | **0070 accepted** (narrow exception to 0029 §§3, 6: map-only admission of ChaosConfig/ChaosRule; positional seed and chaos arrays refused before any mutation). |
| #195 | **0067 and 0068 accepted.** 0069 (bounded CA executor, amends 0041 §2) stays proposed and is split into #220; #195 ships without it. The low-severity shared-blocking-pool availability residual is tracked by #220. |
| #199 | Close #199 by refusing concurrent trials (one trial at a time), a fresh HOME per trial, and documentation and 0055 text saying trials are sequential. Namespace isolation is split into #221; its decision stays proposed. |
| #202 | The upstream-Prometheus-parser proof criterion is dropped. Prove label escaping with in-repo spec-conformance tests (exposition-format escaping of `\\`, `\"`, `\n`; CR/NUL/C1/bidi round-trip byte-exactly through a strict reader written in the test). No new dependency. |
| #205 | `brokkr bundle (pinned)` and `docker smoke` are already required status checks on `main` (verified by GitHub API 2026-10-24; enforce_admins on, strict on). What remains is documentation and inventory alignment. |
| #206 | Publish the pinned brokkr binary (sha256 ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f) as a release artifact. tooling.json records its URL and digest; CI and the verify seat download and check that exact digest. |
| macOS evidence for `evals/` | The `evals/` crate is not in CI until #201. Any macOS-only proof for evals work is owned by #201's CI gating, not by individual eval issues. |
| Codex fast triage on Opus | Lands as its own PR after #215, rebased onto main. |

## Decision number ledger

0062 #180 (RPC edge contracts) · 0063 #200 · 0064 #174 · 0065 #202 (accepted) · 0066 DashScope fallbacks (accepted) · 0067/0068 #195 (accepted) · 0069 #220 (accepted 2026-10-25) · 0070 #181 (accepted) · 0071 #198 (accepted) · 0072 #171 · 0073 #194 · 0074 #197 · 0075 #196 · 0076 #199 (proposed). Next free: 0077. Numbers 0052–0054 are reserved for epic #120; 0059–0061 are not to be reused.

## Rulings of 2026-10-25

| Item | Ruling |
|---|---|
| #195 | Fix the three LOW non-security findings of the last review (run `epic-192-issue-195-refuse-privat-8e00b7a6`): sanitized private-DER diagnostics in tests, the README "never blocking the SMTP path" claim, and the unkeyed `API_PREPARATION_PANIC` seam. Then the operator merges #195 despite the `REVIEW-RESIDUAL-UNSAFE` stop: the only security residual is the low shared-blocking-pool availability residual, owned by #220. Panels record that residual as tracked by #220, not as new. |
| #220 | **0069 accepted** (bounded CA executor; amends accepted 0041 §2). #220 implements it after #195 lands. |
| #191 | Its change to `sigterm_with_a_delivery_in_flight_prints_the_final_counts` fails on macOS CI (a bound, non-listening loopback socket times out instead of refusing). Repair it so the test is portable; macOS CI is the proof. |
| Engine re-pin | The `attempt_id` correlation fix is reviewed in a careful engine delivery before any re-pin; #201 runs last, on the re-pinned engine. |

## Rulings of 2026-10-26 (epic #164)

| Item | Ruling |
|---|---|
| #151 baseline | The operator approves the full baseline's cost: every v2 catalogue task, two models per family (Claude: `claude-opus-5-5`, `claude-sonnet-5-5`; GPT via Codex: `gpt-6.1-sol`, `gpt-6-luna`; GLM via dsh: `dashscope/glm-5.3`, `spark-glm/GLM-5.3-Flash-EXL3`), 5 trials per cell, concurrency 1 (0076). Before any paid trial: the free oracle/null control matrix. The paid run is an operator act; delivery seats build and prove the wrappers and the one command, never run paid trials. |
| Phase 1 now | Only #152 (MCP 2025-06-18/2025-11-25 negotiation) and then #153 (structuredContent + outputSchema) are delivered before the baseline. Their PRs stay unmerged until the baseline exists (#151 AC1). #154–#160 wait for the baseline's re-prioritization. |
| #243 | Give the operator rulings a tracked home and point every tracked link there; refuse tracked links into `.forge/`. |
