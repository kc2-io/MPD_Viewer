# Vendored tiny_http 0.12.0

Source: crates.io tiny_http 0.12.0, upstream commit
212b1c45852fef2093dc1374875a9393c55eb4b9. The crates.io archive checksum is
389915df6413a2e74fb181895f933386023c71110878cd0825588928e64cdc82.
Source files, normalized/original
manifests and both MIT/Apache licenses are preserved. Examples, benchmarks,
upstream tests, workflows and registry bookkeeping are not needed by this build.

Local change: src/util/task_pool.rs reserves workers already needed for queued
jobs when deciding whether to spawn another worker. Previously several rapid
connections could all queue against the same still-counted idle workers. Once
those workers handled persistent connections, remaining connections could wait
indefinitely. The fixed predicate compares idle workers with existing queue length
while holding the same queue lock used when workers leave their waiting state.

The bounded behavioral regression in src-tauri/src/http_pool_regression.rs
includes this exact module and runs in the normal application Rust test suite.
The original source reproduced starvation locally (6 of 12 jobs started, 6 queued,
no idle workers). Hosted GUI results are tracked in PR #22.

Remove this patch when an upstream release includes an equivalent verified fix.
