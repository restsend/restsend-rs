use crate::{
    client::Client,
    tests::{check_until, http_server},
};
use futures_util::stream::StreamExt;
use log::info;
use std::{
    io::Write,
    sync::{Arc, Mutex},
};
use tempfile::NamedTempFile;
use tokio::time::Duration;

impl crate::Callback for TestMediaCallBack {
    fn on_download_done(&self, url: String, file_name: String, size: u32, key: String) {
        info!("on_download_done {} {} {} {}", url, file_name, size, key);
        *self.is_done.lock().unwrap() = true;
    }

    fn on_download_progress(&self, url: String, received: u32, total: u32, key: String) {
        info!(
            "on_download_progress: {} {} {} {}",
            url, received, total, key
        );
    }
    fn on_download_cancel(&self, _url: String, _file_name: String, _reason: String, _key: String) {
        todo!()
    }

    fn on_upload_progress(&self, _file_name: String, _received: u32, _total: u32, _key: String) {
        info!(
            "on_upload_progress {} {} {} {}",
            _file_name, _received, _total, _key
        );
    }

    fn on_upload_done(&self, url: String, file_name: String, size: u32, key: String) {
        info!("on_upload_done {} {} {} {}", url, file_name, size, key);
        *self.is_done.lock().unwrap() = true;
        self.public_url.lock().unwrap().push_str(&url);
    }

    fn on_upload_cancel(&self, url: String, file_name: String, reason: String, key: String) {
        info!("on_upload_cancel {} {} {} {}", url, file_name, reason, key);
        *self.is_fail.lock().unwrap() = true;
        *self.is_done.lock().unwrap() = true;
    }
}

#[derive(Default)]
struct TestMediaCallBack {
    is_done: Arc<Mutex<bool>>,
    is_fail: Arc<Mutex<bool>>,
    public_url: Arc<Mutex<String>>,
}
#[test]
fn test_download() {
    let c = std::sync::Arc::new(Client::new(
        crate::models::MEMORY_DSN.to_string(),
        "".to_string(),
    ));

    let is_done = Arc::new(Mutex::new(false));
    let is_fail = Arc::new(Mutex::new(false));
    let cb = TestMediaCallBack {
        is_done: is_done.clone(),
        is_fail: is_fail.clone(),
        public_url: Arc::new(Mutex::new(String::new())),
    };

    c.set_callback(Some(Box::new(cb)));
    let file_data = format!("hello world test {}", crate::utils::random_text(12));
    let body = file_data.clone();
    let server = http_server::serve(move |req| {
        let body = body.clone();
        async move {
            if req.uri() == "/" {
                http::Response::builder()
                    .status(http::StatusCode::FOUND)
                    .header("location", "/dst")
                    .header("server", "test-redirect")
                    .body(Default::default())
                    .unwrap()
            } else {
                assert_eq!(req.uri(), "/dst");
                http::Response::builder()
                    .header("server", "test-dst")
                    .body(body.into())
                    .unwrap()
            }
        }
    });

    let run_c = c.clone();
    std::thread::spawn(move || {
        run_c.run_loop().unwrap();
    });
    let fname = format!("/tmp/test_download_{}.png", crate::utils::random_text(4));
    let r = c.download(
        format!("http://{}/", server.addr().to_string()),
        fname.clone(),
        "1".to_string(),
    );
    assert!(r.is_ok());

    check_until(Duration::from_secs(10), || *is_done.lock().unwrap())
        .expect("download too long time");
    let data = std::fs::read(fname).unwrap();
    assert!(data.len() > 0);
    assert_eq!(String::from_utf8(data).unwrap(), file_data);
}

#[test]
fn test_upload_with_local_server() {
    //
    // create a test upload http server
    let file_data = format!("hello world test {}", crate::utils::random_text(12));
    let expected_body = file_data.clone();

    let server = http_server::serve(move |mut req| {
        let expected_body = expected_body.clone();
        async move {
            let ct = format!("multipart/form-data; boundary=");

            assert_eq!(req.method(), "POST");
            let content_type = req.headers()["content-type"].to_str().unwrap();
            assert!(content_type.starts_with(&ct));

            let mut full: Vec<u8> = Vec::new();
            while let Some(item) = req.body_mut().next().await {
                full.extend(&*item.unwrap());
            }
            let data = String::from_utf8(full.clone()).unwrap();
            assert!(data.contains("Content-Disposition: form-data; name=\"file\""));
            assert_ne!(full.len(), 0);
            assert!(data.contains(expected_body.as_str()));

            http::Response::default()
        }
    });

    let c = std::sync::Arc::new(Client::new(
        crate::models::MEMORY_DSN.to_string(),
        "".to_string(),
    ));

    let is_done = Arc::new(Mutex::new(false));
    let is_fail = Arc::new(Mutex::new(false));
    let cb = TestMediaCallBack {
        is_done: is_done.clone(),
        is_fail: is_fail.clone(),
        ..Default::default()
    };

    c.set_callback(Some(Box::new(cb)));

    let run_c = c.clone();
    std::thread::spawn(move || {
        run_c.run_loop().unwrap();
    });

    let mut f = NamedTempFile::new().unwrap();
    f.write_all(file_data.as_bytes()).unwrap();
    let fname = f.path().to_str().unwrap().to_string();

    let r = c.upload(
        Some(format!("http://{}/upload", server.addr().to_string())),
        fname.clone(),
        "1".to_string(),
        false,
    );
    assert!(r.is_ok());

    check_until(Duration::from_secs(10), || *is_done.lock().unwrap())
        .expect("upload too long time");
}

#[test]
fn test_upload() {
    let file_data = format!("hello world test {}", crate::utils::random_text(12));
    let c = Arc::new(super::login_with("guido", "guido:demo"));
    let token = c.net_store.auth_token();

    let is_done = Arc::new(Mutex::new(false));
    let is_fail = Arc::new(Mutex::new(false));
    let public_url = Arc::new(Mutex::new(String::new()));
    let cb = TestMediaCallBack {
        is_done: is_done.clone(),
        is_fail: is_fail.clone(),
        public_url: public_url.clone(),
    };

    c.set_callback(Some(Box::new(cb)));

    let run_c = c.clone();
    std::thread::spawn(move || {
        run_c.run_loop().unwrap();
    });

    let mut f = NamedTempFile::new().unwrap();
    f.write_all(file_data.as_bytes()).unwrap();
    let fname = f.path().to_str().unwrap().to_string();

    let r = c.upload(None, fname.clone(), "unittest".to_string(), false);
    assert!(r.is_ok());

    check_until(Duration::from_secs(10), || *is_done.lock().unwrap())
        .expect("upload too long time");
    assert_eq!(*is_fail.lock().unwrap(), false);
    assert!(public_url.lock().unwrap().len() > 0);
    let data = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move {
            let r = reqwest::ClientBuilder::new()
                .build()
                .unwrap()
                .get(public_url.lock().unwrap().as_str())
                .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", token))
                .send()
                .await
                .unwrap();
            assert_eq!(r.status(), reqwest::StatusCode::OK);
            r.text().await.unwrap()
        });
    assert_eq!(data, file_data);
}
