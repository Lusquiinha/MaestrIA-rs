use std::time::Duration;

use rustix::process::{Pid, Signal, kill_process, kill_process_group};

pub const GRACE: Duration = Duration::from_secs(3);

fn signal(pid: u32, sig: Signal) -> bool {
    let Some(pid) = i32::try_from(pid).ok().filter(|p| *p > 1).and_then(Pid::from_raw) else {
        return false;
    };
    kill_process_group(pid, sig).is_ok() || kill_process(pid, sig).is_ok()
}

pub fn hang_up(pid: u32) -> bool {
    signal(pid, Signal::HUP)
}

pub fn force_kill(pid: u32) -> bool {
    signal(pid, Signal::KILL)
}

pub fn exit_code(status: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.code().or_else(|| status.signal().map(|s| -s))
}

pub fn redraw(pty: impl std::os::fd::AsFd) {
    if let Ok(group) = rustix::termios::tcgetpgrp(pty) {
        let _ = kill_process_group(group, Signal::WINCH);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_signals_init_or_zero() {
        assert!(!hang_up(0));
        assert!(!hang_up(1));
        assert!(!force_kill(1));
    }
}
