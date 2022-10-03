use std::time::Duration;

use futures::stream::BoxStream;

pub async fn deadline(unix_time: i64, tolerance: u64) -> BoxStream<'static, ()> {
    Box::pin(futures::stream::unfold(false, move |sent| async move {
        if sent {
            None
        } else {
            loop {
                if attest_util::now() > unix_time {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(tolerance)).await;
            }
            Some(((), true))
        }
    }))
}
