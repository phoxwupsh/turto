use crate::{
    deps::{bun::get_bun_arg, ytdlp::get_ytdlp_path},
    models::config::YtdlpConfig,
    ytdl::playlist::{YouTubeDlPlaylistOutput, YouTubePlaylist},
};
use songbird::input::{AudioStream, Input, LiveInput};
use std::{
    collections::HashMap,
    future::Future,
    io::{Seek, SeekFrom},
    pin::Pin,
    process::Stdio,
    sync::Arc,
};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions, ReadOnlySource};
use tempfile::tempfile;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    process::Command,
};
use url::Url;

pub mod playlist;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct YouTubeDl {
    #[serde(flatten)]
    inner: Arc<YoutubeDlFileInner>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct YouTubeDlMetadata {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub channel: Option<String>,
    pub duration: Option<f64>,
    pub filesize: Option<u64>,
    pub http_headers: Option<HashMap<String, String>>,
    pub release_date: Option<String>,
    pub thumbnail: Option<String>,
    pub title: Option<String>,
    pub track: Option<String>,
    pub upload_date: Option<String>,
    pub uploader: Option<String>,
    pub url: String,
    pub webpage_url: Option<String>,
    pub protocol: Option<String>,
    pub timestamp: Option<i64>,
    pub uploader_url: Option<String>,
    pub channel_url: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct YoutubeDlFileInner {
    url: String,
    #[serde(skip)]
    file: tokio::sync::OnceCell<std::fs::File>,
    #[serde(
        serialize_with = "serialize_oncecell_arc",
        deserialize_with = "deserialize_oncecell_arc"
    )]
    metadata: tokio::sync::OnceCell<std::sync::Arc<YouTubeDlMetadata>>,
}

fn serialize_oncecell_arc<S, T>(
    cell: &tokio::sync::OnceCell<Arc<T>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
    T: serde::ser::Serialize,
{
    let opt = cell.get().map(Arc::as_ref);
    <Option<&T> as serde::Serialize>::serialize(&opt, serializer)
}

fn deserialize_oncecell_arc<'de, D, T>(
    deserializer: D,
) -> Result<tokio::sync::OnceCell<Arc<T>>, D::Error>
where
    D: serde::de::Deserializer<'de>,
    T: serde::de::Deserialize<'de>,
{
    let res = match <Option<T> as serde::Deserialize>::deserialize(deserializer)? {
        Some(value) => tokio::sync::OnceCell::new_with(Some(Arc::new(value))),
        None => tokio::sync::OnceCell::new(),
    };
    Ok(res)
}

