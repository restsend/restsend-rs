mod auth;
mod response;
#[cfg(test)]
mod tests;

const MEDIA_TIMEOUT_SECS: u64 = 300; // 5 minutes
const API_TIMEOUT_SECS: u64 = 60; // 1 minute
