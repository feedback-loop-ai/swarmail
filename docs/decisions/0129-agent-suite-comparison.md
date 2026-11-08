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

## Addendum — 2026-11-07 (rerun `rerun-of-339-operator-direction--305a5aec`)

This addendum records what the pinned bytes changed in §7, under the same
proposed status. It weakens no accepted ruling.

- **§7's lock v4 carries more than a config template.** The MailSlurper entry
  needed fields the v3 shape had no place for, all optional, all refused where
  absent in v3: `tagged_source.tag_kind` (`lightweight`, with `tag_object` and
  `tag_signature` null), an explicitly unresolved `tree` and a
  `source_archive` (name, size, SHA-256 and the commit its pax comment names);
  an `archive` attribution for files read from that archive, bound by a
  locally computed blob id and never by a synthesised tree; an artifact
  `version_note` for a version report that disagrees with the tag; and an
  `smtp_auth` fact holding the source's answer and the probe's result, the
  probe `unresolved` with its reason when it could not run. The v4 reader is
  the v3 reader given an expected-target list; the three historical readers
  keep their three-target list.
- **§2's `smtp_auth` for MailSlurper is the source's answer only.** MailSlurper
  1.15.0 has no SMTP AUTH (reviewed source), so the fixture would omit the AUTH
  step for it; the probe that §2's reading needs before the product record
  freezes has not run, because the pinned linux executable was built with
  `CGO_ENABLED=0` and exits at startup on its SQLite config ("go-sqlite3
  requires cgo to work. This is a stub"). The product record therefore stays
  unwritten. Which MailSlurper executable the pin is, is the operator's to rule;
  nothing here rules it.
- **§5's id bound.** Adapters validate an id (at most 128 bytes, never only
  dots, an alphabet widened only to what the wire corpus shows) before any
  request. Mailpit's tool list is unchanged, so its frozen `tools_sha256`
  holds; its schema keeps the older `maxLength` and the server is stricter.

## Addendum — 2026-11-07 (operator ruling: the source-built MailSlurper)

This addendum records the operator's ruling of 2026-11-07 on which MailSlurper
the comparison measures, under the same proposed status. The first addendum
above is not edited. It weakens no accepted ruling. The ruling, as the second
rerun (`second-rerun-of-339-operator-dir-e9c4d893`) received it:

> The official release-1.15.0 linux executable is published as a result: it
> does not start with its default SQLite storage (CGO_ENABLED=0 stub). The
> comparison's MailSlurper is a cgo build from the pinned release source,
> labelled "built from source". It may be pinned even though it reports
> v1.14.1 under tag release-1.15.0; record that. Record the ruling as a dated
> addendum to 0129.

Authority is that ruling, not this note.

- **The published executable's failure.** The release asset
  `mailslurper-1.15.0-linux.zip` (sha256
  `48f2f9ba090f5385301477406b4f6e08f7bfbba0619c635ee591d6e6af3e55f6`,
  member `mailslurper` sha256
  `0cd9416b766deb26eed8aa01ad0873d06725a3464fdb9ac3cc82201e2c232627`) exits with
  status 1 at startup on the shipped SQLite config: "Binary was compiled with
  'CGO_ENABLED=0', go-sqlite3 requires cgo to work. This is a stub". It never
  opens a listener. That is the result, and it is kept in the lock as
  `artifact.official_release` with its stderr, as bytes. No launcher reads it,
  and it was not launched again.
- **The build that is measured.** The executable is `mailslurper` with sha256
  `eb6cd97d114f881e6a6f7170ddbe1fe914d8f52f025839631e441807f2a458ff`
  (19942072 bytes), built from the pinned source tarball (sha256
  `83f66f25ac000fc2f5680514719b7b75a29dafccb7e782597077325fb900359f`) in
  `docker.io/library/golang:1.18.1-bullseye@sha256:3b1a72af045ad0fff9fe8e00736baae76d70ff51325ac5bb814fe4754044b972`
  (go1.18.1) with `CGO_ENABLED=1`, `go mod verify` ("all modules verified"),
  `GOPROXY=off GOFLAGS=-mod=readonly` and `go build -trimpath`, no `-ldflags`
  and no tags. `go generate` was skipped, and the committed `www.go` was used.
  It is one build, not a reproducible one: a rebuild that differs is a new
  pin and a new product id, never a digest to force. The lock binds the
  inputs (`artifact.build`), and its evidence is committed under
  `docs/evals/sources/mailslurper/build/`.
- **The version report.** The build logs `Starting MailSlurper Server v1.14.1`
  under tag `release-1.15.0`. The tagged source hard-codes
  `SERVER_VERSION "1.14.1"` (`cmd/mailslurper/mailslurper.go:26`), and the
  published executable logs the same line. The lock records it in
  `version_output` and `version_note`, the product record's MailSlurper tuple
  repeats it, and the verifier refuses a product that disagrees with the lock.
