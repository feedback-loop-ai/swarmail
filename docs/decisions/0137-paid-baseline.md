# 0137 — Authorize and resume a paid baseline product

- **Status:** accepted with an amendment (operator, 2026-11-23; see the dated addendum at the end)
- **Date:** 2026-11-22
- **Scope:** Part of #393, epic #164. [Specification](../../specs/paid-baseline/spec.md)

## Proposed ruling

The spending unit is a declared product. Its authorization covers a single run or explicit continuation of that run. The scheduling unit is a task/model chunk. A single repetition's validated retained evidence commits its score. These units differ because a process can die after trial three of five, and a login can expire while another family is running.

Use the direct wrappers and a versioned private selection for paid baseline. Keep the existing v1 inputs usable by their current direct callers. The paid selection pins the absolute executable, expected digest, exact version, profile and auth shape for each family. Verify each against the actual selected installation and local credential before spend. A fixed, bounded credential-free version probe can return only the profile's canonical version. Raw CLI trial streams retain the null sink. The approved profile versions are Claude 2.1.310, Codex 0.164.0 and dsh 0.1.5-rc.1. Operator-owned trusted installations must remain immutable during launch. Forwarding shims with unverifiable downstream bytes cannot establish CLI identity. The harness refuses an interpreter script but cannot detect a native launcher that execs another binary, so the operator attests that each pin is the CLI's own binary; a CLI shipped only as an interpreted package cannot run paid. Installed CLIs of other versions (the published pages record Claude 2.1.316–2.1.319 and dsh 0.2.1-rc.3 and 0.2.2-rc.1) refuse until a reviewed profile for that version lands, and the dsh 0.1.5-rc.1 banner is not yet observed from a real binary. Only the verified profile/version/digest enter public row provenance.

The wrapper owner computes eligibility from the selected source. For dsh the existing two-key file lacks validity evidence. For paid baseline use a separate versioned *shape* of that same private family file: each of the two named routes has its key and an operator-attested UTC valid-through instant in the same record. The operator supplies a conservative horizon justified for that key and atomically replaces the complete private record on rotation. Validate both routes before spending and project only the selected key by the existing mechanism. Unknown validity, an old unannotated key file or a missing route cannot qualify. The assertion trusts the operator's knowledge of that credential, costs preparation on renewal and implies nothing about revocation. No token-derived fingerprint, second secret store or new child environment input is needed.

Keep a stable planned slot and distinguish its successive private attempts. Only a terminal score that passed the existing cancellation and cleanup fences and committed its hash-bound metadata consumes a scored slot. A terminal harness error remains an operational failure. Cancelled or unstarted attempt evidence survives and cannot masquerade as a measured failure. Lock release after forced death cannot prove child cessation. Recovery requires evidence of completed cleanup or explicit operator confirmation after checking descendants and private state. Resume can repeat a proved-ended unscored request. The guarantee covers committed scores across process interruption.

[0055](0055-agent-eval-harness.md), [0075](0075-eval-privacy.md), [0076](0076-sequential-eval-trials.md) and [0083](0083-cli-wrapper-launch.md) govern scoring, private publication, process ownership and direct CLI projection. The 2026-11-22 addendum of 0075 at Git object `3ccbe264` admits a private socket for opt-in value capture. This feature needs only existing trial environment inputs. [0119](0119-eval-brokkr-recipe.md) retains the via-brokkr refusal. [0136](0136-paid-token-reference.md) is proposed and accounting is unmeasured. Only the operator accepts this decision. Any implementation that requires an exception to an accepted ruling waits for operator acceptance under 0045 §8.

## Addendum — 2026-11-23: accepted with an amendment

The operator accepted this decision on 2026-11-23 with one amendment, to the approved profiles. Everything else stands as proposed.

**Amended profiles.** The approved profile versions become:

- **Claude Code 2.1.329**, the native binary.
- **codex-cli 0.166.0.** The pin names the native binary that the volta shim forwards to, never the shim itself. A shim's downstream bytes still cannot establish identity.
- **dsh 0.2.2-rc.1**, admitted as an interpreted CLI. Its pin covers both the node runtime binary that runs it (absolute path, exact version, sha256) and the sha256 of every file in the installed dsh package. Launch verifies all of them before spend and refuses on any mismatch, extra file or missing file. This replaces, for dsh only, the sentence that a CLI shipped only as an interpreted package cannot run paid. The operator's attestation that each pinned binary is the CLI's own extends to the node runtime.

The earlier versions (Claude 2.1.310, Codex 0.164.0, dsh 0.1.5-rc.1) leave the approved set. They were never observed on this host for the cohort, and codex 0.164.0 cannot serve the gpt-6.2 models.

**Why.** The reference cohort of [0136](0136-paid-token-reference.md) needs all three families on the CLIs actually installed. Refusing dsh would drop the GLM family, and with it the epic's per-family measure.

**What follows.** A follow-up delivery (#404, scope item 4) adds the three reviewed profiles and the interpreted-pin verification. Until it lands, paid baseline refuses every installed CLI, as this decision already requires.
