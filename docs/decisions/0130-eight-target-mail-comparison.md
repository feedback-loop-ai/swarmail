# 0130 — Eight comparison targets: source lock v4 and protocol v3

- **Status:** proposed
- **Date:** 2026-11-07
- **Issue:** #340, part of #163 (Epic #164)
- **Specification:** [Eight-target mail comparison](../../specs/mail-compare-eight-targets/spec.md)
- **Amends:** accepted [0091](0091-fair-mail-comparison.md) §4's target count ("three actual server binaries", "fixed targets") and accepted [0093](0093-source-mail-comparison.md) §1's annotated ref → tag → commit chain, by addition only, if accepted.
- **Applies without changing:** accepted [0092](0092-official-mail-comparison.md) §1–§6, [0093](0093-source-mail-comparison.md) §2–§7, 0001–0005, 0020, 0045 §8, 0055 (evals outside the coverage denominator), 0056 and the dependency policy.
- **Number:** 0130, reserved by the dispatcher for this story. Only the operator accepts.

## Context

On 2026-11-07 the operator asked to bench throughput and wait latency against
MailSlurper, smtp4dev, Inbucket and GreenMail, as well as Mailpit and MailHog.
Issue #340 states the ruling as "#163 approved and widened". That phrase is the
issue's paraphrase. This record cites the issue and does not quote the operator
directly.

The driver's targets are a closed list of three (`TargetKind::ALL`). 0091 §4
fixes "three actual server binaries", and its synthesis accepts "fixed targets"
as the cost. Source lock v3 (0093 §1) can bind only an annotated tag: a captured
ref that names a tag object, which names a commit. MailSlurper, smtp4dev and
Inbucket tag their releases lightweight, so the ref names the commit directly
and no tag object exists. GreenMail's REST API cannot read messages. GreenMail
also needs a Java runtime, which no current pin can express. The Alkemio
patched MailSlurper is an OCI image with no release and no upstream tag.

Protocol v2 was frozen and collected on 2026-10-30. That collection stays as
recorded (0091 §1, 0092 §2, 0093 §3).

## Proposed ruling

1. **Eight targets, still fixed.** The closed target list grows to eight:
   `swarmail`, `mailpit`, `mailhog`, `mailslurper`, `smtp4dev`, `inbucket`,
   `greenmail` and `alkemio-mailslurper`. Each target has a typed provenance:
   the candidate, an official upstream repository, or a third-party image.
   Target sets are scoped by version. Protocol v2 and lock v3 accept exactly
   the original three. Protocol v3 and lock v4 require all eight. Every
   historical record keeps the meaning it was written with. The list stays
   closed, with no trait, plugin or registry. 0091's "fixed targets" cost
   stands at eight.
2. **Source lock v4** is lock v3 plus fields that only version 4 may carry.
   Version 3 refuses each of them, and version 4 requires those the pin's
   kind needs. No v3 field changes meaning.
   - A tagged pin states `ref_kind`. `annotated` keeps 0093 §1's chain exactly.
     For `lightweight`, the captured ref must name the commit
     (`object.type == "commit"`). The pin carries no tag object, no tag
     signature and no `git_tag` document. A lightweight tag's signature is
     **absent**, not "unsigned": there is no object that could be signed. A
     lightweight pin therefore carries weaker source evidence than an
     annotated one, and the report says so.
   - The release tag is evidence, not a pattern. It must equal the captured
     ref. Its shape is a version from a closed set: `v1.2.3`, `release-1.2.3`
     or `1.2.3`. `latest`, branch names and anything else are refused.
   - An `image` source binds a third-party OCI image. It records the reference,
     index digest, platform manifest digest, layer digest and executable
     member. The index and manifest bytes are captured documents, and their
     sha256 is their digest (content addressing). The executable is extracted
     once into its own empty directory, its digest basis is
     `locally_computed`, and it is rechecked at every launch. An image pin
     never has an observed `tagged_source` or `binary_build_provenance`.
   - A `runtime` pin binds an interpreter that the target needs but that is not
     a target itself. GreenMail's JRE is the only one. The pin records the
     archive digest, the code-bearing members checked at launch (at least the
     `java` launcher and `libjvm.so`), the version output and the license.
   - A launch states its listeners as a closed set of roles: `smtp`, `http`,
     `pop3`, `imap` and `ui`, each at most once. Every listener is a reserved
     loopback port. After readiness, the owned group must listen on **exactly**
     the declared addresses. A listener on an undeclared address, such as a
     tool's fixed default port, refuses the launch. Cleanup proves every
     declared listener closed. A launch may also state an environment (with
     names the launcher owns refused) and a config file rendered into the
     owned scratch.
   - A declared competitor whose bytes are not available may stand as a
     **pending** entry. Such an entry records its byte-independent launch
     facts and a reason. It is valid structure, never launches, and refuses
     collection by name.
