These raw HTTP bodies / stdio lines were captured before the #153 runtime edit
from the dc67f4a runtime (design-only descendants), using the legacy_bytes
all-tool workflow in tests/mcp_revisions/structured.rs. All four runs (missing
and explicit March HTTP selection, real binary bridges) returned identical bytes.
Only the seed reply's single generated UUID is replaced by $SEED_ID, after
asserting its occurrence count. Restored Email fixtures have fixed IDs and times;
no expected JSON is parsed and reserialized. Lines retain the original escaping,
key order, optional omissions and nullable historical envelope sender.
