use std::path::Path;

use poem::{http::StatusCode, test::TestClient};

use crate::bootstrap::{self, App};

pub async fn get_client() -> TestClient<App> {
    let _ = dotenvy::from_path(Path::new(
        format!("{}/.env.e2e.test", env!("CARGO_MANIFEST_DIR")).as_str(),
    ));
    let app = bootstrap::build_app().await;
    TestClient::new(app)
}

pub async fn get_client_with_user() -> TestClient<App> {
    let _ = dotenvy::from_path(Path::new(
        format!("{}/.env.e2e.test", env!("CARGO_MANIFEST_DIR")).as_str(),
    ));
    let app = bootstrap::build_app().await;
    let client = TestClient::new(app);
    let resp = client
        .post(format!(
            "/auth/login_test/{}",
            std::env::var("TEST_EMAIL").unwrap()
        ))
        .send()
        .await;
    resp.assert_status(StatusCode::CREATED);
    let r = resp.json().await;
    let token: String = r
        .value()
        .object()
        .get("data")
        .object()
        .get("token")
        .string()
        .parse()
        .unwrap();
    TestClient::new(bootstrap::build_app().await).default_header("Authorization", token)
}
