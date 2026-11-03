use std::fs::OpenOptions;
use std::io::{self, ErrorKind, Read};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};

use super::wire::{self, Attach, Decoder, Exit, Failure, Finished, Hibernate, Info, Prune, Ready, Size, Spawn};
use crate::io::{Feeder, LifeIo, LifeOwner, Port, SharedTerm, Sink, Wake, Writer, release_writer};
use crate::session::GridSize;

const HANDSHAKE: Duration = Duration::from_secs(5);
const INPUT_TIMEOUT: Duration = Duration::from_secs(2);
const START_WAIT: Duration = Duration::from_secs(3);
const READ_CHUNK: usize = 0x10000;

#[derive(Clone, Debug)]
pub struct Ptyd {
    socket: PathBuf,
}

impl Ptyd {
    pub fn at(socket: impl Into<PathBuf>) -> Ptyd {
        Ptyd { socket: socket.into() }
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }

    pub fn ensure(socket: impl Into<PathBuf>, exe: &Path, log: Option<&Path>) -> io::Result<Ptyd> {
        let ptyd = Ptyd::at(socket);
        if ptyd.info().is_ok() {
            return Ok(ptyd);
        }
        ptyd.launch(exe, log)?;
        let deadline = Instant::now() + START_WAIT;
        loop {
            match ptyd.info() {
                Ok(_) => return Ok(ptyd),
                Err(err) if Instant::now() >= deadline => return Err(err),
                Err(_) => std::thread::sleep(Duration::from_millis(30)),
            }
        }
    }

    fn launch(&self, exe: &Path, log: Option<&Path>) -> io::Result<()> {
        let stderr = log
            .and_then(|p| OpenOptions::new().create(true).append(true).open(p).ok())
            .map_or_else(Stdio::null, Stdio::from);
        let mut child = Command::new(exe)
            .arg(super::FLAG)
            .arg(&self.socket)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(stderr)
            .spawn()?;
        std::thread::spawn(move || child.wait());
        Ok(())
    }

    fn connect(&self) -> io::Result<UnixStream> {
        let stream = UnixStream::connect(&self.socket)?;
        stream.set_read_timeout(Some(HANDSHAKE))?;
        stream.set_write_timeout(Some(HANDSHAKE))?;
        Ok(stream)
    }

    pub fn info(&self) -> io::Result<Info> {
        let stream = self.connect()?;
        wire::write_frame(&stream, wire::INFO, b"")?;
        match wire::read_frame(&stream)? {
            (wire::INFO, payload) => wire::parse(&payload),
            _ => Err(io::Error::new(ErrorKind::InvalidData, "resposta inesperada do ptyd")),
        }
    }

    pub fn prune(&self, keep: Vec<String>) -> io::Result<usize> {
        let stream = self.connect()?;
        wire::write_json(&stream, wire::PRUNE, &Prune { keep })?;
        match wire::read_frame(&stream)? {
            (wire::PRUNE, payload) => wire::parse(&payload),
            _ => Err(io::Error::new(ErrorKind::InvalidData, "resposta inesperada do ptyd")),
        }
    }

    pub(crate) fn take_finished(&self, key: &str) -> Option<Finished> {
        let file = wire::screen_file(&self.socket, key);
        let bytes = std::fs::read(&file).ok()?;
        let _ = std::fs::remove_file(&file);
        wire::parse(&bytes).ok()
    }

    pub fn sweep_finished(&self, keep: &[String]) {
        let Ok(entries) = std::fs::read_dir(wire::screens_dir(&self.socket)) else { return };
        let kept: Vec<PathBuf> = keep.iter().map(|k| wire::screen_file(&self.socket, k)).collect();
        for entry in entries.flatten() {
            let path = entry.path();
            if !kept.contains(&path) {
                let _ = std::fs::remove_file(path);
            }
        }
    }

    pub fn hibernate_when_idle(&self, keys: Vec<String>, after: Duration) -> io::Result<()> {
        let stream = self.connect()?;
        wire::write_json(&stream, wire::HIBERNATE, &Hibernate { after_secs: after.as_secs(), keys })?;
        wire::read_frame(&stream).map(drop)
    }

    pub fn shutdown(&self) -> io::Result<()> {
        let stream = self.connect()?;
        wire::write_frame(&stream, wire::SHUTDOWN, b"")?;
        let _ = wire::read_frame(&stream);
        Ok(())
    }

    pub(crate) fn spawn(&self, req: &Spawn) -> io::Result<(UnixStream, Ready)> {
        let stream = self.connect()?;
        wire::write_json(&stream, wire::SPAWN, req)?;
        let ready = reply(&stream)?;
        Ok((stream, ready))
    }

