const MEDIA_TIMEOUT_SECS: u64 = 300; // 5 minutes

#[cfg(not(target_family = "wasm"))]
mod reqwest_impl;

#[allow(dead_code)]
mod web_sys_impl;

#[cfg(not(target_family = "wasm"))]
pub use reqwest_impl::*;

#[cfg(target_family = "wasm")]
pub use web_sys_impl::*;

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
