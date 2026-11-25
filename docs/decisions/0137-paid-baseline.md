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

## Addendum — 2026-11-24: the operator's rulings on the nested paid route (#404)

The operator made four rulings on 2026-11-24 for the nested (via-brokkr) paid route, in chat to the dispatcher, while #404 was under review. They are recorded here because they govern this decision's launch boundary. #413 (`1161a535`) delivered them.

1. **The boundary.** A nested paid session runs through the operator's host `swarmail-evals baseline`, with the confinement this decision accepts for direct wrappers.
   - **F5 binds.** The recipe route carries read-only binds in `.brokkr/recipes/eval/bundle.json`. That held path was approved by name for this purpose only. The binds are the four DNS inputs, the CLI and driver install trees, each family's config and auth, the operator's selection directory, and dsh's `profiles` as an overlay. The wrapper re-proves every declared bind in its own mount table before any credential read. A missing source refuses, and so does a writable mount beneath a declared tree.
   - **Proof by a real mount.** The proof is a real mount rejecting a write; reading `os.access` is not a proof. That proof is host-gated.
   - **Fail closed.** A paid launch without a verified binding refuses before spend. The binding is coordinator-owned and bound to the wrapper's own fresh request. Caller argv cannot supply it.
   - **Codex sandbox.** Codex runs only under `read-only` or `workspace-write`, and `danger-full-access` is refused. The row's descriptor and retained provenance record the class actually used.
2. **Multi-agent tasks score.** A trial with several agent sessions reports the checked per-field sum over all of them. Any unavailable session makes that field unavailable, never zero, and an overflow does the same. [0136](0136-paid-token-reference.md)'s 16-task cohort is unchanged.
3. **The usage companion.** Nested usage goes in proposed [0138](0138-nested-driver-usage.md)'s versioned companion in the retained leaf, digest-bound in the metadata commit, and not in the schema-2 row. 0138 itself stays proposed.
4. **The threat model.** The fail-closed boundary guards against *accidental or unbound* paid launches: a bare wrapper, caller-made argv, a stale or replayed grant, a missing bind, a non-release driver. It does not defend against a deliberate adversary running as the operator's uid, who could equally run the agent CLI directly with the operator's credentials. Reviewers route such findings as residuals, not holds. The deliberate same-uid grant class stays an accepted residual of #413.

**Still open.** The 2026-11-23 smoke needed `~/.claude`, `~/.claude.json` and `~/.codex` read-write. Ruling 1 approved read-only binds only. A paid claude or codex session on the *recipe* route is therefore unproven, and needs a further ruling before it runs. The host baseline route is unaffected.
