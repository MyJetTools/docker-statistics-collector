use std::collections::HashMap;

use tokio::sync::Mutex;

pub struct SshCacheCerts {
    data: Mutex<HashMap<String, String>>,
}

impl SshCacheCerts {
    pub fn new() -> Self {
        SshCacheCerts {
            data: Mutex::new(HashMap::new()),
        }
    }

    pub async fn get_cert(&self, cert_path: &str) -> String {
        let mut data = self.data.lock().await;

        if let Some(cert) = data.get(cert_path) {
            return cert.to_string();
        }

        let private_key = rust_extensions::file_utils::format_path(cert_path);

        let private_key = tokio::fs::read_to_string(private_key.as_str())
            .await
            .unwrap();

        data.insert(cert_path.to_string(), private_key.to_string());

        private_key
    }
}
