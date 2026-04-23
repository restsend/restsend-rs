pub fn parse_bearer_token(header: &str) -> Option<&str> {
    header
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|v| !v.is_empty())
}
