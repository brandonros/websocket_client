use crate::futures_provider::io::{BufWriter, AsyncWrite, AsyncWriteExt};
use crate::types::Result;
use crate::frame::WebSocketFrame;

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

    pub async fn write_text_message(&mut self, message: &str) -> Result<()> {
        let frame = WebSocketFrame::build_text_frame(message);
        let frame_bytes = frame.to_bytes();

        // Write the frame to the writer
        self.writer.write_all(&frame_bytes).await?;
        self.writer.flush().await?;
        Ok(())
    }
}
