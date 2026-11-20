# Cite the published agent runs

Issue #394 supplies the README citations for epic #164’s Published criterion. A reader choosing an agent needs to see which models attempted which tasks, how many trials passed, and where the evidence ends. The four historical observations below have different populations. Present each with its own denominator.

## Acceptance Criteria

1. Under the unique “For AI agents” heading, one paragraph of three to five sentences names the models and tiers, quotes every figure in a sentence that links its own published page, and links the [eval index](../../docs/evals/README.md) for details. Use plain house prose without superlatives. The paragraph covers all four observations below.

   | Run | Population and result to attribute |
   |---|---|
   | [Default baseline](../../docs/evals/baseline-2026-11-01.md) | On main `a4f552d`, claude-opus, claude-sonnet, codex-sol, codex-luna and glm-spark attempted 16 default-tier tasks five times each. “395 of 400” passed. |
   | [DashScope baseline cell](../../docs/evals/baseline-2026-11-01-glm-dashscope.md) | glm-dashscope passed 80/80 using the baseline executables. Across six models the total was 475/480. |
   | [Hard run](../../docs/evals/hard-2026-11-03.md) | The original five models passed “74 of 75 scored trials” on the hard tier on main `8732445`. The published count includes the replacement infrastructure cell. |
   | [Hard and expert run](../../docs/evals/expert-2026-11-07.md) | On main `c95386e` the four Claude and Codex models passed 151/160 across both tiers. No GLM ran. codex-luna scored 0/5 on `otp-late`, all `stale_code`. |

2. Give the default baseline’s wide 95% interval of [0.57, 1.0] for a 5/5 cell at five trials. State that tokens per success and transcripts are unavailable (#391) and the client matrix is unmeasured (#392). Keep setup examples scoped to their demonstrated wiring.
3. Audit the whole README, including the lead, intro, feature bullets, inventory, tables, setup and navigation. Each agent claim either links its specific proving test or run, or uses wording that claims nothing measured. Record each original claim, location, disposition, and exact evidence or replacement in the delivery ledger, including the new run claims.
4. Quote no figures from the throughput or agent-comparison pages and give no Mailpit or MailHog ranking. The session comparison measures the model and its context. The README comparison headline requires the operator’s separate ruling under [0132 §13](../../docs/decisions/0132-mail-comparison-protocol-v4.md).
5. Above the current statuses in the eval index, insert a dated line naming the README and the newly cited pages. The existing status entries compare byte-for-byte with their pre-change text.
6. A locked, file-only Rust test checks every eval link occurrence in the agent section, including any fragment, and compares every quoted figure verbatim with its attributed page. Deleting a required citation, introducing an unsupported link form, or citing a different real page for a count fails. The test also detects a missing or duplicate section. The same check passes on valid temporary files and diagnoses deliberately damaged copies without modifying repository sources. The test executes in under one second without launching a child process or requiring a macOS ignore marker.
7. The implementation passes the full house gate inventory with exact production coverage. Scope is README prose, the eval index and the citation test. Report “Closes #394” in the PR handoff.

## Dispatcher amendment, 2026-11-20

Criterion 1 first read "links every quoted figure to its own published page", and the delivery made each figure a link label of its own. That put 13 links into one paragraph, including "[16]" and "[95%]". The dispatcher changed the rule: a figure must sit in a sentence that links the page pinned as its source, that page must still state it verbatim, and any other number in the sentence is refused as uncited. Issue references are clickable links to the tracker, `[#N](https://github.com/feedback-loop-ai/swarmail/issues/N)`, with the same N in label and destination. The negative cases keep their meaning: a deleted figure, a figure whose page no longer states it, a figure cited by another real page, a figure moved into a sentence without its page, and unsupported link forms all still fail.
