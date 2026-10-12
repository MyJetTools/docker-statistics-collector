mod get_statistics;
pub use get_statistics::*;
mod get_logs;
pub use get_logs::*;
mod get_processes;
pub use get_processes::*;
mod exec_permission;
pub use exec_permission::*;

use flurl::{FlUrlError, FlUrlResponse};

/// The most of a collector's answer this service takes into memory. Far above any real
/// answer - a containers payload of a big env is a few megabytes - and well under the
/// memory the container of this service is given.
const MAX_RESPONSE_BODY_LEN: usize = 64 * 1024 * 1024;

/// The whole body of a collector's answer. It is taken out of the response once; the
/// status and the headers stay readable.
async fn read_body(response: &mut FlUrlResponse) -> Result<Vec<u8>, FlUrlError> {
    response
        .get_body()?
        .into_vec(MAX_RESPONSE_BODY_LEN)
        .await
}
