use std::sync::Arc;

use mx_store::testing::{FakeBus, FakeClock, FakeHooks, FakeNotifier, FakeTerms};
use mx_store::{Ports, Store, StoreOptions};
use mx_sys::FakeRunner;
use tempfile::TempDir;

pub struct Rig {
    pub s: Store,
    pub rt: Arc<tokio::runtime::Runtime>,
    pub dir: TempDir,
}

pub fn rig() -> Rig {
    let rt = Arc::new(tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap());
    let dir = tempfile::tempdir().unwrap();
    let ports = Ports {
        clock: Arc::new(FakeClock::new(chrono::Local::now())),
        terms: Arc::new(FakeTerms::default()),
        runner: Arc::new(FakeRunner::new()),
        notifier: Arc::new(FakeNotifier::default()),
        plugins: Arc::new(FakeBus::default()),
        hooks: Arc::new(FakeHooks { port: 4242 }),
        rt: rt.handle().clone(),
    };
    let opts = StoreOptions::at(dir.path().join("state"), dir.path().join("chats"), dir.path().to_str());
    Rig { s: Store::new(opts, ports), rt, dir }
}
