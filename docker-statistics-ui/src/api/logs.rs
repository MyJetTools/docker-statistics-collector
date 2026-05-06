use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LogLineHttpModel {
    pub tp: u8,
    pub line: String,
}

#[get("/api/logs?env&url&id&lines_amount")]
pub async fn get_logs(
    env: String,
    url: String,
    id: String,
    lines_amount: u32,
) -> Result<Vec<LogLineHttpModel>, ServerFnError> {
    let fl_url = crate::server::APP_CTX
        .get_fl_url(env.as_str(), url.as_str())
        .await;
    let result = crate::server::http_client::get_logs(fl_url, id, lines_amount).await;
    let payload = match result {
        Ok(result) => result,
        Err(err) => return Err(ServerFnError::new(err)),
    };

    if payload.len() == 0 {
        return Ok(vec![]);
    }
    let mut result = Vec::new();

    println!("Payload.len = {}", payload.len());

    let mut payload = payload.into_iter();
    loop {
        let tp = payload.next();

        if tp.is_none() {
            break;
        }

        let tp = tp.unwrap();

        let n = payload.next().unwrap_or(255);
        if n != 0 {
            break;
        }
        let n = payload.next().unwrap_or(255);
        if n != 0 {
            break;
        }

        payload.next().unwrap_or(255);
        if n != 0 {
            break;
        }

        let mut size = [0u8; 4];

        size[0] = payload.next().unwrap();
        size[1] = payload.next().unwrap();
        size[2] = payload.next().unwrap();
        size[3] = payload.next().unwrap();

        let size = u32::from_be_bytes(size) as usize;

        let mut str = Vec::with_capacity(size);

        for _ in 0..size - 1 {
            str.push(payload.next().unwrap());
        }

        payload.next().unwrap();

        let item = LogLineHttpModel {
            tp,
            line: String::from_utf8(str).unwrap(),
        };

        result.push(item);
    }

    Ok(result)
}
