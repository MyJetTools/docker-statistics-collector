use rust_extensions::{date_time::DateTimeAsMicroseconds, MyTimerTick, RepeatTimerIteration};

use crate::APP_CTX;

/// Deletes the metric day files which are past `metrics_days_to_keep`, and closes the
/// ones nobody reads or writes any more.
pub struct MetricsGcTimer;

#[async_trait::async_trait]
impl MyTimerTick for MetricsGcTimer {
    async fn tick(&self) -> RepeatTimerIteration {
        let days_to_keep = APP_CTX
            .settings_reader
            .get_settings()
            .await
            .get_metrics_days_to_keep();

        let storage = APP_CTX.metrics_storage.clone();

        let done = tokio::task::spawn_blocking(move || {
            let now = DateTimeAsMicroseconds::now().unix_microseconds / 1_000_000;
            storage.gc(now, days_to_keep)
        })
        .await;

        match done {
            Ok(messages) => {
                for message in messages {
                    println!("MetricsStorage: {}", message);
                }
            }
            Err(err) => println!("MetricsStorage: the GC did not complete: {:?}", err),
        }

        RepeatTimerIteration::WithInterval
    }
}
