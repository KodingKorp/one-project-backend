#[cfg(test)]
mod test {
    use poem::http::StatusCode;
    use poem_openapi::types::{Email, ToJSON};

    use crate::{
        capabilities::{
            iam::{controllers::authentication::CreateUserWithPassword, objects::UserObject},
            lib::common_response::Data,
        },
        test_utils::get_client,
    };
    #[tokio::test]
    async fn register() -> Result<(), std::io::Error> {
        let client = get_client().await;
        let now = chrono::Local::now().timestamp_millis();
        let email = format!("test+{}@test.com", &now);
        let new_user = CreateUserWithPassword {
            email: Email(email.clone()),
            password: "P@ssw0rd".to_owned(),
            confirm_password: "P@ssw0rd".to_owned(),
        };
        let new_user_str = new_user.to_json_string();
        let resp = client
            .post("/api/v1/auth/register-pass")
            .body(new_user_str)
            .header("Content-Type", "application/json")
            .send()
            .await;
        resp.assert_status(StatusCode::CREATED);
        assert_eq!(
            resp.0.header("Content-Type"),
            Some("application/json; charset=utf-8")
        );
        let resp_str = resp
            .0
            .into_body()
            .into_string()
            .await
            .unwrap_or(String::from("Error"));
        let result = serde_json::from_str::<Data<UserObject>>(resp_str.as_str());
        assert!(result.is_ok());
        let user = result.unwrap();
        assert!(user.data.email == email);
        Ok(())
    }
}
