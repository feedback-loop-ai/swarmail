# 0129 — The agent-suite comparison across Swarmail, Mailpit, MailHog and MailSlurper

- **Status:** proposed
- **Date:** 2026-11-07
- **Scope:** epic #164, issue #339 (prerequisite of #163's paid comparison).
  Run `issue-339-agent-suite-comparison-e4de0540`, from `main` `c95386e`, on
  the `claude-careful` recipe. The dispatcher reserved the number 0129 for
  this story. No other number is used.
- **Specification:** [agent-suite comparison](../../specs/agent-suite-comparison/spec.md).
- **Authority:** the operator's ruling of 2026-11-07, quoted in #339:
  "compare with all that we have shims for". It widens #163 to four targets
  (Swarmail, Mailpit, MailHog, MailSlurper) and four agents (claude-opus,
  claude-sonnet, codex-sol, codex-luna), with no GLM. That ruling, not this
  record, overrides accepted 0091's rejection of a MailHog agent eval. The
  ruling is recorded as dated addenda on accepted
  [0091](0091-fair-mail-comparison.md) and
  [0092](0092-official-mail-comparison.md). This record proposes only the
  contracts that implement it.
- **Builds on:** accepted 0091 §1–§7, 0092 §1–§6 and 0093 §1, §4–§7. All
  three were accepted by the operator on 2026-11-02.
- **Preserves:** accepted 0001–0005 (no file under `src/` changes), 0043,
  0055, 0063, 0071, 0073, 0075, 0076, 0115, 0117, 0056 and every gate and
  dependency policy.

## Context

On `main`, `scripts/compare-mail.sh controls|evidence|measure` always stop at
`comparison-integration`. The eval runner can point an agent only at
Swarmail's HTTP `POST /mcp`. The Mailpit adapter speaks only stdio and
refuses JSON-RPC batches, which falls short of MCP 2025-03-26 (0092
addendum). The comparison row refuses MailHog by name. Source-lock v3 holds
exactly three targets, and its reader requires exactly one entry for each.
No product record, controls, contract evidence, identity record or cost
basis exists.

0092's context says that a thin adapter is a new public experiment
interface, and that it needs a proposal even when no frozen server
guarantee changes. This story adds two adapters, a batch contract, a new
lock version, an agent transport for adapters, a frozen product record and
new scopes for the wrapper's modes. Those are the contracts proposed here.
Each of them is compatible with every accepted ruling, so under 0045 §8 the
implementation may land while this record is still proposed.

## Proposed ruling

1. **Targets and applicability.** The agent-suite comparison has four
   targets: `swarmail-mcp`, `mailpit-mcp`, `mailhog-mcp` and
   `mailslurper-mcp`. A competitor cell is applicable only when the task
   needs one agent identity and no chaos. Every task in the hard tier
   (0117) qualifies. Every other catalogue cell for a competitor target is
   declared "not requested in this experiment" in the frozen product record,
   before any run. It is never fabricated, and the report makes no
   full-suite parity claim from it (0091 §3).
2. **Isolation is a fresh instance.** Each competitor trial gets a freshly
   launched backend, owned for its whole life, with sequential admission
   (0076). One mailbox per fresh instance is that backend's real mailbox
   model. The adapter or harness emulates no per-test inbox (0091 §3,
   0092 §3). The fixture's SMTP dialogue is the same on every target except
   for one frozen fact per target, `smtp_auth`. Its value is `plain` when
   the target accepts the fixture's AUTH PLAIN and `none` when it does not.
   With `none`, the dialogue leaves out the AUTH step and nothing else. Each
   asymmetry is stated in the product record, in the row settings and in
   `docs/evals/README.md`.
3. **Delivery semantics are declared, not exported.** For Swarmail the
   product record states that `250` implies queryable (0001) and that the
   native wait subscribes before it checks (0002). For each competitor it
   states that no such promise is made, and that the adapter's wait is the
   labelled common-client poll with the frozen 100 ms interval (0093 §4). No
   adapter emulates subscribe-before-check.
4. **One agent transport.** Every target is offered to agents as
   Streamable HTTP `POST /mcp`. For a competitor, the URL is a loopback
   bridge inside the eval runner process. The bridge forwards each body as
   one line to the adapter's stdio and routes the replies back by JSON-RPC
   id. The runner owns the adapter process, and the adapter owns the
   backend through the bench's existing launcher: lock digest check,
   corroborated binds and the cleanup worker. Only an adapter exit status of
   0, after a complete cleanup, is a cleanup receipt. Any other end leaves
   cleanup unknown, which stops admission (0092 §6, 0093 §7). Agent CLIs
   never spawn a mail adapter themselves. The extra hop is overhead, and
   the record states it (0091 §3).
5. **Adapters.** Mailpit, MailHog and MailSlurper each implement one small
   backend interface: tool schemas, call parsing and call execution. One
   generic half holds the bounds, admission, framing, poll wait, batches and
   the stdio server. Each tool maps onto one or more upstream operations.
   Each operation is source-verified against the bytes the lock binds, and
   each is general: no admin, delete, send or release operation, no task
   hint, no extraction, cache, index or prefetch, no inbox and no chaos
   (0092 §3). The upstream bodies pass through unchanged (0093 §6). Mailpit's
   tool list stays byte-identical, so protocol v2's frozen `tools_sha256`
   still holds.
