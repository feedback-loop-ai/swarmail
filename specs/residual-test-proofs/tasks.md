# Ordered tasks: residual test proofs

Implementation tasks below have fresh local proof. Independent verify/review and
final delivery acceptance remain separate. The chief's design-baseline gates are
not this implementation's evidence. Candidate identity, platform, command,
selected-test count, exit, diagnostic and scoped diffs are recorded under ignored
`.forge/epic192/issue191/implementation-235/`.

The identical focused command passed, rejected the compiling split-lock mutant
at the runtime assertion (two real acquisitions versus one), and passed again
after byte-identical restoration. Every local house gate exited zero; coverage
is exact: 13,871/13,871 lines, 782/782 branches, 1,512/1,512 functions. Its selftest
and fail-closed native-tls absence proof passed. Shipping statements and the
reviewed 23.1–23.4 files are unchanged. The commit records this proof closure;
macOS and other CI-only checks remain CI-owned.

- [x] T1 — Confirm the 23.5 boundary (AC1–AC5/AC7). Proof: compare against `68b4ef8`; Dockerfile, `tests/common/mod.rs`, `tests/binary.rs` and `tests/webhook.rs` remain byte-identical, and audit the actual scrape/renderer path. Preserve attribution against `c3d124d` and the reviewed helper/socket contracts; do not reopen their implementation.
- [x] T2 — Add scrape-local acquisition observation (AC6/AC7). Proof: inspect entirely cfg(test)-only owner support, real lock/guard delegation, unchanged Registry alias/constructors and absence from non-test expansion. The test in T3 must install the seam; both present and absent paths must execute under T7's exact coverage proof. Every scrape acquisition uses the observed path, with no unobserved alias/Deref escape.
- [x] T3 — Replace the source oracle with the runtime assertion (AC6). Proof: remove source extraction, anchors and copied literal; run `cargo test --locked --lib api::tests::cumulative_scrape_uses_one_registry_snapshot -- --exact`, requiring exactly one passing test, exact six-registration/overflow output and exactly one observed acquisition after setup. Exercise observer absence and installation consumption; zero observations fail.
- [x] T4 — Prove the compiling split-lock negative control (AC6). Proof: save repaired bytes/hash and arrange unconditional cleanup first; change only shared scrape access to sum/drop/relock/render through the unchanged observer. T3's identical command must compile, select one test and fail specifically at runtime with two acquisitions versus one. Record precise mutation diff and failure log; test/seam remain unchanged and source/build/timeout failures do not count.
- [x] T5 — Restore and prove green again (AC6/AC7). Proof: restore exact saved bytes even if T4 failed, verify byte equality and rerun T3's identical command with one passing test. Inspect final diff/expansion for unchanged shipping statements and protected files. Never stage or commit the mutant; failed T4 remains incomplete despite restoration.
- [x] T6 — Preserve real-protocol consumers (AC1–AC5/AC7). Proof: run each exact binary/webhook test command and the six consumer suites in plan.md; the focused commands each select one test. Existing bounded refusal, held reservation, SMTP/HTTP, final ledger, baselines and redaction assertions pass unchanged. Linux local; macOS remains CI-owned.
- [x] T7 — Execute every local house gate on restored source (AC8). Proof: every command in the charter's sole inventory exits successfully, with digest-verified candidate-local `.forge/bin/brokkr`, locked builds, zero missed lines/branches/functions, successful coverage exit and selftest, publish/seal proofs and licenses. Validate shim identities and run fail-closed native-tls absence as in plan.md. Save current receipts; historical passes, printed summaries before timeout and CI-only checks are not local success.
- [x] T8 — Commit and hand off the scoped repair (AC6–AC8). Proof: unsigned scoped implementation commit, clean intended worktree, exact-restoration evidence and current acceptance receipts, with only this design directory and permitted `src/api.rs` regions changed relative to design-entry. Reconcile task status only with executed proof, preserve independent 0058 review/security hold, and record unresolved failures honestly. No push, merge, tag, publication, issue closure or decision acceptance.
