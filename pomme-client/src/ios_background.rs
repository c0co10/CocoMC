//! Keeps the app running for a moment after iOS sends it to the background.
//!
//! iOS suspends a backgrounded app within seconds and may then end it without
//! warning. The integrated server only writes a world when it stops, so a
//! world closed on the way out needs a background task's extra time (about
//! half a minute) to finish saving.

use std::sync::{Mutex, MutexGuard, PoisonError};

use block2::RcBlock;
use objc2_foundation::MainThreadMarker;
use objc2_ui_kit::{UIApplication, UIBackgroundTaskIdentifier, UIBackgroundTaskInvalid};

/// The running background task, if any. Only touched on the main thread.
static TASK: Mutex<Option<UIBackgroundTaskIdentifier>> = Mutex::new(None);

fn task() -> MutexGuard<'static, Option<UIBackgroundTaskIdentifier>> {
    TASK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Asks for time to finish saving. A second call while one task runs is a
/// no-op.
pub fn begin() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    if task().is_some() {
        return;
    }
    // UIKit calls this on the main thread shortly before the time runs out;
    // a task still running then has to end, or iOS ends the app.
    let expired = RcBlock::new(end);
    let id = UIApplication::sharedApplication(mtm)
        .beginBackgroundTaskWithExpirationHandler(Some(&expired));
    // SAFETY: a constant UIKit exports, read after the app has launched.
    if id != unsafe { UIBackgroundTaskInvalid } {
        *task() = Some(id);
    }
}

/// Ends the background task, if one is running.
pub fn end() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let running = task().take();
    if let Some(id) = running {
        UIApplication::sharedApplication(mtm).endBackgroundTask(id);
    }
}
