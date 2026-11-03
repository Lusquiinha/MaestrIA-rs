use std::path::Path;
use std::time::{Duration, Instant};

use mx_core::ports::TermFactory;
use mx_term::ptyd::{self, Ptyd};
use mx_term::{TermRegistry, TerminalSession};
use mx_theme::TerminalPalette;

fn until(limit: Duration, mut f: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline {
        if f() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    f()
}

fn screen(s: &TerminalSession) -> String {
    (0..s.size().rows as usize).map(|r| s.row_text(r)).collect::<Vec<_>>().join("\n")
}

fn daemon(dir: &Path) -> Ptyd {
    let socket = ptyd::socket_in(dir);
    let path = socket.clone();
    std::thread::spawn(move || ptyd::serve(&path).unwrap());
    let d = Ptyd::at(socket);
    assert!(until(Duration::from_secs(5), || d.info().is_ok()));
    d
}

fn registry(d: &Ptyd) -> TermRegistry {
    let r = TermRegistry::new(TerminalPalette::default());
    r.use_ptyd(Some(d.clone()));
    r
}

#[test]
fn a_session_outlives_the_app_and_comes_back_with_its_screen() {
    let dir = tempfile::tempdir().unwrap();
    let d = daemon(dir.path());
    let first = registry(&d);
    let mut term = first.create("tab1");
    term.start_command("sh", "/", None);
    let session = first.get("tab1").unwrap();
    assert!(session.is_remote());
    let pid = term.pid().unwrap();
    session.send("echo antes-$((40+2))\r");
    assert!(until(Duration::from_secs(5), || screen(&session).contains("antes-42")));
    term.keep_alive();
    drop((term, session, first));
    assert_eq!(d.info().unwrap().sessions, 1);

    let second = registry(&d);
    let back = second.adopt_session("tab1").expect("the live session is adopted");
    assert_eq!(back.child_pid(), Some(pid));
    assert!(until(Duration::from_secs(5), || screen(&back).contains("antes-42")));
    back.send("echo depois-$((1+1))\r");
    assert!(until(Duration::from_secs(5), || screen(&back).contains("depois-2")));
    let _ = back.kill_and_wait().recv_blocking();
    assert!(until(Duration::from_secs(5), || back.is_exited()));
    assert!(until(Duration::from_secs(5), || d.info().unwrap().sessions == 0));
}

#[test]
fn a_session_that_ended_while_detached_comes_back_finished_with_its_screen() {
    let dir = tempfile::tempdir().unwrap();
    let d = daemon(dir.path());
    let first = registry(&d);
    let mut term = first.create("tab1");
    term.start_command("sh -c 'sleep 0.3; echo feito-$((6*7)); exit 3'", "/", None);
    term.keep_alive();
    drop((term, first));
    assert!(until(Duration::from_secs(5), || d.info().unwrap().sessions == 0));
    let screens = dir.path().join("ptyd-screens");
    assert!(until(Duration::from_secs(5), || screens.join("tab1.json").exists()));

    let back = registry(&d).adopt_session("tab1").expect("the finished session comes back");
    assert!(back.is_exited());
    assert_eq!(back.exit_status(), Some(3));
    assert!(screen(&back).contains("feito-42"));
    assert!(!screens.join("tab1.json").exists());
    assert!(registry(&d).adopt("tab1").is_none());
}

#[test]
fn prune_sweeps_screens_no_panel_claimed() {
    let dir = tempfile::tempdir().unwrap();
    let d = daemon(dir.path());
    let first = registry(&d);
    let mut term = first.create("gone");
    term.start_command("true", "/", None);
    term.keep_alive();
    drop((term, first));
    let file = dir.path().join("ptyd-screens").join("gone.json");
    assert!(until(Duration::from_secs(5), || file.exists()));
    registry(&d).prune_ptyd().unwrap();
    assert!(!file.exists());
}

#[test]
fn dropping_without_keep_alive_hangs_up_as_before() {
    let dir = tempfile::tempdir().unwrap();
    let d = daemon(dir.path());
    let first = registry(&d);
    let mut term = first.create("tab1");
    term.start_command("sleep 30", "/", None);
    assert!(term.pid().is_some());
    drop((term, first));
    assert!(until(Duration::from_secs(5), || d.info().unwrap().sessions == 0));
}

#[test]
fn shutdown_ends_every_session_and_frees_the_socket() {
    let dir = tempfile::tempdir().unwrap();
    let d = daemon(dir.path());
    let first = registry(&d);
    let mut term = first.create("tab1");
    term.start_command("sleep 30", "/", None);
    let session = first.get("tab1").unwrap();
    d.shutdown().unwrap();
    assert!(until(Duration::from_secs(5), || session.is_exited()));
    assert!(d.info().is_err());
}

#[test]
fn without_a_daemon_the_session_runs_in_process() {
    let dir = tempfile::tempdir().unwrap();
    let r = registry(&Ptyd::at(dir.path().join("nobody.sock")));
    let mut term = r.create("tab1");
    term.start_command("sleep 30", "/", None);
    assert!(term.pid().is_some());
    assert!(!r.get("tab1").unwrap().is_remote());
}

#[test]
fn prune_ends_the_sessions_nobody_adopted() {
    let dir = tempfile::tempdir().unwrap();
    let d = daemon(dir.path());
    let first = registry(&d);
    for id in ["kept", "orphan"] {
        let mut term = first.create(id);
        term.start_command("sleep 30", "/", None);
        term.keep_alive();
    }
    drop(first);
    let second = registry(&d);
    let kept = second.adopt_session("kept").unwrap();
    assert_eq!(second.prune_ptyd().unwrap(), 1);
    assert!(until(Duration::from_secs(5), || d.info().unwrap().sessions == 1));
    assert!(!kept.is_exited());
}
