# Plan: API latency

## Technical context and constitution check

Rust benchmark driver and standalone Rust eval crate, on story A commit 9ac77db. Existing resources, Owned, sampler, hold EOF/receipt, cost planner and admission are reused. API clients and cells do not yet exist. wire::Http is bounded GET transport, not yet a persistent mutation transport. McpClient starts IDs at zero; bridge IDs are global and MAX_IN_FLIGHT is 16.

Implement accepted 0132 without amending a ruling; no new ADR. Preserve server src/, both locks, deny.toml and historical bytes. Keep 0055's eval boundary and 0020 test placement. The plan template is absent (only spec-template.md exists); its required sections are supplied here. This is initial design, not a review return.

## Research and reconciliation

Read both panel positions, intake framing, decision index and 0132 including acceptance. Simplicity wins on closed family dispatch and one launch owner; robustness wins on exact ledger, deadlines and correlation. Both exclude new adapters/packages/campaigns. Explicit fixtures cost setup time and mean fill-zero cells still contain measurement fixtures: report fixture counts separately from background fill. Full scans may timeout at 100k, and a 16-slot bridge may refuse 32 callers; publish those results.

Resolve routes and negative capabilities from captured lock-v5 bytes per product, including GreenMail's corrected REST path. Missing evidence is distinct from unsupported. Keep later stories' evidence unresolved; do not freeze. Preserve legacy v2/v3 observers except mechanically behavior-preserving primitive extraction.

## Project structure and approach

- benches/mail_compare/clients/{mod,swarmail,mailpit,mailhog,mailslurper,smtp4dev,inbucket,greenmail}.rs: concrete enum dispatch, typed message/ID/result records, cited routes and strict decoders. Factor only required bounded HTTP primitives from wire.rs into shared transport, with persistent per-worker connections and mutations. Bound per-response and aggregate scan bytes/pages using documented corpus sizes and the 150,000 ledger ceiling; reject malformed framing, repeated cursors and overflow. Prove limits admit legitimate 100k scans and record/freeze their actual values before collection. Escape path/query values; refuse unexpected redirects/proxies.
- Small shared extraction/fixture helpers: factor unchanged six_digit_code and labelled_url from evals/src/oracle.rs, preserving existing exports. Reuse fixture code/link templates in corpus.rs. Compile shared clients/helpers in evals via #[path], without dragging in launcher/scenario modules. Verify direct dependencies in both manifests, not merely transitive presence.
- benches/mail_compare/api.rs: fill verification, idle-fill points, authored fixture ledger, scheduler and exact checks. Fixed order: list/search/get/code/link (each at 1/8/32), MCP future wait (1/8/32), delete (1/8/32), recipient clear (1/8/32) where supported, then single global clear if it is the documented narrowest clear. Unsupported operations emit rows. Preallocate victims as specified; no measured refill or subwindow aggregation. Keep background unchanged until global clear.
- evals/src/{apibench,main,mcp}.rs and bridge tests: api-mcp dispatch, existing tool mappings and absolute per-call deadlines. Benchmark clients receive bounded nonoverlapping numeric ID ranges including initialization, with exhaustion refusal; ordinary clients keep their behavior. Use one bridge and endpoint-only adapter pointing at one hold-owned backend; Swarmail connects directly to held /mcp. Initialize before clocks. Preserve capacities/tools.
- benches/mail_compare/{runner,resources}.rs and CLI: native API uses Owned. For MCP, hold remains sole backend owner and sampler. Add opt-in --control api-v1 bounded JSON-lines commands window_start(id), window_end(id) and idle_fill. Return correlated snapshots, owner-monotonic timestamps/sample boundaries and required disk facts. Unknown/duplicate/unmatched commands or oversized lines refuse and clean up. Existing silent-stdin/EOF behavior remains unchanged. Start completes before assigning operations; end follows completion/cutoff. Evals records local command send/receive brackets; resource subtraction and sample attribution use only the owner's clock, never separate Instant epochs. Explicitly retain control overhead in the resource interval and separately record measured client elapsed time. Final receipt binds windows to launch/limits/samples. After unresolved requests, end the window, close calls/bridge/adapter, close hold stdin and await its real cleanup receipt; never continue that launch. Arbitrary endpoint mode cannot produce admitted resource evidence.
- benches/mail_compare/{protocol,records,report,stats,runner}.rs, benches/mail_compare.rs, scripts/compare-mail.sh: API chunk coordinates, dry-run/dispatch and shared raw schema/admission. Reuse statistics and sealed-directory rules; preserve cost behavior. B adds 144 API launch chunks to 24 cost chunks, with unsupported/nonlaunchable rows separately. Full collect-v4 must still refuse unresolved C/D sections, not announce full support.
- Working docs/evals/ v4/v5 documents and sources plus README: operation citations/capabilities, client digests, commands and labels. Keep draft status for E; no accepted decision-body edits. 0132 index row already exists; add only if absent.
- tests/mail_compare.rs, tests/store_contract_docs.rs and focused eval tests: deterministic parser/arithmetic tests plus real-protocol proofs, no mocked integration substitute.

