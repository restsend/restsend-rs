//use std::io::Write;
use futures_util::TryStreamExt;
use tokio::io::AsyncWriteExt;

use crate::error::ClientError;
use http::StatusCode;
use log::{info, warn};
use reqwest::multipart;
use tokio::select;

use super::CtrlMessageType;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadResp {
    pub path: String,
    pub file_name: String,
    pub ext: String,
    pub size: i64,
}

//implement human readable for u32
pub trait HumanReadable {
    fn human_readable(&self) -> String;
}

impl HumanReadable for u32 {
    fn human_readable(&self) -> String {
        let mut size = *self as f32;
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

impl super::Client {
    pub fn upload(
        &self,
        uploader_url: Option<String>,
        local_file_path: String,
        key: String,
        is_private: bool,
    ) -> crate::Result<()> {
        let uploader_url = match uploader_url {
            Some(url) => url,
            None => format!("{}/api/attachment/upload", self.net_store.endpoint()?),
        };

        if let Some(uploader) = self.external_uploader.lock().unwrap().as_ref() {
            Ok(uploader.upload(local_file_path, key))
        } else {
            self.ctrl_tx
                .send(CtrlMessageType::MediaUpload(
                    uploader_url,
                    local_file_path,
                    key,
                    is_private,
                ))
                .map_err(|e| crate::error::ClientError::SendCtrlMessageError(e.to_string()))
        }
    }

    pub fn download(
        &self,
        file_url: String,
        save_to_local: String,
        key: String,
    ) -> crate::Result<()> {
        if let Some(uploader) = self.external_uploader.lock().unwrap().as_ref() {
            Ok(uploader.download(file_url, save_to_local, key))
        } else {
            self.ctrl_tx
                .send(CtrlMessageType::MediaDownload(file_url, save_to_local, key))
                .map_err(|e| crate::error::ClientError::SendCtrlMessageError(e.to_string()))
        }
    }

    pub fn cancel_download(&self, file_url: String, key: String) -> crate::Result<()> {
        if let Some(uploader) = self.external_uploader.lock().unwrap().as_ref() {
            Ok(uploader.cancel_download(file_url, key))
        } else {
            self.ctrl_tx
                .send(CtrlMessageType::MediaCancelDownload(file_url, key))
                .map_err(|e| crate::error::ClientError::SendCtrlMessageError(e.to_string()))
        }
    }
    pub fn cancel_upload(&self, local_file_path: String, key: String) -> crate::Result<()> {
        if let Some(uploader) = self.external_uploader.lock().unwrap().as_ref() {
            Ok(uploader.cancel_upload(local_file_path, key))
        } else {
            self.ctrl_tx
                .send(CtrlMessageType::MediaCancelUpload(local_file_path, key))
                .map_err(|e| crate::error::ClientError::SendCtrlMessageError(e.to_string()))
        }
    }
}

impl super::Client {
    pub fn handle_media_upload(
        &self,
        url: String,
        file_name: String,
        key: String,
        is_private: bool,
    ) -> crate::Result<()> {
        let (media_tx, mut media_rx) = tokio::sync::mpsc::unbounded_channel::<bool>();

        let tx = self.ctrl_tx.clone();
        self.pending_medias
            .lock()
            .unwrap()
            .insert(key.clone(), media_tx);

        let req = self
            .net_store
            .make_post_request(&url, None, super::MEDIA_TIMEOUT_SECS)?;

        let cancel_url = url.clone();
        let cancel_file_name = file_name.clone();
        let cancel_key = key.clone();

        let upload_runner = async move {
            let file = tokio::fs::File::open(file_name.clone()).await?;
            let total = file.metadata().await?.len() as u32;

            let form = multipart::Form::new();

            let total_in_stream = total;
            let key_in_steam = key.clone();
            let tx_in_stream = tx.clone();
            let url_in_stream = url.clone();
            let mut last_progress_time = tokio::time::Instant::now();

            let file_stream = reqwest::Body::wrap_stream(
                tokio_util::io::ReaderStream::new(file).map_ok(move |buf| {
                    let recived = buf.len() as u32;
                    if last_progress_time.elapsed() > tokio::time::Duration::from_millis(300) {
                        tx_in_stream
                            .send(CtrlMessageType::OnMediaUploadProgress(
                                url_in_stream.clone(),
                                recived,
                                total_in_stream,
                                key_in_steam.clone(),
                            ))
                            .ok();
                        last_progress_time = tokio::time::Instant::now();
                    }
                    buf
                }),
            );

            let file_part = multipart::Part::stream(file_stream)
                .file_name(file_name.clone())
                .mime_str("application/octet-stream")?;

            let private_part = multipart::Part::text(format!("{}", is_private as u32));
            let form = form.part("file", file_part).part("private", private_part);

            info!(
                "handle_media_upload filename:{} key:{} size:{}",
                file_name,
                key,
                total.human_readable()
            );

            tx.send(CtrlMessageType::OnMediaUploadProgress(
                url.clone(),
                0,
                total,
                key.clone(),
            ))?;

            let resp = req.multipart(form).send().await?;

            tx.send(CtrlMessageType::OnMediaUploadProgress(
                url.clone(),
                total,
                total,
                key.clone(),
            ))?;

            match resp.status() {
                StatusCode::OK => {
                    let upload_rsp: UploadResp = resp.json().await?;

                    tx.send(CtrlMessageType::OnMediaUploadDone(
                        upload_rsp.path,
                        upload_rsp.file_name,
                        total,
                        key,
                    ))?;
                }
                _ => {
                    tx.send(CtrlMessageType::OnMediaUploadCancel(
                        url.clone(),
                        file_name,
                        format!("upload bad http status: {} url:{}", resp.status(), url),
                        key,
                    ))?;
                    warn!("bad http status: {} url:{}", resp.status(), url);
                    return Err(ClientError::HTTPError(format!(
                        "bad http status: {} url:{}",
                        resp.status(),
                        url
                    )));
                }
            }
            Ok(())
        };

        let tx = self.ctrl_tx.clone();
        self.runtime.spawn(async move {
            select! {
                _ = media_rx.recv() => {
                    tx.send(CtrlMessageType::OnMediaUploadCancel(
                            cancel_url,
                            cancel_file_name,
                            "canceled".to_string(),
                            cancel_key,
                        )).ok();
                },
                r = upload_runner => {
                    match r {
                        Ok(_) => {},
                        Err(e) => {
                            tx.send(CtrlMessageType::OnMediaUploadCancel(
                                cancel_url,
                                cancel_file_name,
                                e.to_string(),
                                cancel_key,
                            )).ok();
                        }
                    }
                }
            }
        });
        Ok(())
    }

    pub fn handle_media_download(
        &self,
        url: String,
        save_to: String,
        key: String,
    ) -> crate::Result<()> {
        info!(
            "handle_media_download url:{} save_to:{} key:{}",
            url, save_to, key
        );
        let req = self.net_store.make_request(
            http::Method::GET,
            &url,
            None,
            "".to_string(),
            super::MEDIA_TIMEOUT_SECS,
        )?;

        let tx = self.ctrl_tx.clone();
        let (media_tx, mut media_rx) = tokio::sync::mpsc::unbounded_channel::<bool>();

        self.pending_medias
            .lock()
            .unwrap()
            .insert(key.clone(), media_tx);

        let cancel_url = url.clone();
        let cancel_save_to = save_to.clone();
        let cancel_key = key.clone();

        let download_runner = async move {
            let mut resp = req.send().await?;
            match resp.status() {
                StatusCode::OK => {}
                _ => {
                    tx.send(CtrlMessageType::OnMediaDownloadCancel(
                        url.clone(),
                        save_to.clone(),
                        format!("bad http status: {}", resp.status()),
                        key,
                    ))?;
                    warn!("bad http status: {} url:{}", resp.status(), url);
                    return Err(ClientError::HTTPError(format!(
                        "bad http status: {}",
                        resp.status()
                    )));
                }
            }

            let temp_file_name = format!("{}-{}.tmp", save_to, crate::utils::random_text(4));
            let mut file = tokio::fs::File::create(temp_file_name.clone()).await?;
            let mut buf = Vec::new();
            let total = resp.content_length().unwrap_or(0) as u32;
            let mut last_progress_time = tokio::time::Instant::now();

            // begin download
            tx.send(CtrlMessageType::OnMediaDownloadProgress(
                url.clone(),
                0,
                total,
                key.clone(),
            ))?;

            while let Some(chunk) = resp.chunk().await? {
                buf.extend_from_slice(&chunk);
                file.write_all(&chunk).await?;

                if last_progress_time.elapsed() > tokio::time::Duration::from_millis(300) {
                    let recived = buf.len() as u32;
                    tx.send(CtrlMessageType::OnMediaDownloadProgress(
                        url.clone(),
                        recived,
                        total,
                        key.clone(),
                    ))?;
                    last_progress_time = tokio::time::Instant::now();
                }
            }
            // end download
            tx.send(CtrlMessageType::OnMediaDownloadProgress(
                url.clone(),
                total,
                total,
                key.clone(),
            ))?;
            std::fs::rename(temp_file_name, save_to.clone())?;
            tx.send(CtrlMessageType::OnMediaDownloadDone(
                url.clone(),
                save_to,
                total,
                key.clone(),
            ))?;
            Ok(())
        };

        let tx = self.ctrl_tx.clone();
        self.runtime.spawn(async move {
            select! {
                _ = media_rx.recv() => {
                    tx.send(CtrlMessageType::OnMediaDownloadCancel(
                            cancel_url,
                            cancel_save_to,
                            "canceled".to_string(),
                            cancel_key,
                        )).ok();
                },
                r = download_runner => {
                    match r {
                        Ok(_) => {},
                        Err(e) => {
                            tx.send(CtrlMessageType::OnMediaDownloadCancel(
                                cancel_url,
                                cancel_save_to,
                                e.to_string(),
                                cancel_key,
                            )).ok();
                        }
                    }
                }
            }
        });
        Ok(())
    }

    pub fn handle_media_cancel_upload(&self, file_name: String, key: String) -> crate::Result<()> {
        info!(
            "handle_media_cancel_upload key: {} file_name: {}",
            key, file_name
        );
        let binding = self.pending_medias.lock().unwrap();
        let cancel_tx = binding.get(key.as_str());
        if let Some(tx) = cancel_tx {
            tx.send(true)?;
        }
        Ok(())
    }

    pub fn handle_media_cancel_download(&self, url: String, key: String) -> crate::Result<()> {
        info!("handle_media_cancel_download key: {} url: {}", key, url);
        let binding = self.pending_medias.lock().unwrap();
        let cancel_tx = binding.get(key.as_str());
        if let Some(tx) = cancel_tx {
            tx.send(true)?;
        }
        Ok(())
    }
}
