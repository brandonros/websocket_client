use base64::Engine;
use futures_lite::io::{BufReader, BufWriter};
use futures_lite::{AsyncRead, AsyncWrite};
use http::{HeaderValue, Request};
use http_client::HttpClient;
use rand::RngCore;
use simple_error::SimpleResult;

use crate::reader::WebSocketReader;
use crate::writer::WebSocketWriter;

pub struct WebSocketClient;

impl WebSocketClient {
    fn generate_sec_websocket_key() -> String {
        // Create a 16-byte buffer
        let mut key = [0u8; 16];
        
        // Fill the buffer with random bytes
        rand::thread_rng().fill_bytes(&mut key);
        
        // Encode the random bytes using base64
        base64::prelude::BASE64_STANDARD.encode(&key)
    }    
    
    pub async fn open(mut request: Request<Vec<u8>>) -> SimpleResult<(WebSocketReader<impl AsyncRead>, WebSocketWriter<impl AsyncWrite>)> {    
        // Add WebSocket-specific headers
        request.headers_mut().insert("Connection", HeaderValue::from_static("Upgrade"));
        request.headers_mut().insert("Upgrade", HeaderValue::from_static("websocket"));
        request.headers_mut().insert("Sec-WebSocket-Version", HeaderValue::from_static("13"));
        request.headers_mut().insert("Sec-WebSocket-Key", HeaderValue::from_str(&Self::generate_sec_websocket_key())?);
        // TODO: Sec-WebSocket-Extensions?

        // Open connection
        let mut stream = HttpClient::create_connection(&request).await?;

        // Make HTTP request
        let response = HttpClient::request(&mut stream, &request).await?;
        log::debug!("response = {response:?}");
    
        // split connection
        let (reader, writer) = futures_lite::io::split(stream);
        let reader = BufReader::new(reader);
        let writer = BufWriter::new(writer);
    
        // create websocket reader/writer
        let ws_reader = WebSocketReader::new(reader);
        let ws_writer = WebSocketWriter::new(writer);
        
        Ok((ws_reader, ws_writer))
    }
}
