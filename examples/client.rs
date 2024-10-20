use std::sync::Arc;

use async_executor::Executor;
use http::{Request, Uri, Version};
use simple_error::SimpleResult;
use websocket_client::WebSocketClient;

#[macro_rules_attribute::apply(smol_macros::main!)]
async fn main(executor: Arc<Executor<'static>>) -> SimpleResult<()> {
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
        .body(vec![])?;
    let (mut ws_reader, mut ws_writer) = WebSocketClient::open(request).await?;

    // start task for reading in a loop using the reader
    let handle = executor.spawn(async move {
        loop {
            match ws_reader.read_message().await {
                Ok(result) => {
                    match result {
                        Some(frame) => {
                            log::info!("frame_payload = {:02x?}", frame.payload);
                        },
                        None => {
                            // TODO: is this same as close?
                            log::warn!("failed to read frame?");
                            break;
                        }
                    }
                },
                Err(err) => {
                    if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
                        match io_err.kind() {
                            std::io::ErrorKind::UnexpectedEof => {
                                log::warn!("Reached unexpected end of file");
                                break; // Exit the loop if you want to stop on EOF
                            }
                            _ => {
                                log::error!("IO error: {:?}", io_err);
                            }
                        }
                    } else {
                        log::error!("Non-IO error: {:?}", err);
                    }
                },
            }
        }
    });

    // send a frame
    ws_writer.write_text_message(r#"~m~54~m~{"m":"set_auth_token","p":["unauthorized_user_token"]}"#).await.expect("failed to write frame");

    // block on reader task
    handle.await;

    Ok(())
}
