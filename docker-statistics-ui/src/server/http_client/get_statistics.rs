use flurl::{FlUrl, FlUrlError};

use crate::models::StatisticsContract;

pub async fn get_statistics(fl_url: FlUrl) -> Result<StatisticsContract, FlUrlError> {
    let mut url_response = fl_url
        .append_path_segment("api")
        .append_path_segment("containers")
        .get()
        .await?;

    url_response.get_json().await
}
