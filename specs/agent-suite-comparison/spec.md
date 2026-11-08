# The agent-suite comparison: Swarmail, Mailpit, MailHog and MailSlurper on one eval suite

Decision: 0129 — [the agent-suite comparison](../../docs/decisions/0129-agent-suite-comparison.md) (proposed)

- **Issue:** #339 (epic #164; prerequisite of #163's paid comparison)
- **Run:** `issue-339-agent-suite-comparison-e4de0540`, from `main` `c95386e`,
  `claude-careful` recipe. Continued by the rerun
  `rerun-of-339-operator-direction--305a5aec` (operator direction
  2026-11-07) on `repair/issue339-compare-agents` from `51c9f98`, with the
  pinned upstream bytes now present under `.compare-bins/` (section
  "The 2026-11-07 rerun"). Continued again by the second rerun
  `second-rerun-of-339-operator-dir-e9c4d893` from `6e0098b`, under the
  operator's 2026-11-07 ruling that the comparison's MailSlurper is a cgo
  build from the pinned source, labelled "built from source" (section "The
  second 2026-11-07 rerun"). Continued a third time by
  `third-rerun-of-339-integrate-mai-60435935` from `4d89de1`, which merges
  `main` at `55f211d` under accepted 0130 (section "The 2026-11-08 third
  rerun").
- **Authority:** the operator's ruling of 2026-11-07, quoted in #339:
  "compare with all that we have shims for". It sets four targets
  (Swarmail, Mailpit, MailHog, MailSlurper) and four agents (claude-opus,
  claude-sonnet, codex-sol, codex-luna), with no GLM. The paid plan,
  confirmed separately, is the hard tier × 4 targets × 4 agents × 5 trials =
  240 sessions.
- **Accepted rulings this builds on:**
  [0091](../../docs/decisions/0091-fair-mail-comparison.md),
  [0092](../../docs/decisions/0092-official-mail-comparison.md) and
  [0093](../../docs/decisions/0093-source-mail-comparison.md), all accepted
  by the operator on 2026-11-02, plus
  [0076](../../docs/decisions/0076-sequential-eval-trials.md) and
  [0117](../../docs/decisions/0117-hard-eval-tier.md). *(rerun 3)* Also
  [0130](../../docs/decisions/0130-eight-target-mail-comparison.md),
  accepted by the operator on 2026-11-08: its lock v4 and protocol v3
  govern the wire driver and the MailSlurper pin this suite measures.

## Feature and rationale

On `main`, `scripts/compare-mail.sh controls|evidence|measure` always stop at
`comparison-integration`. Eval tasks T7–T10 were never built. There are
three reasons:

- The eval runner can hand an agent only Swarmail's `POST /mcp`.
- The only competitor adapter, Mailpit's, speaks stdio and refuses batches.
- No product record, controls, contract evidence, identity record, receipts
  or cost basis exists.

This story builds the free half of the comparison. It records the
operator's ruling, selects a target in the runner, adds adapters for
MailHog and MailSlurper, adds batch support to every adapter, and adds each
prerequisite the wrapper checks. It ends with real free controls on all four
targets and one end-to-end `measure --dry-run`. No session that bills a
model runs, and no paid matrix runs.

The comparison is an instrument, so it is held to two requirements. Every
difference between targets is either the variable being measured (the
backend and its tools) or a stated asymmetry. And nothing Swarmail
guarantees is exported to a competitor as an equivalence claim.

### Fairness check (the issue's stop-and-report clause)

There is no conflict between the ask and 0091/0092, so the work goes ahead.
The reasons:

- 0091's synthesis rejects a "MailHog agent eval". The operator overrode
  that for this story on 2026-11-07. A dated addendum records the override
  (R1). It is the operator's ruling, not a weakening by this design.
- MailSlurper is not named in 0091/0092. It enters under 0092 §3's contract
  unchanged.
- The prompt header names the target truthfully. The goal text, scorer,
  trap classes and failure classes stay byte-identical (R6). Giving a
  Mailpit agent the Swarmail header would be the fabrication. 0091 §2 asks
  for "common unchanged v2 business meaning", and the goal and the scoring
  carry that meaning.

If review rules either of these a conflict, the run parks for the operator.
The point is not patched around.

### How the two design positions were ruled

Simplicity argued for the smallest seam: one target switch, one bridge, one
fixture, no `src/` change, no 0129. Robustness argued for a target-typed
tuple frozen in the product record, a bounded batch policy, owned lifecycle,
declared delivery semantics and a ledger that keeps the denominator.
Neither is averaged. The rulings:

| Question | Ruling | Winner | What the loser costs or gives up |
|---|---|---|---|
| Agent transport | A loopback HTTP bridge in the runner process. The runner owns the adapter, the adapter owns the backend through `targets.rs`, and only an adapter exit 0 after full cleanup is a receipt. | Simplicity's bridge, with robustness's ownership | Robustness's separately supervised bridge process is not built. One hop of overhead is stated. Stdio entries in the wrappers are rejected because they would make the agent CLI the backend's owner. |
| Backend choice | A typed `Target` that selects a frozen backend tuple (lock entry, adapter, `tools_sha256`, `smtp_auth`, isolation, connection, wait method, delivery semantics, prompt header). The tuple is frozen in the product record. | Robustness | Simplicity's bare `--target` string grows a closed enum and a table. |
| Fixture per backend | One fixture plus the tuple's `smtp_auth` fact. 0091 §2 forbids an alternate harness. | Simplicity | Robustness's per-backend fixture is read as per-backend fixture *configuration*, not as new scenario code. |
| Isolation | A fresh owned backend per trial, sequential (0076). No emulated inbox. Competitor cells only for one-identity, chaos-free tasks, which is the whole hard tier. | Both agree. Robustness's per-trial inboxes are rejected for competitors. | A competitor has no inbox. A shared mailbox inside one trial is its real model, and the record states it. |
| Batches | Cap 16. Empty or over-cap gets one null-id error and executes nothing, exactly as in `src/mcp.rs`. One admission unit, members in order, per-member budgets, a bounded array, `initialize` refused inside a batch. A hostile-client suite. | Robustness on the refusal, simplicity on the cap | Simplicity's per-overflow correlated errors are rejected as asymmetric with Swarmail. |
| Delivery semantics | Declared per target in the product record. No per-trial `ready_after_accept` measurement. | Robustness on declaring them, simplicity on not measuring | The wire bench already measures waits. A per-trial observation would be a second wait method. |
| MailSlurper provenance | Source-lock **v4**. The v3 bytes and reader are kept. Wire protocol v2 is unchanged. No MailSlurper wire bench. | Intake and robustness | Simplicity's in-place v3 entry is rejected because the v3 reader requires exactly three targets, and changing it changes v3's meaning (0093 §1). |
| Decision record | Proposed 0129 for the new experiment contracts. The ruling itself is in addenda. | Robustness and intake | 0092's context says a thin adapter is a new public experiment interface that needs a proposal. Simplicity's "no 0129" would leave two adapters, a lock version and the mode scopes with no record. |
| Cost basis | `unavailable` with a reason. No usage capture. Every started slot keeps its row, which is proved by a test of the existing matrix journal. | Simplicity on capture, robustness on the denominator | Robustness's new start-of-trial ledger is not built, because the matrix already writes a row per slot, and a test proves it instead. |
| Comparison row | Schema 3 keeps its shape. The coordinate gains `agent`. Four target names. The MailHog refusal is removed. | Neither: both proposed more | Simplicity's embedded full v2 row is rejected because it would carry Swarmail-specific target identity into a competitor row (0091 context). |
| `measure --dry-run` | On genuine prerequisites it verifies contents, prints the 240-slot plan, launches nothing and exits 0. CI-only or operator-only items print as pending. Non-dry measure refuses a pending item and refuses without `--authorized-product`. | Simplicity's passing dry run, robustness's refusal of the paid start | Intake's always-refusing dry run cannot be the issue's "prove it". The prerequisites are produced for real, never stubbed. |
| Hostile DATA, cert expiry, webhook, chaos campaigns | Not built. None is on the hard-tier path: loopback plaintext SMTP, no webhook, no chaos. The adapters' bounds refuse and never repair. | Simplicity | The comparison claims nothing about fault tolerance. 0091's synthesis already rejects a cross-product fault suite. |

## The 2026-11-07 rerun

The first run built most of this spec (ten commits, `d5961bb`..`51c9f98`)
and stopped on one missing input: the pinned upstream bytes. The operator
put them in the worktree under `.compare-bins/`. That directory is
git-ignored, is never committed, and is untrusted data. The rerun finishes
the spec; it does not redesign it. Every requirement below stands unless
this section or a requirement marked *(rerun)* changes it.

The bytes raise no new fairness conflict with 0091/0092. MailSlurper
differs from the other targets in three ways: it has no AUTH, its recipient
filter is a substring match, and it stores after the accept. Those
differences are the backend variable being measured. They are stated in the
record, the rows and the README, and the adapter emulates nothing to close
them. 0129 §2 already provides `smtp_auth: none`. So the stop-and-report
clause is not triggered.

### What the pinned bytes establish

Every fact here was read from the bytes on 2026-11-07, by digest and by path
relative to the worktree. Archives were listed or read with `tar -O` /
`unzip -l` and nothing was extracted or executed. Python ran with `-I`. No
upstream fact is written from memory, and the implementer re-reads each one
from the bytes before it enters the lock.

- **Mailpit and MailHog match lock v3.** In `.compare-bins/2026-11-07/`:
  - `mailpit-linux-amd64.tar.gz` has sha256
    `4b5993c978e8604c70d43927e2ad103b311bba3b1822fa0cf1d4ee4ec890a93c`;
  - `mailpit` has sha256
    `262cfa28fae4554be198d1fea5ef83c0f8299317876d658623711c629a8edec8`;
  - `MailHog_linux_amd64` has sha256
    `e2ed634ded49929f089b20045581955ed217672078fd86082dd7a6c67c5d09c7`.

  These are lock v3's artifact and executable digests. That directory
  carries no digest file of its own, so the lock is the expected value.
- **MailSlurper release.**
  `.compare-bins/2026-11-07-mailslurper/mailslurper-1.15.0-linux.zip` has
  sha256 `48f2f9ba090f5385301477406b4f6e08f7bfbba0619c635ee591d6e6af3e55f6`
  and size 6574992.
  - The release metadata,
    `.compare-bins/2026-11-07-meta/mailslurper_mailslurper-releases.json`,
    lists `release-1.15.0` as the newest release (published
    2023-08-03T20:26:53Z, not prerelease, not draft). It publishes no asset
    digest. So the asset digest is `locally_computed`, as MailHog's v3
    entry already is.
  - The zip holds `LICENSE`, `README.md`, `config.json`, two SQL files,
    `createcredentials`, `logo.png` and `mailslurper`. The member run is
    `mailslurper`, and its executable digest is computed at lock time from
    a fresh extraction. `createcredentials` is never run.
- **MailSlurper source.**
  - `.compare-bins/2026-11-07-mailslurper-src/mailslurper-release-1.15.0-src.tar.gz`
    has sha256
    `83f66f25ac000fc2f5680514719b7b75a29dafccb7e782597077325fb900359f`.
    That equals the hex in the adjacent `SHA256SUMS`. That file names an
    absolute path on another machine, so the hex is compared as a constant
    and `sha256sum -c` is never run against it.
  - `.compare-bins/2026-11-07-meta/mailslurper_mailslurper-ref.json` gives
    `refs/tags/release-1.15.0` → object type `commit`,
    `068acffdcd8960f248ea438c3b7ee51ead601bd2`. It is a lightweight tag,
    so no tag object exists to capture.
  - No commit object and no tree listing were captured. *(Superseded by
    the second rerun: the supply's `git-trees/` captures both.)*
- **No SMTP AUTH.**
  - `pkg/mailslurper/SmtpCommands.go` maps only `helo`, `ehlo`, `rcpt to`,
    `mail from`, `send`, `rset`, `quit`, `data` and `noop`.
  - In `pkg/mailslurper/SMTPWorker.go`, an unknown command (lines 149–152)
    goes to the worker's error channel. The worker then calls `SayGoodbye`,
    which sends `221 Bye`, and closes the connection (lines 190–196;
    `pkg/mailslurper/Writer.go:30–31`; `SMTPWorkerResponses.go:14`).
  - EHLO is answered by the single line
    `250 Hello. How very nice to meet you!` (`SMTPWorkerResponses.go:17`),
    which advertises no extension.

  So the fixture's AUTH PLAIN would end the session. The source answer is
  `smtp_auth: none`, and R8 requires the probe to confirm it.
- **Accept precedes storage.** `pkg/mailslurper/DataCommandExecutor.go:123`
  sends the `250` inside `Process`. Only after `Process` returns does
  `SMTPWorker.go` (lines 216–219) put a copy on the `Receiver` channel, and
  `DatabaseReceiver.Receive` stores it. The declared delivery semantics are
  `accept_implies_queryable: not-promised`.
- **Three listeners, one config file.**
  - `cmd/mailslurper/mailslurper.go:50` declares `-config` ("Absolute
    location of the config.json").
  - Lines 69–72 always set up the database, SMTP, the admin (www) listener
    and the service listener.
  - `cmd/mailslurper/config.json` names `wwwAddress`/`wwwPort`,
    `serviceAddress`/`servicePort` and `smtpAddress`/`smtpPort`. It also
    names `dbEngine: "SQLite"` with a relative `dbDatabase`, and
    `authenticationScheme: ""` (service auth off).
- **Service routes** (`cmd/mailslurper/setupServicesListener.go:51–59`):
  - `GET /mail/:id`, `GET /mail/:id/message` and `GET /mail/:id/messageraw`;
  - `DELETE /mail`;
  - `GET /mail`, `GET /mailcount` and the attachment route;
  - `GET /version` and `GET /pruneoptions`.
- **List and filter semantics.**
  - `GET /mail` pages a fixed 50 items (`controllers/ServiceController.go`,
    around line 140).
  - The `message`, `from` and `to` filters are SQL `LIKE '%term%'`
    substring matches. `message` matches body or subject, `to` matches
    `toAddressList` (`pkg/mailslurper/CommonQueries.go:146–171`).
  - `GET /mail/:id` base64-decodes `body` when it can, and copies the
    result into `HTMLBody` (`ServiceController.go` `GetMail`).
  - A mail item is JSON with one `body` string beside `contentType`,
    `boundary` and `transferEncoding` (`pkg/mailslurper/MailItem.go`).
- **Not present.** The framing names `.compare-bins/2026-11-07-jre/` and
  `.compare-bins/2026-11-07-alkemio-mailslurper/`. Neither exists in this
  worktree. Neither is needed: the JRE serves only GreenMail, and MailSlurper
  runs from its own release zip with `-config`.

### How the rerun's positions were ruled

Both positions agree on the frame: finish the committed design, keep the four
targets, no `src/` change, no dependency, nothing paid. They differ on how
much proof the new boundary needs. The rulings:

| Question | Ruling | Winner | What the loser costs or gives up |
|---|---|---|---|
| Scope of the rerun | Finish the committed spec. Tick the built tasks with their commits. Replace the README's "Not built, and why" block with the outcome rather than adding a second block. | Simplicity | Robustness's freeze chain is adopted only where a row below adopts it. |
| Byte verification | The hex is compared to an expected constant, never `sha256sum -c` against a foreign path. Archive *and* member digests. A recorded `digest_basis`. A mismatch stops the run and is reported; the lock is never adjusted to the bytes. | Robustness | None. Simplicity read the same digests. |
| MailSlurper tree | *(Superseded by the second rerun, which resolves the tree from the captured commit and root tree.)* The tree stays **unresolved**, with its reason. Source captures are attributed `from: "archive"`, carrying the archive digest and its pax commit comment. Each capture is bound by size, sha256 and a locally computed blob id. No `git_tree` is synthesised from a tarball. Only the cited files are captured. | Simplicity | The commit→tree link is unproved, and that is labelled. Fetching a tree needs network access this seat does not hold. |
| AUTH PLAIN | The source answer (`none`) must be confirmed by a probe against the pinned binary. A probe that is inconclusive or disagrees with the source refuses to freeze the product record, and the MailSlurper items report blocked. | Robustness on the refusal, simplicity on one ignored test | The probe is a gate, not a footnote. |
| Tool surface | Every announced tool is called once, read-only, against each pinned binary by the existing ignored real-binary tests, before the product record is committed. A failing tool is removed from the adapter first, never emulated. The controls' contract evidence repeats one call per tool. | Robustness on the probe, simplicity on no separate tool | The record cannot freeze until the ignored tests have run against the pins. |
| CI execution of real upstream shapes | A bounded **wire corpus**. The ignored real-binary tests capture each competitor's responses to fixture-generated mail (list, read, empty result, not-found, and a filtered search). They are committed under `docs/evals/wire-corpus/<target>/`, with an index of sha256s and the capture's executable digest. CI tests decode the corpus through the adapter's decoders and the oracle's normalizers on every run. | Robustness | Simplicity pays for about fifteen small JSON files and two tests, and the corpus is recaptured whenever a pin moves. Swarmail's shims stay the CI mechanics; the corpus catches drift between a shim and upstream. |
| Hostile tool arguments | Ids are validated (length and charset) and path-encoded. Over-cap sizes are refused, not clamped. No URL comes from an argument. Redirects are off. Each rule is tested on the generic half and on every backend. | Robustness | A few tests. Anything already enforced is proved, not rebuilt. |
| Bridge | Already bounded: body ≤ 64 KiB, batch ≤ 16, a 120 s reply deadline, one writer channel, and each line re-serialised from parsed JSON so no embedded newline survives. Two additions: a bound on requests in flight, and adapter stderr going to a private trial file, never an undrained pipe. | Robustness, partly | Robustness's separate cold-path catalogue is reduced to those two gaps plus a test with a pretty-printed body. |
| Ledger | No new start-of-trial ledger (the earlier ruling stands). `comparison-check` refuses a controls directory whose row count differs from its planned slots, and refuses a torn final line by name. | Simplicity, with a denominator check | If the runner process itself crashes, its controls are rerun (they are free), not salvaged from a torn file. |
| Receipts | HEAD-bound, with the ancestry rule in R9. Evidence is never spliced across product ids or HEADs. | Robustness | None. |
| Extra shelf upstreams | smtp4dev, Inbucket and GreenMail are not used. They are outside the four named targets, and Swarmail has no shim for them. | Both | The report says they exist and are unused. |
| Wrapper usage capture | None. The cost basis is `unavailable`. | Both | — |

## The second 2026-11-07 rerun (the source-built MailSlurper)

- **Run:** `second-rerun-of-339-operator-dir-e9c4d893`, on
  `repair/issue339-compare-agents` from `6e0098b`. It continues the first
  rerun and redoes nothing that is ticked in `tasks.md`.
- **What blocked the first rerun:** the pinned official MailSlurper linux
  executable (zip `48f2f9ba…55f6`, member `0cd9416b…2627`) is a
  `CGO_ENABLED=0` build. It exits with status 1 at startup on its default
  SQLite storage: "Binary was compiled with 'CGO_ENABLED=0', go-sqlite3
  requires cgo to work. This is a stub". So the AUTH probe, the real-binary
  test, the wire corpus and the product record could not run.

### The operator's ruling, 2026-11-07

As the brief states it: the official release-1.15.0 linux executable is
published as a result, namely that it does not start with its default SQLite
storage (`CGO_ENABLED=0` stub). The comparison's MailSlurper is a cgo build
from the pinned release source, labelled **"built from source"**. It may be
pinned even though it reports v1.14.1 under tag `release-1.15.0`, and that
is recorded. The ruling is recorded as a dated addendum to 0129.

### Fairness check, again

The work goes ahead; no stop-and-report. 0091 §1 binds "official
source/artifact/platform/license evidence", and a source build is not an
official artifact. But the conflict is not this seat's to decide, and it is
not left undecided: the operator ruled on it by name and supplied the label.
This design records the ruling and keeps 0091 §1's binding in every part
that can still hold:

- the **source** is official and bound: the tag commit `068acffd…1bd2`, its
  root tree `5f29f08a…0729`, and the source archive `83f66f25…359f`;
- the **artifact** is bound by its digest and carries its true label;
- the **official artifact** is not dropped. Its digests and its failure are
  bound in the lock as the reason the build exists (R11).

The measured variable does not change. The agent eval measures the
agent-facing surface: the SMTP dialogue, the HTTP routes, and the adapter's
tools, all from the pinned source. The build's differences from upstream's
release recipe (glibc/bullseye instead of musl/alpine, no `-ldflags "-s
-w"`, `-trimpath`, the committed `www.go` instead of `go generate`) change no
route and no wire byte. The one behavioural difference that matters, being
able to store mail at all, is the reason for the ruling.

### What the supply establishes

These facts were read on 2026-11-07 from
`.compare-bins/2026-11-07-supply/`, as text only. Nothing there was executed.
That directory is git-ignored, untrusted and never committed. The
implementer re-reads every fact from the bytes before it enters the lock,
and stops and reports on any disagreement.

- **The build** (`mailslurper-cgo/`):
  - `mailslurper` has sha256
    `eb6cd97d114f881e6a6f7170ddbe1fe914d8f52f025839631e441807f2a458ff` and
    size 19942072. It is an ELF x86-64 executable, dynamically linked against
    glibc (`libc.so.6`, `libdl.so.2`, `libpthread.so.0`), and not stripped.
  - It was built from the source tarball `83f66f25…359f`, in
    `docker.io/library/golang:1.18.1-bullseye@sha256:3b1a72af045ad0fff9fe8e00736baae76d70ff51325ac5bb814fe4754044b972`
    (go1.18.1, glibc 2.31).
  - The build ran with `CGO_ENABLED=1`, then `go mod download`, then
    `go mod verify` ("all modules verified"), then `GOPROXY=off
    GOFLAGS=-mod=readonly`, then `go build -trimpath`. It used no `-ldflags`
    and no tags. `go generate` was skipped: the committed
    `cmd/mailslurper/www/www.go` was used unchanged.
  - `build/go.sum` and `build/go.mod` are byte-identical to the tarball's.
    `build/go-version-m.txt` records go1.18.1, `CGO_ENABLED=1` and
    `-trimpath`.
  - `PROVENANCE.md` records a bounded launch with the shipped config on
    loopback. It opened SQLite, accepted one SMTP mail (`250 Ok`), and served
    it on `GET /mail`.
  - `SHA256SUMS` lists every file in the directory. It is compared as a set
    of constants, never with `sha256sum -c` (spec R8).
- **The official binary's failure** (`mailslurper-cgo/official-release-run/`):
  - `mailslurper.stderr.log` has sha256 `70adef75…6685`. It ends in the
    fatal `CGO_ENABLED=0` stub line.
  - `exit-status.txt` says `exit=1`.
  - The official binary logs `Starting MailSlurper Server v1.14.1` too: the
    tagged source hard-codes `SERVER_VERSION "1.14.1"`
    (`cmd/mailslurper/mailslurper.go:26`).
- **The release commit and its root tree** (`git-trees/`):
  - `mailslurper_mailslurper_release-1.15.0.commit.json` (sha256
    `772686e2…387f`) is the GitHub commit object `068acffd…1bd2`. Its
    `tree.sha` is `5f29f08a747241992f5e8ffc25ca8fb35d550729`.
  - `….tree.json` (sha256 `1ab4ed18…0e52`) is that root tree, with
    `truncated: false`. It is non-recursive. `LICENSE` is blob `c1f191e7…`
    and `README.md` is blob `fa9d065c…`.
  - The supply's `treehash.py` rebuilt the tree object from those entries and
    matched its sha. The tarball has no `.gitattributes` and no symlinks.

### How the second rerun's positions were ruled

Both positions agree on the frame: finish the committed design, keep lock
v4, make no `src/` change, add no dependency, run nothing paid, and keep the
`comparison-check` verifier as the guard that replaces the refusal. They
differ on how much the new artifact needs before it can stand in the lock.
The rulings:

| Question | Ruling | Winner | What the loser costs or gives up |
|---|---|---|---|
| Lock version | Edit v4 in place. v4 exists only on this branch, and nothing committed binds the changed MailSlurper fields: there is no product record and no MailSlurper corpus yet. | Simplicity | None. Robustness did not ask for v5. |
| How the artifact is named | One new `ArtifactKind`, `built_from_source`, which is the operator's label word for word. `sha256 == executable_sha256`, `member: null`, `digest_basis: locally_computed`, `upstream_digest: null`. `download` is the release page, because no binary URL exists. | Simplicity's word | Robustness's separate `origin` field is not added: the kind already says it, and a second field could disagree with it. |
| The build's inputs | A `built_from_source` artifact must carry a `build` record. It holds the `source_sha256` (which must equal `tagged_source.source_archive.sha256`), the `builder_image` (which must be pinned by `@sha256:` and 64 hex characters), the `toolchain`, and the `flags` (`CGO_ENABLED=1`, `-trimpath`). Its `binary_build_provenance` is **observed**: the tag commit plus the `build_evidence` document `PROVENANCE.md`. The record states that this is one build and not a reproducible one: a rebuild that differs is a new pin, never a digest to force. | Robustness | Simplicity's "the evidence document says it" is not enough. Without the source-digest equality, nothing in the lock ties the binary to the tag. Cost: one struct, two cross-checks, and their negative tests. |
| The unstartable official binary | It stays in the lock, never launchable, as `artifact.official_release`: the zip's name, URL, size and sha256 (`48f2f9ba…55f6`), the member and its sha256 (`0cd9416b…2627`), the exit status 1, and the `build_evidence` document holding its stderr. It is required on `built_from_source` and refused on every other kind. No launcher reads it. | Robustness | Simplicity's objection was to two *launchable* executables per entry. This field has no launch path. Without it, the lock would lose the zip digest it binds today and the reason the build exists. Cost: one struct and its tests. |
| Build evidence documents | Committed under `docs/evals/sources/mailslurper/build/`, each bound by size and sha256: `PROVENANCE.md`, `go-version-m.txt`, `go.sum`, `go.mod` and `official-release-stderr.log`. Attribution is `unresolved` with the reason "a local build record from the operator's 2026-11-07 cgo build; no upstream object". The bench `check` reads only the protocol's targets, and MailSlurper is not one, so `check` is not affected. No new attribution variant is added. The binary, the logs and the scripts are not committed. | Simplicity | Robustness's `host_supplied_capture` attribution is not added. The unresolved reason says the same thing without a new word. |
| The release tree | `DocKind::GitCommit` is added. A lightweight tag's `tree` may be **observed** when a `git_commit` document's `/sha` equals the commit and its `/tree/sha` equals the tree, and a `git_tree` document's `/sha` equals the tree with `/truncated` false. The root-level captures `LICENSE` and `README.md` must have the blob the root tree names for them. A test rebuilds the tree object from the committed JSON with `git hash-object -t tree` and matches its sha, so the hex is recomputed and not copied. Nested captures stay `from: archive`. | Simplicity's structure, robustness's recomputation | — |
| Tarball versus tree | A one-time step: extract the tarball into a new scratch directory, `git add -A` and `git write-tree` there, and compare the result with `5f29f08a…`. A mismatch stops and is reported: the build would then not be "from the pinned release source". The transcript is committed beside the controls. | Robustness | Simplicity's two root-blob checks cover two files. The binary is built from every file. Cost: one scratch `git` step, no network. |
| The version string | It is recorded in the lock's `version_output` and `version_note`, and in the product record's MailSlurper tuple (`artifact_kind`, `version_report`). `comparison-check` refuses a product whose tuple disagrees with the lock on either. The README says it. Rows are unchanged: they cite the product by id. | Robustness on the check, simplicity on the rows | Robustness's row-settings copy is not built. A row already binds the product digest, which carries the version. |
| `--target-bin`, `--sources`, `--product` | Before spawning anything, the runner reads `--sources` and `--product` as regular files with a bounded read. It hashes `--target-bin` and refuses unless the digest equals the product tuple's `executable_sha256`, which must itself equal the lock's. The refusal names both digests. No flag amends the product. | Robustness | Simplicity relied on the adapter's own launch check. That check still runs, but it fires after the runner has spawned the adapter. |
| Removing `comparison-integration` | `eval_prerequisite` builds the eval crate (`cargo build --locked --offline` in `evals/`) and names a failure `eval-build`. Then it runs `comparison-check`. The verifier refuses unless the controls hold the planned rows for **all four** targets. That is executed evidence that the runner served every competitor. | Robustness's executed gate, simplicity's verifier | Robustness's `--list-targets` enumeration is rejected: it proves less than the four targets' control rows already do. |
| The wire corpus | The index also binds the sha256 of the lock's launch config template. Its MailSlurper part holds a body that `GetMail` base64-decodes and one it does not. If upstream never produces one of those branches for fixture mail, the index says so and gives the reason. | Robustness | About two more files and one index field. |
| The AUTH probe | Its subject is the source build `eb6cd97d…`. The record says "the source at tag `release-1.15.0`, built as recorded, does not accept AUTH PLAIN; the published linux executable does not start and was not probed". The stop rule of AC6 is unchanged. | Robustness | — |
| Records | A second dated addendum on proposed 0129, which leaves the first unedited. One pointer bullet inside 0091's existing 2026-11-07 addendum, because 0091 §1 names the official artifact. No change to 0092 or 0093: 0092's digest and attribution rules are met by the fields above, and 0093's claim scope belongs to the bench, which is unchanged. | Simplicity | — |
| Extra shelf upstreams, Alkemio's image | Not used (the first rerun's ruling stands). | Both | — |
| Reproducing the build in a gate | Not done. It would need docker and network access to modules inside the gates. 0092 §1 already says "hashes prove local consistency". | Simplicity | A stranger checks the claim with the committed `go.sum` and `go-version-m.txt`, not in CI. |

## The 2026-11-08 third rerun (integration with accepted 0130)

Run `third-rerun-of-339-integrate-mai-60435935` continues from
`repair/issue339-compare-agents` at `4d89de1`. The second rerun built every
item above and stopped only in verify: under the coverage gate's
instrumented, parallel run,
`the_mailslurper_adapter_lists_and_reads_over_real_smtp_into_swarmails_shim`
panicked with `swarmail launches: NotReady("no listener addresses", …)`. The
test reserved a port, released it and passed it as `--mailslurper-listen`;
a parallel test took the port first and Swarmail exited at startup. The
dispatcher's `4d89de1` retries a fresh slot up to five times, requires
proved cleanup on each refused launch and fails at once on any other error.
That fix is kept (operator direction for this run).

Meanwhile `main` moved to `55f211d` (#346, issue #340). The operator
accepted [0130](../../docs/decisions/0130-eight-target-mail-comparison.md)
on 2026-11-08 ("Accept and merge"), keeping its conditions (f) and (g).
0130 grows the wire driver to nine fixed targets with source lock v4 and
protocol v3, and it changes the same files this branch changes. This
rerun merges `main` and passes the gates. It does not redo finished work.

### What main's accepted bytes establish (read from `origin/main`, not assumed)

- `docs/evals/sources.lock.json` on main has sha256
  `ed201599dd57441cbcf0c7e66e7c82a56dd217ef683b7bf4e72ae7ce9d3a04b1` and
  `docs/evals/protocol.json` has sha256
  `229babaac849ae832ecb1a9f8ec742c970d0f36edc9d26d4ff9bb9c3c0d7224e`. Main's
  `tests/mail_compare/eight_targets.rs` freezes both (`FROZEN_LOCK_V4_SHA`,
  `FROZEN_PROTOCOL_V3_SHA`).
- Main's lock v4 has two MailSlurper entries:
  - `mailslurper`, the official release. Protocol v3 records that it does
    not start (`does_not_start`), with its failure as the document
    `sources/mailslurper/mailslurper-official-failure.txt`. That document is
    byte-identical (sha256 `70adef75…6685`) to this branch's
    `sources/mailslurper/build/official-release-stderr.log`.
  - `mailslurper-source-build` (`TargetKind::MailslurperSourceBuild`,
    `Provenance::SourceBuild`). Its artifact is `kind: "executable"`, sha256
    and `executable_sha256` both `eb6cd97d…58ff`, size 19942072,
    `digest_basis: "locally_computed"`, `upstream_digest: null`,
    `version_output` `Starting MailSlurper Server v1.14.1`, download
    `{home}/tree/068acffd…1bd2` (condition (g)). Its build provenance
    document `mailslurper-cgo-provenance.md` is byte-identical (sha256
    `a73d49aa…fa5d`) to this branch's `build/PROVENANCE.md`.

  So main's accepted lock already pins **the same executable** this
  branch's agent suite measures, under the provenance class the operator's
  2026-11-07 ruling asked for ("built from source"). It has its own entry
  and is never a relabelled official row.
- Main's lock binds MailSlurper's licence, README, config, release, ref,
  commit, tree, `ServiceController.go` and `setupServicesListener.go`. It
  does **not** bind the other SMTP and storage sources this branch's
  adapter cites (`mailslurper.go`, `SmtpCommands.go`, `SMTPWorker.go`,
  `SMTPWorkerResponses.go`, `Writer.go`, `HelloCommandExecutor.go`,
  `DataCommandExecutor.go`, `DatabaseReceiver.go`, `CommonQueries.go`,
  `MailSearch.go`, `MailItem.go`, `MailCollectionResponse.go`). It does not
  bind `go.mod` and `go.sum` from the build evidence either.
- `git merge-tree` reports content conflicts in 13 files:
  `benches/mail_compare.rs`, `benches/mail_compare/{protocol,targets}.rs`,
  `tests/mail_compare.rs`, `scripts/compare-mail.sh`,
  `evals/src/{fixture,mail,main,runner}.rs`, `evals/tests/fixture.rs`,
  `docs/decisions/0091-fair-mail-comparison.md`, `docs/decisions/README.md`
  and `docs/evals/README.md`.
- **The silent trap.** `docs/evals/sources.lock.json` merges *without* a
  conflict marker into a third blob (sha256 `ef8309cf…`) that is neither
  side's bytes. A merge that looks clean there has edited a frozen file.

### Contradiction check (the brief's stop-and-report clause)

0129's 2026-11-07 addendum realised the operator's source-build ruling by
repurposing the lock entry `mailslurper` (`built_from_source`,
`official_release`). Accepted 0130 realised the same ruling as a separate
entry, `mailslurper-source-build`, and forbids relabelling the official
row. Both quote one ruling, and both pin the same executable bytes. They
disagree only about where the record lives, and the accepted record
decides that. The resolving reading is: 0129's measurement target is
0130's `mailslurper-source-build` entry, and 0129's lock-schema work (§7 and
its 2026-11-07 lock bullets) is superseded by 0130's lock v4. In the agent
suite, the target name `mailslurper` names the service API the adapter
drives, which is the same for any build. It is not a lock row. Every row,
tuple, identity record and README line binds the lock entry
`mailslurper-source-build` and says "built from source". No unresolvable
contradiction remains, so this rerun proceeds. R12.l's stop conditions
still apply to the bytes the implementer reads after the merge.

### How the third rerun's positions were ruled

- **Simplicity's integration by subtraction is adopted.** Take main's
  frozen bytes and main's `protocol.rs`/`targets.rs` whole. Delete the
  branch's lock-v4 layer, and point the agent suite at main's
  `mailslurper-source-build` through one mapping. Keeping the branch's layer
  would cost two lock vocabularies for one file, a second v3 reader, a
  second launch-config mechanism, and tests asserting a lock the frozen
  bytes no longer are. Main's lock already records everything in that layer
  that the measurement needs.
- **Robustness's separate digest-bound 0129 control file for MailSlurper
  provenance is rejected.** Robustness made it conditional on main's v4
  lacking the source-build class or recording it as an upstream artifact.
  Main's v4 has the class, with a locally computed digest, a null upstream
  digest, observed build provenance and the official entry's failure. The
  condition is false. A second record would restate the accepted one and
  could drift from it. What this ruling costs: the branch's
  `official_release` sub-object (zip `48f2f9ba…55f6`, member
  `0cd9416b…2627`) is no longer in a lock field this branch owns. It stays
  in main's official `mailslurper` entry and in 0129's unedited 2026-11-07
  addendum.
- **The bytes main does not bind are kept, not dropped.** Both positions
  agree that 0129 §5's source verification stands. The twelve adapter-cited
  sources and `go.mod`/`go.sum` move to a branch-owned directory,
  `docs/evals/agent-sources/mailslurper/`, outside main's 0130-governed
  `docs/evals/sources/` tree. A constant digest table in the tests binds
  them (R12.e). Any branch file whose sha256 equals a main-bound document
  is deleted in favour of main's copy.
- **Robustness's exhaustive-match and declared-subset rules are adopted.**
  Code this branch adds or ports never matches `TargetKind` or `Target`
  with a wildcard arm. The agent matrix's targets are the declared four. A
  MailSlurper tuple bound to the official `mailslurper` entry, or to any
  entry other than `mailslurper-source-build`, is refused by name.
- **Robustness's "update, never delete" rule for frozen-method assertions
  is adopted.** Main's frozen-sha tests stay verbatim. The branch's tests of
  its own lock layer go, because the layer goes. Tests of the agent suite's
  behaviour are ported to main's types, not deleted.
- **The port retry: robustness is adopted in part.** Adopted:
  - one shared constant for the refusal reason, so a reword is a compile
    error at both sites;
  - a forced-collision test, with a real listener holding the first
    candidate, which drives the retry arm on every run;
  - an exhaustion test, with all five candidates held.

  Rejected: classifying a refusal by probing the candidate port after
  cleanup. If the thief had already released the port, the probe would
  report "no collision" and the flake would return; the operator also
  directed that `4d89de1`'s semantics be kept. The cost is that a Swarmail
  startup exit for some other reason takes up to five fast launches before
  it fails. It fails loudly, with proved cleanup each time, and is never a
  pass. Also rejected: replacing the retry with `--mailslurper-listen
  127.0.0.1:0` read back from Swarmail's startup log. That would remove the
  race, but it needs the startup reader in main's accepted `targets.rs`
  extended, and it drops the fix the operator said to keep. It is recorded
  as a follow-up in the plan's risks.
- **Robustness's expect-refusal mode for main's
  `real_mailslurper_official_does_not_start` is rejected.** The test is
  `#[ignore]`d and is main's accepted code. It passes only if the child's
  stderr carries go-sqlite3's cgo stub line, which a stolen port cannot
  produce. A collision can only make its "no declared listener is bound"
  check fail falsely, never pass falsely. The test is outside the brief's
  scope ("a port the child binds") and stays byte-identical.
- **The controls are rerun, not patched** (both positions). The committed
  2026-11-07 controls bind the swarmail, adapter and harness digests and
  the product sha, and the merge changes all of them. The verifier already
  refuses stale receipts.
- **The load bound.** Robustness asks for proof at the suite's real width.
  The coverage gate's own instrumented run gives it: it is a house gate,
  and it is the run that reproduced the failure. Any added repeat runs at
  most `nproc/4` workers (`taskset -c 0-31` on this 128-CPU host) under
  `timeout 600`, as a named, timed command.

## Requirements

### R1. Record the ruling

- Add a dated addendum (2026-11-07) to accepted 0091 and to accepted 0092.
  Each quotes the ruling and states:
  - the four targets and four agents, and that there is no GLM;
  - that 0091's rejection of a "MailHog agent eval" does not apply to this
    story;
  - that MailSlurper joins under 0092 §3 unchanged;
  - (0092 only) that this story closes its batch gap;
  - a pointer to proposed 0129.
- The text above each addendum stays byte-identical.
- In `docs/evals/README.md`, 0091–0093 are stated as accepted by the
  operator on 2026-11-02, not as proposed.
- The decision index gains the proposed 0129 row.

### R2. Targets in the eval crate

- `evals` gains a closed `Target` enum: `swarmail`, `mailpit`, `mailhog`,
  `mailslurper`. It selects a backend tuple.
- `matrix --target T` selects the backend. An unknown value is refused by
  name before anything launches.
- Without `--target`, `matrix` behaves byte-identically to today and writes
  native v2 rows.
- With `--target`, `matrix` writes schema-3 comparison rows only.
- A competitor target requires `--adapter PATH` (the `mail_compare`
  executable), `--target-bin PATH` and the lock. It refuses by name in each
  of these cases:
  - `--brokkr`;
  - the dsh wrapper;
  - a task the product record declares not applicable for that target.
- *(rerun 2)* Before it spawns anything, a competitor `matrix` checks its
  inputs and refuses by name:
  - `--sources` or `--product` that is not a regular file, or larger than
    its read bound;
  - a `--target-bin` whose sha256 differs from the product tuple's
    `executable_sha256` (the refusal names both digests);
  - a product tuple whose `executable_sha256` differs from the lock entry's.

  No command-line flag amends the product record.

### R3. The bridge

- `evals/src/bridge.rs` is a loopback-only Streamable HTTP `POST /mcp`
  endpoint, served inside the runner process.
- Each request body is written as one line to the owned adapter's stdin.
  The replies are routed back by id. A batch is forwarded as one line and
  its array reply is returned as one body.
- A body holding only notifications gets `202`. `GET` gets `405`.
- The same Host/Origin guard as Swarmail's endpoint applies (0043).
- An id already in flight gets a correlated `-32600`.
- An adapter that has exited yields `503`. The trial then ends as a visible
  harness failure.
- The bridge logs each session's `clientInfo` and `protocolVersion` into the
  trial's private artifacts. It logs no bodies (0075).
- *(rerun)* At most 16 requests are in flight. Over that bound, a request
  gets a correlated JSON-RPC busy error and nothing is forwarded. A body
  that arrives pretty-printed (with embedded newlines) is forwarded as one
  re-serialised line.

### R4. Runner server block per target

- `swarmail` keeps its existing arm.
- A competitor spawns `mail_compare <t>-mcp --launch BIN --sources LOCK` as
  an owned process. That process launches the backend through the existing
  `targets::Owned` launcher, prints one announcement line (`smtp=…
  tools_sha256=…`) and then serves MCP on stdio.
- When stdin closes, the adapter runs the existing cleanup. It exits 0 only
  when the cleanup facts are all true.
- Any other end makes cleanup unknown and stops admission.
- The announced `tools_sha256` must equal the product record's value, or the
  trial refuses.
- The fixture sends to the announced SMTP address with the target's
  `smtp_auth`.
- *(rerun)* The adapter's stderr goes to a file in the trial's private
  artifacts, never to an undrained pipe.
- *(rerun)* MailSlurper's launch reserves a **third** owned port (www),
  because its admin listener always starts. An unowned bind would escape
  the cleanup receipt. The launcher fills `{www_addr}`/`{www_port}` only
  when a template names them. It writes the lock-bound config template
  into the trial's scratch, with the SQLite database under `{data_dir}`,
  service auth off and the browser off. It runs
  `mailslurper -config <absolute path>` with the scratch as its working
  directory. The Mailpit and MailHog launches are unchanged.

### R5. Adapters

- `benches/mail_compare/mailpit_mcp.rs` is split into a generic `mcp.rs` and
  a Mailpit backend. The generic half holds bounds, admission, line reads,
  frame and poll wait, scan, budget, `serve`, reply shaping, launch mode and
  batches. The backend interface is `tools(&Bounds)`, `parse_call` and
  `run_call`.
- Mailpit's `tools()` output stays byte-identical, and its digest equals
  protocol v2's `budgets.adapter.tools_sha256`.
- `mailhog_mcp.rs` and `mailslurper_mcp.rs` follow the same interface. They
  expose list/search/read and a labelled poll wait, but only over operations
  whose route and response shape are verified against source bytes the lock
  binds. Each tool's description names its upstream route.
  - An operation upstream does not serve is not offered.
  - Nothing Swarmail-only is emulated: no inbox, no extraction, no
    `await`, no chaos, no delete or admin, no hints.
  - Upstream bodies pass through unchanged, and MailHog's raw MIME stays
    raw.
- Every adapter answers batches under 0129 §6.
- *(rerun)* MailSlurper's adapter offers:
  - list: `GET /mail?pageNumber=N`, pages of 50, upstream's fixed size;
  - search: the `to`, `from` and `message` filters of the same route;
  - read: `GET /mail/:id`;
  - the labelled poll wait.

  It does not offer `DELETE /mail`, prune options, attachments,
  login/logout or version. The search tool's description states upstream's
  semantics truthfully: a SQL `LIKE` substring match, with the term wrapped
  in `%…%` and not escaped (`CommonQueries.go`). The adapter neither narrows
  nor escapes the term. That is upstream's real filter, and the record
  states it.
- *(rerun)* Hostile arguments, on every adapter:
  - an id longer than 128 bytes, or outside `[A-Za-z0-9._-]`, is refused
    before any request, and an accepted id is path-encoded;
  - an over-cap page, count or byte size is refused, never clamped;
  - no tool argument names a URL or host;
  - the upstream client follows no redirect;
  - an unknown or oversized upstream JSON shape is a typed adapter error,
    never a panic.

  Where an upstream id can fall outside that charset, the implementer
  widens the charset to the observed id alphabet, cited from the wire
  corpus. The id is never percent-decoded or passed through raw.

### R6. Prompt and wrappers

- `PROMPT_TEMPLATE` stays byte-identical for Swarmail.
- A competitor template renders the same `{goal}` with a header that names
  the target, states one mailbox per trial, and drops the inbox line and the
  `"inbox"` argument instruction.
- Both headers' digests are frozen in the product record.
- For competitor targets the runner passes `EVAL_MCP_SERVER` (the server
  key) and `EVAL_MCP_TOOLS` (the adapter's own announced tool names). The
  claude allowlist and the codex server table are built from them.
- `SWARMAIL_TOOLS` stays for the Swarmail target.
- The dsh wrapper refuses a competitor target by name.

### R7. Oracle per backend

- The oracle's mail access sits behind a `MailSource` trait. The Swarmail
  implementation is today's calls, unchanged.
- Each competitor implementation calls only that adapter's own tools. It
  polls the adapter's list or search until the count is met, reads the
  messages, and maps them to `{to, subject, text, html}`.
- The recipient filter uses the backend's own source-verified filter where
  upstream has one. Where it has none, the oracle filters the complete
  listing itself, and the product record states this.
- MIME decoding needed for MailHog's raw bodies (7bit/8bit,
  quoted-printable, base64, multipart/alternative) lives in the oracle,
  which is a control. It does not live in the adapter. An unknown shape is
  an oracle error, never a pass.
- No task logic is duplicated, and no dependency is added.
- *(rerun)* MailSlurper's normalizer reads `GET /mail/:id`'s `body` according
  to its `contentType`. Where the body is multipart, it reuses the MIME
  decoder already in `evals/src/mailsource.rs` and adds no second one. The
  oracle uses the upstream `to` filter only as a pre-filter. It then keeps
  only messages whose `toAddresses` contains the recipient **exactly**,
  because the upstream filter is a substring match. A wire-corpus test over
  real MailSlurper responses decides which shapes occur; nothing is
  assumed.

### R8. MailSlurper provenance and AUTH PLAIN

*(rerun 3: the lock-schema parts of R8 are superseded by R12. Main's
accepted lock v4 pins MailSlurper. The AUTH answer, the probe and the
download discipline still hold, and the cited sources move under R12.e.)*

- MailSlurper's latest official release is pinned in source-lock v4. The
  entry carries:
  - the asset digest (upstream if published, otherwise locally computed and
    labelled);
  - the executable digest;
  - the tagged source chain and tree;
  - the licence, README and service-API/SMTP source bytes, bound by size,
    SHA-256 and blob id;
  - build provenance, unresolved;
  - a launch config template if MailSlurper reads its ports from a file.
- Lock v3's bytes move to `docs/evals/history/2026-11-07-sources.lock.v3.json`
  with a v3 reader. The v1, v2 and v3 readers keep a three-target
  expectation.
- Whether MailSlurper accepts the fixture's AUTH PLAIN is answered twice:
  once from its reviewed SMTP source, and once by an unscored setup probe
  against the pinned binary. The answer is written to the lock finding, the
  product record's `smtp_auth`, the row settings and the README.
- Every download goes into its own new directory and is verified by digest
  before use. Python runs only with `-I`. If the bytes cannot be obtained,
  the MailSlurper items are reported blocked. They are never built from
  memory.
- *(rerun)* The bytes are the ones in "What the pinned bytes establish". The
  MailSlurper v4 entry records:
  - `release-1.15.0`, the zip digest `48f2f9ba…55f6` with
    `digest_basis: "locally_computed"` and `upstream_digest: null`;
  - `artifact.kind: "archive"`, `member: "mailslurper"`, and the member's
    `executable_sha256`, computed from a fresh extraction into a new
    directory; *(replaced by R11: the artifact is the source build, and
    these digests move to `official_release`)*
  - the tagged source as observed: `tag_kind: "lightweight"`,
    `tag_object: null`, commit `068acffd…1bd2`;
  - the source archive, `83f66f25…359f`;
  - the tree as **unresolved**, with the reason "no commit object or tree
    listing was captured; the archive's pax comment names the commit but
    does not prove its tree"; *(replaced by R11: observed)*
  - build provenance, unresolved; *(replaced by R11: observed)*
  - the licence SPDX id as `LICENSE`'s text decides it;
  - the launch config template;
  - the AUTH finding: "MailSlurper 1.15.0 has no SMTP AUTH command; the
    fixture omits the AUTH step for it; every other target receives AUTH
    PLAIN".
- *(rerun)* `docs/evals/sources/mailslurper/` commits only the cited files:
  - `LICENSE` and `README.md`;
  - under `cmd/mailslurper/`: `mailslurper.go`, `config.json`,
    `setupServicesListener.go` and `controllers/ServiceController.go`;
  - under `pkg/mailslurper/`: `SmtpCommands.go`, `SMTPWorker.go`,
    `SMTPWorkerResponses.go`, `Writer.go`, `HelloCommandExecutor.go`,
    `DataCommandExecutor.go`, `DatabaseReceiver.go`, `CommonQueries.go`,
    `MailSearch.go`, `MailItem.go` and `MailCollectionResponse.go`;
  - the release and ref metadata.

  Each is bound by size, sha256 and locally computed blob id. `vendor/`,
  `www/` and the tarball itself are never committed.
- *(rerun)* The v4 reader is the v3 reader with an expected-target list.
  v1–v3 pass the three-target list and v4 passes the four. No reader code is
  copied.

### R9. The prerequisites and the wrapper

- `docs/evals/agent-comparison.json` is the frozen product record (0129 §8).
  It is committed before the controls run. *(rerun)* It is also committed
  only after two things have passed against the pinned executables: the
  ignored real-binary adapter tests (every announced tool called once) and
  the MailSlurper AUTH probe. Its `smtp_auth`, `tools_sha256` and delivery
  semantics are copied from those results, never written ahead of them.
- `docs/evals/agent-comparison-cost.json` is
  `{"basis":"unavailable","reason":…}`.
- `compare-mail.sh controls --output DIR --product P --mailpit B --mailhog B
  --mailslurper B` runs, per target and sequentially, `matrix --target T
  --tier hard` with the builtin `oracle` and `null` agents for the product's
  trial count. It writes:
  - the rows;
  - an index bound to the product digest;
  - per-target contract evidence: negotiated `protocolVersion`,
    capabilities, `tools/list` digest, a batch probe (array in, array
    out) and *(rerun)* one read-only call per announced tool, each with its
    outcome;
  - `identity.json`, with every measured executable and the adapter checked
    against the lock and the product;
  - `receipts.json`: HEAD, a digest of the per-slot cleanup and admission
    receipts, the locked eval-suite command and its status, and
    `ci: {"status":"pending"}` or an operator-supplied run reference.
- `swarmail-evals comparison-check` verifies contents and cross-bindings and
  names the first failure:
  - the baseline is the published #151 report;
  - the contracts and controls come from one controls dir bound to this
    product;
  - the receipts' HEAD is the candidate, or an ancestor of it whose diff to
    the candidate touches only `docs/evals/agent-controls-*/`,
    `docs/evals/README.md` and `specs/`. *(rerun:* committing the evidence
    necessarily moves HEAD. A change to any code, lock, product or script
    path makes the receipts stale, and they are refused.*)*
  - the controls rows number exactly the product's planned control slots,
    and the file has no torn final line *(rerun)*;
  - the cost basis is genuine;
  - identity re-measures equal;
  - the product is valid.
- *(rerun 2)* `comparison-check` also refuses:
  - a product tuple whose `artifact_kind` or `version_report` differs from
    the lock entry's;
  - a controls directory that lacks the planned rows of any of the four
    targets.
- `evidence` and `measure` keep the fixed stat order, run the verifier and
  then delegate. The `comparison-integration` refusal is removed.
  *(rerun 2)* Before the verifier, the wrapper builds the eval crate
  (`cargo build --locked --offline` in `evals/`). A build failure is named
  `eval-build`, never `comparison-integration`.
- `measure --dry-run` follows 0129 §9.
- Non-dry `measure` requires `--authorized-product ID` equal to the product
  id, refuses a pending item by name, and delegates `matrix --target …` per
  target in product order.

### R10. Rows and the denominator

- In `ComparisonRow`, the coordinate gains `agent`.
- `COMPARISON_TARGETS` holds four entries. Swarmail's connection is
  `native-mcp`. Each competitor's is `bridged-stdio-adapter`.
- The `mailhog` refusal is deleted. A bare or unknown name is still
  refused.
- Settings gain `smtp_auth`.
- A trial killed mid-run keeps its row as a visible failure.
- `report::cite_comparison` keys on (target, agent, task).

### R11. The source-built MailSlurper *(rerun 2)*

*(rerun 3: superseded by R12. Main's accepted `mailslurper-source-build`
entry pins the same executable `eb6cd97d…58ff`. This branch no longer
edits the lock, and none of R11's lock fields or validator refusals are
built. The "built from source" wording rule still holds.)*

- Lock v4's MailSlurper entry is edited in place. Its `artifact` is:
  - `kind: "built_from_source"`, `file_name: "mailslurper"`, size 19942072;
  - `sha256` and `executable_sha256` both
    `eb6cd97d114f881e6a6f7170ddbe1fe914d8f52f025839631e441807f2a458ff`;
  - `member: null`, `digest_basis: "locally_computed"`, `upstream_digest:
    null`, and `download` set to the release's `html_url`;
  - `version_output` set to the startup line `Starting MailSlurper Server
    v1.14.1`, with a `version_note` that says: the source-built executable
    logs v1.14.1 under tag `release-1.15.0`, the tagged source hard-codes it,
    and the operator's 2026-11-07 ruling pins it anyway;
  - `build`: `source_sha256` `83f66f25…359f`, `builder_image` with its full
    `@sha256:` digest, `toolchain: "go1.18.1"`, and `flags:
    ["CGO_ENABLED=1", "-trimpath"]`;
  - `official_release`: `mailslurper-1.15.0-linux.zip`, its URL, its size
    6574992, sha256 `48f2f9ba…55f6`, member `mailslurper` with sha256
    `0cd9416b…2627`, `exit_status: 1`, and `evidence` naming
    `sources/mailslurper/build/official-release-stderr.log`.
- `binary_build_provenance` is observed: `commit` `068acffd…1bd2` and
  `evidence` `sources/mailslurper/build/PROVENANCE.md`.
- `tagged_source.tree` is observed as `5f29f08a…0729`. It is bound by the
  `git_commit` and `git_tree` documents under
  `sources/mailslurper/git/`.
- The validator refuses each of these:
  - a `built_from_source` artifact without `build` or `official_release`;
  - `build` or `official_release` on any other kind;
  - `source_sha256` different from the source archive's;
  - a builder image without a `@sha256:` digest;
  - `sha256` different from `executable_sha256`;
  - a non-null `member` or `upstream_digest`;
  - unresolved build provenance on `built_from_source`;
  - an `official_release.evidence` or `binary_build_provenance.evidence`
    path that is not a `build_evidence` document of the entry;
  - an observed lightweight tree without matching `git_commit` and
    `git_tree` documents;
  - a root-level capture whose blob differs from the root tree's.

  The `archive` and `executable` kinds keep exactly their rules. The v1–v3
  readers and their tests are untouched.
- The launcher checks `eb6cd97d…` at every launch, exactly as it checks any
  `executable_sha256`. The config template, the three owned ports and the
  scratch working directory are unchanged.
- The README, the product record and the 0129 addendum use the words
  "built from source". None of them calls the build the release artifact.

### R12. Integrate `main` under accepted 0130 *(rerun 3)*

- **R12.a — the merge.** Run `git merge origin/main` (`55f211d`) on top of
  `4d89de1`. Never rebase, squash or amend. The merge commit has both
  parents. Conflicts are resolved in the merge commit or in commits after
  it, and no conflict marker survives.
- **R12.b — frozen bytes are main's bytes.** After the merge, restore from
  `origin/main`:
  - `docs/evals/sources.lock.json` and `docs/evals/protocol.json`;
  - `docs/evals/history/`;
  - every file main tracks under `docs/evals/sources/`.

  Each sha256 must equal main's: the lock is `ed201599…04b1` and the
  protocol is `229babaa…224e`. Main's `FROZEN_LOCK_V4_SHA` and
  `FROZEN_PROTOCOL_V3_SHA` test stays verbatim. No frozen protocol or lock
  is edited, and no merge product of one is kept. Delete the branch's
  `docs/evals/history/2026-11-07-sources.lock.v3.json`. It is
  byte-identical to main's `2026-10-30-sources.lock.v3.json`, which main's
  reader already reads.
- **R12.c — the driver is main's.** `benches/mail_compare/protocol.rs` and
  `targets.rs` are main's files. The only additions are what the branch's
  adapter modes need to compile: at most one read accessor, and R13's
  shared constant. The branch's lock-v4 layer is removed:
  - in `protocol.rs`: `HISTORICAL_TARGETS`, the 4-entry `ALL`,
    `LaunchConfig` and its slots, lock-level `SmtpAuth`, `TagKind`,
    `ArtifactKind::BuiltFromSource`, `Build`, `OfficialRelease`,
    `version_note`, the `git_commit`/`git_tree` documents and their
    validators, and the branch's `v3` module;
  - in `targets.rs`: `config`, `fill_slots`, `www_addr` and
    `path_segment`.

  The adapter's launch builds its spec with main's `LaunchSpec::from_pin`,
  the same builder the wire run uses.
- **R12.d — the adapter modes.** `benches/mail_compare.rs` keeps main's
  usage text and dispatch, and the branch's adapter modes are added to
  them. `mailslurper-mcp` launches `TargetKind::MailslurperSourceBuild`,
  never `TargetKind::Mailslurper`. `mailpit-mcp` and `mailhog-mcp` do not
  change. Every `TargetKind` match in code this branch adds or ports is
  exhaustive, with no `_ =>` arm. Where an adapter entry point receives a
  target, every target but its own is a typed refusal that names the
  target, and nothing is spawned.
- **R12.e — the sources main does not bind.** The twelve MailSlurper
  sources the adapter cites, plus `go.mod` and `go.sum`, move to
  `docs/evals/agent-sources/mailslurper/`. A constant table in
  `tests/mail_compare.rs` holds `(path, archive member, sha256)` for all
  fourteen files. Two tests use it:
  - a default test rehashes the committed bytes against the table;
  - an `#[ignore]` test re-extracts each member from the source archive
    main pins (`83f66f25…359f`), using main's archive helpers, and compares
    it.

  The rest of the branch's `docs/evals/sources/mailslurper/` (`build/`,
  `git/` and the unprefixed copies) is expected to be sha256-equal to
  documents main binds, and is deleted. A file that is not equal moves to
  `agent-sources/` with a table row. Nothing is dropped silently.
- **R12.f — evals: one mapping.** `evals/src/target.rs` keeps its four
  variants and gains `Target::lock_entry()`, an exhaustive match:
  - Swarmail is `None`;
  - Mailpit is `"mailpit"`;
  - MailHog is `"mailhog"`;
  - MailSlurper is `"mailslurper-source-build"`.

  `validate_tuple` and `tuple_agrees_with_lock` compare a tuple's
  `lock_entry` with this value and read the lock entry it names. The
  agent-facing name `mailslurper` stays, so tool names, `tools_sha256`,
  prompt-header digests and applicability keys do not change.
- **R12.g — the product record.** In `docs/evals/agent-comparison.json`,
  only two fields of the MailSlurper tuple change: `lock_entry` becomes
  `"mailslurper-source-build"` and `artifact_kind` becomes `"executable"`
  (main's word). The executable, adapter, tasks, agents, trials, order and
  statistics are the same. This is a provenance pointer, not a change of
  method (0129 §8), so the id stays. Every other tuple field must agree
  with main's entry exactly as the verifier checks it. A disagreement is a
  stop-and-report, never an edit to fit.
- **R12.h — the eval crate.** Main's and the branch's additions to
  `evals/src/{fixture,mail,main,runner}.rs` and `evals/tests/fixture.rs`
  are both kept: the branch's `smtp_auth`, and main's `swarmail_http` and
  reports. There is one SMTP send helper: main's `send_stored` takes the
  `SmtpAuth` argument, and the branch's `send_auth` is deleted. Main's
  deliverability task is not in the frozen product's `hard` tier, so it
  adds no comparison cell.
- **R12.i — the wrapper.** `scripts/compare-mail.sh` uses main's grammar:
  `--bin NAME=PATH`, `--runtime NAME=DIR` and the `--mailpit`/`--mailhog`
  aliases. The branch's `--mailslurper BIN` flag is removed. The agent
  modes (`controls`, `evidence`, `measure`) need a `--bin` for each lock
  entry among the product record's competitor tuples (`mailpit`,
  `mailhog`, `mailslurper-source-build`). When one is missing, the mode
  refuses by name.
- **R12.j — the wire corpus.** `docs/evals/wire-corpus/` binds each
  target's executable digest and launch-config-template sha256 against the
  lock entry its target maps to. If main's `mailslurper-source-build`
  `config_file` renders the same template bytes the corpus index records,
  nothing changes. If not, the MailSlurper corpus is recaptured under
  main's pin with the free, ignored real-binary capture. The index is never
  edited to agree.
- **R12.k — records.**
  - Accepted records (0091, 0092, 0093 and 0130) take dated pointers only,
    as added lines.
  - In 0091, main's pointer text comes first and the branch's addenda
    follow.
  - 0130 gains one dated 2026-11-08 pointer line to 0129's new addendum.
  - The decisions index keeps main's rows and 0129's row. In 0129's row,
    "source-lock v4" becomes "on 0130's lock v4".
  - `docs/evals/README.md` is main's text followed by the branch's
    section. Its MailSlurper paragraph names `mailslurper-source-build`,
    cites 0130 and says "built from source".
  - 0129 gains a section headed "Addendum — 2026-11-08 (integration with
    accepted 0130)". It records that:
    - §7 and the 2026-11-07 lock bullets are superseded by 0130's lock v4;
    - the agent `mailslurper` binds `mailslurper-source-build`;
    - MailSlurper is now a wire-bench target under 0130, so "not a
      wire-bench target" no longer holds;
    - `smtp_auth` lives in the product record and its probe transcript,
      not in the lock;
    - the fourteen unbound sources are now in
      `docs/evals/agent-sources/mailslurper/`;
    - the controls were rerun, with the new directory and product sha.

    0129 stays `proposed`, and its earlier sections and addenda are not
    edited.
- **R12.l — stop conditions.** The implementer stops and reports, and does
  not decide, if:
  - a frozen file cannot be made byte-identical to main's;
  - main's `mailslurper-source-build` executable digest is not
    `eb6cd97d…58ff` (at `55f211d` it is);
  - a product tuple field other than R12.g's two disagrees with main's
    entry;
  - a merged test would need a `src/` change to pass;
  - 0129 and 0130 contradict in a way no reading resolves.

### R13. The bounded launch-slot retry *(rerun 3)*

- `4d89de1`'s semantics stay: a fresh slot each attempt, at most five
  attempts, `facts.proved()` asserted on each refused launch, and any other
  `LaunchError` fails at once.
- The refusal reason becomes one constant in `targets.rs`:
  `pub const NO_LISTENER_ADDRESSES: &str = "no listener addresses"`. Every
  producer of that `NotReady` reason uses it, at main's sites and any the
  merge keeps, and the test's match arm uses it as its pattern. A reword is
  then compile-checked at both ends.
- The loop moves into one test helper, `launch_shim_with_fresh_slot`. It
  takes a candidate chooser (by default, reserve and release
  `127.0.0.1:0`) and returns the slot, the `Owned` and the attempt count.
  The MailSlurper adapter test calls it with the default chooser.
- After the merge, run
  `grep -n 'bind("127.0.0.1:0")' tests/mail_compare.rs tests/mail_compare/*.rs`.
  Any hit that reserves a port, releases it and passes it to a child that
  is expected to bind it uses the same helper. At `55f211d`, no other
  default-suite test does this.
- Main's `#[ignore]`d `real_mailslurper_official_does_not_start` passes
  ports to a child that must not bind them. It stays byte-identical (see
  the ruling above).
- The launcher-internal `reserve_port` is out of scope, and no retry is
  added to `Owned::launch`.

## Acceptance Criteria

- **AC1 — ruling recorded.**
  - `git diff c95386e -- docs/decisions/0091-fair-mail-comparison.md
    docs/decisions/0092-official-mail-comparison.md` shows only added
    lines, each in a section headed "Dated addendum — 2026-11-07" that
    quotes "compare with all that we have shims for".
  - `docs/evals/README.md` no longer calls 0091–0093 "proposed".
  - The decision-index test (`tests/store_contract_docs.rs`) passes with the
    0129 row.
  - *(rerun 2)* 0129 gains a second section headed "Addendum — 2026-11-07"
    that records the source-built MailSlurper ruling. `git diff 6e0098b --
    docs/decisions/0129-agent-suite-comparison.md` shows only added lines.
    0091's 2026-11-07 addendum gains one pointer bullet to it, still as
    additions only. 0092 and 0093 are byte-identical to `6e0098b`.
- **AC2 — targets.**
  - `matrix --target bogus` exits non-zero naming the value, and launches
    nothing.
  - Each of the following refuses by name: `--target mailhog --brokkr`, a
    dsh profile with a competitor, and a non-applicable task with a
    competitor.
  - A `matrix` run without `--target` writes rows byte-identical in shape to
    `main`'s, and the existing suite stays green.
  - A sealed MailHog comparison row validates. A row with target `mailhog`
    or an unknown target is refused. Native readers refuse schema-3 rows.
  - *(rerun 2)* Each R2 input refusal has a test that asserts nothing was
    spawned: a FIFO or directory as `--sources`, an over-bound `--product`,
    a `--target-bin` with the wrong digest (both digests named), and a
    product whose digest differs from the lock's.
- **AC3 — bridge.**
  - An evals test drives the bridge against the real `swarmail mcp` stdio
    transport (`SWARMAIL_BIN`, no download): `initialize`, `tools/list`, a
    tool call, a batch, a notifications-only `202`, `GET` `405`, a foreign
    Host refused, a duplicate in-flight id `-32600`, and an exited child
    `503`.
  - A runner test proves the adapter's cleanup receipt: exit 0 is a
    receipt. A non-zero exit and a kill each stop admission.
  - *(rerun)* The 17th concurrent in-flight request gets a correlated busy
    error and is not forwarded. A pretty-printed request reaches the
    adapter as one line. The adapter's stderr lands in the trial's
    private file.
- **AC4 — adapters.**
  - Mailpit's `tools()` digest equals protocol v2's
    `budgets.adapter.tools_sha256`.
  - Every existing `tests/mail_compare.rs` test passes.
  - The MailHog and MailSlurper adapters pass mechanics tests over real
    SMTP against Swarmail's own MailHog and MailSlurper compat shims, in
    CI. Their ignored tests pass against the pinned upstream executables
    (`MAIL_COMPARE_MAILHOG`, `MAIL_COMPARE_MAILSLURPER`).
  - Each tool's upstream route is cited and bound in lock v4. *(rerun 3:
    bound either by a document of main's lock v4 or by R12.e's
    `agent-sources` digest table.)*
  - No tool name or description offers inbox, await, extraction, delete or
    admin.
  - *(rerun)* Each ignored real-binary test calls every announced tool once
    against its pin. Those tests passed before `agent-comparison.json` was
    committed, and their log is part of the controls evidence.
  - *(rerun)* `docs/evals/wire-corpus/{mailpit,mailhog,mailslurper}/`
    exists. Its index's sha256s match the files, and its executable digests
    match the lock. CI tests decode every file through the adapter decoders
    and the oracle normalizers.
  - *(rerun 2)* The corpus index binds the sha256 of each target's launch
    config template, and a test refuses a mismatch with the lock. The
    MailSlurper corpus holds a body that `GetMail` base64-decodes and one it
    does not, each mapped by an oracle-normalizer test. If one branch cannot
    occur, the index records the reason.
  - *(rerun)* Each hostile-argument rule of R5 has a test on every adapter.
    An upstream request counter shows that a refused call made no request.
  - *(rerun)* MailSlurper's search description states the `LIKE` substring
    semantics. An oracle test shows that a lookalike recipient returned by
    the upstream `to` filter is excluded by the exact match.
- **AC5 — batches.** A hostile-client suite runs on the generic half:
  - an array of 1 and of 16 members gets an ordered array;
  - 17 members, and an empty array, each get one null-id error with no call
    executed (asserted by an upstream request counter);
  - notifications-only writes nothing;
  - `initialize` inside a batch gets `-32600`;
  - duplicate ids are answered as sent;
  - an over-bound reply gives that member `reply_too_large`;
  - a full queue gives busy errors for all members;
  - a 64 KiB line is refused;
  - EOF mid-batch ends cleanly.
- **AC6 — MailSlurper.** *(rerun 3: the lock and protocol bullets are
  superseded by AC14, because main's accepted bytes govern. The AUTH
  bullets and the three-bind launch bullet still hold, through main's
  launcher.)*
  - Lock v4 validates, and `mail_compare check --protocol
    docs/evals/protocol.json --sources docs/evals/sources.lock.json` passes.
  - `docs/evals/protocol.json` is byte-identical to `c95386e`.
  - The archived v3 lock is byte-identical to `c95386e`'s
    `docs/evals/sources.lock.json`, and its reader accepts it.
  - The v1/v2 reader tests are unchanged and pass.
  - The AUTH PLAIN answer is stated, with its source citation and probe
    transcript, in the README, the lock finding and the product record.
  - *(rerun)* The answer is `none`, as the source predicts. The record
    refuses to freeze if the EHLO reply advertises AUTH. It also refuses
    unless the probe result is "the `AUTH PLAIN …` line drew `221 Bye`, and
    the server closed the connection, with no `235` and no `334`".
  - *(rerun 2, replacing the first rerun's binding list)* The lock binds:
    - the `built_from_source` executable `eb6cd97d…58ff`;
    - the official zip `48f2f9ba…55f6` and its member `0cd9416b…2627`, as
      `official_release` with exit status 1;
    - the source archive `83f66f25…359f`, equal to `build.source_sha256`;
    - the lightweight tag at `068acffd…1bd2`;
    - the observed root tree `5f29f08a…0729`.

    The blob, size and sha256 checks pass over every capture. The first
    rerun's "unresolved tree" binding was right for the bytes it had. The
    supply's commit and tree objects resolve the tree, so the criterion now
    asks for more.
  - *(rerun 2)* The AUTH probe and the real-binary test run against
    `eb6cd97d…`. The probe transcript names that digest and says that the
    published executable does not start and was not probed.
  - *(rerun)* A MailSlurper launch test proves three owned binds, and that
    cleanup releases all three.
- **AC7 — oracle and null controls.** `compare-mail.sh controls` on the
  real pinned executables of all four targets gives:
  - every oracle trial passes, and every null trial fails with the task's
    idle class;
  - for each (task, seed), the fixture's public projection digest is the
    same on all four targets;
  - the index binds the product digest.
  The controls output is committed under `docs/evals/agent-controls-2026-11-07/`
  (*rerun 3:* `docs/evals/agent-controls-2026-11-08/`, AC18)
  (rows, index, contracts, identity, receipts). Each per-target run is a
  named, timed command of at most 10 minutes with one worker, and the load
  before and after is reported.
- **AC8 — prerequisites.**
  - `controls` runs with only `--product` and the executables.
  - `evidence` and `measure` refuse, by the name of the first prerequisite
    in the fixed order, when it is missing; when its contents fail the
    verifier (one negative test per prerequisite); and when the receipts'
    HEAD differs or identity re-measurement differs.
  - The string `comparison-integration` is gone from `compare-mail.sh`.
  - *(rerun 2)* A wrapper test shows that an eval crate which does not build
    is refused as `eval-build`. A verifier test refuses a controls directory
    that is missing one target's rows.
- **AC9 — dry run.** One `compare-mail.sh measure --dry-run` with every
  prerequisite produced for real (no stub file) exits 0. It prints the
  240-slot plan, plus each pending CI-only or operator-only item. It starts
  no process other than the verifier, which a test asserts, and it does not
  open `--config`. The transcript is committed beside the controls.
- **AC10 — no paid start.**
  - Non-dry `measure` without `--authorized-product`, with a wrong id, or
    with a pending item, is refused by name, and no agent is spawned.
  - The delegation is proved only with an inert config whose agents are
    builtin `null`, on a synthetic one-trial product in a scratch dir on
    `/home` (never the `/tmp` tmpfs).
  - No paid session runs in this story.
- **AC11 — denominator and cost.**
  - A test kills a competitor trial's adapter mid-trial. The slot keeps a
    row as a visible failure, and admission stops until cleanup is
    confirmed.
  - The cost record is `unavailable`. No row or report writes a cost of 0.
  - *(rerun)* `comparison-check` refuses, each by name:
    - a controls directory with one row missing;
    - one with a torn final line;
    - receipts whose HEAD is not an ancestor of the candidate;
    - receipts whose diff to the candidate touches a path outside the
      evidence allowance.
- **AC12 — scope.** *(rerun 3: once `main` is merged, the base for the
  `src/`, lock, `deny.toml`, `.github/` and `.brokkr/` checks is
  `origin/main`, not `c95386e` (AC18). The intent is unchanged: this
  branch changes none of them.)*
  - `git diff c95386e -- src/` is empty.
  - `Cargo.lock`, `evals/Cargo.lock` and `deny.toml` are unchanged.
  - No file under `.github/` or `.brokkr/` changes.
  - *(rerun)* No file under `.compare-bins/` is tracked:
    `git ls-files .compare-bins` is empty.
  - The whole house gate inventory passes `--locked`, including the evals
    crate against the candidate's `swarmail`, the coverage gate and its
    selftest, the publish-guard and seal proofs, and `cargo deny check
    licenses`. If the sandbox has no brokkr binary, the bundle-compile gate
    is reported as run by verify.

- **AC13 — the source-built MailSlurper *(rerun 2)*.** *(rerun 3:
  superseded by AC14, AC15 and R12.e. The executable `eb6cd97d…58ff`,
  the provenance document `a73d49aa…fa5d` and the official failure log
  `70adef75…6685` are now bound by main's lock. `go.mod` and `go.sum`
  are bound by the `agent-sources` table. The "built from source" and
  v1.14.1 wording bullets, and the `.compare-bins` bullet, still hold.)*
  - The lock validates with the R11 entry, and
    `mail_compare check --protocol docs/evals/protocol.json --sources
    docs/evals/sources.lock.json` exits 0.
  - Each R11 validator refusal has a negative test in
    `tests/mail_compare.rs`.
  - A test rebuilds the root tree object from the committed `git_tree`
    JSON with `git hash-object -t tree` and gets `5f29f08a…0729`. Another
    test checks that `LICENSE` and `README.md` have the root tree's blobs.
  - The one-time tarball tree recomputation gives `5f29f08a…0729`. Its
    transcript is committed beside the controls.
  - The build-evidence documents match the supply's `SHA256SUMS` constants:
    `PROVENANCE.md` `a73d49aa…fa5d`, `go-version-m.txt` `de1ac6ec…f53b`,
    `go.sum` `ca08ec83…140d`, `go.mod` `2d59cd3c…21a2`, and the official
    stderr log `70adef75…6685`.
  - The product record's MailSlurper tuple says `artifact_kind:
    "built_from_source"` and `version_report: "v1.14.1"`. The README says
    "built from source", gives the official binary's failure, and names
    v1.14.1 under `release-1.15.0`.
  - `git ls-files .compare-bins` is empty, and no executable is committed.

- **AC14 — the merge and the frozen bytes *(rerun 3)*.**
  - `git log --merges -1 --format=%P` names `origin/main` (`55f211d`) as
    a parent. `git merge-base --is-ancestor 4d89de1 HEAD` holds, and no
    commit before the merge is rewritten.
  - `git grep -nE '^(<<<<<<<|>>>>>>>)( |$)'` is empty.
  - `git diff origin/main -- docs/evals/sources.lock.json
    docs/evals/protocol.json docs/evals/history/ docs/evals/sources/` is
    empty. `sha256sum` gives `ed201599…04b1` for the lock and
    `229babaa…224e` for the protocol. Main's frozen-sha test passes
    unchanged.
  - `mail_compare check --protocol docs/evals/protocol.json --sources
    docs/evals/sources.lock.json` exits 0.
- **AC15 — the agent suite on main's nine-target types *(rerun 3)*.**
  - `git diff origin/main -- benches/mail_compare/protocol.rs` adds no
    type, field, lock rule or protocol rule.
    `git diff origin/main -- benches/mail_compare/targets.rs` adds at most
    R12.c's accessor and R13's constant.
  - A test launches the `mailslurper-mcp` adapter's backend from main's
    `mailslurper-source-build` pin through `LaunchSpec::from_pin`, and
    asserts that the spec's executable digest is `eb6cd97d…58ff`.
  - The adapters' mechanics tests against Swarmail's shims pass. Their
    ignored real-binary tests pass when the executables are present.
  - An evals test refuses a MailSlurper tuple with `lock_entry:
    "mailslurper"` (the official `does_not_start` entry). Another refuses
    any `lock_entry` other than `mailslurper-source-build`.
    `Target::lock_entry()` is tested for all four variants.
  - `git diff origin/main -- benches tests evals | grep -nE '^\+.*\b_ =>'`
    shows no arm of a `TargetKind` or `Target` match.
  - The fourteen-row `agent-sources` digest test passes by default. Its
    `#[ignore]` re-extraction test passes against the pinned source archive
    when that archive is present.
  - `compare-mail.sh` refuses `--mailslurper` as an unknown flag. Each
    agent mode refuses by name when the `--bin` for `mailpit`, `mailhog`
    or `mailslurper-source-build` is missing.
  - The product record's MailSlurper tuple says `lock_entry:
    "mailslurper-source-build"` and `artifact_kind: "executable"`, and
    `comparison-check` validates it against main's lock.
- **AC16 — the launch-slot retry *(rerun 3)*.**
  - `the_mailslurper_adapter_lists_and_reads_over_real_smtp_into_swarmails_shim`
    passes through `launch_shim_with_fresh_slot`.
  - A forced-collision test runs in the default suite. It holds a real
    `TcpListener` on the first candidate for the whole launch and asserts:
    - the helper returns an attempt count of at least 2 (the fresh
      candidate is itself reserved and released, so an exact count could
      race; *review return 4* records why);
    - the refused attempt's cleanup is proved;
    - the held listener still owns the port.
  - An exhaustion test holds all five candidates. The helper panics with
    "slots refused in a row" (the test's `should_panic` matches that text, not
    the any-other-error arm's), after the fifth attempt and before any sixth.
    The message does not print the launch error.
  - `grep -rn '"no listener addresses"' benches tests` finds only the
    constant's definition.
  - The coverage gate's instrumented run passes with this test in it.
  - Any added repeat of `tests/mail_compare` runs as a named command under
    `timeout 600 taskset -c 0-$(( $(nproc)/4 - 1 ))`, and the gates
    transcript records uptime before and after it.
- **AC17 — records *(rerun 3)*.**
  - `git diff origin/main -- docs/decisions/0091-fair-mail-comparison.md
    docs/decisions/0092-official-mail-comparison.md
    docs/decisions/0093-source-mail-comparison.md
    docs/decisions/0130-eight-target-mail-comparison.md` shows only added
    lines. 0130's only addition is one dated 2026-11-08 pointer.
  - 0129's status line still says `proposed`. The new section is headed
    "Addendum — 2026-11-08 (integration with accepted 0130)", and
    `git diff 4d89de1 -- docs/decisions/0129-agent-suite-comparison.md`
    shows only added lines.
  - The decision-index test (`tests/store_contract_docs.rs`) passes with
    main's rows and 0129's row.
- **AC18 — controls and gates on the final head *(rerun 3)*.**
  - The oracle and null controls rerun on the final code head into
    `docs/evals/agent-controls-2026-11-08/`. AC7's assertions hold there,
    and `comparison-check` accepts the directory.
  - `docs/evals/agent-controls-2026-11-07/` is removed with `git rm`
    (history keeps it). The path constants in `evals/tests/comparison.rs`
    and the README links name the new directory.
  - The dry-run transcript (AC9) and the gates transcript are regenerated
    beside the new controls.
  - Each per-target controls command runs one worker under
    `timeout 600 taskset -c 0-31` (at most `nproc/4`). It is named and
    timed, with load recorded before and after.
  - AC12 is read against the merged base:
    `git diff origin/main -- src/ Cargo.lock evals/Cargo.lock deny.toml
    .github .brokkr` is empty, and `git ls-files .compare-bins` is empty.
  - The whole house gate inventory passes `--locked` on the final tree.
    That includes the evals crate against the candidate's own `swarmail`,
    the coverage gate and its selftest, the publish-guard and seal proofs,
    and `cargo deny check licenses`. If this sandbox has no brokkr binary,
    the report says that the bundle-compile gate runs in verify. Nothing
    paid runs.

## Non-goals and deferrals

- No paid session, no paid matrix and no comparison claim. #163 runs the 240
  sessions after the operator confirms them.
- No wire bench for MailSlurper, no wire protocol v3, and no best-wait
  stream client (0093 §4 keeps it blocked). *(rerun 3: main's accepted 0130
  now owns a wire bench with protocol v3 and a `mailslurper-source-build`
  row. This branch adds no wire work, and the best-wait client stays
  blocked.)*
- No competitor cells outside the hard tier. No dsh/GLM support for
  competitors: spark is down, and the DashScope quota is out until
  2026-11-22.
- No `.github/` change. CI keeps its jobs. Upstream-backed proofs are
  ignored tests plus the committed controls receipts, as in #287.
- *(rerun 2)* The build is not reproduced in any gate: no docker, no Go
  toolchain and no module network in the gates. The official binary is not
  launched again; its 2026-11-07 failure is recorded as bytes.
- *(rerun; rerun 3: they are wire-bench targets under 0130, and they are
  still not agent-suite targets)* smtp4dev, Inbucket and GreenMail are not targets, even though
  their bytes are on the shelf. Adding one is a new product record and the
  operator's call. MailSlurper runs from its own release zip, not from
  Alkemio's image, whose directory is absent from this worktree.
- Real model-CLI negotiation against the bridge is not proved free. It is
  logged in the first paid session's trial artifacts, and a failure there
  is a visible `harness_error` row.

## Open items for the operator (not blocking the design)

- The dry-run reading (exit 0 on genuine prerequisites, with pending items
  printed) is ruled here under 0129 §9. Intake flagged it for confirmation.
- The `controls` scope (product record and executables only) is ruled
  here. Intake flagged it too.
- Acceptance of 0129 is the operator's. The PR is held regardless, because
  `scripts/compare-mail.sh` changes.

- **AC19 — review return 4 (2026-11-08): what the product record claims.**
  - The MailHog adapter offers `mailhog_list`, `mailhog_search` and
    `mailhog_wait` only. `mailhog_read` is neither offered nor parsed
    (`-32602`), because `GET /api/v1/messages/{id}` binds to no document the
    lock holds and to no reviewed source (0092 §3, 0129 §5). A test holds the
    offered names to those three, and the hostile-argument test sees no
    upstream request for a MailHog `read`.
  - The product record's `tools_sha256` for Mailpit, MailHog and MailSlurper
    each equal the digest of the tool list the adapter serves; a default test
    holds them. Mailpit's stays `77ee96eb…9eaa`.
  - `mailslurper_read`'s description says what the pinned executable does:
    an unknown id is HTTP 200 and an empty item, reported as `not_found`.
  - The product record's `order` is "target, then task, then agent, then
    attempt", the order `compare-mail.sh measure` runs (one matrix per
    target), and `comparison-check`'s dry-run plan prints that order. A test
    holds the plan lines (`slots 1-20 … on swarmail`, `21-40 recipient-crowd on
    swarmail`, `61-80 otp-backlog on mailpit`, `221-240 … on mailslurper`).
  - The competitors' `wait_method` states the 100 ms interval.
  - The controls are rerun on the head after these edits.