3. **Protocol v3** keeps v2's shape, workloads, corpus, budgets, resource
   profile and statistics unchanged. It adds three things: the eight targets;
   a per-target `capabilities` record with fixed keys (`server_search`,
   `native_wait`, `retention_cap`, `raw_source`); and a per-product `scope`
   (`official_artifact`, or `third_party_image` for Alkemio). Each capability
   is `supported` with a citation, `not_supported` with a citation, or
   `not_inspected`. A citation must name a document or reviewed source in that
   target's lock pin. A capability nobody reviewed is `not_inspected` and is
   never assumed.
   - Every row states its storage and cap as launched. The cap is either the
     value set explicitly at launch or `no_cap_observed`. No default retention
     may apply silently: the launch sets the cap, and the freeze-time overflow
     refusal covers it.
   - Protocol v3 has the id `fair-mail-comparison-v3` and supersedes v2 with a
     change reason that cites this record. It is a separate run with a separate
     denominator. No table, statistic or claim merges v2 and v3 cells.
   - Protocol v2 pairs only with lock v3, and protocol v3 only with lock v4.
   - Protocol v3 freezes only when every fact is observed. Until then it is a
     draft with named `unresolved` entries.
4. **Waits** stay on 0093 §4's common client: immediate scan, then a 100 ms
   post-scan poll. No stream client is added.
   - A target without a verified server search is scanned completely and
     filtered on the client by token. The method is labelled per row.
   - GreenMail is read over POP3: a fresh session per scan, no `DELE` and no
     cache between scans. The POP3 client is written in the driver on the tokio
     already in the tree, with no new crate. It honours the request deadline,
     byte bounds and cancellation. IMAP is recorded as not implemented.
5. **Native runs, for their own reason.** Every target runs as a native child
   of the driver, GreenMail on its pinned JRE and the Alkemio row from its
   extracted binary. The reason is evidence, not 0092 §6 as cited. 0092 §6
   rules on lifecycle ownership, not on the choice of runtime. The reason has
   three parts:
   1. Only the driver's own child lets the one owner of 0092 §6 and 0093 §7
      corroborate binds through the process group, reap the leader and observe
      cessation. A container is the Docker daemon's child, so owning it would
      need the second supervisor that 0092's synthesis rejects.
   2. A pinned JRE and a digested jar can be rechecked at every launch. A
      mutable image base cannot.
   3. All v2 rows ran natively, so v3 keeps one runtime shape across rows.
   The cost: the Alkemio row measures the extracted executable, not the image
   as Alkemio runs it, and its label says so.
6. **Licenses.** Measured products keep the bench's permissive list. The JRE is
   neither linked nor measured, but its terms still have to be checkable. Its
   license (OpenJDK: GPL-2.0 with the Classpath exception) is checked against
   a separate closed runtime list that holds exactly that expression. The list
   is named here so that the operator can refuse it. If the operator refuses,
   GreenMail stays pending. `deny.toml` and the dependency allowlist do not
   change. If the Alkemio image's captured terms fall outside the permissive
   list, the row stays pending and is reported. The list is not widened.
7. **Claims.** For any competitor, v3 claims throughput and observed
   visibility within the declared deadline, and nothing more. It claims no
   durability, no accept==stored and no equivalence. The Alkemio row is never
   counted as an official competitor row.

## Synthesis and costs

The simplicity position wins on shape:
- the closed enum with match arms;
- one lock type and one protocol type, gated by version;
- v2's workloads and corpus copied unchanged, so v2 and v3 differ only in
  targets and per-target facts;
- POP3 instead of IMAP;
- no Docker, no stream clients, no MCP adapters for the new tools.

The robustness position loses on two points. Representable but invalid field
combinations are refused by tested validators rather than by the type system.
The measured corpus also gains no hostile edge kind; that fidelity is proved in
decoder tests instead.

The robustness position wins on evidence:
- version-scoped target sets, version pairing, and historical bytes that stay
  launchable;
- the release tag held as evidence;
- the typed third-party provenance;
- N listeners with an exact listening-set proof and per-listener cleanup;
- per-target readiness;
- an async, cancellable POP3 reader;
- caps frozen as launched;
- decoder tests where the gates can see them.

That costs more validators, negative tests and lifecycle tests than the small
design.

## Proof and delivery

The linked spec's acceptance criteria and paired tasks are normative.
Implementation adds this record's index row and dated pointer addenda to 0091
§4 and 0093 §1, with no body edits. It moves the protocol v2 and lock v3 bytes
to `docs/evals/history/` with their sha256 unchanged. It runs every house gate
`--locked`. The operator holds the PR because it changes `scripts/`. No paid
work, collection, push, merge, publication, tag, issue closure or acceptance
is authorized.
