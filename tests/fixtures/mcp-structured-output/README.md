These raw HTTP bodies / stdio lines were captured before the #153 runtime edit
from the dc67f4a runtime (design-only descendants), using the legacy_bytes
all-tool workflow in tests/mcp_revisions/structured.rs. All four runs (missing
and explicit March HTTP selection, real binary bridges) returned identical bytes.
Only the seed reply's single generated UUID is replaced by $SEED_ID, after
asserting its occurrence count. Restored Email fixtures have fixed IDs and times;
no expected JSON is parsed and reserialized. Lines retain the original escaping,
key order, optional omissions and nullable historical envelope sender.

## march-current-main.jsonl (thirteen tools, added 2026-10-28)

The current integration serves thirteen tools, so `march.jsonl` above stays as
the historical twelve-tool record and is no longer the live March comparison.
`march-current-main.jsonl` was captured from the unmodified local pre-#153
#152 candidate 3a2eef9 (tree ce2e41e), built and run inside the worktree with a
test-only capture harness (`capture_march_current_main.rs.txt`) driving the
case list in `march-current-main.cases.json`. Missing and explicit March over
HTTP and over the real binary bridge again returned identical bytes. Commit,
tree, build invocation, executable digests before/after, request log and the
single `$SEED_ID` substitution are recorded in
`march-current-main.provenance.json`. 3a2eef9 is a local baseline candidate,
not a protected #152 acceptance receipt.

Two values in that file have moved by hand since the capture, each recorded in
the provenance `updated` field: the report descriptor wording adopted from main
(#151/#286) and, on the seed report row, `rule_set_id` from `swarmail-bulk/1` to
`swarmail-html/3` (#259), on 2026-11-03 to `swarmail-spam/1` (#260, proposed 0122), on 2026-11-05 to `swarmail-html/4` (#258, proposed 0121, merged after #260), on 2026-11-05 to `swarmail-html/5` with the report's new `dns` member at its default (#253, proposed 0120, merged after #258), and on 2026-11-05 to `swarmail-html/6` (#254, proposed 0125, merged after #253), when the provenance `fixture_sha256` was recomputed. On 2026-11-06 (#255, proposed 0126) the same row moved to `swarmail-html/7` and, unlike the earlier edits, gained findings and lookups, because the report now carries real SPF and DMARC evidence; it was replaced by the report this tree serves for the same seeded mail, again by hand and not by re-capture, and `fixture_sha256` was recomputed. On 2026-11-07 (#337, decision 0122 §2) the same row moved to `swarmail-html/8` and gained a `weight` on every finding and the report's `total` (both last in their objects, so in MCP's sorted-key order after every key that was already there), again by hand from the served bytes and not by re-capture, and `fixture_sha256` was recomputed. The fixture was not re-captured. The baseline pins the
March envelope, order, escaping and text-only shape around the report; the
report's content is versioned by that id (proposed 0085) and every adapter
serves the same report (proposed 0105, 0106). See the note in proposed 0107.
