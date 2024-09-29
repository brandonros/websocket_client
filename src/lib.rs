#[cfg(not(any(feature = "futures", feature = "futures-lite")))]
compile_error!(
    "You must enable either the `futures` or `futures-lite` feature to build this crate."
);

#[cfg(feature = "futures")]
use futures as futures_provider;

#[cfg(feature = "futures-lite")]
use futures_lite as futures_provider;

use futures_provider::io::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use bytes::{Buf, BytesMut};
use base64::Engine;
use rand::RngCore;
use nom::{
    bytes::streaming::take,
    number::streaming::{be_u16, be_u64, be_u8},
    IResult,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub trait AsyncConn: AsyncRead + AsyncWrite + Send + Sync + Unpin {}

impl<T: AsyncRead + AsyncWrite + Send + Sync + Unpin> AsyncConn for T {}

/// Represents a WebSocket frame.
#[derive(Debug)]
pub struct WebSocketFrame {
    pub fin: bool,
    pub opcode: u8,
    pub payload: Vec<u8>,
}

impl WebSocketFrame {
    pub fn parse(input: &[u8]) -> IResult<&[u8], WebSocketFrame> {
        let original_input_len = input.len();
        let (input, b0) = be_u8(input)?;
        let fin = b0 & 0x80 != 0;
        let opcode = b0 & 0x0F;
    
        let (input, b1) = be_u8(input)?;
        let masked = b1 & 0x80 != 0;
        let payload_len = (b1 & 0x7F) as usize;
    
        // Read extended payload length if necessary
        let (input, payload_len) = match payload_len {
            126 => {
                let (input, len) = be_u16(input)?;
                (input, len as usize)
            }
            127 => {
                let (input, len) = be_u64(input)?;
                (input, len as usize)
            }
            _ => (input, payload_len),
        };
    
        // Log the frame header information
        log::debug!(
            "Frame header: fin={}, opcode={}, masked={}, payload_len={}",
            fin,
            opcode,
            masked,
            payload_len
        );
    
        // Read masking key if the frame is masked
        let (input, masking_key) = if masked {
            let (input, key) = take(4usize)(input)?;
            (input, Some([key[0], key[1], key[2], key[3]]))
        } else {
            (input, None)
        };
    
        // Read the payload data
        let (input, payload_bytes) = take(payload_len)(input)?;
    
        let mut payload = payload_bytes.to_vec();
    
        // Apply the masking key if necessary
        if let Some(masking_key) = masking_key {
            for i in 0..payload_len {
                payload[i] ^= masking_key[i % 4];
            }
        }
    
        let total_parsed_len = original_input_len - input.len();
        log::debug!("Total parsed length: {}", total_parsed_len);
    
        Ok((
            input,
            WebSocketFrame {
                fin,
                opcode,
                payload,
            },
        ))
    }

    /// Creates a WebSocket frame from a message string (text frame).
    pub fn from_message(message: &str) -> WebSocketFrame {
        WebSocketFrame {
            fin: true,
            opcode: 0x1, // Text frame
            payload: message.as_bytes().to_vec(),
        }
    }

    /// Serializes the WebSocket frame into bytes for sending over the network.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut frame = Vec::new();

        // First byte: FIN, RSV1-3, Opcode
        let mut fin_rsv_opcode = 0;
        if self.fin {
            fin_rsv_opcode |= 0x80;
        }
        fin_rsv_opcode |= self.opcode & 0x0F;
        frame.push(fin_rsv_opcode);

        // Payload length and MASK bit
        let payload_len = self.payload.len();
        let mut payload_len_byte = 0x80; // MASK bit set to 1 for client-to-server frames

        if payload_len <= 125 {
            payload_len_byte |= payload_len as u8;
            frame.push(payload_len_byte);
        } else if payload_len <= 65535 {
            payload_len_byte |= 126;
            frame.push(payload_len_byte);
            frame.extend_from_slice(&(payload_len as u16).to_be_bytes());
        } else {
            payload_len_byte |= 127;
            frame.push(payload_len_byte);
            frame.extend_from_slice(&(payload_len as u64).to_be_bytes());
        }

        // Generate a random 4-byte masking key
        let mut masking_key = [0u8; 4];
        rand::thread_rng().fill_bytes(&mut masking_key);
        frame.extend_from_slice(&masking_key);

        // Mask the payload data
        let masked_payload: Vec<u8> = self
            .payload
            .iter()
            .enumerate()
            .map(|(i, byte)| byte ^ masking_key[i % 4])
            .collect();
        frame.extend_from_slice(&masked_payload);

        frame
    }
}

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

/// WebSocketWriter writes WebSocket frames to the underlying stream.
pub struct WebSocketWriter<W>
where
    W: AsyncWrite + Unpin,
{
    writer: BufWriter<W>,
}

impl<W> WebSocketWriter<W>
where
    W: AsyncWrite + Unpin,
{
    pub fn new(writer: W) -> Self {
        Self {
            writer: BufWriter::new(writer),
        }
    }

    pub async fn write_frame(&mut self, message: &str) -> Result<()> {
        let frame = WebSocketFrame::from_message(message);
        let frame_bytes = frame.to_bytes();

        // Write the frame to the writer
        self.writer.write_all(&frame_bytes).await?;
        self.writer.flush().await?;
        Ok(())
    }
}
pub struct WebSocketClientHelpers;

impl WebSocketClientHelpers {
    pub fn generate_sec_websocket_key() -> String {
        // Create a 16-byte buffer
        let mut key = [0u8; 16];
        
        // Fill the buffer with random bytes
        rand::thread_rng().fill_bytes(&mut key);
        
        // Encode the random bytes using base64
        base64::prelude::BASE64_STANDARD.encode(&key)
    }    
}