6. **Batches (MCP 2025-03-26) match Swarmail's bound.** An array line of 1
   to 16 messages is answered with one array, in member order, holding
   one reply per request member. Notifications get no reply, and a batch of
   only notifications writes nothing. An empty array, or more than 16
   members, gets one error with id null and executes nothing. This is a
   refusal, never a clamp, as `src/mcp.rs` does it. `initialize` inside a
   batch gets a correlated `-32600`. A batch is one admission unit. Its
   members run one at a time in member order, each under the existing
   per-call budgets. The whole array is bounded by the reply bound: a
   member whose reply would cross that bound gets its own correlated
   `reply_too_large`. Duplicate ids are answered as sent. When the admission
   queue is full, every request member gets a correlated busy error and
   nothing runs. Members run one at a time in the adapters and concurrently
   in Swarmail. The product record states that difference.
7. **Source-lock v4.** The lock gains a MailSlurper entry, so its version
   becomes 4. The fields keep their meaning, plus one optional launch
   `config` template for a target that reads its ports from a file. The
   current v3 bytes move byte-identical to `docs/evals/history/`, and the
   v3 reader keeps reading them, as v1 and v2 do. The historical readers
   keep their own three-target expectation. Wire protocol v2 is unchanged:
   MailSlurper is an agent-comparison target and not a wire-bench target, and
   the wire `check` and `collect` still pass on protocol v2 with lock v4.
   MailSlurper's digest basis, licence, tagged source and build provenance
   are recorded as found. A missing upstream digest is locally computed,
   and unknown build provenance stays unresolved (0093 §1–§2).
8. **The frozen product record (0091 §1).**
   `docs/evals/agent-comparison.json` is committed before any run. It holds
   the ruling, the tier and its tasks, applicability per (task, target), one
   backend tuple per target, the four agent labels and their wrapper
   profiles, the trial count, matrix order, deadlines and seed policy, the
   planned 240 cells, the control plan, the statistics, the prompt header
   digests and the cost basis reference. The backend tuple holds the lock
   entry, the adapter name and `tools_sha256`, `smtp_auth`, the isolation,
   the connection, the wait method and the delivery semantics. Any change
   of method needs a new record id.
9. **The wrapper's modes.**
   - `controls` needs only the product record and the pinned executables,
     because controls run before any paid session (0091 §5). It produces
     the free oracle and null controls, the contract evidence, the identity
     record and the local receipts.
   - `evidence` and `measure` keep the fixed prerequisite order. After the
     stat chain they run one content verifier, which checks contents and
     cross-bindings and names the first failure. The `comparison-integration`
     refusal is removed.
   - `measure --dry-run` on genuine prerequisites verifies them, prints the
     full plan, launches nothing, opens no wrapper config and exits 0. A
     prerequisite that only CI or the operator can supply is printed as
     pending. The dry run stubs nothing.
   - Non-dry `measure` refuses a pending item, and it refuses unless
     `--authorized-product ID` equals the product record's id. No delivery
     run executes a paid selection.
10. **Rows and cost.** The schema-3 comparison row keeps its shape. Its
    coordinate gains the agent label, its target set becomes the four names
    above, and the MailHog refusal is removed under the ruling. No schema-3
    row exists yet, so nothing migrates. It does not embed the native v2 row
    and its Swarmail-specific target identity (0091 context). The cost basis
    is `unavailable`, with its reason: subscription logins carry no
    per-call price, and the wrappers keep no vendor output (0075, 0091 §7).
    An unavailable cost is never written as 0. Every started slot keeps its
    row, and a killed trial is a visible failure, never an absent one.

## Rejected, with the cost

- **Stdio MCP entries in the agent wrappers for the mail adapter.** The CLI
  would own the adapter and its backend, which breaks one cleanup owner, and
  the agent transport would differ by target. Cost of the bridge: one hop
  and one more owned component.
- **A second fixture, or per-backend scenario code.** 0091 §2 requires the
  existing fixture and the sole scorer. The issue's "fixture per backend"
  is served by the per-target backend tuple. Cost: `smtp_auth` is the only
  per-target fixture knob.
- **Adding MailSlurper to lock v3 in place.** That would change what v3
  means and break its exactly-three reader. Cost: one more archived lock and
  reader.
- **Per-member overflow errors for an over-cap batch.** That would be
  asymmetric with Swarmail's whole refusal.
- **Usage capture in the wrappers.** That needs a privacy review of vendor
  output for a basis that cannot be priced.
- **Embedding the full native v2 row in the comparison row.** It carries
  Swarmail-specific identity into a competitor row.
- **A hostile-body, TLS-expiry, webhook or chaos campaign across targets.**
  0091's synthesis rejects a new cross-product fault suite. None of these is
  on the hard-tier path, and no equivalence claim follows from leaving them
  out.

## Consequences

No file under `src/` changes, so the frozen guarantees and the exact lib
coverage denominator are untouched by construction. No dependency is added.
`scripts/compare-mail.sh` and `docs/evals/` change, so the operator's
assistant holds the PR for the operator. This record authorizes no paid
session, publication, push, merge, acceptance or comparison claim. The
operator confirms the planned 240 sessions separately.
