use crate::callback::{DownloadCallback, UploadCallback};
use crate::error::ClientError::{StdError, HTTP};
use crate::media::HumanReadable;
use crate::models::Attachment;
use crate::services::response::Upload;
use crate::services::{handle_response, make_post_request};
use crate::Result;
use log::info;
use reqwest::multipart;
use std::time::Duration;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::oneshot;
use wasm_bindgen_futures::JsFuture;

pub async fn upload_file(
    uploader_url: String,
    token: Option<&str>,
    attachment: Attachment,
    callback: Box<dyn UploadCallback>,
    _cancel: oneshot::Receiver<()>,
) -> Result<Option<Upload>> {
    let (_, mut progress_rx) = unbounded_channel::<(u64, u64)>();
    let file_stream = attachment.file.unwrap();
    let total = file_stream.size() as u64;

    let form = multipart::Form::new();
    //let mut last_progress_time = now_millis();
    //let mut total_sent: u64 = 0;

    let data_stream = match JsFuture::from(file_stream.array_buffer()).await {
        Ok(v) => v,
        Err(_) => {
            let reason = format!("ready blob failed");
            callback.on_fail(HTTP(reason.clone()));
            return Err(HTTP(reason));
        }
    };

    let buf = js_sys::Uint8Array::new(&data_stream).to_vec();

    let file_part = multipart::Part::stream(buf)
        .file_name(attachment.file_name.clone())
        .mime_str("application/octet-stream")?;

    let private_part = multipart::Part::text(format!("{}", attachment.is_private as u32));
    let form = form.part("file", file_part).part("private", private_part);

    info!(
        "upload url:{} file_name:{} size:{} private:{}",
        uploader_url,
        attachment.file_name,
        total.human_readable(),
        attachment.is_private,
    );

    callback.on_progress(0, total);

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
            callback.on_fail(HTTP(reason.clone()).into());
            return Err(HTTP(reason).into());
        }
    };

    let r = handle_response::<Upload>(resp).await;
    match r {
        Ok(r) => {
            callback.on_progress(total, total);
            callback.on_success(r.clone());
            Ok(Some(r))
        }
        Err(e) => {
            let reason = format!("upload failed: {}", e.to_string());
            callback.on_fail(HTTP(reason.clone()).into());
            Err(HTTP(reason).into())
        }
    }
}

pub async fn download_file(
    _download_url: String,
    _token: Option<String>,
    _save_file_name: String,
    _callback: Box<dyn DownloadCallback>,
    _cancel: oneshot::Receiver<()>,
) -> Result<String> {
    Err(StdError("not implemented".to_string()).into())
}
