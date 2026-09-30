# Integrations VPN Status Prop Profile

Date: 2026-09-26

These React Profiler measurements use the existing Integrations Vitest/JSDOM
test harness. The read-only VPN owner is updated by changing the forwarded port
in its runtime-state prop. Twenty update commits are recorded. Times are
milliseconds. The profile wraps the full Integrations tree, not browser paint
or deployed-device performance.

| State | Median | p95 | Commits |
| --- | ---: | ---: | ---: |
| Before extraction, run 1 | 6.521 | 12.599 | 20 |
| Before extraction, run 2 | 6.698 | 14.397 | 20 |
| After extraction, run 1 | 6.967 | 12.943 | 20 |
| After extraction, run 2 | 6.320 | 13.647 | 20 |

The read-only VPN status view moved into `VpnPanel.jsx`. The root fell from
1,417 to 1,235 lines. The full-tree timings are noisy and show no speedup.

Profiles used `npx vitest run src/components/System/Integrations/index.test.jsx
-t "profiles VPN status prop render commits" --disableConsoleIntercept`. All 22
focused Integrations tests passed. The Web production build, targeted ESLint, and
diff checks passed.
