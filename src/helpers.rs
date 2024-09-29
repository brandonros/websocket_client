use base64::Engine;
use rand::RngCore;

pub struct WebSocketHelpers;

impl WebSocketHelpers {
    pub fn generate_sec_websocket_key() -> String {
        // Create a 16-byte buffer
        let mut key = [0u8; 16];
        
        // Fill the buffer with random bytes
        rand::thread_rng().fill_bytes(&mut key);
        
        // Encode the random bytes using base64
        base64::prelude::BASE64_STANDARD.encode(&key)
    }    
}
