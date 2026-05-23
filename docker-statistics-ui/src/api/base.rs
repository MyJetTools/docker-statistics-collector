pub fn get_base_url() -> String {
    let settings = dioxus_utils::js::GlobalAppSettings::new();
    let origin = settings.get_origin();
    origin.trim_end_matches('/').to_string()
}

/// URL-encode a query parameter value. Stays minimal — only escapes the bytes
/// that are reserved in a query string (`&`, `=`, `+`, `#`, `%`, space and
/// any non-ASCII byte). Container ids, env names and URLs do not need
/// full RFC3986 compliance here.
pub fn url_encode(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~'
            | b':'
            | b'/'
            | b','
            | b';' => result.push(byte as char),
            _ => result.push_str(&format!("%{:02X}", byte)),
        }
    }
    result
}
