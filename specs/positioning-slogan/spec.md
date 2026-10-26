# Adopt Swarmail's positioning slogan

Date: 2026-10-26. Status: draft design. Scope: one crate, documentation,
metadata and presentation strings, with their existing proofs.

## Authority and rationale

The operator's Positioning (2026-10-26) ruling chooses the copy. This design
implements that choice; it neither accepts a decision nor reopens branding.
The issue-0 evidence README and operator rulings matched their supplied
SHA256SUMS (README: `805ddb339cd258a2cd15ae24f5665fac0882d542a6b8edd5080f552cb65ee3a5`;
rulings: `09bf99c1d435f29dae334103cc331278e8f92db93cf955d1ab0fc014030f2f05`).
The packet has no prior-run entries. The requested run-local tasks directory
is absent; [the committed intake](intake.md) and verified packet provide the
framing. Both simplicity and robustness panel positions were read.

Visitors, package consumers and programs reading machine documentation
should encounter the same positioning: Swarmail serves programs waiting on
mail. Unmeasured whole-product superiority distracts from that concrete
purpose. Benchmark-backed figures remain evidence of their measured scope.

The exact full slogan is:

> Most mail mocks are built for a human looking at a web UI. Swarmail is built for a program waiting on mail.

The standalone short form is:

> Built for a program waiting on mail.

The llms.txt summary prefix is:

> An SMTP mail mock built for a program waiting on mail, not a human at a web UI:

The exact Cargo and chart description is:

> An SMTP mail mock built for a program waiting on mail: lossless bursts, per-test inboxes, wait_for assertions, chaos injection, MCP built in.

The motto remains: **The swarm never drops a letter.**

## Chief's rulings on the panel

- Adopt simplicity's literal edits and existing test suites. Reject shared
  slogan registries, generators, runtime substitution, new dependencies and
  a general banned-word framework. The accepted cost is duplicated static
  copy and a few independent expected strings in tests.
- Adopt robustness's complete current-description sweep and independent
  assertions on served content. Reject a five-file-only edit and equality
  to source as the sole proof: both can leave obsolete or consistently wrong
  wording in shipped surfaces. The cost is additional Rust string-literal
  edits and focused HTTP/help/UI assertions, without behavior changes.
- Resolve the hero detail in favor of robustness's preservation of supporting
  explanation: make `header#top p.tagline` the exact full slogan, and retain
  the old explanatory paragraph as adjacent supporting copy. Simplicity's
  replacement-only approach loses here; preserving the paragraph costs one
  extra paragraph and inspection of wrapping, but avoids deleting useful
  product explanation. Retain the existing H1 and measured figures.
- Reconcile semantic metadata proof with simplicity: use Cargo's effective
  package description and the existing chart verifier's Python/PyYAML stack.
  The chart copy proof must be runnable locally without Helm and invoked by
  normal chart verification. Do not impose today's copy on historical chart
  fixtures or change the generic metadata output contract. The cost is a
  narrowly scoped current-chart check and its refusal proof.
- Adopt robustness's failure analysis as a scope boundary, not new runtime
  work. Reject outboxes, new storage/concurrency machinery, certificate
  renewal, broader guarantees and comparative benchmarks. The slogan does
  not promise exactly-once mail, indefinite retention, durable webhooks,
  memory-only restart survival, unlimited sessions or admission of rejected
  mail. Existing qualifications remain intact.

## Contract classification

This is a copy change, not a semantic API, schema, protocol or guarantee
change. No new decision document is warranted. Accepted authorities remain
[0001](../../docs/decisions/0001-synchronous-insert-lossless.md),
[0002](../../docs/decisions/0002-awaits-subscribe-before-checking.md),
[0003](../../docs/decisions/0003-oldest-only-pruning.md),
[0004](../../docs/decisions/0004-per-test-inboxes-via-smtp-auth.md) and
[0005](../../docs/decisions/0005-extraction-at-ingest.md), with their accepted
amendments. No ruling or historical specification is rewritten. Presentation
strings in Rust and OpenAPI's informational description are in scope; their
control flow, schemas, status codes and routing are not.

