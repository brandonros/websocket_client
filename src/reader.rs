use bytes::{Buf, BytesMut};
use futures_lite::io::{BufReader, AsyncRead, AsyncReadExt};
use simple_error::SimpleResult;

use crate::frame::WebSocketFrame;
use crate::message::WebSocketMessage;

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
    fn parse_frame(&mut self) -> SimpleResult<Option<WebSocketFrame>> {
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
    pub async fn read_frame(&mut self) -> SimpleResult<Option<WebSocketFrame>> {
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

    /// Reads a complete WebSocket message, handling fragmented frames.
    pub async fn read_message(&mut self) -> SimpleResult<Option<WebSocketMessage>> {
        let mut message = WebSocketMessage::new();

        // Keep reading frames until the entire message is accumulated
        loop {
            // Read the next frame
            match self.read_frame().await? {
                Some(frame) => {
                    // Append the frame to the message
                    message.append_frame(frame)?;

                    // If the message is complete, return it
                    if message.is_complete {
                        return Ok(Some(message));
                    }
                }
                None => {
                    // If no frame is returned, it means the stream has ended
                    if message.payload.is_empty() {
                        return Ok(None); // No message to return
                    } else {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "Stream closed before message was fully received",
                        )
                        .into());
                    }
                }
            }
        }
    }
}
