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

struct TradingViewFrame;

impl TradingViewFrame {
    pub fn serialize(input: &str) -> String {
        let input_len = input.len();
        format!("~m~{input_len}~m~{input}")
    }
}

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
        let mut ws_client = WebSocketClient::new(reader, writer);

        // get server hello frame
        let _server_hello_frame = ws_client.read_frame().await.expect("failed to read frame").expect("failed to get server hello");

        // set auth token
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"set_auth_token","p":["unauthorized_user_token"]}"#)).await.expect("failed to write");
    
        // set locale
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"set_locale","p":["en","US"]}"#)).await.expect("failed to write");

        // create chart session
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"chart_create_session","p":["cs_2AzYWhCHOwik",""]}"#)).await.expect("failed to write");        

        // switch chart timezone
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"switch_timezone","p":["cs_2AzYWhCHOwik","exchange"]}"#)).await.expect("failed to write");        

        // create quote session
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"quote_create_session","p":["qs_EaDCc5CHTQaa"]}"#)).await.expect("failed to write");        
        // add symbol to quote session
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"quote_add_symbols","p":["qs_EaDCc5CHTQaa","={\"adjustment\":\"splits\",\"session\":\"extended\",\"symbol\":\"AMEX:SPY\"}"]}"#)).await.expect("failed to write");                

        // read series loading frame
        let _series_loading_frame = ws_client.read_frame().await.expect("failed to read frame").expect("failed to get series loading");

        // resolve symbol
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"resolve_symbol","p":["cs_2AzYWhCHOwik","sds_sym_1","={\"adjustment\":\"splits\",\"session\":\"extended\",\"symbol\":\"AMEX:SPY\"}"]}"#)).await.expect("failed to write");                        

        // add symbol to chart session as series
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"create_series","p":["cs_2AzYWhCHOwik","sds_1","s1","sds_sym_1","15",300,""]}"#)).await.expect("failed to write");                                

        // create quote session
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"quote_create_session","p":["qs_CbGU6IdgHeyC"]}"#)).await.expect("failed to write");                                        

        // set quote session fields
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"quote_set_fields","p":["qs_CbGU6IdgHeyC","base-currency-logoid","ch","chp","currency-logoid","currency_code","currency_id","base_currency_id","current_session","description","exchange","format","fractional","is_tradable","language","local_description","listed_exchange","logoid","lp","lp_time","minmov","minmove2","original_name","pricescale","pro_name","short_name","type","typespecs","update_mode","volume","variable_tick_size","value_unit_id","unit_id","measure"]}"#)).await.expect("failed to write");                                                

        // add symbol to quote session
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"quote_add_symbols","p":["qs_CbGU6IdgHeyC","AMEX:SPY"]}"#)).await.expect("failed to write");                                                        

        // read quote session data frame
        let _quote_session_data_frame = ws_client.read_frame().await.expect("failed to read frame").expect("failed to get quote session data");

        // request more tickmarks from symbol
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"request_more_tickmarks","p":["cs_2AzYWhCHOwik","sds_1",10]}"#)).await.expect("failed to write");                                                        

        // turn on quote fast symbols for quote session
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"quote_fast_symbols","p":["qs_EaDCc5CHTQaa","={\"adjustment\":\"splits\",\"currency-id\":\"USD\",\"session\":\"extended\",\"symbol\":\"BATS:SPY\"}"]}"#)).await.expect("failed to write");                                                        

        // add studies to series in chart session
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"create_study","p":["cs_2AzYWhCHOwik","st1","sessions_1","sds_1","Sessions@tv-basicstudies-241",{}]}"#)).await.expect("failed to write");                                                        
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"create_study","p":["cs_2AzYWhCHOwik","st2","st1","sds_1","Script@tv-scripting-101!",{"text":"bmI9Ks46_4gc7PrwVT7kbVtIYR/9uGg==_z7csuIzdePr6JPnbZAwqsKwEUkTI7TrTZSgEmde35UKVbdqohGhUD1yCWiHpt+B1Q0vIrmanbmURoNk6YsXJIkATAAugCZIFBchkXEXEtHVYTL93KHjI70Y6xwlV7ajXNuA2vCc+i7Ir3NLpZEhOidaTK/p+FivGCrxfI3A6ooM4GZLFC0oEYkpcGCLSdP9IpSP9SKquOsBQmgpMoZ384QSD8qMK924eJbLjN1wSpjSp8LVHF+IqTWbYpx9ZlmXUU20bs6EY3EwAOG3qf40qdHoyPAL4UG6TrP5+V3h2I5CootDH13gZAtI75hdNhpJbUJDNAgvkKcVRDx6O8BIbmjzSeJ6C2+btsFxFmIFfcZHPie5dPPyAsd7ewSjmFVToSbvXw6KF+2y0+H3uk09hqj0a2F1F8WRJ15zmyCpuRNNHxOJtl3OatH2/MbJcKWn61/3bD+lY9HODKmnhLsZ8sZNF0uV2+QShPIlBARfnh3Nl8eUDQ+g4Z/2KZihDzb7hJZvQbkPAd/BDyXK7h4jvuZ0PBm07SxVaQapPfDrgeLJiimT9unatDTdgZNthoW7WoqwdtuENC76CEGiq3/llQYn7i/VAwaBMM/QnQBTlXdB7l+k=","pineId":"PUB;K07lTKE3tPla7glvOhmesGYj7Veuq4eX","pineVersion":"1.0","in_0":{"v":14,"f":true,"t":"integer"}}]}"#)).await.expect("failed to write");                                                        
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"create_study","p":["cs_2AzYWhCHOwik","st3","st1","sds_1","Dividends@tv-basicstudies-241",{}]}"#)).await.expect("failed to write");                                                        
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"create_study","p":["cs_2AzYWhCHOwik","st4","st1","sds_1","Splits@tv-basicstudies-241",{}]}"#)).await.expect("failed to write");                                                        
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"create_study","p":["cs_2AzYWhCHOwik","st5","st1","sds_1","Earnings@tv-basicstudies-241",{}]}"#)).await.expect("failed to write");                                                        
        ws_client.write_frame(&TradingViewFrame::serialize(r#"{"m":"create_study","p":["cs_2AzYWhCHOwik","st6","st1","sds_1","BarSetContinuousRollDates@tv-corestudies-28",{"currenttime":"now"}]}"#)).await.expect("failed to write");                                                        

        // Read back frames
        loop {
            match ws_client.read_frame().await.unwrap() {
                Some(frame) => {
                    let text = String::from_utf8(frame.payload).unwrap();
                    log::info!("Received frame: {}", text);
                }
                None => {
                    panic!("stream closed");
                }
            }
        }
    })
}
