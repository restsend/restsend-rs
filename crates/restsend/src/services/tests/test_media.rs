use crate::{services::response::Upload, utils::check_until};
use std::{
    convert::Infallible,
    io::Write,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use futures_util::stream::once;
use http_body_util::BodyExt;
use hyper::body::Bytes;
use multer::Multipart;
use tempfile::NamedTempFile;
use tokio::sync::oneshot;

struct TestDownloadCallback {
    is_done: Arc<AtomicBool>,
}
impl crate::callback::DownloadCallback for TestDownloadCallback {
    fn on_progress(&self, sent: u64, total: u64) {
        println!("on_progress: {}/{}", sent, total);
    }
    fn on_success(&self, url: String, file_name: String) {
        println!("on_success: {} {}", url, file_name);
        self.is_done.store(true, Ordering::Relaxed);
    }
    fn on_fail(&self, err: anyhow::Error) {
        println!("on_fail: {}", err.to_string());
    }
}
struct TestUploadCallback {
    is_done: Arc<AtomicBool>,
}

impl crate::callback::UploadCallback for TestUploadCallback {
    fn on_progress(&self, sent: u64, total: u64) {
        println!("on_progress: {}/{}", sent, total);
    }
    fn on_success(&self, result: Upload) {
        println!("on_success: {} ", result.path);
        self.is_done.store(true, Ordering::Relaxed);
    }
    fn on_fail(&self, err: anyhow::Error) {
        println!("on_fail: {}", err.to_string());
    }
}

#[tokio::test]
async fn test_download_file() {
    let addr = super::open_port();
    let url = format!("http://{}/hello.txt", addr);

    super::serve_test_server(&addr, |_| async {
        let body = "hello world";
        let mut resp = hyper::Response::new(http_body_util::Full::new(Bytes::from(body)));
        resp.headers_mut()
            .insert(hyper::header::CONTENT_TYPE, "text/plain".parse().unwrap());
        Ok(resp)
    })
    .await
    .unwrap();

    let file_name = "/tmp/hello.download.txt";
    let is_done = Arc::new(AtomicBool::new(false));

    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();

    let r = crate::services::media::download_file(
        url.to_string(),
        None,
        file_name.to_string(),
        Box::new(TestDownloadCallback {
            is_done: is_done.clone(),
        }),
        cancel_rx,
    )
    .await;
    assert!(r.is_ok());

    check_until(Duration::from_secs(3), || is_done.load(Ordering::Relaxed))
        .await
        .unwrap();

    let data = std::fs::read(file_name).unwrap();
    assert_eq!(String::from_utf8(data).unwrap(), "hello world");
    _ = cancel_tx;
}

#[tokio::test]
async fn test_upload_file() {
    let addr = super::open_port();
    let url = format!("http://{}/upload", addr);

    super::serve_test_server(&addr, |req| async move {
        let save_file_name = "/tmp/upload.txt";

        let ctype = req
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let boundary = ctype.split("boundary=").collect::<Vec<&str>>()[1];
        let body = req.collect().await?.to_bytes();
        let stream = once(async move { Result::<Bytes, Infallible>::Ok(Bytes::from(body)) });
        let mut multipart = Multipart::new(stream, boundary);
        let mut total = 0;
        let mut ext = String::new();

        while let Some(mut field) = multipart.next_field().await? {
            match field.name() {
                Some("file") => {
                    ext = field.content_type().unwrap().to_string();
                    let mut data = Vec::new();
                    while let Some(chunk) = field.chunk().await? {
                        data.extend_from_slice(&chunk);
                    }

                    total = data.len() as u64;
                    std::fs::write(save_file_name, data).unwrap();
                }
                _ => {}
            }
        }

        let data = serde_json::json!({
            "fileName": save_file_name,
            "ext":ext,
            "size":total,
            "path": save_file_name,
        });

        let resp = hyper::Response::new(http_body_util::Full::new(Bytes::from(data.to_string())));
        Ok(resp)
    })
    .await
    .unwrap();

    let is_done = Arc::new(AtomicBool::new(false));

    let mut f = NamedTempFile::new().unwrap();
    let file_data = "hello world/upload";
    f.write_all(file_data.as_bytes()).unwrap();

    let file_name = f.path().to_str().unwrap().to_string();
    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();

    let r = crate::services::media::upload_file(
        url.to_string(),
        None,
        file_name.to_string(),
        false,
        Box::new(TestUploadCallback {
            is_done: is_done.clone(),
        }),
        cancel_rx,
    )
    .await;
    assert!(r.is_ok());
    let r = r.unwrap();

    check_until(Duration::from_secs(3), || is_done.load(Ordering::Relaxed))
        .await
        .unwrap();

    let data = std::fs::read(r.unwrap().path).unwrap();
    assert_eq!(String::from_utf8(data).unwrap(), file_data);
    _ = cancel_tx;
}

#[tokio::test]
async fn test_download_file_with_redirect() {
    let addr = super::open_port();
    let url = format!("http://{}/", addr);

    super::serve_test_server(&addr, |req| async move {
        let uri = req.uri().to_string();
        let resp = if uri.eq_ignore_ascii_case("/") {
            let mut resp = hyper::Response::new(http_body_util::Full::new(Bytes::new()));
            *resp.status_mut() = hyper::StatusCode::FOUND;
            resp.headers_mut()
                .insert(hyper::header::LOCATION, "/hello.txt".parse().unwrap());
            resp
        } else {
            let body = "hello world";
            let mut resp = hyper::Response::new(http_body_util::Full::new(Bytes::from(body)));
            resp.headers_mut()
                .insert(hyper::header::CONTENT_TYPE, "text/plain".parse().unwrap());
            resp
        };
        Ok(resp)
    })
    .await
    .unwrap();

    let file_name = "/tmp/hello.txt";

    let is_done = Arc::new(AtomicBool::new(false));
    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();

    let r = crate::services::media::download_file(
        url.to_string(),
        None,
        file_name.to_string(),
        Box::new(TestDownloadCallback {
            is_done: is_done.clone(),
        }),
        cancel_rx,
    )
    .await;
    assert!(r.is_ok());

    check_until(Duration::from_secs(3), || is_done.load(Ordering::Relaxed))
        .await
        .unwrap();

    let data = std::fs::read(file_name).unwrap();
    assert_eq!(String::from_utf8(data).unwrap(), "hello world");
    _ = cancel_tx;
}
