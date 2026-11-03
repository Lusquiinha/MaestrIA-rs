use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::session::GridSize;

pub(crate) const VERSION: u32 = 1;

pub(crate) const SPAWN: u8 = b'S';
pub(crate) const ATTACH: u8 = b'A';
pub(crate) const INPUT: u8 = b'I';
pub(crate) const RESIZE: u8 = b'R';
pub(crate) const READY: u8 = b'O';
pub(crate) const ERROR: u8 = b'E';
pub(crate) const DATA: u8 = b'D';
pub(crate) const EXIT: u8 = b'X';
pub(crate) const INFO: u8 = b'?';
pub(crate) const SHUTDOWN: u8 = b'Q';
pub(crate) const PRUNE: u8 = b'P';
pub(crate) const HIBERNATE: u8 = b'H';
pub(crate) const TAKEN: u8 = b'T';

const HEADER: usize = 5;
const MAX_FRAME: usize = 64 << 20;

pub(crate) type Frame = (u8, Vec<u8>);

pub(crate) fn encode(kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER + payload.len());
    out.push(kind);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

pub(crate) fn write_frame(mut w: impl Write, kind: u8, payload: &[u8]) -> io::Result<()> {
    w.write_all(&encode(kind, payload))
}

pub(crate) fn write_json(w: impl Write, kind: u8, value: &impl Serialize) -> io::Result<()> {
    write_frame(w, kind, &serde_json::to_vec(value).map_err(io::Error::other)?)
}

fn too_big() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "ptyd frame too large")
}

pub(crate) fn read_frame(mut r: impl Read) -> io::Result<Frame> {
    let mut header = [0u8; HEADER];
    r.read_exact(&mut header)?;
    let len = u32::from_le_bytes([header[1], header[2], header[3], header[4]]) as usize;
    if len > MAX_FRAME {
        return Err(too_big());
    }
    let mut payload = vec![0u8; len];
    r.read_exact(&mut payload)?;
    Ok((header[0], payload))
}

pub(crate) fn parse<'a, T: Deserialize<'a>>(payload: &'a [u8]) -> io::Result<T> {
    serde_json::from_slice(payload).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[derive(Default)]
pub(crate) struct Decoder {
    buf: Vec<u8>,
}

impl Decoder {
    pub fn push(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    pub fn next(&mut self) -> Option<io::Result<Frame>> {
        if self.buf.len() < HEADER {
            return None;
        }
        let len = u32::from_le_bytes([self.buf[1], self.buf[2], self.buf[3], self.buf[4]]) as usize;
        if len > MAX_FRAME {
            return Some(Err(too_big()));
        }
        if self.buf.len() < HEADER + len {
            return None;
        }
        let kind = self.buf[0];
        let payload = self.buf[HEADER..HEADER + len].to_vec();
        self.buf.drain(..HEADER + len);
        Some(Ok((kind, payload)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Size {
    pub cols: u16,
    pub rows: u16,
    pub cell_width: u16,
    pub cell_height: u16,
}

impl From<GridSize> for Size {
    fn from(s: GridSize) -> Self {
        Size { cols: s.cols, rows: s.rows, cell_width: s.cell_width, cell_height: s.cell_height }
    }
}

impl From<Size> for GridSize {
    fn from(s: Size) -> Self {
        GridSize { cols: s.cols.max(2), rows: s.rows.max(1), cell_width: s.cell_width, cell_height: s.cell_height }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Spawn {
    pub v: u32,
    pub key: String,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: HashMap<String, String>,
    pub scrollback: usize,
    pub size: Size,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Attach {
    pub v: u32,
    pub key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Ready {
    pub pid: Option<u32>,
    pub exited: bool,
    pub code: Option<i32>,
    pub size: Size,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Exit {
    pub code: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Info {
    pub v: u32,
    pub rev: String,
    pub pid: u32,
    pub sessions: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Finished {
    pub code: Option<i32>,
    pub size: Size,
    pub screen: String,
    #[serde(default)]
    pub hibernated: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Hibernate {
    pub after_secs: u64,
    pub keys: Vec<String>,
}

pub(crate) fn screens_dir(socket: &Path) -> PathBuf {
    socket.with_file_name("ptyd-screens")
}

pub(crate) fn screen_file(socket: &Path, key: &str) -> PathBuf {
    let name: String = key
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    screens_dir(socket).join(format!("{name}.json"))
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Prune {
    pub keep: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Failure {
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_waits_for_whole_frames_and_splits_glued_ones() {
        let mut bytes = encode(DATA, b"abc");
        bytes.extend(encode(EXIT, b"{}"));
        let mut d = Decoder::default();
        d.push(&bytes[..4]);
        assert!(d.next().is_none());
        d.push(&bytes[4..]);
        assert_eq!(d.next().unwrap().unwrap(), (DATA, b"abc".to_vec()));
        assert_eq!(d.next().unwrap().unwrap(), (EXIT, b"{}".to_vec()));
        assert!(d.next().is_none());
    }

    #[test]
    fn read_frame_round_trips() {
        let bytes = encode(INPUT, b"ls\r");
        assert_eq!(read_frame(&bytes[..]).unwrap(), (INPUT, b"ls\r".to_vec()));
    }
}
