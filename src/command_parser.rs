use err_derive::Error;
use heapless::Vec;

use crate::hal::time::Nanoseconds;

#[derive(Error)]
pub enum Error {
    #[error(display = "invalid command")]
    InvalidCommand,
    #[error(display = "command buffer full")]
    BufferFull,
    #[error(display = "invalid UTF-8")]
    Utf8(#[error(source)] core::str::Utf8Error),
}

pub struct Command {
    pub period: Nanoseconds,
    pub amplitude: u8,
}

pub struct Parser {
    buffer: Vec<u8, 32>,
}

impl Parser {
    pub const fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn parse(&mut self, data: u8) -> Result<Option<Command>, Error> {
        if let Err(_) = self.buffer.push(data) {
            self.buffer.clear();
            return Err(Error::BufferFull);
        }

        if data != b'\n' && data != b'\r' {
            // Haven't reached the end of the line yet
            return Ok(None);
        }

        let buffer = core::mem::replace(&mut self.buffer, Vec::new());
        let line = core::str::from_utf8(&buffer)?.trim();
        let mut parts = line.split_whitespace();

        let freq_str = parts.next().ok_or(Error::InvalidCommand)?;
        let amplitude_str = parts.next().ok_or(Error::InvalidCommand)?;

        let freq: f32 = freq_str.parse().map_err(|_| Error::InvalidCommand)?;
        let amplitude: u8 = amplitude_str.parse().map_err(|_| Error::InvalidCommand)?;

        let period = Nanoseconds((1_000_000_000f32 / freq) as u32);

        Ok(Some(Command { period, amplitude }))
    }
}
