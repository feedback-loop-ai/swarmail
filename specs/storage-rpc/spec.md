# Feature specification — storage and RPC boundaries

- **Feature:** Epic #120, batch 2: #129, #130, #132, #133.
- **Branch:** `epic120-storage-rpc`.
- **Date:** 2026-10-18.
- **Status:** design draft; implementation and operator acceptance remain separate.
- **Base inspected:** `fd1d604`, the landed batch-1 change after `2a325c9`.
- **Semantic decision:** [0047 — Storage and RPC boundaries](../../docs/decisions/0047-storage-rpc.md), **proposed**.

Decision: 0047 (proposed; relative link above).

## Framing and rationale

One database reached through two names must not admit two servers with
independent memory views and WAL files. Overflow accounting must not borrow
a real inbox's identity. Malformed JSON-RPC objects must receive errors, and
the bridge must connect to ordinary HTTP URLs and correlate local refusals.
These repairs protect acknowledged mail, trustworthy assertions and clients
waiting for replies; they introduce no new protocol capability.

The chief read both current panel positions and the full local snapshots of
epic #120 and issues #129/#130/#132/#133, including all evidence supplied in
those files. `.brokkr/state/tasks/` contains only older run12f/run12gc/run12gd
intakes, none for this batch. The current commission and issue snapshots
therefore supply this batch's framing. In particular, the commission
strengthens #129 beyond the issue's canonical-path suggestion: **hard links
must refuse too**. The batch-1 inbox rule already rejects blank/control
names; this design builds on it without changing the admitted name domain.

## User scenarios and requirements

