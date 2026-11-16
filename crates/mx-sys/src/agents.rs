use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub use mx_core::model::AgentInfo;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use crate::runner::CommandRunner;

pub const EAGER_EVERY: Duration = Duration::from_secs(2);
pub const IDLE_EVERY: Duration = Duration::from_secs(30);
pub const COMMAND: &str = "claude agents --json";
pub const TIMEOUT: Duration = Duration::from_secs(20);

pub fn parse_agents(stdout: &str) -> Option<Vec<AgentInfo>> {
    AgentInfo::list_from_stdout(stdout)
}

pub fn background_job_on_disk(sessions_dir: &Path, session_id: &str) -> Option<String> {
    std::fs::read_dir(sessions_dir).ok()?.flatten().find_map(|entry| {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            return None;
        }
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).ok()?).ok()?;
        if v.get("kind")?.as_str()? != "bg" || v.get("sessionId")?.as_str()? != session_id {
            return None;
        }
        v.get("jobId").and_then(serde_json::Value::as_str).map(str::to_owned)
    })
}

pub type AgentList = Arc<Vec<AgentInfo>>;

struct Shared {
    runner: Arc<dyn CommandRunner>,
    in_flight: AtomicBool,
    latest: watch::Sender<AgentList>,
}

impl Shared {
    async fn tick(self: Arc<Self>) {
        if self.in_flight.swap(true, Ordering::AcqRel) {
            return;
        }
        let r = tokio::time::timeout(TIMEOUT, self.runner.run(COMMAND.to_owned(), None)).await;
        if let Ok(r) = r
            && r.ok()
            && let Some(list) = parse_agents(&r.stdout)
        {
            self.latest.send_replace(Arc::new(list));
        }
        self.in_flight.store(false, Ordering::Release);
    }
}

pub struct AgentsWatcher {
    shared: Arc<Shared>,
    eager: watch::Sender<bool>,
    task: Option<JoinHandle<()>>,
}

impl AgentsWatcher {
    pub fn new(runner: Arc<dyn CommandRunner>) -> Self {
        let (latest, _) = watch::channel(Arc::new(Vec::new()));
        let (eager, _) = watch::channel(true);
        Self {
            shared: Arc::new(Shared { runner, in_flight: AtomicBool::new(false), latest }),
            eager,
            task: None,
        }
    }

    pub fn subscribe(&self) -> watch::Receiver<AgentList> {
        self.shared.latest.subscribe()
    }

    pub fn latest(&self) -> AgentList {
        self.shared.latest.borrow().clone()
    }

    pub fn eager(&self) -> bool {
        *self.eager.borrow()
    }

    pub fn set_eager(&self, value: bool) {
        self.eager.send_if_modified(|e| std::mem::replace(e, value) != value);
    }

    pub fn is_running(&self) -> bool {
        self.task.is_some()
    }

    pub fn start(&mut self) {
        self.stop();
        let shared = self.shared.clone();
        let mut eager = self.eager.subscribe();
        self.task = Some(tokio::spawn(async move {
            tokio::spawn(shared.clone().tick());
            loop {
                let period = if *eager.borrow_and_update() { EAGER_EVERY } else { IDLE_EVERY };
                tokio::select! {
                    _ = tokio::time::sleep(period) => {
                        tokio::spawn(shared.clone().tick());
                    }
                    changed = eager.changed() => {
                        if changed.is_err() {
                            break;
                        }
                        if *eager.borrow() {
                            tokio::spawn(shared.clone().tick());
                        }
                    }
                }
            }
        }));
    }

    pub fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

impl Drop for AgentsWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}
