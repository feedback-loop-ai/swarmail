# Tracked ruling links — issue #243

## Feature and rationale

Readers of a clean checkout must be able to follow the operator authority
behind the eval contracts. Today tracked Markdown links into delivery-only
`.forge/` packets; their presence inside a delivery masks the failure in
`sequential_contracts_and_decision_links_are_honest`. Give the complete supplied
rulings one tracked home, repair all tracked Markdown citations into `.forge/`,
and make the existing eval boundary suite refuse that dependency repository-wide.

Authority: the checksum-verified issue243 packet and its 2026-10-26 #243 ruling;
[intake framing](../../docs/tasks/issue-243-tracked-rulings.md) records the
verification and ownership. The source ruling digest is
`03a4460d368cc77d7c2c927cd40aff6bab7568fbee5a5ca5b8369f5643c54534`.
The README digest is
`518cfc062c7dabbf66e625ed57c0bb009fa9ef115c30419382bba4fba92dacf6`.
The packet has no prior-run outcomes. `.brokkr/state/tasks/` is absent;
the committed intake supplies the framing without consulting the journal.

This is a citation and regression-proof repair, not a semantic amendment.
No new decision is proposed: accepted [0055](../../docs/decisions/0055-agent-eval-harness.md)
changes only its authorized link destination; proposed
[0075](../../docs/decisions/0075-eval-privacy.md) and
[0076](../../docs/decisions/0076-sequential-eval-trials.md) retain their statuses.
Copying dated rulings accepts nothing further. Preserve 0001–0005, existing
eval contracts, dependencies, lockfiles, license policy and bundle routing.

## Panel synthesis and rulings

Read both run-local positions, `.brokkr/state/design/positions/simplicity.md`
and `.brokkr/state/design/positions/robustness.md`; neither is a tracked citation.

- Adopt both positions' byte-for-byte authority copy, whole tracked-document
  inventory, destination-only 0055 edit, honest historical provenance and
  isolated export proof. Reject a four-document-only or existence-only check:
  local ignored files must never make a prohibited citation pass.
- Choose simplicity's single owner, `evals/tests/boundary.rs`, over robustness's
  preferred root checker. The operator permits this owner. The losing option
  would provide immediate root-suite execution, but adds a second executable
  and wiring. The chosen cost is explicit separate eval verification until
  #201 supplies CI integration; root `cargo test` is not evidence for evals.
- Adopt robustness's NUL-safe inventory, fail-closed I/O, isolated Git fixtures,
  images, lexical normalization and percent-encoded local-path controls.
  Choose simplicity's bounded Markdown syntax over robustness's HTML/general
  rendering scope: inline links/images and reference definitions, with angle
  destinations and optional titles, are required. Raw HTML links and arbitrary
  renderer extensions are outside this checker. The cost is that this is a
  documentation convention check, not a universal rendered-link/security audit;
  do not advertise universal CommonMark support. No new parser dependency.
- Reject redirecting historical panel/intake citations indiscriminately to the
  rulings file. Remove unavailable hyperlinks with truthful historical wording,
  retaining path text as code where useful, or cite the tracked synthesis that
  actually records the adopted substance. The accepted cost is loss of access
  to raw historical panel transcripts; do not manufacture those transcripts.
- Reject extra ruling registries, migration frameworks, CI routes, production
  helpers and unrelated runtime repairs. No new SMTP, storage, extraction,
  webhook, trial-isolation or paid-baseline work belongs to this issue.

## Requirements

The tracked authority is `docs/rulings/epic-192.md`, exactly the complete packet
`operator-rulings.md`, including all three dates, the ledger, and the original
2026-10-26 epic #164 heading. Preserve historical observations as dated claims.
The ongoing checker must not depend on access to that source packet.

Enumerate all Git-tracked Markdown files using a checked NUL-delimited inventory,
including tracked files under dot-directories. Read candidate working bytes;
never recursively scan ignored evidence or use a fixed list of documents.
Reject Git discovery, inventory, path-decoding or document-read failures rather
than silently producing an empty successful scan. An empty Markdown inventory
also refuses. Diagnostics identify the source path and original destination.

