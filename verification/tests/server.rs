use pretty_assertions::assert_eq;
use verification::{run_http_server, Settings};

#[actix_rt::test]
async fn server_start() {
    let mut settings = Settings::default();
    settings.solidity.enabled = false;
    settings.sourcify.enabled = false;
    let base = format!("http://{}", settings.server.addr);
    let metrics_base = format!("http://{}", settings.metrics.addr);

    let _server_handle = {
        let settings = settings.clone();
        tokio::spawn(async move { run_http_server(settings).await })
    };
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    let resp = reqwest::get(format!("{base}/health"))
        .await
        .expect("failed to connect to server");
    assert_eq!(resp.status(), 200);

    let resp = reqwest::get(format!("{metrics_base}/metrics"))
        .await
        .expect("failed to connect to server");
    assert_eq!(resp.status(), 200);

    let body = resp.text().await.unwrap();
    for s in vec![
        "# TYPE verification_http_requests_duration_seconds histogram",
        "verification_http_requests_duration_seconds_bucket{endpoint=\"/health\",method=\"GET\",status=\"200\"",
    ] {
        assert!(body.contains(s), "body doesn't have string {s}:\n{body}");
    }
}
