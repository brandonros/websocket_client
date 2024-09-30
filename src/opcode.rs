#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WebSocketOpcode {
    Continuation = 0x0,
    Text = 0x1,
    Binary = 0x2,
    Close = 0x8,
    Ping = 0x9,
    Pong = 0xA,
}

impl WebSocketOpcode {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x0 => Some(WebSocketOpcode::Continuation),
            0x1 => Some(WebSocketOpcode::Text),
            0x2 => Some(WebSocketOpcode::Binary),
            0x8 => Some(WebSocketOpcode::Close),
            0x9 => Some(WebSocketOpcode::Ping),
            0xA => Some(WebSocketOpcode::Pong),
            _ => None,
        }
    }
}
