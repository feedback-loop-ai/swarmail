# 0120 — A declared, offline DNS policy that every deliverability report records

- **Status:** proposed
- **Date:** 2026-11-03; revised 2026-11-04 after the review of `e80aa822`
  returned the design (run `issue-253-the-deliverability-dns-c76ebc80`). The
  revision corrects §2, §4, §8 and §9 and one alternative below; the
  specification's review return adjudication lists each finding.
- **Scope:** epic #251, issue #253 (operator direction 2026-11-03: "start the
  remaining deliverability stories"). Run
  `issue-253-the-deliverability-dns-c704444e`, from `main` `8732445`, on the
  `claude-careful` recipe. The dispatcher assigned the number 0120. No other
  number is used.
- **Specification:** [deliverability DNS policy](../../specs/deliverability-dns-policy/spec.md).
- **Amends, only if accepted:** the normative report shape of accepted
  [0085](0085-deliverability-report.md) §1, which is
  [the contract specification](../../specs/deliverability-report-contract/spec.md#contract)'s
  literal `{rule_set_id, findings}`. A report becomes
  `{rule_set_id, findings, dns}` and `RULE_SET_ID` advances from
  `swarmail-html/3` to `swarmail-html/4`, which is the version rule of 0085
  §1 applied to a shape change. Under accepted 0045 §8 the report-shape part
  waits for the operator's acceptance of this record. **The delivery PR is
  held for the operator. It is not merged on green.**
- **Fulfils without amending:** 0085 §8 and the contract spec's "#253
  extension seam". #253 supplies an explicit immutable policy snapshot to
  the pure collector, puts its full identity in the cache key, and owns
  expiry, refresh and wake semantics. This record states those semantics.
- **Preserves:** [0001](0001-synchronous-insert-lossless.md),
  [0002](0002-awaits-subscribe-before-checking.md),
  [0003](0003-oldest-only-pruning.md) and 0005. Nothing in this decision
  runs on the SMTP path, at ingest, inside the Store's mutation boundary, or
  in the data file. Also preserves 0085 §3–§7: lazy analysis, the 128-slot
  Weak-identity cache, the four job permits, the 256-finding and
  65,536-byte limits, and the freeze of inputs for one request.
- **Dependencies:** none. No TXT resolver exists in the tree, and nothing
  in this story looks up real DNS. `Cargo.toml`, `Cargo.lock`,
  `evals/Cargo.lock` and `deny.toml` are unchanged. The operator's #251
  ruling of 2026-10-26 lets #254 widen the allowlist once for an existing
  permissive crate such as `mail-auth` when it brings the resolver. This
  story does not take that widening.

## Decision proposed

1. **One Swarmail-owned policy type, offline by default.** A DNS policy is
   a declared TXT map. It holds owner names, each with a nonempty set of
   TXT values, and a set of owner names declared `tempfail`. A name may not
   appear in both. With no configuration the policy is the empty map,
   reported as source `default`. A server map comes from
   `serve --dns-policy FILE` (env `SWARMAIL_DNS_POLICY`) and is reported as
   source `server`. A per-inbox map comes from
   `PUT /api/v1/inboxes/{inbox}/dns` and is reported as source `inbox`. An
   inbox map **replaces** the server map for that inbox. There is no merge.
2. **One declaration shape, one parser, one bound.** The file, the API body
   and the report's `dns.policy` all use one JSON shape:
   `{"txt":[{"name":…,"values":[…]}],"tempfail":[…]}`. Both members are
   optional on input and always present on output. Unknown members are
   refused. The two inputs share one parser.
   - One constant, `MAX_DNS_POLICY_BYTES` = 8,192, bounds a declaration's
     **raw** bytes (the file, and the API body, where over it is `413`) and
     its **canonical** serialization, checked after folding and sorting
     (over it is refused, `400` on the API). Canonicalization is
     idempotent, so a report embeds at most 8,192 policy bytes and every
     report's `dns.policy` can be declared back verbatim.
   - *Corrected 2026-11-04.* This record first said the raw bound "also
     caps what a report embeds". It did not: the canonical form always
     spells both members, so an accepted 8,192-byte input that omitted one
     became an 8,205-byte canonical form that a report embedded and the
     API then refused. The bound is now measured on the embedded form too.
   - At most `MAX_DNS_POLICY_ENTRIES` = 256 names and values together:
     every `txt` owner, every value and every `tempfail` name counts one.
     It bounds the parsed tree, which the byte bound does not (§8).
   - Owner names are ASCII. They are folded to lowercase and one trailing
     dot is stripped. Each label is 1–63 bytes of `[a-z0-9_-]`, and the
     whole name is 1–253 bytes. There is no IDNA and no wildcard
     expansion: a declared name matches only itself.
   - A TXT value is the record's logical value, its character-strings
     already concatenated. Values are exact printable ASCII
     (`0x20`–`0x7E`). They are never trimmed or case-folded, because
     DKIM and SPF values are byte-sensitive.
   - These declarations are refused, never repaired: two entries whose
     names fold to one owner, a duplicate value under one owner, an empty
     `values` list, a name in both `txt` and `tempfail`, a name or value
     outside the grammar, raw or canonical bytes over the bound, and more
     than 256 names and values.
   - The canonical form sorts names by byte order and sorts each owner's
     values by byte order. Insertion order never reaches a report.
3. **The lookup interface #254/#255 plug into is concrete, not a trait.**
   The type is a `Lookups` recorder, built per evaluation over the frozen
   snapshot. Its method is
   `resolve_txt(&mut self, &OwnerName) -> Txt`, and the outcomes are
   `Records(canonical values)`, `NoRecords` and `TempFail`. The method is
   synchronous and pure: no network, clock, resolver, global or lock. It is
   the only path that answers, and it records every name it answers, so a
   verifier cannot read DNS without the report saying so. Header-derived
   names reach it only through `OwnerName::parse`. An invalid name is the
   verifier's verdict to give and is never looked up. The contract spec
   forbids a registry or plug-in trait, and there is one implementation,
   so no trait ships.
4. **Outcome semantics bind the verifiers.** `NoRecords` is a definite
   negative answer (NXDOMAIN and NODATA are not distinguished). `TempFail`
   is an operational failure. It is **never a pass and never a clean
   result**. A #254/#255 rule that meets it owes a `not_evaluated` finding
   or an analysis failure, the same discipline `AuthEvidence::Unknown`
   follows today. A declared `tempfail` is the only way to inject that
   outcome, which keeps it at the poles under 0020 §5.
   - *Enforced (2026-11-04).* `Report::qualifies` returns false at every
     threshold when any recorded lookup's outcome is `temp_fail`, whatever
     the findings say, so a verifier that forgets its `not_evaluated`
     finding still cannot make a temporarily failing world clean. An exact
     assert never matches such a report, and a wait with a threshold does
     not return it; it reaches 0085's existing deadline outcome unless
     another candidate qualifies. This is the mechanical form of the
     sentence above. Production `lookups` is empty until #254, so nothing
     observable changes in this story. The changed qualify semantics are
     part of this proposal and wait for the operator with the rest of it.
5. **Every report records the policy and the records it used.** `Report`
   gains one required member:
   `"dns":{"source":"default"|"server"|"inbox","policy":{"txt":[…],"tempfail":[…]},"lookups":[{"name":…,"type":"txt","outcome":"records"|"no_records"|"temp_fail","values":[…]}]}`.
   - `policy` is the **complete effective policy**, canonical, not only the
     names used. Under this story no production rule resolves a name, so
     `lookups` is always empty in production. A report that carried only
     the source or only the used names could not show a content change, and
     could not reproduce a verdict.
   - `lookups` is sorted by name. Each name appears once, with the outcome
     and the values it returned.
   - Neither the file path, a timestamp nor a generation enters the report.
     The same message under the same configuration gives byte-identical
     report JSON across processes and working directories.
   - `Report` keeps `deny_unknown_fields`, and `dns` has no serde default.
     The existing 65,536-byte report bound covers the whole object. A
     report over it is `AnalysisError::Failed`, never truncated.
6. **Cache identity and snapshot.** The effective policy (source plus
   canonical content) is a member of the typed `Inputs` and therefore of
   0085's exact cache key. A changed declaration can never reuse an
   old-policy report. An unchanged one reuses its slot. No invalidation
   code exists.
   - Each request resolves the effective policy **once**, from the
     message's inbox, at its entry: one message report, one exact assert,
     one REST await or MCP wait. Every candidate and every wait iteration
     in that request uses the same snapshot. For waits, the snapshot is
     taken before the subscribe. 0002's subscribe-then-scan order is
     unchanged, because a policy is configuration, not mail.
7. **Expiry, refresh and wake (owned here per 0085 §8).**
   - Declared maps never expire.
   - Refresh is a new declaration (`PUT`, `DELETE`, or a restart with a
     changed file).
   - A declaration change wakes no waiter. An in-flight request finishes
     under the snapshot it recorded, and the next request sees the change.
   - Live-answer TTL, refresh and wake semantics are deferred to the story
     that introduces live answers (#254), which must define them before it
     lands.
8. **Per-inbox declarations are bounded, process-local configuration.**
   - They live in the Store-owned deliverability service, under one short
     mutex that is never held across an await or a Store guard.
   - At most 4,096 inboxes hold a declaration. A `PUT` that would add a
     4,097th is refused `503`, and no live declaration is evicted.
     Replacing an existing inbox's declaration at capacity succeeds.
   - A `PUT` label may be at most `MAX_DNS_INBOX_LABEL_BYTES` = 512 bytes
     (RFC 5321's command-line length); a longer one is `400` before the
     body is parsed. Such an inbox can hold mail but not a declaration.
   - *Resident cost, stated (2026-11-04).* The inbox cap counts
     declarations and the byte bounds count bytes; neither bounds the
     parsed tree, which is why §2's entry count exists. One declaration
     holds at most 8,192 bytes of strings plus at most 256 strings and
     tree slots of overhead (about 25 KiB) plus a key of at most 512
     bytes: under 34 KiB. A completely full registry is under 140 MiB,
     from at most about 34 MiB of request bytes, and the 128 report-cache
     slots may keep one replaced snapshot each alive (under 5 MiB). The
     memory is process-local, released on `DELETE` or restart, and inside
     the accepted model of an unbounded mail store on a test server
     (0038). The route stays unauthenticated under the plane-wide Origin
     guard, like every inbox route (0004).
   - A `PUT` validates fully before effect. An invalid body is `400`, an
     oversized body is `413`, and either leaves the previous declaration
     exactly as it was. A `200` means the declaration is in effect for the
     next request. The body echoes `{inbox, source, policy}`.
   - `DELETE` is idempotent. It returns `200` with the resulting effective
     `{inbox, source, policy}`. That echo is the fallback itself, the
     server map or else the default, returned without reading the inbox
     declarations again after the removal. A `PUT` racing the `DELETE`
     therefore cannot make the echo name `source` `inbox`; it is the next
     state, and the next request sees it.
   - Declarations are not persisted and do not survive a restart. Every
     report names its source, so that reversion is visible, never silent.
     The server file is the durable policy.
   - Clearing or deleting an inbox's mail does not touch its declaration.
   - There is no `GET`: the `PUT`/`DELETE` echo and every report show the
     policy.
   - There is no MCP setter, and the UI does not render `dns`.
9. **The server file is checked before any listener binds.**
   - `run_on` loads the `--dns-policy` file first, before the data file
     opens and before SMTP, POP3, HTTP or MailSlurper bind.
   - The file must be a regular file (a FIFO, device or directory is
     refused). It is read through a bound of `MAX_DNS_POLICY_BYTES + 1`, so
     `/dev/zero` cannot exhaust memory. It must be valid UTF-8 and a valid
     declaration.
   - *Accepted residual (2026-11-04).* The regular-file check reads the
     path's metadata and then opens the path. A FIFO swapped in between
     the two calls would block startup on the open. Closing that needs a
     non-blocking open, which needs `libc` as a production dependency;
     this story takes no dependency, and the epic's one widening belongs
     to #254. The path is operator configuration, so the swap needs write
     access to a path the operator chose, and the effect is a startup that
     hangs, not a bypass or a false-clean report. The code comment says
     the same rather than claiming a FIFO cannot block startup.
   - Any refusal makes `serve` exit non-zero with
     `dns policy file <path>: <reason>`. It never falls back to the
     default, because an empty map would make every later DKIM/SPF/DMARC
     check falsely clean.
   - An explicit `{}` is a valid, empty server map, reported as `server`.
10. **Real DNS is configured and refused.**
    - `--dns-resolver <declared|real>` (env `SWARMAIL_DNS_RESOLVER`)
      defaults to `declared`. The value `real` is refused by the argument
      parser with the fixed message
      `real DNS needs the resolver that #254 adds; this build has none`.
      Swarmail therefore exits before any listener binds or any lookup
      can happen.
    - Real mode is server configuration only. The API can never select it.
    - No `Real` policy variant exists in this story. The unreachable state
      is unrepresentable, not guarded.
    - #254 owns the resolver. It must resolve **before** `evaluate`, with
      its own deadline and outside the four report permits, and then hand
      the evaluator an immutable recorded snapshot of the same shape. It
      takes the `mail-auth` widening under the 2026-10-26 ruling, and it
      adds the policy variant and its report source together.
11. **No rule ships.** #253 owns no DKIM, SPF, DMARC or ARC verdict, so no
    rule id is added, and the passing/failing-fixture requirement holds
    vacuously. `bulk.dmarc_presence` and its siblings keep reading
    `AuthEvidence`, which stays `Unknown` until #255. A declared `_dmarc`
    record does not satisfy them in this story.

## Alternatives ruled and costs

The 2026-11-03 design panel argued two positions. The specification's
[panel adjudication](../../specs/deliverability-dns-policy/spec.md#panel-adjudication--2026-11-03)
lists every ruling. The ones that shape this record:

- **Full policy in the report** (robustness) over used lookups only
  (simplicity). With no rule in this story, used-only reports are constant
  under every map, so a policy change is unobservable and a verdict cannot
  be reproduced. *Cost:* reports grow by up to 8 KiB when a map is
  declared, and the declaration bound drops from the intake's suggested
  64 KiB to 8 KiB.
- **One byte constant** (simplicity) over six count and length limits
  (robustness), *corrected 2026-11-04*. The constant bounds the raw and
  the canonical form; round one's claim that the raw bound alone capped
  the embedded form was false. One combined entry count (256) is added,
  because the byte bound does not bound resident memory. Robustness's two
  byte constants (raw 16,384, canonical 8,216) were declined: they close
  the round trip no better and put a second number on every surface.
  *Cost:* refusals name "too large" or the entry count rather than which
  per-owner limit was exceeded, and an input within 24 bytes of the bound
  that omits a member is refused although it fitted on the wire.
- **Concrete recorder, no `Real` variant** (simplicity) over a fail-closed
  `Real` arm in the evaluator (robustness). *Cost:* #254 must add the arm
  together with its resolver. There is no pre-built defensive branch to
  inherit.
- **Bounded declarations refusing at capacity** (robustness) over an
  unbounded map (simplicity). *Cost:* one constant, and a `503` that
  a 4,096-inbox test must prove.
- **Pre-bind load in `run_on` from a `Config` path** over loading inside
  the clap value parser (simplicity). This keeps every DNS type
  crate-private and orders the load like the data file. *Cost:* each
  `Config` struct literal in the tests gains one `dns_policy: None`
  member.
- **No split delivery.** Robustness offered landing the source and the
  configuration first and the report member later. That is refused,
  because the report's record of the policy is the issue's purpose and a
  source nobody can observe is dead weight. *Cost:* nothing of #253 merges
  before the operator accepts this record.

## Proof and disposition

The specification's acceptance criteria and its paired tasks hold the
proofs: unit tests for the parser, the name grammar and the three
outcomes; real HTTP and SMTP proofs in `tests/deliverability.rs`; a
two-process byte comparison; refusal proofs for the file and for real
mode; a cache-identity evaluation count; the canonical-bound round trip;
a real `PUT` during a held evaluation; the `temp_fail` qualify fixtures;
the entry and label bounds; and every house gate,
`--locked`, with exact coverage. Sibling stories #258 and #260 also
advance `RULE_SET_ID`. Whichever merges later takes the next version. That
is a one-line integration fix and does not reopen this design. This record
stays proposed. Only the operator accepts it.

## Integration note — 2026-11-05 (merge of `main` after #258 and #260)

`main` now carries `swarmail-html/4` for #260's spam signals (proposed
[0122](0122-deterministic-spam-rules.md)) and #258's link rules (proposed
[0121](0121-deliverability-link-checks.md), see its 2026-11-05 note), while
this branch had taken `swarmail-html/4` for the `dns` member. The two
reports differ in bytes, and `main`'s `/4` is no longer never-shipped. Under
the last paragraph above, 0121 §5 and 0122 §1.4, the branch integrating
second takes the next never-shipped id: the merged report carries the spam
signals, the link rules and the `dns` member under `swarmail-html/5`. Where
§1 and §5 name `swarmail-html/4`, read `swarmail-html/5`. No rule id,
severity, hint or location of any story changes, and the `dns` member's
shape, sources, bounds and qualify semantics are as recorded above. 0121's
opt-in link reachability probe resolves link hosts' addresses outside the
evaluator when an operator enables it; it reads no TXT record and neither
consults nor changes this policy, and both default to no network access.
This note changes no status; the record stays proposed.

## Integration note — 2026-11-05 (proposed 0125, issue #254)

Proposed [0125](0125-offline-dkim-verification.md) is the first verifier to
call `Lookups` from production code. It moves three things this record said,
and changes no status; the record stays proposed.

- **§4/§5, "production lookups are empty".** They are no longer empty. A
  message with a `DKIM-Signature` records every key, ATPS and
  `_report._domainkey` name the DKIM verifier asks for, once, sorted, with
  its outcome. A message with no signature still records none. The `dns`
  member's shape, the sources and the bounds are unchanged, and a report
  with a `temp_fail` lookup still never qualifies. The recorder is fed
  through a cache adapter inside the evaluator, so the report remains a
  function of the stored bytes, the receipt second and the frozen policy
  snapshot.
- **§7, live-answer semantics.** The wake and expiry story stays with #255.
  What moves is the resolver wording: a name that is outside the owner-name
  grammar is never looked up and never recorded, and the verifier's own
  resolver is compiled in but unreachable (the adapter answers every name,
  and the one authenticator has no name servers). The one place the report
  can differ between two evaluations of the same bytes is 0125 §8's clock
  residual, a proposed amendment of accepted 0085 §8 held for the operator.
- **§10, "real DNS is configured and refused".** The `declared` default
  stands. The refusal text does not: its fixed message ('real DNS needs the
  resolver that #254 adds; this build has none') is corrected by 0125 §3 to
  'real DNS is not supported; declare records with --dns-policy or PUT
  /api/v1/inboxes/{inbox}/dns', because #254 adds no resolver that answers. The promise that a later story adds a `Real`
  variant moves to a later story with its own decision; 0125 adds none, and
  no network lookup exists in this build.

The `swarmail-html/6` id belongs to 0125 (it has never shipped), after this
record's `/5`.

## Integration note — 2026-11-06 (proposed 0126, issue #255)

Proposed [0126](0126-spf-dmarc-alignment.md) is the second verifier to call
`Lookups` from production code, and it changes two things this record said. It
changes no status; the record stays proposed.

- **§3, the declaration grammar.** The declaration gains one optional member,
  `spf_client_ip`: an IPv4 or IPv6 address literal, stored canonical (an
  IPv4-mapped IPv6 address is the IPv4 it maps) and present in `dns.policy`
  only when declared, so a policy that does not declare it keeps the bytes it
  had. It is the address SPF evaluates instead of the one the SMTP session
  saw, for every peer, and it never makes a message with no observed peer
  evaluable. Anything but an address literal is refused by the one parser
  (`400` on the API, a startup refusal for the server file, before any
  listener binds). It counts toward the byte bounds (the canonical form
  is measured) and not toward the 256 names and values.
- **§4, non-TXT answers.** The map stays a TXT map. SPF's `a`, `mx`, `ptr` and
  `exists` mechanisms and the `%{p}` macro need A, AAAA, MX or PTR answers it
  cannot hold; they are answered with a fixed negative and a named finding
  whose fact is Unknown, never a failure (0126 §4). DMARC's one non-TXT
  query, the A query that decides `np=`, is answered by a fixed rule over the
  declared names (a name exists when it or any name below it is declared as a
  TXT owner or in `tempfail`). `dns.lookups` still lists TXT answers only
  (`type` is `txt`), and each non-TXT answer is reproducible from the
  embedded policy.

The report's byte-identity claim (§5) holds as 0125 §8 states it, plus one
more stated exception proposed in 0126 §8: SPF's monotonic lookup timer inside
the crate, a proposed amendment of accepted 0085 §8 held for the operator.
The `swarmail-html/7` id belongs to 0126 (it has never shipped), after
0125's `/6`.