- **What the label claims.** "Built from source" claims that these bytes were
  built from the pinned, tag-named source as recorded. It does not claim to
  reproduce upstream's release, and none of the lock, the product record, the
  README or this addendum calls the build the release artifact. It differs
  from upstream's release recipe (glibc and bullseye instead of musl and
  alpine, no `-s -w`, `-trimpath`) without changing a route or a wire byte;
  being able to store mail at all is the reason for the ruling.
- **The first addendum's open question.** That addendum left "which MailSlurper
  executable the pin is" to the operator. This ruling answers it. The AUTH
  probe the first addendum required ran against this build, through the
  launcher: EHLO advertises no AUTH, and the `AUTH PLAIN` line draws `221 Bye`
  (twice, because the reviewed source says one goodbye per worker error) and a
  close, with no `235` and no `334`. So `smtp_auth: none` rests on the source
  and on the probe, and the published executable was not probed.
- **The lock fields.** Lock v4's MailSlurper entry is edited in place, because
  v4 exists only on this branch and nothing committed binds the changed
  fields. The artifact kind `built_from_source` carries `sha256` equal to
  `executable_sha256`, no member, a locally computed digest basis and no
  upstream digest, with `download` the release page. It requires `build`
  (`source_sha256` equal to the tagged archive's, a builder image pinned by
  digest, toolchain, flags) and `official_release`, and the validator refuses
  each on any other kind. `binary_build_provenance` is observed (the tag
  commit and the build's `PROVENANCE.md`). The new document kind `git_commit`
  and the existing `git_tree` bind a lightweight tag's tree: the release
  commit `068acffdcd8960f248ea438c3b7ee51ead601bd2` names root tree
  `5f29f08a747241992f5e8ffc25ca8fb35d550729`, whose captured listing rehashes
  to that id, and each root-level capture is the blob that tree names. The
  tarball was extracted into a new scratch directory and `git write-tree`
  gave the same tree, so the build is from the pinned source. The tree was
  unresolved before the supply held the commit and tree objects.
- **The glibc floor.** The build links the host's glibc dynamically (the
  builder image ships glibc 2.31), so it needs glibc 2.31 or newer. On a host
  without it the launch fails loudly: the launcher's corroborated binds never
  appear. It cannot mis-measure, and CI launches no MailSlurper binary.
- **No change to 0092 or 0093.** Their digest and attribution rules are met by
  the fields above. 0091 §1 asks for official evidence; for MailSlurper in this
  story it is met by the official source plus this labelled source build,
  under the ruling, and 0091's 2026-11-07 addendum points here.

## Addendum — 2026-11-08 (integration with accepted 0130)

This addendum only adds. Every section above and both 2026-11-07 addenda are
unchanged, and this record stays **proposed**; only the operator accepts it.
It records how the agent-suite comparison integrates with accepted
[0130](0130-eight-target-mail-comparison.md), which the operator accepted on
2026-11-08 ("Accept and merge", keeping its conditions (f) and (g)) and which
merged to `main` as `55f211d`. It was written after reading `main`'s bytes,
not from this record's earlier assumptions.

0130 grew the wire driver to nine fixed targets with source lock v4 and
protocol v3, in the files this record's work also touches. Both records quote
the operator's one ruling of 2026-11-07 (the source-built MailSlurper) and pin
the same executable bytes. They differ only in where the record lives, and the
accepted record decides that. No contradiction remains that a reading does not
resolve.

- **§7 and the 2026-11-07 lock bullets are superseded by 0130's lock v4.**
  This record's lock-schema work (a lock v4 of its own, the `built_from_source`
  artifact kind, `build`, `official_release`, `version_note`, the `git_commit`
  document kind and the observed-tree rules, the config template and the third
  owned listener) is withdrawn. `docs/evals/sources.lock.json`
  (`ed201599…04b1`), `protocol.json` (`229babaa…224e`), `history/` and every
  file under `sources/` are `main`'s bytes, and `main`'s frozen-sha tests stay
  verbatim. This branch adds no lock version, entry, field or schema. The
  `official_release` sub-object this record's first lock carried (zip
  `48f2f9ba…55f6`, member `0cd9416b…2627`) now lives only where 0130 put it:
  the official `mailslurper` row and its failure document, and the first
  2026-11-07 addendum above.
- **The agent `mailslurper` binds `mailslurper-source-build`.** The target name
  `mailslurper` names the service API the adapter drives, which is the same for
  any build. Its executable is 0130's `mailslurper-source-build` row (sha256
  and executable digest `eb6cd97d114f881e6a6f7170ddbe1fe914d8f52f025839631e441807f2a458ff`,
  19942072 bytes, kind `executable`, version output `Starting MailSlurper
  Server v1.14.1`), "built from source" as the operator ruled. It is never the
  official `mailslurper` row. One exhaustive mapping carries this
  (`Target::lock_entry()` in the eval crate; `Backend::TARGET` in the adapter
  driver), with no wildcard arm; a product tuple bound to the official row, or
  to any entry but its own, is refused by name. In the product record only two
  fields of the MailSlurper tuple changed: `lock_entry` is
  `mailslurper-source-build` and `artifact_kind` is `executable`. The id stays,
  because this is a provenance pointer and not a change of method (§8). Every
  other field agrees with `main`'s entry as the verifier checks it.
