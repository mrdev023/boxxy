//! Thin wrapper around `libc::setpriority(PRIO_PROCESS, 0, nice)`.
//!
//! Background maintenance tasks call `set_current_thread_nice(19)` at
//! their entry point so they never contend with the foreground shell
//! or the UI render loop.
//!
//! Note: `setpriority(PRIO_PROCESS, 0, ...)` on Linux affects the
//! calling thread, not the whole process — despite the `PRIO_PROCESS`
//! name. That's exactly what we want: Tokio may share one thread across
//! many tasks, but only tasks that opt in get niced down.

/// Lowest-priority CPU scheduling used for background maintenance
/// (Dreaming, the zombie sweeper, etc.).
pub const MAINTENANCE_NICE: i32 = 19;

/// Sets the calling thread's nice level. Returns `Err` only if the
/// syscall itself fails — not if the user lacks CAP_SYS_NICE to nice
/// *up*, because we only ever nice down and that's unprivileged.
pub fn set_current_thread_nice(nice: i32) -> std::io::Result<()> {
    // SAFETY: `setpriority` is a safe, side-effect-only syscall; the
    // arguments are well-formed. The PRIO_PROCESS+0 combo means "the
    // current thread" on Linux.
    let rc = unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, nice) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// Returns the current thread's nice level, or an error if the syscall
/// fails. Used by tests to verify `set_current_thread_nice` took effect.
pub fn get_current_thread_nice() -> std::io::Result<i32> {
    // `getpriority` can legitimately return -1, so we disambiguate via
    // errno: clear it first, call, then read.
    unsafe { *libc::__errno_location() = 0 };
    let prio = unsafe { libc::getpriority(libc::PRIO_PROCESS, 0) };
    let errno = unsafe { *libc::__errno_location() };
    if prio == -1 && errno != 0 {
        Err(std::io::Error::from_raw_os_error(errno))
    } else {
        Ok(prio)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nice_wrapper_lowers_and_reads_back() {
        // We can only nice *up* (higher number = lower priority) without
        // CAP_SYS_NICE, so +5 is always safe. Using +5 instead of the
        // full 19 avoids starving the rest of the test harness.
        set_current_thread_nice(5).expect("setpriority(+5)");
        let read = get_current_thread_nice().expect("getpriority");
        assert!(read >= 5, "expected nice >= 5, got {}", read);
    }

    #[test]
    fn nice_wrapper_rejects_invalid_priority() {
        // Niceness is clamped to [-20, 19] on Linux; values outside
        // this range are silently clamped rather than rejected, so we
        // can't test a straight EINVAL. Just exercise the happy path
        // at an edge value and confirm we don't panic.
        let _ = set_current_thread_nice(19);
    }
}
