//! Concurrent hardware dispatch of planned writes.

use super::plan::{PlannedWrite, WorkItem};
use crate::apply::dispatch::apply_monitor_target;
use crate::apply::types::{ApplyRecord, ApplySettings};
use crate::backends::{BackendError, BackendKind, BackendWrite, ProcessRunner};
use std::thread::ScopedJoinHandle;

/// The result of one planned target after the write phase.
pub(super) enum Outcome<'a> {
    Skipped(ApplyRecord),
    Written {
        write: PlannedWrite<'a>,
        result: Result<BackendWrite, BackendError>,
    },
}

/// Runs every planned write concurrently, one scoped thread per target.
///
/// Invariant: all threads are spawned (in plan order) before any is joined,
/// and outcomes keep plan order. Skip items pass through untouched. Writes
/// share no mutable state; runtime state is only updated in the commit phase.
pub(super) fn dispatch_all<'a, R: ProcessRunner + Sync>(
    runner: &R,
    settings: &ApplySettings,
    work: Vec<WorkItem<'a>>,
) -> Vec<Outcome<'a>> {
    enum Pending<'a, 'scope> {
        Skipped(ApplyRecord),
        Running(
            PlannedWrite<'a>,
            ScopedJoinHandle<'scope, Result<BackendWrite, BackendError>>,
        ),
    }

    std::thread::scope(|scope| {
        let pending = work
            .into_iter()
            .map(|item| match item {
                WorkItem::Skip(record) => Pending::Skipped(record),
                WorkItem::Dispatch(write) => Pending::Running(
                    write,
                    scope.spawn(move || {
                        apply_monitor_target(
                            runner,
                            write.monitor,
                            write.target,
                            write.applied_percent,
                            settings,
                        )
                    }),
                ),
            })
            .collect::<Vec<_>>();

        pending
            .into_iter()
            .map(|pending| match pending {
                Pending::Skipped(record) => Outcome::Skipped(record),
                Pending::Running(write, handle) => Outcome::Written {
                    write,
                    // A panicking backend must not take the daemon down: it
                    // becomes an I/O failure so backoff handles it.
                    result: handle.join().unwrap_or_else(|_| Err(dispatch_panicked())),
                },
            })
            .collect()
    })
}

fn dispatch_panicked() -> BackendError {
    BackendError::Io {
        backend: BackendKind::Backlight,
        program: String::from("apply_monitor_target"),
        message: String::from("thread panicked during hardware dispatch"),
        attempts: 0,
    }
}
