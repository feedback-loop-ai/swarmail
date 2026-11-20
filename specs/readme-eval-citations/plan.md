# Implementation plan

## Technical context and research

The checkout matches the published origin/main at `68c3839`. The local main predates the reports. The supplied #394 framing is the intake record available here. Use the four source spellings and populations in the [spec](spec.md). In particular, the hard page also mentions the earlier default score. A global search across reports could certify the wrong attribution.

## Project structure and claim audit

Edit `README.md` and `docs/evals/README.md`. Give each run a sentence and use the fifth for uncertainty, missing measurements and the index. Group setup links separately. Add a dated status above the first existing status. Read the resulting diff against the original index to verify insertion only.

Use the following dispositions as the starting ledger. In the implementation result notes, record each final row with original wording, location, replacement or proof path and relevant test name. The ship seat generates its ledger from the journal. Include the four run rows and uncertainty row from the spec with their actual citations.

| README location and original wording | Disposition |
|---|---|
| Lead, “Most mail mocks are built for a human” / “built for a program waiting on mail” | Remove the unsupported comparison. Describe a program waiting for mail. |
| Intro, “Email infrastructure for tests and agents” | Keep as intended use. |
| Intro, “native MCP for AI agents” and “swallowing mail faster than your tests can produce it” | Name the MCP interface and remove the speed claim. |
| Why swarmail, “Built for AI agents” and its HTTP, stdio, wait, code, link, seed and chaos claims | Replace the pitch and examples with navigation to the agent section. |
| Inventory, “Eighteen MCP tools”, and MCP table row, “eighteen tools over HTTP … and the stdio bridge” | Use neutral interface and guide labels or link the specific inventory and transport proofs for retained assertions. Audit both occurrences. |
| Table, “Client fixtures” and “Focused waits” | Name the documented helpers and link their guides as navigation. |
| Table, “a private inbox per agent” | Name recipient routing for per-agent addresses. Remove the unsupported confidentiality claim. |
| Agent setup, “Wire Claude Code, Codex, Cursor or dsh” | Describe the guide’s four examples. Keep the existing scoped wiring sentence including its install exception. Add a separate clickable proof link to tests/agent_setup_docs.rs so its README_CLAIM pin still verifies the prose. |
| Contents, machine-readable surface links, MCP guide and contributing pointers | Retain as navigation. Record each location. |

## Data model and test contract

Add `tests/readme_eval_citations.rs` beside the setup proof. Adopt simplicity’s small document scope and robustness’s standalone test and explicit figure-to-source inventory. An extra test target costs a build unit but can run and be timed independently of the setup suite’s binary exercises. Pinning a few expected pairs costs an intentional test edit when evidence changes. It catches deleted citations and misattribution even when both pages contain the same figure.

Use one validator taking a repository root. Read the unique level-two agent section through the next level-one or level-two heading or EOF. Consume the entire section with a restricted inline Markdown grammar. Permit canonical relative inline destinations for the guide and setup proof and the five specified eval files. Reject residual link syntax, reference definitions/uses, images, HTML, bare URLs or eval paths, malformed links, encoded aliases and traversal. Handle inline code in prose and a code span around the literal bracketed interval inside its link label. This small grammar makes unsupported forms visible to an editor through a diagnostic.

Use distinct inline figure links for the two GLM counts and the two hard/expert scores. Pin expected literal/page pairs for each score, the interval, its confidence level and the perfect cell. Check every occurrence against its own file with numeric token boundaries. Link run identifiers and other numeric setup details in their clauses to the same source using literal labels where numeric (e.g. “16”); express model/trial counts in words where that reads better. Reject additional uncited numeric measurements in the paragraph. For each optional fragment, resolve a unique supported GitHub heading slug on that file, excluding fenced text. Restrict support to simple unambiguous headings and diagnose any unfamiliar fragment. Plain page links suffice in the README.

Start each negative case with a passing temporary root using std and the existing locked tempfile dependency. Vary one input, then assert the specific diagnostic. The source-drift case must remove every occurrence of its figure from that page. Test an accepted heading fragment as well as an absent one. Keep this checker local to the test target.

## Constitution check

This is historical reporting. It changes no API, runtime guarantee or ruling and needs no ADR. Preserve the existing real-protocol and setup proofs. Use the complete [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change), including the locked eval workspace against the candidate binary and the coverage and publish-guard proofs. Leave src, locks, tooling and license policy untouched. Run no collection, freeze, publication or push.

## Risks

- Literal matches alone can mislabel a model or tier. Review each clause against the spec’s source row and record its scope in the ledger.
- A missing paragraph or unsupported syntax could evade a prefix scan. Require the complete evidence inventory and consume all link syntax, with a malformed extra link tested alongside valid ones.
- A generic Markdown renderer adds dependencies and maintenance. Restrict syntax to what this section uses and exercise its accepted and rejected forms. A future author using a different form will need to extend the test or edit the link.
- Quoting model outcomes as server guarantees would exceed the measurements. Retain the limits sentence and exclude both comparison pages. Robustness’s crash, webhook, capacity, transport and TLS scenarios describe separate runtime contracts. Adding those tests here would spend the implementer’s deadline without verifying a citation.

## Quickstart

Run `cargo test --locked --test readme_eval_citations` and inspect the harness execution time after compilation. Run `cargo test --locked --test agent_setup_docs the_readme_points_at_the_guide_and_not_at_an_unexercised_block` to verify the existing prose contract. Commit coherent increments unsigned and run the remaining house gates in the foreground.
