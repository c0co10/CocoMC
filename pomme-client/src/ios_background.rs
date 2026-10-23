//! Keeps the app running for a moment after iOS sends it to the background.
//!
//! iOS suspends a backgrounded app within seconds and may then end it without
//! warning. The integrated server only writes a world when it stops, so a
//! world closed on the way out needs a background task's extra time (about
//! half a minute) to finish saving.

use std::sync::{Mutex, PoisonError};

use block2::RcBlock;
use objc2_foundation::MainThreadMarker;
use objc2_ui_kit::{UIApplication, UIBackgroundTaskIdentifier, UIBackgroundTaskInvalid};

/// The running background task, if any. Only touched on the main thread.
static TASK: Mutex<Option<UIBackgroundTaskIdentifier>> = Mutex::new(None);

/// Asks for time to finish saving. A second call while one task runs is a
/// no-op.
pub fn begin() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    if TASK.lock().unwrap_or_else(PoisonError::into_inner).is_some() {
        return;
    }
    // UIKit calls this on the main thread shortly before the time runs out;
    // a task still running then has to end, or iOS ends the app.
    let expired = RcBlock::new(end);
    let task = UIApplication::sharedApplication(mtm)
        .beginBackgroundTaskWithExpirationHandler(Some(&expired));
    // SAFETY: a constant UIKit exports, read after the app has launched.
    if task != unsafe { UIBackgroundTaskInvalid } {
        *TASK.lock().unwrap_or_else(PoisonError::into_inner) = Some(task);
    }
}

/// Ends the background task, if one is running.
pub fn end() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let task = TASK.lock().unwrap_or_else(PoisonError::into_inner).take();
    if let Some(task) = task {
        UIApplication::sharedApplication(mtm).endBackgroundTask(task);
    }
}
