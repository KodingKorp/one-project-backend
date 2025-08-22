#[cfg(test)]
mod test {
    use crate::{
        base::{Health, Ping},
        test_utils::get_client,
    };

    #[tokio::test]
    async fn health() -> Result<(), std::io::Error> {
        let client = get_client().await;
        let resp = client.get("/api/v1/health").send().await;
        let resp_str = resp
            .0
            .into_body()
            .into_string()
            .await
            .unwrap_or(String::from(""));
        assert_ne!(&resp_str, "");
        let health_response: Health = serde_json::from_str(&resp_str)?;
        assert!(
            health_response.db,
            "testing db connection with {} and {}",
            &health_response.db, true
        );
        assert!(
            health_response.mailer,
            "testing db mailer with {} and {}",
            &health_response.db, true
        );
        assert!(
            health_response.redis,
            "testing db redis with {} and {}",
            &health_response.db, true
        );
        Ok(())
    }

    #[tokio::test]
    async fn ping() -> Result<(), std::io::Error> {
        let client = get_client().await;
        let resp = client.get("/api/v1/ping").send().await;
        let resp_str = resp
            .0
            .into_body()
            .into_string()
            .await
            .unwrap_or(String::from(""));
        assert_ne!(&resp_str, "");
        let ping_response: Ping = serde_json::from_str(&resp_str)?;
        assert_eq!(
            ping_response.up, true,
            "testing db connection with {} and {}",
            &ping_response.up, true
        );
        Ok(())
    }
}
