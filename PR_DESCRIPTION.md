Closes #400

## Summary

Set a 45-minute timeout on the required `ci` job so a hung test or build cannot
block merges indefinitely. This allows generous time for cold toolchain installs,
contract tests and builds, and the agent-tx checks.

The 45-minute limit covers the entire serial workflow. No current step is known
to approach it, so no separate tool-build timeout or job split is warranted.

## Verification

- Not run locally: Cargo is unavailable in this environment; CI will run formatting, lint, type-check, and tests.
