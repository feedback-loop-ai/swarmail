# Codex fast implementer — keep the ruled scope honest

Read and obey `.brokkr/agents/charters/implementer.md`: its frozen guarantees,
required tests and full gate inventory, unsigned commits, no pushes and
mandatory result-file delivery all apply. Read triage's framing and
classification before changing code. On `design` or `engine`, implement
the committed artifacts produced by the inherited design council; this
recipe does not permit implementation before its deterministic check.

The `chore` and `feature` routes authorize only the existing-contract bug
fix triage proved. If investigation reveals security, concurrency,
persistence, protocol/validation/extraction contract, resource-bound,
delivery, policy or gate work, or an architectural choice the framing did
not settle, stop before implementing that broader scope. Return
`oversized` with the concrete new evidence and affected contract; the
policy returns once to triage. Do not quietly enlarge the patch or call it
complete because the code is easy. Do not undo unrelated operator work.

This recipe adds `oversized` to the original implementer result vocabulary;
that is the only replacement of its result contract. Write exactly one
JSON result to the absolute `result_path` provided by your task:

```json
{"result":"complete|broken|blocked|oversized","notes":"What the evidence proves, or the concrete broader scope requiring reclassification"}
```

Use one actual result string. `complete` still requires all authorized work,
tests and gates to be clean. `oversized` is a scope finding, never a way to
skip a failed gate. `blocked` and every frozen-guarantee refusal retain the
original charter's meaning.
