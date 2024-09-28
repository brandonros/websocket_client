#[cfg(not(any(feature = "futures", feature = "futures-lite")))]
compile_error!(
    "You must enable either the `futures` or `futures-lite` feature to build this crate."
);

#[cfg(feature = "futures")]
use futures as futures_provider;

#[cfg(feature = "futures-lite")]
use futures_lite as futures_provider;

use futures_provider::io::{BufReader, BufWriter};
use http::{Request, Uri, Version};
use http_client::HttpClient;
use websocket_client::{WebSocketClient, WebSocketClientHelpers};

fn main() {
    futures_provider::future::block_on(async {
        // init logging
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
        
        // Build the URI for the request
        let uri: Uri = "wss://data.tradingview.com/socket.io/websocket?from=chart%2F&date=2024_09_25-14_09&type=chart".parse().expect("Failed to parse URI");

        // Build the GET request
        let request = Request::builder()
            .method("GET")
            .version(Version::HTTP_11)
            .uri(uri)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Safari/537.36")
            .header("Host", "data.tradingview.com")
            .header("Origin", "https://www.tradingview.com")            
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")      
            .header("Sec-WebSocket-Version", "13")                        
            .header("Sec-WebSocket-Key", WebSocketClientHelpers::generate_sec_websocket_key())    
            //.header("Sec-WebSocket-Extensions", "permessage-deflate; client_max_window_bits")
            .body(())
            .expect("Failed to build request");

        // Get the response
        let mut stream = HttpClient::connect(&request).await.expect("connect failed");
        let response = HttpClient::send::<(), String>(&mut stream, &request).await.expect("request failed");
        log::info!("response = {response:?}");

        // split
        let (reader, writer) = futures_provider::io::split(stream);
        let reader = BufReader::new(reader);
        let writer = BufWriter::new(writer);

        // create websocket client
        let ws_client = WebSocketClient::new(reader, writer);

        // TODO: read/write
    })
}
