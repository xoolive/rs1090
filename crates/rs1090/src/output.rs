//! Shared JSONL file output with optional streaming Zstandard compression.
//!
//! Filenames ending in `.jsonl.zst` select compression. Writers flush once a
//! second, including while idle, so readers can follow an unfinished stream.
//! Call [`JsonlWriter::finish`] before dropping to write the frame footer.

use async_compression::tokio::bufread::ZstdDecoder;
use async_compression::tokio::write::ZstdEncoder;
use std::future::Future;
use std::io;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::fs::{File, OpenOptions};
use tokio::io::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, BufWriter,
    ReadBuf,
};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{Duration, MissedTickBehavior};

// EOF means "no new bytes yet", not the end of a still-growing stream.
struct GrowingFile {
    file: File,
    retry: Option<Pin<Box<tokio::time::Sleep>>>,
}

impl AsyncRead for GrowingFile {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if buf.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        loop {
            if let Some(retry) = &mut this.retry {
                std::task::ready!(retry.as_mut().poll(cx));
                this.retry = None;
            }
            let before = buf.filled().len();
            std::task::ready!(Pin::new(&mut this.file).poll_read(cx, buf))?;
            if buf.filled().len() > before {
                return Poll::Ready(Ok(()));
            }
            this.retry =
                Some(Box::pin(tokio::time::sleep(Duration::from_millis(100))));
        }
    }
}

/// Copy a growing JSONL file from its first byte, optionally decompressing it.
///
/// Unlike regular file readers, temporary EOF waits for newly appended bytes.
/// This follows the opened file, not rotation/replacement or truncation. Cancel
/// the future to stop. Output chunks are flushed immediately for live consumers.
pub async fn follow_file(
    path: impl AsRef<Path>,
    mut output: impl AsyncWrite + Unpin,
) -> io::Result<()> {
    let path = path.as_ref();
    let file = GrowingFile {
        file: File::open(path).await?,
        retry: None,
    };
    let mut reader: Pin<Box<dyn AsyncRead + Send>> = if is_compressed(path) {
        let mut decoder = ZstdDecoder::new(BufReader::new(file));
        decoder.multiple_members(true);
        Box::pin(decoder)
    } else {
        Box::pin(file)
    };
    let mut buffer = [0; 16 * 1024];
    loop {
        let count = reader.read(&mut buffer).await?;
        if count == 0 {
            return Ok(());
        }
        output.write_all(&buffer[..count]).await?;
        output.flush().await?;
    }
}

fn is_compressed(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.to_string_lossy().ends_with(".jsonl.zst"))
}

struct State {
    writer: Pin<Box<dyn AsyncWrite + Send>>,
    flush_error: Option<(io::ErrorKind, String)>,
    finished: bool,
}

impl State {
    fn check(&self) -> io::Result<()> {
        if let Some((kind, message)) = &self.flush_error {
            return Err(io::Error::new(*kind, message.clone()));
        }
        if self.finished {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Output already finished",
            ));
        }
        Ok(())
    }
}

/// Buffered file writer shared by the live and batch applications.
///
/// Compression is selected exclusively by the `.jsonl.zst` filename suffix.
/// Appending starts a new Zstandard frame; readers must support concatenated
/// frames. Periodic flush failures are returned on the next write or finish.
pub struct JsonlWriter {
    state: Arc<Mutex<State>>,
    flush_task: JoinHandle<()>,
}

impl JsonlWriter {
    /// Open a file, preserving existing content only when `append` is true.
    pub async fn open(
        path: impl AsRef<Path>,
        append: bool,
    ) -> io::Result<Self> {
        let path = path.as_ref();
        let compressed = is_compressed(path);
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(append)
            .truncate(!append)
            .open(path)
            .await?;
        let buffered = BufWriter::with_capacity(1024 * 1024, file);
        let writer: Pin<Box<dyn AsyncWrite + Send>> = if compressed {
            Box::pin(ZstdEncoder::new(buffered))
        } else {
            Box::pin(buffered)
        };
        let state = Arc::new(Mutex::new(State {
            writer,
            flush_error: None,
            finished: false,
        }));
        let flush_state = state.clone();
        let flush_task = tokio::spawn(async move {
            let period = Duration::from_secs(1);
            let mut interval = tokio::time::interval_at(
                tokio::time::Instant::now() + period,
                period,
            );
            interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let mut state = flush_state.lock().await;
                if state.finished {
                    break;
                }
                if let Err(error) = state.writer.flush().await {
                    tracing::error!(%error, "Could not flush JSONL output");
                    state.flush_error = Some((error.kind(), error.to_string()));
                    break;
                }
            }
        });
        Ok(Self { state, flush_task })
    }

    /// Write one JSON record and its newline without flushing a partial line.
    pub async fn write_line(&mut self, json: &str) -> io::Result<()> {
        let mut state = self.state.lock().await;
        state.check()?;
        state.writer.write_all(json.as_bytes()).await?;
        state.writer.write_all(b"\n").await
    }

    /// Flush all bytes and finalize the compressed stream before closing.
    pub async fn finish(&mut self) -> io::Result<()> {
        self.flush_task.abort();
        let mut state = self.state.lock().await;
        if state.finished {
            return Ok(());
        }
        state.check()?;
        state.writer.shutdown().await?;
        state.finished = true;
        Ok(())
    }
}

