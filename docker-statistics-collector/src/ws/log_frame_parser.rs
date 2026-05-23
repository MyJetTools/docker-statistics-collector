/// Parses the docker multiplexed log stream framing
/// (`[stream_type:u8, 0, 0, 0, size:u32_be, payload(size bytes)]`).
/// Each frame's payload may contain one or more newline-separated lines.
/// Returns the parsed `(stream_type, line)` pairs and leaves any incomplete
/// trailing bytes in `buf`.
pub struct LogFrameParser {
    /// Bytes accumulated across calls — a frame may be split across two chunks
    /// (header partially in chunk N, payload in chunk N+1, etc.).
    buf: Vec<u8>,
}

impl LogFrameParser {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Drain as many complete frames as we can, splitting each frame's
    /// payload on `\n` into individual log lines.
    pub fn take_lines(&mut self) -> Vec<(u8, String)> {
        let mut out = Vec::new();
        loop {
            if self.buf.len() < 8 {
                break;
            }
            let tp = self.buf[0];
            // Bytes 1..4 are reserved zeros in docker's framing.
            let size = u32::from_be_bytes([self.buf[4], self.buf[5], self.buf[6], self.buf[7]]) as usize;
            let frame_total = 8 + size;
            if self.buf.len() < frame_total {
                break;
            }

            let payload = &self.buf[8..frame_total];
            // A single frame can carry multiple log lines. Split on \n and
            // discard a trailing empty fragment (common when payload ends with \n).
            for raw_line in payload.split(|b| *b == b'\n') {
                if raw_line.is_empty() {
                    continue;
                }
                match std::str::from_utf8(raw_line) {
                    Ok(s) => out.push((tp, s.to_string())),
                    Err(_) => out.push((tp, String::from_utf8_lossy(raw_line).into_owned())),
                }
            }

            self.buf.drain(..frame_total);
        }
        out
    }
}
