# MediaCore content-ID resolver render profile

Date: 2026-09-24  
Source revision: 0dfab48c plus the then-current worktree  
Environment: Node v22.23.2, npm 12.0.2, React 19.2.8, Vitest 4.1.11

Command used: npm test -- src/components/System/MediaCore/index.test.jsx -t
'profiles content-ID registry input render commits' --reporter=dot

The test wraps the full MediaCore page in React Profiler, warms the external-ID
resolver field three times, then records the first update commit for 20
successive text changes. It measures React actualDuration in Vitest's JSDOM
development environment.

| Run | Commits | Median | p95 | Long commits |
| --- | ---: | ---: | ---: | ---: |
| Baseline 1 | 20 | 9.39 ms | 179.64 ms | 5 samples from 174.63–180.18 ms |
| Baseline 2 | 20 | 10.48 ms | 181.40 ms | 5 samples from 176.09–182.11 ms |
| Baseline 3 | 20 | 9.90 ms | 184.41 ms | 5 samples from 179.48–185.53 ms |

The resolver query, loading state, and result state were then moved into a
memoized ResolveExternalIdPanel. The existing Use Example helper still sets
the panel query through its ref-based setter.

| Run | Commits | Median | p95 |
| --- | ---: | ---: | ---: |
| After 1 | 20 | 0.130 ms | 0.399 ms |
| After 2 | 20 | 0.321 ms | 0.410 ms |
| After 3 | 20 | 0.329 ms | 0.413 ms |

Across the three run medians, the median fell from 9.90 ms before extraction
to 0.321 ms after extraction. Across the three per-run p95 values, the median
fell from 181.40 ms to 0.410 ms. This measures local React development-renderer
work in JSDOM; it does not estimate production device latency.
