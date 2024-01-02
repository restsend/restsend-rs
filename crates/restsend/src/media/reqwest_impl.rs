use crate::callback::{DownloadCallback, UploadCallback};
use crate::error::ClientError::{StdError, UserCancel, HTTP};
use crate::models::Attachment;
use crate::services::response::Upload;
use crate::services::{handle_response, make_get_request, make_post_request};
use crate::utils::{elapsed, now_millis};
use crate::Result;
use futures_util::TryStreamExt;
use log::info;
use reqwest::multipart;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::select;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::oneshot;

pub async fn upload_file(
    uploader_url: String,
    token: Option<&str>,
    attachment: Attachment,
    callback: Box<dyn UploadCallback>,
    cancel: oneshot::Receiver<()>,
) -> Result<Option<Upload>> {
    let file = tokio::fs::File::open(attachment.file_path.clone()).await?;
    let total = file.metadata().await?.len();
    let callback = Arc::new(Mutex::new(callback));
    let callback_ref = callback.clone();
    let upload_runner = async move {
        let form = multipart::Form::new();
        let mut last_progress_time = now_millis();
        let mut total_sent: u64 = 0;
        let file_stream = reqwest::Body::wrap_stream(
            tokio_util::io::ReaderStream::new(file).map_ok(move |buf| {
                let sent = buf.len() as u64;
                if elapsed(last_progress_time) > Duration::from_millis(300) {
                    callback_ref.lock().unwrap().on_progress(total_sent, total);
                    last_progress_time = now_millis();
                }
                total_sent += sent;
                buf
            }),
        );

        let file_part = multipart::Part::stream(file_stream)
            .file_name(attachment.file_name.clone())
            .mime_str("application/octet-stream")?;

        let private_part = multipart::Part::text(format!("{}", attachment.is_private as u32));
        let form = form.part("file", file_part).part("private", private_part);

        let req = make_post_request(
            "",
            &uploader_url,
            token,
            None,
            None,
            Some(Duration::from_secs(super::MEDIA_TIMEOUT_SECS)),
        );

        let resp = match req.multipart(form).send().await {
            Ok(resp) => resp,
            Err(e) => {
                let reason = format!("upload failed: {}", e.to_string());
                return Err(HTTP(reason));
            }
        };

        info!("upload {} response: {}", uploader_url, resp.status());
        handle_response::<Upload>(resp).await
    };

    callback.lock().unwrap().on_progress(0, total);

    select! {
        _ = cancel => {
            info!("upload runner cancel");
            callback.lock().unwrap().on_fail(UserCancel("canceled".to_string()).into());
            Err(UserCancel("canceled".to_string()).into())
        },
        r = upload_runner => {
            info!("upload runner finished");
            let cb = callback.lock().unwrap();
            match r {
                Ok(r) => {
                    cb.on_progress(total, total);
                    cb.on_success(r.clone());
                    Ok(Some(r))
                },
                Err(e) => {
                    let reason = format!("upload failed: {}", e.to_string());
                    cb.on_fail(HTTP(reason.clone()).into());
                    Err(HTTP(reason).into())
                }
            }
        }
    }
}

pub async fn download_file(
    download_url: String,
    token: Option<String>,
    save_file_name: String,
    callback: Box<dyn DownloadCallback>,
    cancel: oneshot::Receiver<()>,
) -> Result<String> {
    let (progress_tx, mut progress_rx) = unbounded_channel::<(u64, u64)>();
    let req = make_get_request(
        "",
        &download_url,
        token,
        Some(Duration::from_secs(super::MEDIA_TIMEOUT_SECS)),
    );

    let download_runner = async move {
        let mut resp = req.send().await.map_err(|e| HTTP(e.to_string()))?;
        let total = resp.content_length().unwrap_or(0);
        let file = tokio::fs::File::create(save_file_name.clone()).await;

        if file.is_err() {
            let reason = format!("create file failed: {}", file.err().unwrap().to_string());
            return Err(StdError(reason));
        }

        let mut file = file.unwrap();
        let mut buf = Vec::new();
        let mut total_recived: u64 = 0;
        let mut last_progress_time = now_millis();

        // begin download
        while let Some(chunk) = resp.chunk().await? {
            buf.extend_from_slice(&chunk);
            file.write_all(&chunk).await?;

            if elapsed(last_progress_time) > Duration::from_millis(300) {
                let recived = buf.len() as u64;
                total_recived += recived;
                progress_tx.send((total_recived, total)).ok();
                last_progress_time = now_millis();
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
                match progress_rx.recv().await {
                    Some((sent, total)) => {
                        callback.on_progress(sent, total);
                    },
                    None => {
                        break;
                    }
                }
            }
        } => {
            Err(UserCancel("canceled".to_string()).into())
        },
        r = download_runner => {
            match r {
                Ok((file_name, total)) => {
                    callback.on_progress(total, total);
                    callback.on_success(download_url, file_name.clone());
                    Ok(file_name)
                },
                Err(e) => {
                    let reason = format!("download failed: {}", e.to_string());
                    callback.on_fail(HTTP(reason.clone()).into());
                    Err(HTTP(reason).into())
                }
            }
        }
    }
}
