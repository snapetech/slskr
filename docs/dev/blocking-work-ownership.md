# Daemon blocking-work ownership inventory

Audited 2026-09-28 UTC for RF-006. This records production `spawn_blocking`
call sites after external visualizer waits moved to the child registry.
An awaited blocking-task handle is not a shutdown join: once Tokio starts a
blocking closure, cancellation of its requesting future cannot stop that closure.

| Owner | Work and existing bound | Cancellation and remaining proof |
| --- | --- | --- |
| `file_transfer_runtime_owners/audio_metadata.rs` | Prefix hash reads at most 32 KiB; technical metadata reads at most 1 MiB. | Both handles are awaited by metadata enrichment. Blocking filesystem operations can outlive a canceled enrichment future. |
| `private_gateway_owners/gateway_services.rs` | Authenticated, confined local-file range; validated size and bounded range allocation. | Handle is awaited by the call handler. Already running filesystem I/O is not canceled by handler cancellation. |
| `share_backfill_controller.rs` | Authenticated share range; indexed file size and offset are checked; each blocking seek/read is capped at 46,000 bytes. | Handle is awaited by the MeshContent handler, and gateway connection admission bounds concurrent callers. Cancellation cannot stop an already-running filesystem syscall. |
| `mesh_sync.rs` | Confined local-file chunk with validated offset, length, and indexed size. | Handle is awaited by the chunk handler. Request cancellation does not establish a join of running filesystem I/O. |
| `share_index_runtime.rs` | Directory/entry/pending-descriptor bounds; shared cancellation flag. | Worker checks cooperative cancellation. Mounted delayed I/O and shutdown during a running worker still need acceptance proof. |
| `controller_feature_state.rs` | At most 4,096 records and 8 MiB persisted feature state; serialized mutation turn. | Worker deliberately retains the turn through publication after request cancellation, preventing stale writes. Shutdown completion of running workers remains unproved. |
| `database_maintenance.rs` | Retention recursively scans configured download trees and avoids symlinks. | The owning async task now signals cancellation on drop, and the worker checks between entries and removals. There is no total traversal budget during normal operation; an active filesystem syscall cannot be cancelled. |
| `preview_stream_controller.rs` / `core_dump_process.rs` | At most one application dump worker; admission is held through blocking completion after request cancellation. Linux gcore retains its 60-second deadline, owned process group, child reap, ptrace cleanup and partial-output guard. | Returned temporary stream output has a last-consumer cleanup owner, including an unconsumed worker result. The running filesystem worker is not joined by the daemon managed registry; shutdown can still wait for the remaining child deadline or mounted file I/O. |

There are nine production call sites: two audio metadata workers and one in each
other listed owner. The remaining `spawn_blocking` in focused transfer tests is
fixture work. Neither production nor the opt-in legacy visualizer dispatcher
uses a detached blocking child wait after this change. Its separate direct-child
registry retains the four-process semaphore, periodically reaps exited children,
and closes admission before killing and waiting for owned children at shutdown.

After managed async shutdown returns, the daemon now calls Tokio's
`Runtime::shutdown_timeout` with a five-second deadline. A focused regression
holds a real `spawn_blocking` closure, confirms runtime teardown returns at its
deadline, then releases the worker and observes it complete. This bounds normal
runtime teardown. File-retention traversal now observes cancellation when its
owning async task is dropped, so it stops after the active filesystem call
returns. Neither Tokio nor the operating system can cancel a filesystem syscall
stuck in an uninterruptible kernel state; the runtime deadline bounds daemon
teardown without claiming to join that call. RF-006 still needs fresh
hosted/service-overlap proof.