1. **Storage ownership (#129, first priority):** an operator starting a
   second current server through either a symlink or hard link gets the
   named startup refusal before SQLite work, restore or listener readiness.
   The first owner continues serving; stopping it releases ownership.
2. **Metric attribution (#130):** an operator tracking a legal `_uncounted`
   inbox sees only its own registrations in that row, even after registry
   capacity is exhausted. Folded registrations have a distinct, exact sample.
3. **JSON-RPC validity (#132):** a client whose template loses its method
   receives an invalid-request error over HTTP and stdio. Valid id-less
   string-method notifications retain effectful silence.
4. **Bridge transport (#133):** a client using an HTTP URL without a port
   connects to port 80. Oversized parsed requests receive correlated local
   errors without sending the body or executing its effects.

## Architectural rulings

| Question | Adopted ruling | Rejected position and cost of the choice |
|---|---|---|
| File identity | Both panels' data-file identity lock, in addition to the existing sidecar; simplicity's unchanged sidecar spelling. | Reject canonicalization as either sufficient or an additional required layer. It cannot cover hard links and adds parent/missing-file cases. Cost: aliases may leave separate inert sidecars; old sidecar-only binaries through different aliases are outside the promise. |
| Creation and lifetime | Robustness's explicit Unix creation mode matching SQLite (0644 subject to umask), nontruncating open, and connection-before-guard destruction. | Reject implicit default mode that could broaden new database permissions. Cost: a small platform-specific creation setting and a permission regression. |
| Overflow | Simplicity's nonempty LF sentinel, impossible under the shared inbox rule, escaped by the existing encoder. | Reject robustness's new family/registry shape and any reservation of `_uncounted`. Cost: dashboards must select the escaped newline sample; the representation depends on the validator and encoder, pinned together by tests. |
| Scrape consistency | Robustness's one-lock snapshot for cumulative total and rows. | The current renderer receives a total computed under an earlier lock, so simply retaining it cannot promise a consistent snapshot. Cost: a small cumulative-rendering adjustment while retaining the existing map and other metric families. |
| Notifications | Both panels' single `reply_id` predicate shared by classification and dispatch. | Reject a handler-only fix; it leaves bridge reply obligations wrong. Cost: formerly silent malformed objects now emit errors. |
| Bridge oversize | Both panels' local refusal against the shared body limit, with shared server wording and envelope-shaped errors. | Reject robustness's additional upstream null-ID rewriting. Cost: a server/proxy with a smaller limit may still return an uncorrelated error; no version-skew guarantee is added. |
| Port-80 proof | Simplicity's real bridge/server exchange on port 80, supplemented by authority/header tests. | Reject robustness's helper/listener-only substitute for the explicit acceptance criterion. Cost: the proof needs an available port-80-capable environment; inability to bind is an unmet proof, never a skipped pass. |
| Test scale | Local registry fixture for exact cap/escaping proof, plus existing real protocol suites. | Reject thousands of HTTP waits merely to exercise a constant. Cost: cap arithmetic is proven at its owner rather than through a large global-state wire fixture. |

## Acceptance Criteria

- **AC1 — Alias admission (#129):** real second `serve` processes opening a
  held data file through a symlink and a hard link each exit nonzero with
  `the data file is already in use by another server: <path>`, without
  readiness. Test both acquisition orders. First-owner SMTP deliveries
  accepted with 250 remain queryable and survive a controlled restart;
  exercise oldest-only cap behavior without expecting evicted mail to survive.
- **AC2 — Ownership lifecycle (#129):** same-path refusal, symlinked-parent
  first creation, lock-before-SQLite/restore/bind, graceful release, immediate
  reuse after SIGKILL and reap, and failed-open release all hold. Existing
  sidecars are never truncated or unlinked. Raw SQLite inspection and
  `BEGIN IMMEDIATE` still work beside an owner, and the 5000 ms busy bound,
  WAL/FULL and commit-failure proof family remain unchanged. On Unix a new
  data file is no more permissive than 0644 subject to umask; existing modes
  and contents are retained. Every open/lock error refuses with its cause.
- **AC3 — Honest lock scope (#129):** README and the persistence module
  document symlink/hard-link refusal for cooperating current servers on
  supported local filesystems. They name live namespace replacement,
  sidecar-only old binaries through other aliases, noncooperating writers,
  unproven single-file bind mounts and remote-filesystem lock semantics as
  outside the proven guarantee. Same-path old-binary sidecar exclusion stays.
  Keep one configured database pathname across restarts: ownership exclusion
  does not relocate crash-recovery WAL files between hard-link basenames.
- **AC4 — Disjoint overflow (#130):** the sentinel is exactly one LF byte,
  `"\n"` in Rust notation, and `validate_inbox` rejects it. `_uncounted` and
  the literal two-character backslash-n name remain valid. Their encoded
  labels differ from the overflow sample. Register `_uncounted` before
  filling a local 4096-name registry, overflow with at least three new names,
  then increment `_uncounted`: its row increases only for its own calls.
  There are at most 4096 real rows plus one overflow row; existing tracked
  names retain exact attribution, untracked names fold exactly, and the
  total equals the sum of all labeled rows in the same rendered snapshot.
  Names first seen after capacity do not gain individual rows, including
  `_uncounted` if first seen then; this preserves the existing bounded policy.
  Document the escaped `inbox="\n"` sample and dashboard migration.
- **AC5 — Invalid objects (#132):** HTTP `{}`, `{"foo":"boo"}` and
  `{"method":1}` each return 200/-32600/null ID.
  `[{"foo":"boo"},{"jsonrpc":"2.0","method":"x"}]` returns exactly
  one array element, the invalid-request error. Sixteen malformed objects
  yield sixteen errors; empty/over-16 batches keep whole-envelope refusal.
- **AC6 — Shared reply obligations (#132):** `notifications/initialized`
  without ID remains 202 with no body. Unknown string-method notifications
  remain silent; valid effectful notifications still execute. Explicit null
  IDs are answered. A real bridge writes exactly one -32600/null-ID line for
  `{"foo":"boo"}` and processes a following ping. Classification, dispatch
  and transport-failure shaping agree on malformed/null/duplicate/batch IDs.
- **AC7 — Dial authority (#133):** a real bridge using
  `--url http://127.0.0.1` answers ping against a real port-80 server. Omitted
  ports default to 80 for hostnames, IPv4 and bracketed IPv6; explicit ports
  are preserved, malformed supplied ports refuse, and Host retains the
  original authority spelling. Existing URL path and HTTP-only behavior stay.
- **AC8 — Local size refusal (#133):** 3 MiB and 40 MiB valid JSON request
  bodies with ID 7 each yield exactly one -32600 line with ID 7 and the
  server's size-refusal wording. An oversized admitted two-request batch
  yields one line containing exactly two ordered errors. Preserve duplicate,
  string and null IDs and malformed-member null positions; omit notification
  positions. An oversized notification-only envelope is silent with no
  effect. Empty/over-cap arrays get one null-ID envelope refusal. No oversized
  parsed body is dialed or posted; a following valid request succeeds.
- **AC9 — Boundary and recovery (#133):** compare the exact UTF-8 byte
  length of the trimmed body that would be POSTed, excluding the line
  delimiter, against 2,097,152 bytes. Equality is admitted; one byte above
  is refused, including multibyte fixtures. Invalid UTF-8/JSON still gets
  local -32700, blank lines stay silent and EOF drains admitted work.
  Correct stale scalar-silence comments. Document that holding/parsing stdin
  remains unbounded and that slow stdout/version skew remain residuals.
- **AC10 — Evidence and preservation (all issues):** each behavior repair
  has a named regression demonstrably red on `2a325c9` behavior and green
  on the repair; compilation failure is not a red. Implementation commit
  bodies map each issue to its decisive tests. All house gates pass on the
  final implementation tree, exact coverage stays 100/100/100, and frozen
  insert/wait/prune/extraction guarantees and the listed protected features
  remain intact. This design draft alone claims none of those repairs proven.

## Scope limits

No dependency, allowlist, schema, inbox-validation, store-mutation, awaits
admission, chart or delivery-policy change. No lock service, retries, lock
stealing, SQLite EXCLUSIVE mode, new metric family, streaming stdin parser,
HTTPS client or broad JSON-RPC version/ID/params validation. Proposed 0047
records semantic amendments without changing any accepted decision's text.