impl YouTubeDl {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(YoutubeDlFileInner {
                url: url.into(),
                file: tokio::sync::OnceCell::new(),
                metadata: tokio::sync::OnceCell::new(),
            }),
        }
    }

    pub fn new_with(
        url: impl Into<String>,
        file: Option<std::fs::File>,
        metadata: YouTubeDlMetadata,
    ) -> Self {
        Self {
            inner: Arc::new(YoutubeDlFileInner {
                url: url.into(),
                file: tokio::sync::OnceCell::new_with(file),
                metadata: tokio::sync::OnceCell::new_with(Some(Arc::new(metadata))),
            }),
        }
    }

    pub fn has_yt_playlist(&self) -> bool {
        match Url::parse(&self.inner.url) {
            Ok(url) => match url.host_str() {
                Some("www.youtube.com")
                | Some("youtube.com")
                | Some("youtu.be")
                | Some("music.youtube.com") => url.query_pairs().any(|(k, _)| k == "list"),
                _ => false,
            },
            Err(_) => false,
        }
    }

    pub async fn fetch_yt_playlist(&self) -> Result<YouTubePlaylist, YouTubeDlError> {
        let args = vec![self.inner.url.as_str(), "--flat-playlist", "-J"];

        let output = Command::new(get_ytdlp_path().as_path())
            .args(args)
            .stdout(Stdio::piped())
            .output()
            .await?;

        let output = serde_json::from_slice::<YouTubeDlPlaylistOutput>(&output.stdout)?;
        let yt_playlist = YouTubePlaylist {
            id: output.id,
            title: output.title,
            author: output.channel.or(output.uploader),
            url: output.webpage_url.or(output.original_url),
            entries: output.entries,
        };
        Ok(yt_playlist)
    }

    pub fn title(&self) -> Option<&str> {
        self.inner.metadata.get()?.title.as_deref()
    }

    pub fn url(&self) -> &str {
        self.inner.url.as_str()
    }

    pub async fn fetch_file(&self, config: Arc<YtdlpConfig>) -> Result<Input, YouTubeDlError> {
        let file = self
            .inner
            .file
            .get_or_try_init(|| async {
                let mut args = vec![
                    "--js-runtimes",
                    get_bun_arg(),
                    "-q",
                    "--no-warnings",
                    self.inner.url.as_str(),
                    "-f",
                    "ba[abr>0][vcodec=none]/best",
                    "--no-playlist",
                    "-o",
                    "-",
                ];

                if let Some(cookie_path) = config.cookies_path.as_deref() {
                    args.extend(["--cookies", cookie_path]);
                }

                let mut child = tokio::process::Command::new(get_ytdlp_path().as_path())
                    .args(args)
                    .stdout(Stdio::piped())
                    .spawn()?;
                let mut stdout = child.stdout.take().unwrap();

                let tmp = tokio::fs::File::from_std(tempfile::tempfile()?);
                let mut writer = tokio::io::BufWriter::new(tmp);
                tokio::io::copy(&mut stdout, &mut writer).await?;

                let mut file = writer.into_inner().into_std().await;
                file.seek(std::io::SeekFrom::Start(0))?;

                std::io::Result::<std::fs::File>::Ok(file)
            })
            .await?;

        let mut res = file.try_clone()?;
        res.seek(SeekFrom::Start(0))?;

        let input = Input::Live(
            LiveInput::Wrapped(AudioStream {
                input: MediaSourceStream::new(Box::new(res), MediaSourceStreamOptions::default()),
                hint: None,
            }),
            None,
        );

        Ok(input)
    }

    pub async fn fetch_metadata(
        &self,
        config: Arc<YtdlpConfig>,
    ) -> Result<Arc<YouTubeDlMetadata>, YouTubeDlError> {
        let val = self
            .inner
            .metadata
            .get_or_try_init(|| async {
                let mut args = vec![
                    "--js-runtimes",
                    get_bun_arg(),
                    "-q",
                    "--no-warnings",
                    "-j",
                    self.inner.url.as_str(),
                    "-f",
                    "ba[abr>0][vcodec=none]/best",
                    "--no-playlist",
                ];

                if let Some(cookie_path) = config.cookies_path.as_deref() {
                    args.extend(["--cookies", cookie_path]);
                }

                let mut child = tokio::process::Command::new(get_ytdlp_path().as_path())
                    .args(args)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?;
                let stdout = child.stdout.take().unwrap();
                let mut line = String::new();
                let mut reader = tokio::io::BufReader::new(stdout);
                reader.read_line(&mut line).await?;

                let ytdlp_data = serde_json::from_str::<YouTubeDlMetadata>(&line)?;
                std::io::Result::Ok(Arc::new(ytdlp_data))
            })
            .await?;
        Ok(val.clone())
    }

    pub async fn play(
        &self,
        config: Arc<YtdlpConfig>,
    ) -> Result<
        (
            Pin<Box<dyn Future<Output = Result<Arc<YouTubeDlMetadata>, YouTubeDlError>> + Send>>,
            Input,
        ),
        YouTubeDlError,
    > {
        if let Some(file) = self.inner.file.get() {
            let mut file = file.try_clone()?;
            file.seek(SeekFrom::Start(0))?;

            let input = Input::Live(
                LiveInput::Wrapped(AudioStream {
                    input: MediaSourceStream::new(
                        Box::new(file),
                        MediaSourceStreamOptions::default(),
                    ),
                    hint: None,
                }),
                None,
            );
            let self_inner = self.clone();
            let meta_fut = async move { self_inner.fetch_metadata(config.clone()).await };
            return Ok((Box::pin(meta_fut), input));
        }

        let mut args = vec![
            "--js-runtimes",
            get_bun_arg(),
            "-q",
            "--no-warnings",
            "-j",
            "--no-simulate",
            self.inner.url.as_str(),
            "-f",
            "ba[abr>0][vcodec=none]/best",
            "--no-playlist",
            "-o",
            "-",
        ];

        if let Some(cookie_path) = config.cookies_path.as_deref() {
            args.extend(["--cookies", cookie_path]);
        }

        let mut child = tokio::process::Command::new(get_ytdlp_path().as_path())
            .args(args)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let stderr = child.stderr.take().unwrap();
        let mut stdout = child.stdout.take().unwrap();

        let mut line = String::new();
        let mut reader = tokio::io::BufReader::new(stderr);

        let self_inner = self.inner.clone();
        let meta_fut = async move {
            reader.read_line(&mut line).await?;
            let output = serde_json::from_str::<YouTubeDlMetadata>(&line)?;
            let meta = Arc::new(output);
            let _ = self_inner.metadata.set(meta.clone());
            Result::Ok(meta)
        };

        let self_inner = self.inner.clone();
        let (mut tx, rx) = tokio::io::duplex(32 * 1024);
        tokio::spawn(async move {
            let mut buf = vec![0u8; 32 * 1024];
            let file = tokio::fs::File::from_std(tempfile()?);
            let mut writer = tokio::io::BufWriter::new(file);
            let complete = loop {
                let n = stdout.read(&mut buf).await?;
                if n == 0 {
                    break true;
                }

                if let Err(err) = tx.write_all(&buf[..n]).await {
                    if err.kind() == std::io::ErrorKind::BrokenPipe {
                        break false;
                    } else {
                        return Err(err);
                    }
                } else {
                    writer.write_all(&buf[..n]).await?;
                }
            };
            let _ = tx.shutdown().await;
            writer.flush().await?;
            if complete {
                let mut file = writer.into_inner();
                file.seek(SeekFrom::Start(0)).await?;
                let _ = self_inner.file.set(file.into_std().await);
                tracing::warn!("write file complete");
            } else {
                tracing::warn!("write file not complete");
            }

            Ok(())
        });

        let reader = tokio_util::io::SyncIoBridge::new(rx);
        let input = Input::Live(
            LiveInput::Wrapped(AudioStream {
                input: MediaSourceStream::new(
                    Box::new(ReadOnlySource::new(reader)),
                    MediaSourceStreamOptions::default(),
                ),
                hint: None,
            }),
            None,
        );
        Ok((Box::pin(meta_fut), input))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum YouTubeDlError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
