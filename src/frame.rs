use nom::{
    bytes::streaming::take as take_streaming,
    number::streaming::{be_u8 as be_u8_streaming, be_u16 as be_u16_streaming},

    IResult,
};
use rand::RngCore;

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
        let (input, b0) = be_u8_streaming(input)?;
        let fin = b0 & 0x80 != 0;
        let opcode = b0 & 0x0F;
    
        let (input, b1) = be_u8_streaming(input)?;
        let masked = b1 & 0x80 != 0;
        let payload_len = (b1 & 0x7F) as usize;
    
        // Read extended payload length if necessary
        let (input, payload_len) = match payload_len {
            126 => {
                let (input, len) = be_u16_streaming(input)?;
                (input, len as usize)
            }
            127 => {
                let (input, len) = be_u16_streaming(input)?;
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
            let (input, key) = take_streaming(4usize)(input)?;
            (input, Some([key[0], key[1], key[2], key[3]]))
        } else {
            (input, None)
        };
    
        // Read the payload data
        let (input, payload_bytes) = take_streaming(payload_len)(input)?;
    
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