    pub(crate) fn attach(&self, key: &str) -> io::Result<(UnixStream, Ready)> {
        let stream = self.connect()?;
        wire::write_json(&stream, wire::ATTACH, &Attach { v: wire::VERSION, key: key.to_owned() })?;
        let ready = reply(&stream)?;
        Ok((stream, ready))
    }
}

fn reply(stream: &UnixStream) -> io::Result<Ready> {
    match wire::read_frame(stream)? {
        (wire::READY, payload) => wire::parse(&payload),
        (wire::ERROR, payload) => {
            let failure: Failure = wire::parse(&payload)?;
            Err(io::Error::other(failure.message))
        }
        _ => Err(io::Error::new(ErrorKind::InvalidData, "resposta inesperada do ptyd")),
    }
}

pub(crate) fn send_input(stream: &UnixStream, bytes: &[u8]) -> io::Result<()> {
    wire::write_frame(stream, wire::INPUT, bytes)
}

pub(crate) fn send_resize(stream: &UnixStream, size: GridSize) -> io::Result<()> {
    wire::write_json(stream, wire::RESIZE, &Size::from(size))
}

pub(crate) fn start(
    stream: UnixStream,
    generation: u64,
    term: SharedTerm,
    port: Arc<Port>,
    feeder: Arc<Mutex<Feeder>>,
    owner: Weak<dyn LifeOwner>,
) -> io::Result<LifeIo> {
    let input = stream.try_clone()?;
    input.set_write_timeout(Some(INPUT_TIMEOUT))?;
    if let Ok(mut slot) = port.writer.lock() {
        *slot = Some(Writer { generation, sink: Sink::Remote(input) });
    }
    let stop = Arc::new(AtomicBool::new(false));
    let wake = stream.try_clone()?;
    let thread_stop = stop.clone();
    std::thread::Builder::new().name(format!("mx-ptyd-{generation}")).spawn(move || {
        Remote { stream, generation, term, port, feeder, owner, stop: thread_stop }.run();
    })?;
    Ok(LifeIo { stop, wake: Wake::Socket(wake) })
}

struct Remote {
    stream: UnixStream,
    generation: u64,
    term: SharedTerm,
    port: Arc<Port>,
    feeder: Arc<Mutex<Feeder>>,
    owner: Weak<dyn LifeOwner>,
    stop: Arc<AtomicBool>,
}

impl Remote {
    fn run(&mut self) {
        let mut decoder = Decoder::default();
        let mut buf = vec![0u8; READ_CHUNK];
        let code = 'outer: loop {
            if self.stop.load(Ordering::Acquire) {
                return;
            }
            let timeout = self
                .deadline()
                .map(|d| d.saturating_duration_since(Instant::now()).max(Duration::from_millis(1)));
            if self.stream.set_read_timeout(timeout).is_err() {
                break None;
            }
            match self.stream.read(&mut buf) {
                Ok(0) => break None,
                Ok(n) => {
                    decoder.push(&buf[..n]);
                    while let Some(frame) = decoder.next() {
                        match frame {
                            Ok((wire::DATA, bytes)) => self.feed(&bytes),
                            Ok((wire::EXIT, payload)) => {
                                break 'outer wire::parse::<Exit>(&payload).ok().and_then(|e| e.code);
                            }
                            Ok(_) => {}
                            Err(_) => break 'outer None,
                        }
                    }
                }
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted) => {}
                Err(_) => break None,
            }
            self.expire();
        };
        if self.stop.load(Ordering::Acquire) {
            return;
        }
        if let Some(owner) = self.owner.upgrade() {
            owner.reap(self.generation, &mut || Some(code));
        }
    }

    fn feed(&self, bytes: &[u8]) {
        if let Ok(mut f) = self.feeder.lock() {
            f.feed_from(self.generation, &self.term, &self.port, bytes);
        }
    }

    fn deadline(&self) -> Option<Instant> {
        self.feeder.lock().ok().and_then(|f| f.deadline())
    }

    fn expire(&self) {
        if self.deadline().is_some_and(|d| Instant::now() >= d)
            && let Ok(mut f) = self.feeder.lock()
            && f.generation == self.generation
        {
            f.expire(&self.term, &self.port);
        }
    }
}

impl Drop for Remote {
    fn drop(&mut self) {
        release_writer(&self.port, self.generation);
    }
}

pub(crate) fn spawn_request(
    key: &str,
    plan: crate::spawn::SpawnPlan,
    scrollback: usize,
    size: GridSize,
) -> Spawn {
    Spawn {
        v: wire::VERSION,
        key: key.to_owned(),
        program: plan.program,
        args: plan.args,
        cwd: plan.cwd,
        env: plan.env,
        scrollback,
        size: size.into(),
    }
}