- **MailSlurper is now a wire-bench target under 0130**, so this record's
  statement that it is "not a wire-bench target" no longer holds. The agent
  suite does not depend on that: it runs the same pinned executable through the
  wire driver's own launcher (`LaunchSpec::from_pin`), with 0130's declared
  listeners and config file.
- **`smtp_auth` lives in the product record and its probe transcript, not in
  the lock.** The probe against the source build is repeated in the new
  controls directory: EHLO advertises no AUTH, and `AUTH PLAIN` draws `221 Bye`
  (twice) and a close.
- **The sources 0130 does not bind are kept.** 0130 binds MailSlurper's licence,
  README, config, release, ref, commit, tree, `ServiceController.go` and
  `setupServicesListener.go`. The twelve other files this record's adapter cites
  (`mailslurper.go`, `SmtpCommands.go`, `SMTPWorker.go`,
  `SMTPWorkerResponses.go`, `Writer.go`, `HelloCommandExecutor.go`,
  `DataCommandExecutor.go`, `DatabaseReceiver.go`, `CommonQueries.go`,
  `MailSearch.go`, `MailItem.go`, `MailCollectionResponse.go`) and `go.mod` and
  `go.sum` are in `docs/evals/agent-sources/mailslurper/`, outside 0130's
  `sources/` tree. A constant table in `tests/mail_compare.rs` binds each of the
  fourteen to its member of the source archive (`83f66f25…359f`), with a default
  rehash test and an ignored re-extraction test. Every other file this record
  had committed under `sources/mailslurper/` was sha256-equal to a document 0130
  binds and was deleted in favour of `main`'s copy.
- **The wire corpus was recaptured.** The MailSlurper corpus index bound the old
  lock's config template (`48f56620…349c`); 0130's `config_file` renders a
  different one (`2b53b35e…7af2`), so the corpus was recaptured under 0130's pin
  with the free, ignored capture test, not edited to agree.
- **`scripts/compare-mail.sh` takes `main`'s grammar.** `--bin NAME=PATH` and
  `--runtime NAME=DIR` repeat; `--mailpit` and `--mailhog` stay aliases;
  `--mailslurper` is gone. The agent modes need a `--bin` for each competitor
  lock entry the product record names (`mailpit`, `mailhog`,
  `mailslurper-source-build`) and refuse by name when one is missing. A bare
  `controls` keeps `main`'s refusal (the first prerequisite of the chain), with a
  hint on what it does need.
- **A task that reads Swarmail's deliverability reports is not applicable to a
  competitor.** `main`'s `newsletter-findings` is outside the frozen `hard` tier
  and adds no comparison cell; the runner and `check_tasks` refuse it for a
  competitor by name, because a competitor has no Swarmail HTTP plane.
- **The controls were rerun, and the earlier ones removed.** The 2026-11-07
  controls bound the old head's `swarmail`, adapter and harness digests and the
  old product sha, all of which the merge changed. The oracle and null controls
  ran again on the merged head (named in the directory's `receipts.json`) into
  `docs/evals/agent-controls-2026-11-08/` (120 rows, 4 targets, one worker per
  target, each its own named command under `timeout 600 taskset -c 0-31`), for
  product `agent-suite-comparison-2026-11-07` with sha256
  `b9e9f606e2b54363e25b6d1f4def488da57092c845be379e695cb53f7508667d`. Every oracle
  trial passes, every null trial fails with the idle class, and the projection
  digests are equal across targets. The 2026-11-07 directory is removed.
- **The bounded launch-slot retry stays.** The mechanics test that launches
  Swarmail with a MailSlurper shim on a reserved slot lost the slot to a parallel
  test in the coverage gate's instrumented run (`NotReady("no listener
  addresses")`). A fresh slot is tried at most five times, each refused launch
  must have proved its cleanup, and any other launch error fails at once. The
  reason is one constant (`NO_LISTENER_ADDRESSES`), the loop is one helper, and
  two tests drive it on every run: a held first candidate (two attempts, the
  holder still accepting) and five held candidates (five draws, then a loud
  failure). Probing the candidate after cleanup was rejected: a thief that had
  released the port would read as "no collision". `real_mailslurper_official_does_not_start`
  is `main`'s accepted code and stays byte-identical.
- **What stays with the operator.** Acceptance of this record; the CI run (the
  receipts' `ci` item is pending); the confirmation of the 240 paid sessions; the
  hold on the PR, because `scripts/compare-mail.sh` changes. No paid session ran.
