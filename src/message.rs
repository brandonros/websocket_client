use crate::frame::WebSocketFrame;
use crate::opcode::WebSocketOpcode;
use crate::types::Result;

#[derive(Debug)]
pub struct WebSocketMessage {
    pub opcode: Option<WebSocketOpcode>,  // Opcode of the first frame in the message
    pub payload_buffer: Vec<u8>,               // Accumulated payload data
    pub is_complete: bool,                // True when the message is fully received
}

impl WebSocketMessage {
    pub fn new() -> Self {
        WebSocketMessage {
            opcode: None,
            payload_buffer: Vec::new(),
            is_complete: false,
        }
    }

    pub fn reset(&mut self) {
        self.opcode = None;
        self.payload_buffer.clear();
        self.is_complete = false;
    }

    pub fn append_frame(&mut self, frame: WebSocketFrame) -> Result<()> {
        if frame.opcode == WebSocketOpcode::Continuation && self.opcode.is_none() {
            return Err("Invalid continuation frame without a starting frame".into());
        }

        // Handle the first frame of a fragmented message
        if self.opcode.is_none() {
            self.opcode = Some(frame.opcode);
        }

        // Accumulate the payload
        self.payload_buffer.extend_from_slice(&frame.payload);

        // If the `fin` flag is set, mark the message as complete
        if frame.fin {
            self.is_complete = true;
        }

        Ok(())
    }

    pub fn get_message(&self) -> Option<&[u8]> {
        if self.is_complete {
            Some(&self.payload_buffer)
        } else {
            None
        }
    }
}
