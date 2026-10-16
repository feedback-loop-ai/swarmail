# Spec — prove and ship the fidelity closures (run 11 prove-and-ship pass: issues #39, #54, #55, #56, #57)

- **Run:** run 11 (`brokkr-state-run11-feature-txt-44aa2a76`) · **Phase:** design —
  the chief architect synthesizing the robustness and simplicity positions ·
  **Feature slug:** `fidelity-closures-prove-ship`
- **Issues:** [#39](https://github.com/feedback-loop-ai/swarmail/issues/39)
  (extraction fidelity, P1), [#54](https://github.com/feedback-loop-ai/swarmail/issues/54)
  (restored state matches live state), [#55](https://github.com/feedback-loop-ai/swarmail/issues/55)
  (Mailpit/MailHog compat fidelity), [#56](https://github.com/feedback-loop-ai/swarmail/issues/56)
  (thread normalization + web UI), [#57](https://github.com/feedback-loop-ai/swarmail/issues/57)
  (documentation and comment drift).
- **Base:** HEAD `24df038` — the implement seat's fidelity closures,
  operator-committed (28 files, +1579/−195, 440→468 tests) after all six
  gates were re-run at commit time. The implement attempt died mid-verify
  on an environment failure (the coverage gate's instrumented build filled
  /tmp's tmpfs); the commit carries the seat's content, not a re-run of it.
- **Design of record:** [specs/fidelity-closures/spec.md](../fidelity-closures/spec.md)
  (committed at `2cfe08c`, amended at `24df038`) with its
  [plan](../fidelity-closures/plan.md) and [tasks](../fidelity-closures/tasks.md).
  **This pass mints no new design vocabulary.** It commits the contract the
  remaining phases (verify, review, ship) prove and rule against: the red
  lines, the demonstrations, and the repair protocol.
- **Decisions:** [0031](../../docs/decisions/0031-sender-honest-stored-bodies.md)
  and [0032](../../docs/decisions/0032-emptied-inboxes-are-removed.md) —
  both **proposed**, filed with the batch at `2cfe08c`, implemented by the
  tree, awaiting the operator. **No new decision is minted by this pass**:
  every ruling below upholds the landed shape, and none amends a standing
  decision.

## The problem

The five closures are implemented and committed; what the phase machine owes
them now is proof, review and ship — not another design. This pass re-ran
the contrarian panel on the committed tree and both seats returned **pass**:
robustness re-grounded every argument at post-implement HEAD and endorsed
the landed shapes (the spine untouched, `MAX(seq)+1` and `(seq, rowid)`
restore, the deletion-reached emptied-inbox rule, the `IDs` alias narrowed
to spelling, the token-ordered UI, the asserted CSP, docs told at HEAD);
simplicity measured the implement commit and found it added zero paths,
kept one declared dependency, reused existing machinery for the one new
route, and landed every closure in the smallest priced shape — including
one cut nothing asked for (the dead POP3 `PASS` arm, deleted rather than
tested into existence).

A spec is still owed, because the phases that remain prove and rule against
a committed artifact, and the run's deterministic check rules on the
artifacts alone. What must not happen is the failure mode both positions
name: a successor seat re-deriving, re-arguing, or "improving" a landed
design — the first new vocabulary of the pass, growth without a criterion
behind it. This spec exists to bind the remaining work to the record:
criteria to demonstrate, red lines to hold, and a protocol for the case a
demonstration fails.

## The positions, ruled

Both positions agree on the skeleton — the landed design is spine work, the
record is sender-honest (0031), one emptied-inbox rule reached by deletion
(0032), refuse-loudly over plausible-wrong, and the refusal ledger stands.
Five things needed a chief; each is ruled, not averaged, with the losing
side's cost named.

**Ruling 1 — the spec of record stays; this document is the prove-and-ship
contract, not a successor design.** Simplicity: "no second spec or
synthesis document for this pass — the committed spec is the synthesis."
The substance is adopted wholesale: nothing below re-derives or extends the
landed design; the red lines and criteria are bindings and demonstrations,
not new mechanisms. The letter is refused, because the phase machine's
later seats work from a committed artifact and the deterministic check
rules on artifacts alone — a hand-off that lives only in `.brokkr/state`
is run-local evidence, not the record. **Cost of losing the letter:** one
committed document. **Cost of the alternative:** verify proving against an
unstated contract, or a review free to re-open a closed design.

**Ruling 2 — the fold is `str::to_lowercase()`, and the claim says what
the code does.** Robustness demanded a full Unicode case fold
("`to_lowercase` is not a fold — final sigma, dotted/dotless i");
simplicity priced std `to_lowercase()` with the residual documented, no
`icu_*`, no folding crate. **Adopted: the landed `to_lowercase()`.** The
framing's D1 criterion (`AW: Plan`, `RE : Plan`, `Über`/`über` → one key)
is green at HEAD (`the_subject_fallback_folds_unicode_case`); a total fold
needs a new dependency or hand-rolled tables for residuals the framing
never tests; and the README plus the doc comment now say
"Unicode-lowercased" and name the `ß`/`ss` residual — docs matching code
is #57's own truth discipline applied to this very claim. **Cost of losing
the fold:** `ß` vs `ss` (and the special-casing pairs) stay separate thread
keys for chain-less mails. The harm is bounded by construction — the
subject fallback groups orphans only and never bridges a References chain,
so the worst case is a misfile, never a lie about mail. A demonstrated need
is a one-sentence-defensible dependency conversation (license check
included), not a silent widen.

**Ruling 3 — the body-view document is governed by inheritance, and the
red line names the mechanism.** Robustness: "the CSP must also bind the
iframe document the body renders into, or remote images load anyway — the
test should prove the header on the route that serves the body view." The
tree has no separate body-view route: the preview is a sandboxed `srcdoc`
iframe (`src/ui.rs:138`) whose document inherits the embedding document's
policy container — the single CSP meta at `page()` governs it, as the code
comment records (`src/ui.rs:44–49`). **Adopted: the landed shape**,
robustness's demand met by inheritance rather than by a per-route header.
**Cost, named:** the defense is a browser behavior asserted only through
the shell's meta (`the_shell_ships_a_content_security_policy`); if a
future edit swapped srcdoc for a `src=` iframe, that test would stay green
while the defense vanished. Red line 8 therefore binds the preview to
srcdoc (or to its own policy container); any such swap is a review finding.

**Ruling 4 — no prefix-stripping loop.** Robustness wanted localized
prefixes stripped "with a loop bounded by the line length"; the tree strips
**one** leading marker (`strip_marker`, `src/threads.rs:105`), which is
what the design of record priced. **Adopted: the single strip.** **Cost of
refusing the loop:** a chain-less `Re: Re: Plan` keys as its own thread —
the same bounded misfile class as Ruling 2, never a lost or lying record.
A loop is additive; it needs its own criterion and test and is not
smuggled in by a repair.

**Ruling 5 — the CDLA-Permissive-2.0 allowance stays, loudly.** The house
rules name seven licenses; the tree needs an eighth for `webpki-roots`
(rustls's offline root store, via reqwest's TLS leg): removing it fails
the license gate, and the alternative (`rustls-native-certs`) is a TLS
behavior change, not a drop-in. **Adopted as landed** — the allowance with
its inline comment naming the crate, recorded in
`.brokkr/state/run11-operator-edits.md`: E3's own escape hatch, never a
silent widen. The house-rules sentence is the operator's to amend. **Cost
of losing the letter of the prior AC-57.2** (identical allowlists): one
documented exception, named in three places. **Cost of the alternative:** a
red license gate or a behavior change in the TLS leg.

Where the positions agree, the synthesis is one sentence: **the landed
design is the simplest correct design on the table, and the cheapest
remaining work is proof.** Robustness's ten refusals (no await in the
boundary, no durable outbox, no restore-time pruning, no seq-gap
reclamation, no server-side retry, no ingest sanitization, no source-wide
decode, no parse timer, no selector softening, no checked-in certs) and
simplicity's residual ledger become this pass's red lines; simplicity's
red lines become verify's and review's bindings.

## The design of record and the red lines

The design is [specs/fidelity-closures/spec.md](../fidelity-closures/spec.md)
§The design — A1–A4 (#39), B1–B3 (#54), C1–C3 (#55), D1–D4 (#56), E1–E5
(#57) — as implemented at `24df038`. Nothing below re-states it; this is
what the later phases may not move:

1. **No re-derivation.** The committed spec is the design of record; verify
   proves criteria on the tree and does not re-open them.
2. **No new files in a repair.** A false criterion is repaired inside the
   file that owns it — inline `#[cfg(test)]` and measured (0020 rule 1); a
   `src/<module>/<name>_tests.rs` harness only when a concrete cold path
   demands it, parent-declared, harness names reserved.
3. **No new dependency, route, or decision document in a repair** — unless
   the repair is genuinely semantic, and then exactly one `proposed`
   decision, per the charter.
4. **The stored record stays sender-honest** (0031): no synthesis accessor
   returns; rendering stays confined to the named sites (`src/compat.rs:560`,
   `:801`, and the codes haystack in `src/extract.rs`). A fourth render
   site is extraction wearing a costume.
5. **The restore stays `MAX(seq)+1`-seeded, `(seq, rowid)`-ordered,
   index-backed.** No persisted counter row, no format bump, no version
   gate. Sequence gaps stay honest: gaps forever, duplicates never.
6. **The emptied-inbox rule stays reached by deletion** (0032); no
   tombstone vocabulary; the watcher release moment does not move (0021).
7. **Refusal discipline stands:** 0011's explicit-selector-or-400 survives
   the `IDs` alias verbatim; every refusal keeps 0018 §3's sentence pattern
   and 0025's naming; supplied values are validated wherever they reach a
   wire (0029).
8. **The view's defenses stay minimal:** the CSP is one string at `page()`;
   the body preview stays a sandboxed srcdoc iframe (or gains its own
   policy container) per Ruling 3; the paint token stays four lines — no
   abort plumbing, no middleware layer.
9. **The spine is untouchable** (robustness's ledger, verbatim): no await
   in the mutation boundary; no durable webhook outbox; no restore-time
   pruning; no seq-gap reclamation on commit failure; no server-side
   commit retry; no ingest sanitization or re-serialization; no
   source-wide entity decode; no parse timer; no `Option`-softening of
   selector refusals; no checked-in certificate fixtures.
10. **The gate stays exact and disk-backed:** missed == 0 on lines,
    branches and functions; no `coverage(off)`; no harness trickery; the
    instrumented run's `TMPDIR` is `/home/valentin/scratch/cov-tmp` —
    never tmpfs again.
11. **Review findings are line- and sentence-level.** "Over-built" names a
    line to delete; it does not re-argue the design. The do-not-redo rule
    binds reviewers harder than anyone else.

## Acceptance Criteria

The tree is committed: these are **demonstrations**, not re-derivations.
Each names its proof site; a demonstration that fails is a review finding
repaired per [plan.md](plan.md)'s protocol — minimal, test-covered, in the
owning file — never a redesign. All commands run at the repository root;
the coverage gate's `TMPDIR` rule is absolute (AC-G.2).

### The gates

- **AC-G.1** `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace --locked` and
  `brokkr compile --bundle bundles/self` all exit 0; the suite is 468
  tests. *Proof:* the commands themselves.
- **AC-G.2** `TMPDIR=/home/valentin/scratch/cov-tmp bash
  scripts/coverage-gate.sh` reports exact 100/100/100 — lines, branches
  and functions, missed == 0 — with no `coverage(off)` anywhere in `src/`.
  *Proof:* the gate's own output, run with the disk-backed `TMPDIR`,
  never on tmpfs.
- **AC-G.3** `cargo deny check licenses` is green, and `deny.toml` names
  the house rules' allowlist plus exactly one recorded exception
  (CDLA-Permissive-2.0 for `webpki-roots`, inline comment + operator
  note). *Proof:* the gate plus the allowlist diff against
  `docs/house-rules.md`.
- **AC-G.4** The implement commit added no path:
  `git diff --name-status --diff-filter=A 2cfe08c..24df038` is empty; its
  only declared dependency is `percent-encoding = "2.3.2"` (MIT OR
  Apache-2.0, already resolved via `form_urlencoded`); its only new route
  is `/api/v1/message/latest`. *Proof:* the three greps.

### #39 — extraction honesty

- **AC-39.1** Real-SMTP mail with `&amp;` and `&#38;` inside hrefs stores
  the decoded URLs in `links`; the text/plain leg stays byte-honest.
  *Proof:* `tests/api.rs::smtp_html_hrefs_decode_character_references`;
  `src/extract.rs::named_refs_decode_per_url`.
- **AC-39.2** The decoder is total and never grows its input: unknown
  names, malformed numerics, surrogates and > U+10FFFF stay literal.
  *Proof:* `src/extract.rs::decode_refs_is_total_and_never_grows`,
  `::numeric_refs_decode_and_unknown_names_stay_literal`.
- **AC-39.3** `links` and `codes` keep first-occurrence document order
  while deduplicating. *Proof:* `src/extract.rs::links_keep_document_order`,
  `::codes_keep_document_order`.
- **AC-39.4** Codes come from rendered text only — `<style>` blocks and
  attribute values yield nothing. *Proof:*
  `src/extract.rs::codes_ignore_style_script_and_attributes`,
  `::codes_render_from_visible_html_text`.
- **AC-39.5** The record is sender-honest: a text-only mail answers
  `html: null` and an html-only mail `text: null` across REST, MCP,
  Mailpit and MailHog; `raw` holds the sender's bytes; the two 0031 §3
  display fallbacks still serve a text view for html-only mail and derive
  no links or codes from it. *Proof:*
  `src/smtp.rs::bodies_hold_only_what_the_sender_sent`,
  `tests/mcp.rs::seed_with_both_bodies_stores_html_and_drops_text_over_mcp`,
  `tests/compat.rs::mailpit_message_plain_and_headers_endpoints`.

### #54 — restored state matches live state

- **AC-54.1** Open → insert → reopen → insert → reopen → insert yields no
  duplicate `seq`. *Proof:*
  `src/persist.rs::the_first_insert_after_a_reopen_never_reuses_a_seq`.
- **AC-54.2** Restore is `(seq, rowid)`-ordered and index-backed, including
  a hand-built legacy duplicate-seq fixture. *Proof:*
  `src/persist.rs::a_legacy_duplicate_seq_restores_in_write_order`,
  `::the_restore_scan_reads_the_seq_index`.
- **AC-54.3** An inbox emptied by `delete` is absent from the listing live
  and after a reopen; `clear` and restore follow the same rule. *Proof:*
  `tests/persistence.rs::an_emptied_inbox_is_absent_from_the_listing_live_and_after_a_reopen`,
  `::deletes_and_clears_survive_a_restart`.
- **AC-54.4** A vanished maildrop is refused at `PASS` and after a reopen,
  exactly like a never-seen one. *Proof:*
  `tests/pop3.rs::a_maildrop_vanished_between_user_and_pass_is_refused_at_pass`,
  `::a_vanished_maildrop_is_refused_after_a_reopen_too`.
- **AC-54.5** Old data files still open: the index creation is idempotent,
  no version gate fires, and `Store::insert` stays synchronous. *Proof:*
  `src/persist.rs::open_fails_on_an_unusable_file`,
  `src/store/persist_tests.rs::open_refuses_an_unusable_data_file`,
  `src/persist.rs::a_pre_thread_data_file_migrates_and_keeps_its_mail`.

### #55 — compat fidelity

- **AC-55.1** `{"IDs": ["…"]}` deletes exactly the named mails; absent,
  empty and wrong-typed selectors still refuse with 0011's named 400.
  *Proof:* `tests/compat.rs::the_ids_alias_deletes_and_a_wrong_typed_one_still_refuses`,
  `::shim_deletes_go_through_the_store`.
- **AC-55.2** An envelope-only recipient surfaces in every compat shape
  (Hog `To`/`Raw.To`, `kind=to` search, Mailpit `Bcc`); header to/cc stay
  in `Content.Headers`. *Proof:*
  `tests/compat.rs::an_envelope_only_recipient_surfaces_in_every_compat_shape`.
- **AC-55.3** `limit=0` pages the default on every compat surface;
  over-cap still refuses (0030 §3). *Proof:*
  `tests/compat.rs::a_zero_limit_pages_the_default_on_every_compat_surface`,
  `::the_shim_pages_refuse_over_the_cap_and_honor_the_boundary`.
- **AC-55.4** `/api/v1/message/latest` serves the newest mail and 404s only
  on an empty store. *Proof:*
  `tests/compat.rs::the_latest_route_serves_the_newest_mail_and_404s_an_empty_store`.
- **AC-55.5** A `to:`-prefixed query is refused by name (0018 §3's
  sentence, 0025's naming), never silently empty; Mailpit
  `total`/`unread` report mailbox totals. *Proof:*
  `tests/compat.rs::a_to_prefixed_query_is_refused_by_name`,
  `::mailpit_search_supports_each_kind_and_scopes`.

### #56 — threads + web UI

- **AC-56.1** `AW:` and a padded `RE :` strip; `Über`/`über` share one
  thread key; the `ß`/`ss` residual is stated in the doc comment and README
  (Ruling 2). *Proof:* `src/threads.rs::the_aw_marker_and_a_padded_colon_strip`,
  `::the_subject_fallback_folds_unicode_case` plus README inspection.
- **AC-56.2** An `&` title is escaped exactly once (no `&amp;amp;`).
  *Proof:* `tests/ui.rs::an_ampersand_title_is_escaped_exactly_once`.
- **AC-56.3** An inbox named `a#b` links to `/ui/inbox/a%23b` and resolves.
  *Proof:* `tests/ui.rs::a_hash_inbox_name_is_url_encoded_in_the_href`.
- **AC-56.4** A missing message answers 404, not a 200 card. *Proof:*
  `tests/ui.rs::message_view_renders_text_html_and_missing`.
- **AC-56.5** The served shell carries the CSP, and the body preview is a
  sandboxed srcdoc iframe inside that policy container (Ruling 3).
  *Proof:* `tests/ui.rs::the_shell_ships_a_content_security_policy`,
  `::ui_is_escaped_not_injected`, plus `src/ui.rs:138` inspection.
- **AC-56.6** The served asset carries the paint token; a stale `load()`
  cannot paint over a newer SSE frame. *Proof:*
  `tests/ui.rs::the_live_shell_guards_paints_with_a_token`.

### #57 — the record

- **AC-57.1** Every decision-index row matches its file's recorded status,
  and the header has four columns. *Proof:* the row-by-row comparison
  shipped as review evidence.
- **AC-57.2** The license allowlists agree except the one recorded CDLA
  exception (Ruling 5). *Proof:* the allowlist diff +
  `cargo deny check licenses`.
- **AC-57.3** CONTRIBUTING.md, README.md and openapi.json match the code at
  `24df038`; the E5 comments match the code they describe. *Proof:*
  inspection per the design of record's E4/E5 lists; openapi.json greps.

### Review bindings

- **AC-R.1** 0031 §3's display-rendering carve-out is review's named item:
  the only render sites are `src/compat.rs:560`, `:801` and the codes
  haystack in `src/extract.rs` — no fourth site exists. *Proof:* a review
  count of `html_to_text` call sites in `src/`.
- **AC-R.2** The review panel's verdicts are recorded; a security-hold is a
  hard stop (the constitution's protected gate). *Proof:* the run journal.
- **AC-R.3** The frozen three are re-proved by the standing suites,
  unchanged from `24df038` unless a criterion above failed: accept ==
  stored (`tests/burst.rs::the_burst_guarantee`,
  `::a_cap_sized_data_is_stored_as_soon_as_the_250_leaves`),
  subscribe-before-check (`tests/burst.rs::await_waits_for_arrival`
  family), oldest-only pruning across a restart
  (`tests/persistence.rs::a_refused_insert_is_never_exposed_as_accepted_mail`
  cap legs). *Proof:* those suites.
