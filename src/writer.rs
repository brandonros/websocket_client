use futures_lite::io::{BufWriter, AsyncWrite, AsyncWriteExt};
use simple_error::SimpleResult;

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

    pub async fn write_text_message(&mut self, message: &str) -> SimpleResult<()> {
        let frame = WebSocketFrame::build_text_frame(message);
        let frame_bytes = frame.to_bytes();

        // Write the frame to the writer
        self.writer.write_all(&frame_bytes).await?;
        self.writer.flush().await?;
        Ok(())
    }

    pub async fn write_close_message(&mut self) -> SimpleResult<()> {
        let frame = WebSocketFrame::build_close_frame();
        let frame_bytes = frame.to_bytes();

        // Write the frame to the writer
        self.writer.write_all(&frame_bytes).await?;
        self.writer.flush().await?;

        // close the writer
        self.writer.close().await?;
        Ok(())
    }
}
