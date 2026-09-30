# Description

Closes #40

This PR adds a permissionless helper `refresh_deadman` that allows anyone to explicitly record and emit an event (`dms_auto_frozen`) when the dead-man switch has expired. Because the dead-man switch is deliberately lazy, the account previously became frozen silently without any on-chain event. This helper provides observability for operators.

### Changes
*   **Types:** Added `AutoFrozenAt` to `DataKey` in `src/types.rs`.
*   **Lib:** Added `EventDmsAutoFrozen` and `refresh_deadman()` to explicitly record `AutoFrozenAt` and emit `dms_auto_frozen` if the grace has elapsed.
*   **SPEC:** Updated §3, §5, §7, and §9 to document the new `refresh_deadman` visibility mechanism, noting that it does not weaken the core lazy evaluation (rule #2).
*   **Tests:** Added `refresh_deadman_records_auto_freeze_and_emits_event` to verify the no-op behavior when not expired, and correct recording and emitting when expired.

### Acceptance Criteria Checklist
- [x] Design note in SPEC §5: mechanism, why it doesn't weaken freeze semantics, auth placement.
- [x] Implementation + tests: silent expiry → helper call records + emits → subsequent spends still blocked identically. Helper no-ops when not expired.
- [x] No change to the rule #2 truth: expiry requires no flag to take effect.
- [x] Lint, type-check, and tests all pass locally.
- [x] PR description references the issue (Closes #40).
