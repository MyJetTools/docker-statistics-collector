use my_http_server::{HttpContext, HttpRequestHeaders};

/// Header injected by the upstream reverse proxy / ingress to identify the
/// authenticated user. We never validate it ourselves — the proxy is the
/// authority. An empty string is treated as "no user".
pub const SSL_USER_HEADER: &str = "x-ssl-user";

/// Extract `x-ssl-user` from an HTTP request handled by an action.
pub fn user_from_http(ctx: &HttpContext) -> String {
    ctx.request
        .get_headers()
        .try_get_case_insensitive_as_str(SSL_USER_HEADER)
        .ok()
        .flatten()
        .map(|s| s.to_string())
        .unwrap_or_default()
}
