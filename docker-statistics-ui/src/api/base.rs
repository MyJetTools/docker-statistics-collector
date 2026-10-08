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

/// Turns FlUrl's answer into a typed result: 2xx → the body as `T`; any other
/// status → `Err` carrying the response body, which is where the api puts the
/// reason for a refusal. Every FlUrl call goes through here, so no call site
/// decodes a status of its own.
pub async fn handle_http_response<T: serde::de::DeserializeOwned>(
    response: Result<flurl::FlUrlResponse, flurl::FlUrlError>,
) -> Result<T, crate::models::RequestError> {
    let mut response = response?;

    if (200..300).contains(&response.get_status_code()) {
        return Ok(response.get_json().await?);
    }

    let message = response
        .get_body_as_str()
        .await
        .map(|body| body.to_string())
        .unwrap_or_else(|err| err.to_string());

    Err(crate::models::RequestError { message })
}
