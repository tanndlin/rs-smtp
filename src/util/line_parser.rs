use std::{io::Read, net::TcpStream};

pub struct LineParser {
    stream: TcpStream,
    buf: Vec<u8>,
    read_buf: [u8; 4096],
}

impl LineParser {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            buf: vec![],
            read_buf: [0; 4096],
        }
    }

    pub fn next_line(&mut self) -> Result<String, String> {
        // TODO: This might be improved by an event system instead of linear scan

        println!("Buf len {}", self.buf.len());

        // Check for \r\n in current buf
        for (i, window) in self.buf.windows(2).enumerate() {
            if window == b"\r\n" {
                let message = str::from_utf8(&self.buf[..i + 2])
                    .map_err(|_| "Failed to parse message")?
                    .to_string();

                self.buf.drain(0..message.len());
                println!("Buf len {}", self.buf.len());

                return Ok(message);
            }
        }

        let bytes_read = self
            .stream
            .read(&mut self.read_buf)
            .map_err(|_| "Failed to read from stream")?;

        if bytes_read == 0 {
            // Connection closed
            return Err("Connection Closed".to_string());
        }

        println!("read {bytes_read} bytes");
        self.buf.extend(&self.read_buf[..bytes_read]);
        self.next_line()
    }
}
