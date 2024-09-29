use bytes::{Buf, BytesMut};

use crate::futures_provider::io::{BufReader, AsyncRead, AsyncReadExt};
use crate::types::Result;
use crate::frame::WebSocketFrame;

/// WebSocketReader reads WebSocket frames from the underlying stream.
pub struct WebSocketReader<R>
where
    R: AsyncRead + Unpin,
{
    reader: BufReader<R>,
    buffer: BytesMut,
}

impl<R> WebSocketReader<R>
where
    R: AsyncRead + Unpin,
{
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
            buffer: BytesMut::with_capacity(1024 * 1024),
        }
    }

    /// Parses a WebSocket frame from the buffer using nom.
    fn parse_frame(&mut self) -> Result<Option<WebSocketFrame>> {
        if self.buffer.is_empty() {
            return Ok(None); // Need more data
        }

        let input = &self.buffer[..];

        match WebSocketFrame::parse(input) {
            Ok((remaining, frame)) => {
                let parsed_len = input.len() - remaining.len();
                self.buffer.advance(parsed_len);
                Ok(Some(frame))
            }
            Err(nom::Err::Incomplete(_)) => {
                // Not enough data, need to read more
                Ok(None)
            }
            Err(e) => {
                // Parsing error
                Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Parsing error: {:?}", e),
                )
                .into())
            }
        }
    }

    /// Reads the next WebSocket frame, handling partial frames and buffering.
    pub async fn read_frame(&mut self) -> Result<Option<WebSocketFrame>> {
        loop {
            // Try to parse a frame from the buffer
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            // Read more data into the buffer
            let mut temp_buffer = [0u8; 4096];
            let n = self.reader.read(&mut temp_buffer).await?;
            if n == 0 {
                if self.buffer.is_empty() {
                    return Ok(None); // Stream closed and buffer empty
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "Stream closed with incomplete frame",
                    )
                    .into());
                }
            }
            self.buffer.extend_from_slice(&temp_buffer[..n]);
        }
    }
}
