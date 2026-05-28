/// Read a query-string parameter from the current URL (e.g. `?service=foo`).
pub fn read_url_query(key: &str) -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    params.get(key).filter(|v| !v.is_empty())
}

/// Set/remove a query-string parameter on the current URL without adding a new
/// history entry (uses `history.replaceState`). Empty `value` removes the key.
/// The path and other query params are preserved.
pub fn set_url_query(key: &str, value: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let location = window.location();
    let search = location.search().unwrap_or_default();
    let Ok(params) = web_sys::UrlSearchParams::new_with_str(&search) else {
        return;
    };
    if value.is_empty() {
        params.delete(key);
    } else {
        params.set(key, value);
    }

    let query = params.to_string().as_string().unwrap_or_default();
    let pathname = location.pathname().unwrap_or_else(|_| "/".to_string());
    let new_url = if query.is_empty() {
        pathname
    } else {
        format!("{pathname}?{query}")
    };

    if let Ok(history) = window.history() {
        let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&new_url));
    }
}

pub fn format_mem(mem: i64) -> String {
    let mem = mem as f64;
    if mem < 1024.0 {
        return format!("{}B", mem);
    }

    let mem = mem / 1024.0;

    if mem < 1024.0 {
        return format!("{:.2}KB", mem);
    }

    let mem = mem / 1024.0;

    if mem < 1024.0 {
        return format!("{:.2}MB", mem);
    }

    let mem = mem / 1024.0;

    return format!("{:.2}GB", mem);
}