## Acceptance Criteria

1. **README placement and preservation.** The first nonblank line after
   `# Swarmail` is the exact full slogan wrapped in `**`, before badges.
   The existing four-line paragraph beginning `**Email infrastructure for
   tests and agents.** One Rust binary:` remains byte-for-byte unchanged
   and follows in the introduction. Existing badges may remain between the
   slogan and paragraph. `*The swarm never drops a letter.*` remains.
   Raw-text assertions reject missing, unbolded, moved or altered slogans.
2. **Visible landing hero.** The text of `header#top p.tagline`, normalized
   only for HTML markup and whitespace, equals the full slogan. A copy only
   in metadata or elsewhere on the page fails. Preserve the original hero
   explanation as adjacent supporting copy, H1, measurement figures,
   benchmark citations and qualifications; retain natural wrapping.
3. **Machine summary.** The opening llms.txt blockquote starts with the
   exact summary prefix above, followed by the existing single-binary,
   SMTP/REST/MCP/UI explanation and immediate-queryability guarantee.
   The ruling's ellipsis is a continuation instruction, not literal copy.
   Real HTTP `/llms.txt` returns the complete source asset and independently
   satisfies the summary assertion.
4. **Effective package metadata.** Cargo's package description and the
   YAML-decoded Chart.yaml description equal the exact package sentence,
   including punctuation. Folded YAML is allowed. Bump chart version from
   `0.1.2` to `0.1.3`; preserve Cargo version, chart appVersion and image
   annotation coherence. Local proofs inspect effective values, not only
   raw substrings. A wrong description or unbumped chart fails.
5. **Current claims.** Remove product-wide `fastest` claims case-insensitively
   from all current descriptions, including Cargo, llms.txt, OpenAPI
   `info.description`, CLI about, UI header and the main module summary.
   Use the ruled standalone short form where suitable, preserving OpenAPI's
   existing protocol and synchronous-queryability explanation. Preserve
   measured throughput claims with their citations and the unchanged README
   introduction's `faster` wording. Historical specs, decision quotations and
   rejection assertions are not current product claims and are not rewritten.
6. **Independent, exercised proofs.** Extend existing doc-claim, served
   llms.txt/OpenAPI, CLI/UI and package/chart proofs. Preserve full parsed
   served-OpenAPI equality. Expected ruled words must be independent of the
   asset under test. Wrong wording, README placement, hero-only-outside-copy,
   old package/chart descriptions and reintroduced current superlatives each
   make their owning proof fail; focused disposable mutations or small
   fixture tests demonstrate sensitivity, with all candidate bytes restored.
7. **Scope preservation.** The implementation diff contains only current
   documentation, metadata, display/help strings and their proofs. No runtime
   behavior, dependency, license allowance, coverage exclusion, frozen
   guarantee, extraction path, recipe route or gate strength changes. The
   existing full regression suite remains green.
8. **Local delivery proof.** On committed candidate work run every local
   gate from [house rules](../../docs/house-rules.md#gates--part-of-every-change)
   using the digest-bound candidate engine and pinned proof scripts; exact
   coverage must miss zero lines, branches and functions and its selftest
   must pass. Also check shim integrity, native-tls absence, design selftest
   and delivery proof as specified in intake. Record actual failures without
   calling implementation complete. The design phase commits only this spec,
   [plan](plan.md) and [tasks](tasks.md); its successful structural check is
   not implementation acceptance.

## Proof ownership

Local copy, metadata and real-server assertions prove this feature. CI owns
macOS, Helm lint/render, event-baseline chart history and its selftest,
Docker smoke, network RustSec, privileged port-80 and tag-only release proofs
as inventoried in house rules. The operator owns live settings, sealing and
release acts. These are not new local acceptance criteria. No network,
namespace isolation, GitHub/CI/journal read, sibling worktree, push, merge,
tag, publication, issue closure or decision acceptance is part of this work.
