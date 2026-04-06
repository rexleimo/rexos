use std::sync::Arc;

use tokio::io::AsyncReadExt;

use crate::process_runtime::ProcessOutputBuffer;

pub(crate) fn spawn_process_output_reader(
    mut stream: impl tokio::io::AsyncRead + Unpin + Send + 'static,
    buffer: Arc<tokio::sync::Mutex<ProcessOutputBuffer>>,
) {
    tokio::spawn(async move {
        let mut tmp = [0u8; 4096];
        while let Ok(n) = stream.read(&mut tmp).await {
            if n == 0 {
                break;
            }

            let mut buf = buffer.lock().await;
            buf.push(&tmp[..n]);
        }
    });
}
