//! Bidirectional message-level bridge between two WebSocket streams.

use futures_util::{SinkExt, Stream, StreamExt};

/// Like `tokio::io::copy_bidirectional` but operates on typed WS messages
/// instead of raw bytes, preserving message types (text, binary, etc.)
/// across the bridge via the provided conversion functions.
pub async fn ws_copy_bidirectional<A, B, MA, MB, EA, EB>(
    a: A,
    b: B,
    a_to_b: fn(MA) -> MB,
    b_to_a: fn(MB) -> MA,
) -> anyhow::Result<()>
where
    A: Stream<Item = Result<MA, EA>> + futures_util::Sink<MA, Error = EA> + Unpin,
    B: Stream<Item = Result<MB, EB>> + futures_util::Sink<MB, Error = EB> + Unpin,
    EA: Into<anyhow::Error>,
    EB: Into<anyhow::Error>,
{
    let (mut a_sink, mut a_stream) = a.split();
    let (mut b_sink, mut b_stream) = b.split();

    let forward = async {
        while let Some(msg) = a_stream.next().await {
            let msg = msg.map_err(Into::into)?;
            b_sink.send(a_to_b(msg)).await.map_err(Into::into)?;
        }
        let _ = b_sink.close().await;
        Ok::<(), anyhow::Error>(())
    };

    let backward = async {
        while let Some(msg) = b_stream.next().await {
            let msg = msg.map_err(Into::into)?;
            a_sink.send(b_to_a(msg)).await.map_err(Into::into)?;
        }
        let _ = a_sink.close().await;
        Ok::<(), anyhow::Error>(())
    };

    tokio::select! {
        result = forward => result,
        result = backward => result,
    }
}
