# Hoon LSP implementation status

## Current Architecture (As Implemented)

### Crate and Entry Point
- `crates/hoon-lsp` is a standalone binary crate using `tower-lsp` over stdio.
- Main wiring is in:
  - `crates/hoon-lsp/src/main.rs`
  - `crates/hoon-lsp/src/backend/mod.rs`

### Backend Module Layout
- Backend logic is split into focused modules under `crates/hoon-lsp/src/backend/`:
  - `server.rs`: `LanguageServer` trait methods and request routing.
  - `requests.rs`: core request handlers (`hover`, `goto_definition`).
  - `navigation.rs`: definition matching and qualified lookup helpers.
  - `hover_helpers.rs`: hover markdown assembly and local-binding helpers.
  - `value_render.rs`: parser-backed type/value rendering.
  - `source_lookup.rs`: source extraction and type/field lookup from definitions.
  - `tokens.rs`: tokenization and namespace-aware token helpers.
  - `tasks.rs`: parse scheduling and workspace indexing task orchestration.
  - `status.rs`: status/user messaging and throttled notifications.
  - `tests.rs`: backend integration/protocol tests.

### Supporting Modules
- `document_store.rs`: open-doc snapshots and incremental text updates.
- `symbol_index.rs`: per-file and workspace symbol/import index.
- `parser_adapter.rs`: diagnostics adapter from parser errors to LSP diagnostics.
- `position_mapper.rs`: UTF-8 byte/UTF-16 LSP position conversions.
- `stdlib.rs`: embedded stdlib bootstrap and source registration.

## Feature Status by LSP Capability

### `initialize` / lifecycle / config
#### Done
- Initialize/shutdown flow is implemented.
- `didChangeConfiguration` updates runtime config.
- Startup indexes embedded stdlib in background and reports readiness.
#### Needs Further Work
- Better structured config validation and explicit user-facing errors for bad config.
#### Planned
- Expose more runtime toggles for expensive features (index refresh, hints, semantic tokens).

### Document sync (`didOpen`/`didChange`/`didSave`/`didClose`)
#### Done
- Incremental text sync is implemented.
- Open/change/save/close update `DocumentStore` and index as expected.
- Stale parse cancellation logic is in place.
#### Needs Further Work
- More resilience around pathological edit streams and very large files.
#### Planned
- Optional hard limits and richer warnings for oversized documents.

### Diagnostics (`publishDiagnostics`)
#### Done
- Parser-backed diagnostics via `parser::native_parser`.
- Debounced parse on open/change and immediate parse on save.
- UTF-16-safe diagnostic range mapping.
#### Needs Further Work
- Latency and throughput tuning for very large desks.
#### Planned
- Perf baselines against real corpora and tighter SLO tracking.

### Hover (`textDocument/hover`)
#### Done
- Parser-first hover pipeline (no JSON AST serialization fallback).
- Type/value snippets for atoms/auras, list forms, type declarations (`+$`), gates, literals, and many local bindings.
- Local-scope hover for `=/` and gate sample arguments.
#### Needs Further Work
- Remaining unresolved cases around complex scope projection (notably some `=*`/wing-heavy patterns such as `src.bowl` contexts).
- Expand composite rendering fidelity and reduce unresolved responses.
#### Planned
- Broader structural hover coverage for cells/cores and deeper field-projection inference.

### Definition (`textDocument/definition`)
#### Done
- Cross-file definition with import-aware resolution.
- Precedence and response-shape handling (`Location` vs `Location[]`).
- Local binding resolution for `=/` and argument scopes.
- Namespace/member lookups and module-qualified resolution paths.
#### Needs Further Work
- Scope-correct resolution for more advanced local projection patterns and edge-case visibility.
- Additional strictness around ambiguous matches in mixed local/import/stdlib contexts.
#### Planned
- Complete parser-backed import semantics end-to-end for all `/` rune forms and collision scenarios.

### Document symbols (`textDocument/documentSymbol`)
#### Done
- Top-level symbol extraction (`++` arms and `+$` types).
#### Needs Further Work
- Hierarchical symbols and richer symbol details.
#### Planned
- Expand to deeper symbol trees with improved kind/detail metadata.

### Workspace index / background indexing
#### Done
- Background indexing is active with refresh loop.
- Canonical path dedupe and traversal guards are implemented.
- Embedded stdlib is indexed and merged into lookup paths.
- User-visible status messages for indexing/ready/under-load states.
#### Needs Further Work
- Further startup responsiveness optimization and better progress granularity.
- Deterministic regression coverage for all cycle/loop guard behavior.
#### Planned
- Optional work-done progress integration and smarter incremental refresh.

### Embedded stdlib support
#### Done
- Embedded sources are bootstrapped and indexed at startup.
- Stdlib definitions participate in hover/definition lookup.
#### Needs Further Work
- Full precedence semantics and remaining edge cases in mixed local/import/stdlib lookup.
#### Planned
- Harden precedence rules and document the exact lookup order.

### Inlay hints (`textDocument/inlayHint`)
#### Done
- Not implemented yet.
#### Needs Further Work
- Add protocol capability and request handler.
#### Planned
- First release: parameter name hints for gates and optional type hints for inferred locals.

### Semantic tokens
#### Done
- Not implemented yet.
#### Needs Further Work
- Token taxonomy and stable mapping from parser AST nodes.
#### Planned
- Phase 2 semantic tokens provider behind config flag.

## Testing Status

### Done
- Extensive backend protocol tests exist in `crates/hoon-lsp/src/backend/tests.rs`.
- Unit tests cover position mapping, document store behavior, symbol/index logic, and import precedence cases.
- Current refactor steps are being validated with `cargo check -p hoon-lsp` and targeted backend tests.

### Needs Further Work
- Expand regression tests for known hover/definition brittle cases from real desk files.
- Add performance-focused test cases over larger corpora.

### Planned
- End-to-end corpus runs for `example-hoon`, `groups`, and `landscape` with latency/memory tracking.

## Priority Next Steps

1. Complete scope/visibility hardening for unresolved hover/definition edge cases in real desk code (`=*` and projected-field patterns).
2. Finalize import/definition semantics to remove remaining heuristic behavior.
3. Improve startup/indexing UX with clearer progress and faster time-to-first-use.
4. Implement `inlayHint` as the next editor-value feature after scope correctness stabilizes.
5. Add semantic tokens and richer symbols in phase 2 once correctness/perf baseline is stable.

