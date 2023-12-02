use super::response::Upload;
use crate::callback::{DownloadCallback, UploadCallback};
use crate::error::ClientError::{HTTPError, StdError, UserCancel};
use anyhow::Result;
use futures_util::TryStreamExt;
use log::info;
use reqwest::multipart;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::select;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::oneshot;
use tokio::time::Instant;

//implement human readable for u64
pub trait HumanReadable {
    fn human_readable(&self) -> String;
}

impl HumanReadable for u64 {
    fn human_readable(&self) -> String {
        let mut size = *self as f64;
        let mut unit = "B";
        if size > 1024.0 {
            size /= 1024.0;
            unit = "KB";
        }
        if size > 1024.0 {
            size /= 1024.0;
            unit = "MB";
        }
        if size > 1024.0 {
            size /= 1024.0;
            unit = "GB";
        }
        format!("{:.2}{}", size, unit)
    }
}

pub(crate) fn build_upload_url(endpoint: &str, url: &str) -> String {
    if url.starts_with("http") {
        return url.to_string();
    }

    format!("{}/api/attachment/upload", endpoint)
}

pub(crate) fn build_download_url(endpoint: &str, url: &str) -> String {
    if url.starts_with("http") {
        return url.to_string();
    }

    format!("{}{}", endpoint, url)
}

pub(crate) async fn upload_file(
    uploader_url: String,
    token: Option<&str>,
    file_path: String,
    is_private: bool,
    callback: Box<dyn UploadCallback>,
    cancel: oneshot::Receiver<()>,
) -> Result<Option<Upload>> {
    let file = tokio::fs::File::open(file_path.clone()).await?;
    let total = file.metadata().await?.len();

    let (progress_tx, mut progress_rx) = unbounded_channel::<(u64, u64)>();

    let upload_runner = async move {
        let form = multipart::Form::new();
        let mut last_progress_time = Instant::now();
        let mut total_sent: u64 = 0;

        let file_stream = reqwest::Body::wrap_stream(
            tokio_util::io::ReaderStream::new(file).map_ok(move |buf| {
                let sent = buf.len() as u64;
                if last_progress_time.elapsed() > Duration::from_millis(300) {
                    progress_tx.send((total_sent, total)).ok();
                    last_progress_time = Instant::now();
                }
                total_sent += sent;
                buf
            }),
        );

        let file_part = multipart::Part::stream(file_stream)
            .file_name(file_path.clone())
            .mime_str("application/octet-stream")?;

        let private_part = multipart::Part::text(format!("{}", is_private as u32));
        let form = form.part("file", file_part).part("private", private_part);

        info!(
            "upload url:{} filename:{} size:{} private:{}",
            uploader_url,
            file_path,
            total.human_readable(),
            is_private,
        );

        let req = super::make_post_request(
            "",
            &uploader_url,
            token,
            None,
            None,
            Some(Duration::from_secs(super::MEDIA_TIMEOUT_SECS)),
        );

        let resp = req.multipart(form).send().await?;
        info!("upload {} response: {:?}", uploader_url, resp);
        super::handle_response::<super::response::Upload>(resp).await
    };

    callback.on_progress(0, total);

    select! {
        _ = cancel => {
            callback.on_fail(UserCancel("canceled".to_string()).into());
            Err(UserCancel("canceled".to_string()).into())
        },
        _ = async {
            loop {
                if let Some((sent, total)) = progress_rx.recv().await {
                    callback.on_progress(sent, total);
                }
            }
        } => {
            Ok(None)
        },
        r = upload_runner => {
            match r {
                Ok(r) => {
                    callback.on_progress(total, total);
                    callback.on_success(r.path.clone());
                    Ok(Some(r))
                },
                Err(e) => {
                    let reason = format!("upload failed: {}", e.to_string());
                    callback.on_fail(HTTPError(reason.clone()).into());
                    Err(HTTPError(reason).into())
                }
            }
        }
    }
}

pub(crate) async fn download_file(
    download_url: String,
    token: Option<String>,
    save_file_name: String,
    callback: Box<dyn DownloadCallback>,
    cancel: oneshot::Receiver<()>,
) -> Result<()> {
    let (progress_tx, mut progress_rx) = unbounded_channel::<(u64, u64)>();
    let req = super::make_get_request(
        "",
        &download_url,
        token,
        Some(Duration::from_secs(super::MEDIA_TIMEOUT_SECS)),
    );

    let download_runner = async move {
        let mut resp = req.send().await.map_err(|e| HTTPError(e.to_string()))?;
        let total = resp.content_length().unwrap_or(0);
        let file = tokio::fs::File::create(save_file_name.clone()).await;

        if file.is_err() {
            let reason = format!("create file failed: {}", file.err().unwrap().to_string());
            return Err(StdError(reason));
        }

        let mut file = file.unwrap();
        let mut buf = Vec::new();
        let mut total_recived: u64 = 0;
        let mut last_progress_time = Instant::now();

        // begin download
        while let Some(chunk) = resp.chunk().await? {
            buf.extend_from_slice(&chunk);
            file.write_all(&chunk).await?;

            if last_progress_time.elapsed() > Duration::from_millis(300) {
                let recived = buf.len() as u64;
                total_recived += recived;
                progress_tx.send((total_recived, total)).ok();
                last_progress_time = Instant::now();
            }
        }
        file.flush().await?;
        Ok((save_file_name.clone(), total))
    };

    callback.on_progress(0, 0);

    select! {
        _ = cancel => {
            callback.on_fail(UserCancel("canceled".to_string()).into());
            Err(UserCancel("canceled".to_string()).into())
        },
        _ = async {
            loop {
                if let Some((sent, total)) = progress_rx.recv().await {
                    callback.on_progress(sent, total);
                }
            }
        } => {
            Ok(())
        },
        r = download_runner => {
            match r {
                Ok((file_name, total)) => {
                    callback.on_progress(total, total);
                    callback.on_success(download_url, file_name);
                    Ok(())
                },
                Err(e) => {
                    let reason = format!("download failed: {}", e.to_string());
                    callback.on_fail(HTTPError(reason.clone()).into());
                    Err(HTTPError(reason).into())
                }
            }
        }
    }
}
