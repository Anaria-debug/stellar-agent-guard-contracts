# Description

Closes #141

This PR adds a `context_index` field to the `auth_checked` event to unambiguously identify the specific context verdict in a multi-context authorization batch. When a batch authorization is processed via `__check_auth`, the engine evaluates all contexts and emits an `auth_checked` event for each one in order. The `context_index` maps each event to its corresponding `ContractContext` in the transaction payload.

### Changes
*   **Types:** Updated `EventAuthChecked` to include `context_index: u32`.
*   **Engine:** Modified `decide` to return an `alloc::vec::Vec<Decision>` containing the verdict for each evaluated context. Removed `contracttype` from `Decision` and implemented standard traits to decouple the engine return type from the host `Env` macro constraints.
*   **Lib:** Refactored `__check_auth` to iterate over all verdicts returned by the engine and emit individual, indexed `auth_checked` events.
*   **Tests:** Added `batch_events_emit_in_order_with_context_index` to verify that a multi-context batch emits events with correct indexes (0, 1, 2, etc.) matching the batch order, even when the overall transaction fails and emits diagnostic events.

### Acceptance Criteria Checklist
- [x] Emission behavior for N-context batch documented (events-per-context confirmed by test).
- [x] `context_index: u32` (0-based) present in per-context events.
- [x] Tests: 3-context batch with mixed verdicts -> indexed events in order.
- [x] Lint, type-check, and tests all pass locally.
- [x] PR description references the issue (Closes #141).