impl Drop for JsonlWriter {
    fn drop(&mut self) {
        self.flush_task.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_compression::tokio::bufread::ZstdDecoder;
    use tokio::io::AsyncReadExt;

    fn directory() -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rs1090-output-{}-{}-{}",
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        path
    }

    async fn decode(bytes: &[u8]) -> (Vec<u8>, io::Result<usize>) {
        let mut decoder = ZstdDecoder::new(bytes);
        decoder.multiple_members(true);
        let mut decoded = Vec::new();
        let result = decoder.read_to_end(&mut decoded).await;
        (decoded, result)
    }

    #[tokio::test]
    async fn follow_reads_idle_writes_and_later_sessions() {
        use tokio::io::AsyncBufReadExt;
        let directory = directory();
        for name in ["live.jsonl", "live.jsonl.zst"] {
            let path = directory.join(name);
            let mut writer = JsonlWriter::open(&path, false).await.unwrap();
            let (sink, source) = tokio::io::duplex(4096);
            let input = path.clone();
            let follower =
                tokio::spawn(async move { follow_file(input, sink).await });
            let mut reader = BufReader::new(source);
            for session in 0..2 {
                let expected = format!("{{\"session\":{session}}}");
                writer.write_line(&expected).await.unwrap();
                let mut line = String::new();
                tokio::time::timeout(
                    Duration::from_secs(5),
                    reader.read_line(&mut line),
                )
                .await
                .unwrap()
                .unwrap();
                assert_eq!(line, format!("{expected}\n"));
                writer.finish().await.unwrap();
                if session == 0 {
                    writer = JsonlWriter::open(&path, true).await.unwrap();
                }
            }
            follower.abort();
            let _ = follower.await;
        }
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn periodic_flush_errors_are_not_silently_lost() {
        struct FailedFlush;
        impl AsyncWrite for FailedFlush {
            fn poll_write(
                self: Pin<&mut Self>,
                _: &mut Context<'_>,
                bytes: &[u8],
            ) -> Poll<io::Result<usize>> {
                Poll::Ready(Ok(bytes.len()))
            }
            fn poll_flush(
                self: Pin<&mut Self>,
                _: &mut Context<'_>,
            ) -> Poll<io::Result<()>> {
                Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "test flush failure",
                )))
            }
            fn poll_shutdown(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
            ) -> Poll<io::Result<()>> {
                self.poll_flush(cx)
            }
        }
        let directory = directory();
        let mut writer =
            JsonlWriter::open(directory.join("error.jsonl"), false)
                .await
                .unwrap();
        writer.state.lock().await.writer = Box::pin(FailedFlush);
        tokio::time::timeout(Duration::from_secs(5), async {
            while writer.state.lock().await.flush_error.is_none() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            writer.write_line("{}").await.unwrap_err().kind(),
            io::ErrorKind::BrokenPipe
        );
        assert_eq!(
            writer.finish().await.unwrap_err().kind(),
            io::ErrorKind::BrokenPipe
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn suffix_selection_and_frame_finalization() {
        let directory = directory();
        for (name, compressed) in [
            ("capture.jsonl", false),
            ("capture.zst", false),
            ("capture.jsonl.zst", true),
        ] {
            let path = directory.join(name);
            let mut writer = JsonlWriter::open(&path, false).await.unwrap();
            writer.write_line("{\"callsign\":\"ÉCHO\"}").await.unwrap();
            writer.finish().await.unwrap();
            writer.finish().await.unwrap();
            assert!(writer.write_line("{}").await.is_err());
            let bytes = tokio::fs::read(path).await.unwrap();
            let expected = "{\"callsign\":\"ÉCHO\"}\n".as_bytes();
            if compressed {
                let (decoded, result) = decode(&bytes).await;
                result.unwrap();
                assert_eq!(decoded, expected);
            } else {
                assert_eq!(bytes, expected);
            }
        }
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn appended_sessions_are_concatenated_frames() {
        let directory = directory();
        let path = directory.join("capture.jsonl.zst");
        for (index, append) in [false, true].into_iter().enumerate() {
            let mut writer = JsonlWriter::open(&path, append).await.unwrap();
            writer
                .write_line(&format!("{{\"session\":{index}}}"))
                .await
                .unwrap();
            writer.finish().await.unwrap();
        }
        let bytes = tokio::fs::read(&path).await.unwrap();
        let (decoded, result) = decode(&bytes).await;
        result.unwrap();
        assert_eq!(decoded, b"{\"session\":0}\n{\"session\":1}\n");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn idle_compressor_flush_exposes_data_before_finish() {
        let directory = directory();
        let path = directory.join("capture.jsonl.zst");
        let mut writer = JsonlWriter::open(&path, false).await.unwrap();
        writer.write_line("{\"live\":true}").await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let bytes = tokio::fs::read(&path).await.unwrap();
                let (decoded, _) = decode(&bytes).await;
                if decoded == b"{\"live\":true}\n" {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("Idle writer must flush before finalization");
        writer.write_line("{\"live\":false}").await.unwrap();
        writer.finish().await.unwrap();
        let (decoded, result) =
            decode(&tokio::fs::read(&path).await.unwrap()).await;
        result.unwrap();
        assert_eq!(decoded, b"{\"live\":true}\n{\"live\":false}\n");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