Recognize inline links/images and reference definitions (including definitions
used by full, collapsed or shortcut references). Support angle-delimited
Markdown destinations, optional titles, escaped delimiters and balanced
parentheses in destinations. Exclude inline code and backtick/tilde fenced code
examples; ordinary prose path mentions are allowed. This bounded grammar must
be stated beside the checker and covered by fixtures.

For local destinations, separate query/fragment suffixes, decode valid percent
escapes once, then resolve dot components lexically relative to the document or
the repository root for root-relative paths. Refuse entry into the repository
root's `.forge` directory, including `.forge` itself; over-parented relative
spellings aimed at that directory must also refuse, not escape the policy by
walking above the root. Do not canonicalize the filesystem or depend on target
existence. External scheme URLs, network URLs, fragment-only references and
names merely containing `.forge` are not packet dependencies. No remote fetch,
symlink-confinement policy or generalized broken-link audit is introduced.

## Acceptance Criteria

1. **AC1 — Durable authority.** Git tracks `docs/rulings/epic-192.md`; `cmp`
   against the checksum-verified issue243 ruling source succeeds. Its SHA-256
   is the source digest above. No ruling is omitted, added or reinterpreted.
2. **AC2 — Complete citation repair.** A Git-tracked Markdown inventory and
   the new checker find zero prohibited destinations. Every replacement local
   citation resolves to tracked content supporting its claim; unavailable
   panel/intake evidence is described honestly without a hyperlink into
   `.forge/`. Review the inventory, not only the issue's examples.
3. **AC3 — Accepted-history preservation.** Compare 0055 to the implementation
   base: replacing its one old destination with the new tracked relative path
   accounts for the entire byte diff. Its label, status and prose are unchanged;
   0075/0076 and their index statuses remain proposed. Existing boundary
   assertions and standalone unpublished eval workspace/lock contracts remain.
4. **AC4 — Executable refusal.** The boundary command scans all tracked Markdown.
   Isolated Git fixtures prove a tracked file outside the current four documents
   (including a filename with spaces) linking to an existing ignored `.forge/`
   target fails with path/destination diagnostics, and repair makes it pass.
   Prove inline/image/reference, angle/title, escaped/balanced destination,
   relative/root-relative/dot/over-parented/percent-encoded cases; allowed
   tracked citations, external URLs, fragments, prose and code; and explicit
   refusal on Git/inventory/read failures. Never mutate the delivery index from
   parallel tests. Preserve all pre-existing boundary assertions.
5. **AC5 — No-packet proof.** Export committed HEAD to an isolated directory,
   create its own Git inventory, assert `.forge/epic192` is absent and run
   `cargo test --manifest-path evals/Cargo.toml --locked --test boundary`
   successfully there. Also run the actual boundary command with a temporarily
   tracked prohibited fixture in that export, require failure and the expected
   diagnostic, restore the fixture/index and require success. Keep the live
   evidence packet intact; retain commit identity and command statuses.
6. **AC6 — Local delivery proof.** On the committed implementation, pass every
   local gate in [the house inventory](../../docs/house-rules.md#gates--part-of-every-change),
   including exact coverage and its selftest, using the verified candidate-local
   engine. Pass independent eval formatting, locked Clippy and tests, pinned
   shim/design/delivery proofs and fail-closed native-tls absence. Retain current
   receipts; do not substitute this design's structural validation for feature
   implementation or its required gates.

## Proof ownership

CI retains its charter-owned network, platform, privileged and publication
proofs; #201 owns macOS eval evidence and eval CI integration. The operator owns
live settings, sealing and decision acceptance. These are not additional local
acceptance criteria. No GitHub, CI service, journal or sibling-worktree access,
network installation, paid trial, namespace proof, push, merge, tag, publication
or issue closure is required or authorized here.
