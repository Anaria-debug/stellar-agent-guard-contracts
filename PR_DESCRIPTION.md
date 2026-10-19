Closes #400

## Summary

Set a 45-minute timeout on the required `ci` job so a hung test or build cannot
block merges indefinitely. This allows generous time for cold toolchain installs,
contract tests and builds, and the agent-tx checks.

The 45-minute limit covers the entire serial workflow. No current step is known
to approach it, so no separate tool-build timeout or job split is warranted.

## Verification

- Not run locally: Cargo is unavailable in this environment; CI will run formatting, lint, type-check, and tests.
Closes #413

## Summary

Replace inline agent-secret placeholders in README command examples with hidden
`AGENT_SECRET` prompts, and document the safe environment-variable form in the
fixture and `agent-tx` guides. The prompts keep secrets out of shell history.

## Verification

- `./scripts/verify-docs-reachability.sh` (passed)
- `./scripts/verify-readme.sh` (could not complete: Cargo and the WASM artifact are unavailable)
