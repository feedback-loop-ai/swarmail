# Implementation plan — STARTTLS fixture recovery

Implements [spec.md](spec.md); ordered proof obligations are in
[tasks.md](tasks.md). No semantic amendment is proposed.

## Technical context

One Rust crate; the existing Tokio TcpSocket, rustls fixture and real SMTP
listener already provide everything required. No package or runtime interface
is added. The repository has a spec template but no plan template; this plan
supplies the required context, constitution check, structure, research, data
model, contracts and quickstart directly. Use the isolated candidate's build
output and workspace tool; the engine owns its journal.

## Constitution check

Apply accepted 0020 rule 1: keep the branch-stable test inline and measured.
No cold path or new seam is introduced by changing the connection expression.
Preserve accepted 0046 and all six #189 criteria as detailed in spec R4–R5.
Storage, watcher ordering, pruning, ingest extraction, protocol semantics,
listener task ownership and gate policies are unchanged. Preserve exact
coverage and the dependency allowlist. Accepted 0056 governs unsigned author
commits; 0057 governs pinned engine identity, engine-lane review and providers.
The prior REVIEW-RESIDUAL-UNSAFE stop remains evidence requiring fresh review,
not a journal entry this seat can supersede.

## Research and resolved questions

Read both current positions, both intake records, all four retained review
receipts, the decision index, placement/session/route authorities, the retained
cleanup spec and the live fixture. The code configures a 1024-byte receive
buffer and then connects a different stream. The previous sibling fixture at
`548982a^:src/smtp/protocol_edges_tests.rs` used `socket.connect(addr)`.
Restoring that owner requires neither a new abstraction nor the old harness.

The current reviewers identify one shared defect, not a new production
vulnerability. Their old gate/mutation receipts cannot validate a changed
fixture. Structural inspection proves the configured socket is used;
behavioral mutation proves it still distinguishes bounded failed-session
drop from an unbounded shutdown. The kernel need not report an exact
1024-byte effective allocation. No unresolved API or data-model question
blocks this bounded repair.

## Project structure and touched files

| Path | Role and allowed change |
|---|---|
| `src/smtp.rs` | Only the stalled peer's connect expression inside the existing inline test changes. |
| `specs/starttls-fixture-recovery/{spec,plan,tasks}.md` | This design commit's exact tracked footprint; describes recovery obligations. |
| `specs/smtp-pop3-tidy/spec.md` | Read-only retained AC1–AC8 authority. |
| `src/pop3.rs`, `src/api.rs`, `src/store.rs`, `tests/{smtp_edges,starttls,pop3}.rs` | Preservation-test sources; no new edits authorized. |
| `.forge/evidence/starttls-fixture-recovery/` | Ignored implementation-owned commands, patches, logs, exits and receipt index. |
| `target/coverage/` | Generated exact coverage reports, referenced by receipts. |

Do not edit bundles, gate shims, accepted decisions, prior specs or historical
review records. No new source file, harness, dependency or feature flag.

## Implementation approach

1. Reconfirm baseline ancestry, clean tracked state, current fixture and all
   four historical member facts. Locate any historical survival mutation
   evidence. Check the candidate-local engine digest against tooling.json.
2. Replace only the stalled connection's `TcpStream::connect(addr)` with
   `socket.connect(addr)` under its existing timeout. Keep the fresh client,
   TLS trust, flood, deadlines, completion observer and cleanup byte-for-byte.
   Run the exact targeted test and inspect the diff.
3. On a reversible working copy or with a saved exact patch, restore unbounded
   failed-session TLS shutdown in the existing close path. Run the corrected
   test unchanged; save a bounded runtime failure, then restore production
   bytes and prove green. Ensure the filter executes the intended test.
   Do not commit the mutant. Unexpected greens/failures require diagnosis;
   do not tune fixture parameters or move test bodies to get a desired verdict.
4. Rerun preservation suites. For retained SMTP/POP3 abort-on-accept-error
   mutants, verify inspectable historical receipts against their unchanged
   source and commands; if unavailable, reproduce each separately with exact
   patches, behavioral failures and restored passing runs. This closes the
   evidence gap without reopening completed production changes.
