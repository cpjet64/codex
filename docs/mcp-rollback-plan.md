# MCP Migration — Rollback Plan

Strategy

- Use feature flag `rmcp_sdk` to control path selection.
- Maintain legacy path alongside SDK until acceptance gates pass.

Pre-requisites

- CI green on both paths.
- Canary cohort enabled with `rmcp_sdk` and monitoring dashboards live.

Flip to SDK (canary → staged)

1) Enable `rmcp_sdk` for canary users/instances.
   - Expectation: Identical tools list, successful calls, no error spikes.
   - Health checks: startup failures per server, list latency, call success rate, memory.
2) Gradually increase percentage.
3) Full cutover once stability confirmed and tests pass in CI.

Rollback (fast)

- Disable `rmcp_sdk` feature (env/config or build feature) and redeploy.
- Restart affected services/clients to clear any resident state.

Validation after rollback

- Confirm legacy path resumes normal operation: tool listing and calls.
- Compare logs against pre-cutover baseline for anomalies.

Post-mortem

- Record root cause, corrective actions, and whether additional guards/tests are needed.
