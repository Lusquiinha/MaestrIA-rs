mod client;
mod replay;
mod server;
mod wire;

pub use client::Ptyd;
pub(crate) use client::{send_input, send_resize, spawn_request, start};
pub use server::{run, serve, socket_in};
pub(crate) use wire::{Finished, Ready};
pub use wire::Info;

pub const FLAG: &str = "--ptyd";
pub const PROTOCOL: u32 = wire::VERSION;

const SOURCES: [&str; 5] = [
    include_str!("server.rs"),
    include_str!("wire.rs"),
    include_str!("replay.rs"),
    include_str!("../scrubber.rs"),
    include_str!("../signals.rs"),
];

const fn fnv(sources: &[&str]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut i = 0;
    while i < sources.len() {
        let bytes = sources[i].as_bytes();
        let mut j = 0;
        while j < bytes.len() {
            hash ^= bytes[j] as u64;
            hash = hash.wrapping_mul(0x0100_0000_01b3);
            j += 1;
        }
        i += 1;
    }
    hash
}

const REV: u64 = fnv(&SOURCES);

pub fn rev() -> String {
    format!("{REV:016x}")
}