5. Verify that the only source difference from 548982a is the socket method
   call. Run the complete authoritative gate inventory and auxiliary pinned
   checks on the fully restored final implementation. Bind receipts to the
   candidate commit/tree and name pending CI evidence. Hand off to protected
   independent review with the old unsafe facts and fresh resolution evidence.

## Data model and contracts

No schema, persisted state, protocol message or public interface changes.
The existing TcpSocket owns configuration until connect consumes it; the
resulting TcpStream becomes the TLS transport. The unread half remains alive
through task completion observation and fresh greeting, independently of the
later fresh connection. This is a fixture ownership requirement, not a new
runtime buffer contract.

Failed SMTP cleanup still drops without more peer I/O; clean cleanup has one
bounded window and STARTTLS its dedicated handshake deadline. POP3 positive
write progress can retain a slot indefinitely; no minimum rate or whole-reply
lifetime is introduced. Store commit/reply ambiguity, oldest-only retention,
best-effort webhook loss and memory-only restart behavior remain unchanged.
A 250 still means queryable storage, not permanent retention or exactly-once
client delivery.

## Quickstart and proof receipts

Verify `.forge/bin/brokkr` against `bundles/self/tooling.json` before using it;
never substitute a sibling executable or a host PATH engine. Targeted commands:

```bash
cargo test --workspace --locked smtp::tests::starttls_stalled_reply_releases_slot
cargo test --locked --lib smtp::tests
cargo test --locked --lib pop3::tests
cargo test --locked --lib a_degraded_smtp_listener_answers_503_naming_it_on_the_wire
cargo test --locked --lib store::persist_tests
cargo test --locked --test smtp_edges --test starttls --test pop3
```

Each mutation receipt records baseline/candidate identity, exact patch,
command/filter, executed test count, exit, behavioral diagnostic, elapsed
bound evidence, restoration check and green rerun. Keep an index under the
ignored evidence directory with real paths; never invent missing prior
receipts. Temporary mutations must be absent before final gates.

Execute every gate in [the single house inventory](../../docs/house-rules.md#gates--part-of-every-change),
plus fail-closed native-tls absence and configured design/delivery-boundary
and recipe proofs. Use pinned scripts and verify shim identities; the
hermetic verifier installs nothing. Exact coverage must include the inline
body with zero missed lines/branches/functions. Record statuses honestly;
missing execution is not a pass. All corrected-candidate gates remain
required even if the design-only baseline passed them.

CI-owned proofs remain separately identified until available: chart contract,
bump history/selftest, real-Docker persistence/context/volume smoke, network
RustSec audit, real port-80 exchange and tag-only release-rate-5k where
applicable. Do not run a publication to obtain evidence. Fresh four-lens
review must address the prior typed residual; the author cannot grant security
clearance or change the engine-selected route.

## Risks

| Risk | Mitigation and accepted limitation |
|---|---|
| A green test still uses a default-buffer peer. | Inspect exact configuration-to-connect ownership; require both structure and behavior. |
| OS buffer rounding or scheduling changes the stall. | Preserve real constrained socket, existing flood, bounds and held peer; require a behavioral mutation failure. No byte-exact kernel claim or silent tuning. |
| Failure or timeout leaves tasks alive or closes the peer too early. | Preserve existing JoinSet ownership, scenario fence, open unread half through admission, and abort/join before final result assertions. |
| Inline changes introduce a cold path or vacuous measured branch. | Keep one-expression scope; inspect exact report. If diagnosis requires a new cold path, return the concrete conflict under 0020 instead of excluding coverage. |
| Missing historical receipts are mistaken for fresh proof. | Record fresh TLS red/restored-green; locate and verify or reproduce both survival mutation receipts. Bind logs to the real candidate. |
| Low severity is treated as clearance. | Carry all four old facts into full protected review; require explicit fresh resolution through policy. Accept the full lane's latency. |
| Scope expands into runtime or normative changes. | Stop for reframing on evidence of another defect; preserve all six findings, accepted contracts and existing failure limits. A contradictory change requires operator acceptance of an amendment. |
| Baseline/design checks are reported as delivery completion. | Separate design, implementation, local gates, independent review and CI receipts; keep tasks unchecked until their actual proof exists. |