## Data model and contracts

Explicit Surface, Operation, ClearScope, Outcome, family client and strict result types. Fixture entries retain authored expected content, resolved target ID, scope and role: background/read/warmup/victim/survivor/future. Evidence retains route/filter/scope and document identity. Unsupported is not an execution error.

Each cell owns planned indices, an append-only start ledger, one client measurement interval and one owner resource interval. Every start terminates exactly once. Mutation postchecks finalize exactness before sealing. Unresolved calls abandon the launch; failures during setup/warmup block dependent cells. Retain raw deltas and both intervals, including control overhead, so reporting cannot imply impossible cross-process timing precision.

Native CLI adds api using existing protocol/source/documents/bin/runtime/scratch/output inputs and an API chunk coordinate. Eval api-mcp takes the validated coordinate and driver/adapter paths needed to own hold/bridge. Unknown coordinate, mismatched pairing, missing evidence or resources refuse before measurement. No paid/model path is enabled.

## Quickstart and verification

Build the candidate with cargo build --locked --bin swarmail and resolve its artifact path for eval SWARMAIL_BIN. Build driver/eval by existing locked paths. Wrapper dry-run first shows coordinates without launch. Run cargo test --locked --test mail_compare and cargo test --locked --test store_contract_docs, then locked focused eval tests.

Add ignored api_native_real, api_mcp_bridge_real and api_fill_100k_real proofs (document final exact names in verification evidence). Run each separately under timeout 600 taskset -c 0-31, supplying pinned bytes/delegation, recording /proc/loadavg before/after. Native proof covers reads/extraction and meaningful recipient/global destructive cycles on supported targets. MCP proof exercises concurrent real bridge calls/refusals, waits, exactness and cleanup. Fill proof verifies actual retained 100k IDs and a full scan. Missing prerequisites fail, never skip.

Run the complete docs/house-rules.md gate inventory on the implementation candidate: all locked root/eval checks with candidate SWARMAIL_BIN, recipe compilation/refusals, exact coverage/selftest, publish/seal and license/native-tls proofs; identify CI-owned checks separately. Structural design validation is not implementation verification.

## Risks

- Missing/variant evidence: per-product citation checks and fail-closed preflight; no unsupported claims from transport failure.
- Shared refactoring changes frozen behavior: keep legacy observers, frozen hash/behavior proofs and extractor parity in both crates.
- Destructive races/timeout residue: distinct victims/scopes, final batch ledger check, terminal start accounting and launch abandonment.
- Bridge IDs/capacity: disjoint ranges, overlap tests, preserved limits and visible busy rows without retry.
- Cross-process resource misattribution: owner clock snapshots/sample boundaries, local brackets and admission rejection of foreign windows.
- Large scans and fixture growth: precomputed counts, documented finite bounds, legitimate 100k proof and cap refusal before launch.
- Missing host/artifacts: fail with prerequisite named, never report skipped proof as success; bounded sequential real runs only.
