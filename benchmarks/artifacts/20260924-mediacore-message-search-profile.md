# MediaCore message-search render profile

Date: 2026-09-24  
Source revision: `0dfab48c` plus the then-current worktree  
Environment: Node `v22.23.2`, npm `12.0.2`, React `19.2.8`, Vitest `4.1.11`

Command:

```sh
npm test -- src/components/System/MediaCore/index.test.jsx -t 'profiles message-search input render commits' --reporter=verbose
```

The test wraps the whole `MediaCore` page in `React.Profiler`, warms the search
field three times, and records 20 successive text changes per run. It measures
React `actualDuration` in Vitest's JSDOM development environment.

| Run | Commits | Median | p95 | Long commits |
| --- | ---: | ---: | ---: | ---: |
| Baseline 1 | 20 | 9.81 ms | 182.01 ms | 5 samples from 176.53–186.96 ms |
| Baseline 2 | 20 | 10.86 ms | 180.75 ms | 5 samples from 177.60–180.91 ms |
| Baseline 3 | 20 | 9.73 ms | 181.42 ms | 5 samples from 179.93–182.96 ms |

In all three runs, editing this one field updates parent-owned query/results
state in the 8,730-line `MediaCore` component. The repeated group of long React
commits is the evidence for isolating this stateful search panel. These numbers
describe the local development renderer only; they do not estimate production
device latency. The same test and environment will be used to record the
post-extraction result below.

After extraction, the query, loading, error, and result state lives in a
memoized message-search panel. The same 20-commit sampling harness produced:

| Run | Commits | Median | p95 |
| --- | ---: | ---: | ---: |
| After 1 | 20 | 0.130 ms | 0.242 ms |
| After 2 | 20 | 0.152 ms | 0.313 ms |
| After 3 | 20 | 0.156 ms | 0.312 ms |

Across the three run medians, the median fell from 9.81 ms before extraction
to 0.152 ms after extraction. Across the three per-run p95 values, the median
fell from 181.42 ms to 0.312 ms. This measures local React development-renderer
work in JSDOM; it does not estimate production device latency.
