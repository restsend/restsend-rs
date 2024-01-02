use crate::callback::{DownloadCallback, UploadCallback};
use crate::error::ClientError;
use crate::models::Attachment;
use crate::services::response::Upload;
use crate::utils::{elapsed, now_millis};
use crate::Result;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
pub async fn upload_file(
    uploader_url: String,
    token: Option<&str>,
    attachment: Attachment,
    callback: Box<dyn UploadCallback>,
    _cancel: oneshot::Receiver<()>,
) -> Result<Option<Upload>> {
    let callback = Arc::new(Mutex::new(callback));
    let callback_ref = callback.clone();

    match upload_file_with_xmlhttprequest(
        uploader_url,
        token,
        attachment,
        callback_ref.clone(),
        _cancel,
    )
    .await
    {
        Ok(r) => {
            callback_ref.lock().unwrap().on_success(r.clone());
            Ok(Some(r))
        }
        Err(e) => {
            callback_ref.lock().unwrap().on_fail(e.clone());
            Err(e)
        }
    }
}

pub async fn upload_file_with_xmlhttprequest(
    uploader_url: String,
    token: Option<&str>,
    attachment: Attachment,
    callback: Arc<Mutex<Box<dyn UploadCallback>>>,
    _cancel: oneshot::Receiver<()>,
) -> Result<Upload> {
    let mut last_progress_time = now_millis();
    let mut total_sent: u64 = 0;
    let mut total: u64 = 0;
    let (on_completed_tx, on_completed_rx) = oneshot::channel();
    let on_completed_tx = Arc::new(Mutex::new(Some(on_completed_tx)));

    let xhr = web_sys::XmlHttpRequest::new().map_err(|e| ClientError::from(e))?;
    let upload_event = xhr
        .upload()
        .map_err(|_| ClientError::StdError("upload is none".to_string()))?;

    let form_data = web_sys::FormData::new()?;
    form_data.append_with_str("private", &format!("{}", attachment.is_private))?;
    form_data.append_with_str("file_name", &attachment.file_name)?;
    match attachment.file {
        Some(file) => {
            total = file.size() as u64;
            form_data.set_with_blob_and_filename("file", &file, &attachment.file_name)?;
        }
        None => {}
    }

    let callback_ref = callback.clone();
    let onprogress_callback = Closure::wrap(Box::new(move |event: web_sys::ProgressEvent| {
        let sent = event.loaded() as u64;
        if elapsed(last_progress_time) > Duration::from_millis(300) {
            callback_ref.lock().unwrap().on_progress(total_sent, total);
            last_progress_time = now_millis();
        }
        total_sent += sent;
    }) as Box<dyn FnMut(_)>);

    upload_event.set_onprogress(Some(onprogress_callback.as_ref().unchecked_ref()));
    onprogress_callback.forget();

    let on_completed_tx_ref = on_completed_tx.clone();

    let onerror_callback = Closure::wrap(Box::new(move |event: web_sys::Event| {
        if let Some(tx) = on_completed_tx_ref.lock().unwrap().take() {
            let e = match event.dyn_into::<web_sys::ErrorEvent>() {
                Ok(e) => ClientError::HTTP(e.message()),
                Err(e) => ClientError::HTTP(e.to_string().as_string().unwrap_or_default()),
            };
            tx.send(Err(e)).ok();
        }
    }) as Box<dyn FnMut(_)>);
    upload_event.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    let callback_ref = callback.clone();
    let on_completed_tx_ref = on_completed_tx.clone();
    let onloadend_callback = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let xhr = match event.target() {
            None => {
                return;
            }
            Some(target) => match target.dyn_into::<web_sys::XmlHttpRequest>() {
                Ok(xhr) => xhr,
                Err(_) => {
                    return;
                }
            },
        };

        callback_ref.lock().unwrap().on_progress(total, total);
        let json = xhr.response_text().ok().unwrap_or_default().unwrap();

        if let Some(tx) = on_completed_tx_ref.lock().unwrap().take() {
            let r =
                serde_json::from_str::<Upload>(&json).map_err(|e| ClientError::HTTP(e.to_string()));
            tx.send(r).ok();
        }
    }) as Box<dyn FnMut(_)>);

    xhr.set_onloadend(Some(onloadend_callback.as_ref().unchecked_ref()));
    onloadend_callback.forget();

    let on_completed_tx_ref = on_completed_tx.clone();
    let ontimeout_callback = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        if let Some(tx) = on_completed_tx_ref.lock().unwrap().take() {
            tx.send(Err(ClientError::HTTP("timeout".to_string()))).ok();
        }
    }) as Box<dyn FnMut(_)>);
    xhr.set_ontimeout(Some(ontimeout_callback.as_ref().unchecked_ref()));
    ontimeout_callback.forget();

    xhr.set_timeout((super::MEDIA_TIMEOUT_SECS * 1000) as u32);
    xhr.open_with_async("POST", &uploader_url, true)?;

    match token {
        Some(token) => xhr.set_request_header("Authorization", &format!("Bearer {token}"))?,
        None => {}
    }
    callback.lock().unwrap().on_progress(0, total);

    xhr.send_with_opt_form_data(Some(form_data.as_ref()))?;

    on_completed_rx
        .await
        .map_err(|e| ClientError::StdError(e.to_string()))?
}

pub async fn download_file(
    _download_url: String,
    _token: Option<String>,
    _save_file_name: String,
    _callback: Box<dyn DownloadCallback>,
    _cancel: oneshot::Receiver<()>,
) -> Result<String> {
    Err(ClientError::StdError("not implemented".to_string()).into())
}
