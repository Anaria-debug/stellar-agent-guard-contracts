agent-tx — transaction submission helper

This tool signs Soroban auth entries for a guard (contract) address and submits transactions.

New: cost-estimate output

Before submitting a transaction `agent-tx` now prints a human-friendly block with the simulation's estimates and also emits a machine-readable JSON line prefixed with `MACHINE_OUTPUT:`. SDKs and other tools should parse that JSON for programmatic decisions.

Fields included in `MACHINE_OUTPUT`:

- simulated.min_resource_fee: string/number — the RPC `minResourceFee` from the simulation.
- simulated.simulated_resource_fee: number — the simulation's `resource_fee` value (before the local guard bump).
- simulated.instructions: number — the simulated instruction count.
- simulated.disk_read_bytes: number
- simulated.write_bytes: number
- bump: number — `GUARD_KEY_FEE_BUMP` added locally to cover guard storage keys (visible so operators can see its contribution).
- final_fee: number — the final fee assembled (minResourceFee + inclusion fee + bump).
- onchain: { status, hash, fee_charged?, resultXdr? } — if available after submission, appended when the transaction is observed on-chain.

Example lines (human):

SIMULATION ESTIMATE (pre-submit, simulated values):
  min_resource_fee: 123
  simulated_resource_fee: 200
  instructions: 42
  disk_read_bytes: 1024
  write_bytes: 512
  note: these are simulated estimates, not on-chain actuals

Example machine line (parseable):

MACHINE_OUTPUT: {"simulated": {"min_resource_fee":123, "simulated_resource_fee":200, "instructions":42, "disk_read_bytes":1024, "write_bytes":512}, "bump":100000, "final_fee":100223, "onchain": {"status":"PENDING","hash":"<txhash>"}}

Notes

- The `bump` constant is intentionally exposed so operators and SDKs can see the tool's added margin for guard storage keys.
- The simulated values are advisory; the real on-chain fee may differ — when available, `onchain.fee_charged` is added to the JSON after confirmation.

