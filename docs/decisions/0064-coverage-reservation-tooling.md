# 0064 — Direct exclusive coverage reservation with checked tooling

- **Status:** proposed
- **Date:** 2026-10-24
- **Scope:** Epic #192 issue #174; coverage explicit-target reservation and its selftest.
- **Design:** [spec](../../specs/coverage-selftest/spec.md),
  [plan](../../specs/coverage-selftest/plan.md),
  [tasks](../../specs/coverage-selftest/tasks.md).
- **Preserves:** accepted [0020](0020-concurrency-tests-and-the-coverage-gate.md),
  [0035](0035-coverage-gate-refuses-by-effect.md),
  [0050](0050-delivery-gates.md), frozen 0001–0003 and ingest extraction 0005.
- **Numbering:** next number after the existing 0061 record; earlier gaps are
  not reused. Acceptance and any integration renumbering belong to the operator.

## Context

The retained full-context and direct same-resolver traces at candidate f3cee46
show uutils mkdir 0.8.0 returning success after kernel EEXIST. One plain leaf
mkdir is therefore not reliable evidence of exclusive creation on this host.
Two accepted coverage builds can share artifacts even while the arithmetic
still claims exact 100/100/100. The spec records trace paths and their limits.

The exact fresh-leaf and caller-owned retention contract already exists in
reviewed #173. Restoring it does not redefine ownership or coverage. Requiring
Python for this operation and the repaired selftest does change standalone
gate tooling requirements; this record makes that cost explicit. Python's
existing use by publish-guard does not prove availability in every coverage
environment. The chief observed isolated Python 3.14.4 stdlib imports in the
current box only; verifier/platform and implementation proofs remain due.

## Proposed ruling

1. For a nonempty explicit target, the pinned canonical Bash script uses an
   inline installed Python 3 stdlib operation under `-I -S`. Pass the exact
   path as argv and call `os.mkdir(path, 0o777)` once, inheriting umask. Any
   OSError, including EEXIST, refuses; never retry or infer success from later
   existence. Parent creation remains checked and idempotent. Do not normalize
   the returned path or choose another leaf.
2. Require installed isolated Python capability for explicit reservation and
   selftest, with checked startup and operation completion. Missing/broken
   tooling or malformed operation output produces a distinct stable
   `reservation tool unavailable` refusal, status 1. Filesystem errors retain
   the `fresh, unused leaf` category, exact path and status 1 with truthful
   operation detail. No silent mkdir fallback, auto-install, network access,
   version blacklist, GNU-only restriction or third-party Python package.
   Unset/empty default allocation continues using the existing mktemp path
   without requiring Python from the resolver.
3. Assign resolved path and ownership 0 only after validated creation success.
   Explicit targets, including evidence left by later failure, belong to the
   caller. Never remove them on loser, tooling, preflight or build failure.
   Default ownership/cleanup, free-space behavior, scanner and exact coverage
   arithmetic remain unchanged. No marker, lease, stale-target recovery or
   distributed lock guarantee is added.
4. Keep the two-worker selftest local to the same pinned script. It may use a
   specialized inline stdlib supervisor for private channels, monotonic
   deadlines and owned POSIX process groups. Two live readiness acknowledgements
   precede checked release; the 30-second deadline covers setup through joins,
   with at most 5 seconds teardown grace. Expiry fails. Preserve descriptors,
   bound descendant teardown and finish it before root removal. No sleep
   establishes readiness or success. Infrastructure completion and resolver
   outcome are distinct; the loser must supply its own category/path refusal.
5. Keep Linux/macOS compatibility and verify actual installed capabilities in
   the candidate and hermetic verifier. A missing prerequisite refuses honestly;
   an actual supported-environment incompatibility requires an explicit return,
   not a hidden platform restriction. The source stays inline and pinned; no
   scanner command, compiled bootstrap, dependency graph or allowance widening.
6. This proposal accepts no decision. Compatible work preserving accepted
   authority may proceed under [0045 §8](0045-delivery-repairs.md); an actual
   contradiction waits for the operator. No frozen guarantee may be weakened.
   All house gates, protected reviews and security hold remain required.

## Alternatives and costs

A Bash no-clobber claim file would avoid Python, but adds a second ownership
fact inside the build tree and a lifetime obligation across downstream clean
and build commands while another contender may be delayed. Neither panel
proved that lifecycle. Choosing direct creation costs interpreter startup,
capability checks and explicit standalone-tool documentation; it avoids a new
marker protocol. This is not a claim that a particular Cargo release was
observed deleting that marker.

Prechecks/postchecks alone retain a race. Selecting GNU mkdir or blacklisting
uutils versions adds an implementation-support policy and does not prove other
versions correct. A Rust bootstrap helper performs compiler work before the
reservation/free-space boundary; a scanner subcommand violates its preserved
scope. Locks, aliases, leases and reclamation add lifetime questions unrelated
to the demonstrated EEXIST translation. A readiness-only fix leaves the
production admission defect intact.

A reporting failure after successful creation can leave an unused retained
leaf; refusal and caller recovery are safer than deleting evidence. Cooperating
callers must retain the namespace. Mixed old/new resolvers, hostile external
replacement, distributed filesystems and crash reclamation are not new
promises. A killed supervisor or uninterruptible host I/O cannot be guaranteed
to reap within a wall-clock bound; neither circumstance may yield a green
proof or permit cleanup of a root with live workers.

## Proof obligations

AC10–AC16 and paired T1–T8 require retained causal evidence, deterministic
fault-restored red/green through the real resolver/controller, direct EEXIST
refusal, real concurrent {0,1} with loser-local diagnostics, complete worker
records, bounded negative lifecycle controls, actual observed ownership and
sentinel preservation, checked missing/broken tooling, and path/umask proofs.
Revalidate AC1–AC9, every house gate and four protected candidate-bound reviews.
Proposal status, availability probes and structural design validation are not
implementation or delivery verdicts. Only the operator accepts this record.
