use crate::logger;
use poem::{
    http::StatusCode, session::Session, Endpoint, Error, IntoResponse, Middleware, Request,
    Response, Result,
};

use super::{constants, objects::SessionObject};

pub struct AuthorizationMiddleware;

impl<E: Endpoint> Middleware<E> for AuthorizationMiddleware {
    type Output = AuthorizationMiddlewareImpl<E>;

    fn transform(&self, ep: E) -> Self::Output {
        AuthorizationMiddlewareImpl(ep)
    }
}

pub struct AuthorizationMiddlewareImpl<E>(E);

impl<E: Endpoint> Endpoint for AuthorizationMiddlewareImpl<E> {
    type Output = Response;

    async fn call(&self, req: Request) -> Result<Self::Output> {
        let session = match req.extensions().get::<Session>() {
            Some(session) => session,
            None => {
                logger::error("Session extension not found in request. Ensure ServerSession middleware is applied.");
                return Err(Error::from_status(StatusCode::INTERNAL_SERVER_ERROR));
            }
        };
        if session
            .get::<SessionObject>(constants::SESSION_KEY_NAME)
            .is_none()
        {
            // Adjust key/type if needed
            logger::info("Unauthorized access attempt: No 'session' key found in session.");
            Err(Error::from_status(StatusCode::UNAUTHORIZED))
        } else {
            let res = self.0.call(req).await;

            match res {
                Ok(resp) => {
                    let resp = resp.into_response();
                    Ok(resp)
                }
                Err(err) => Err(err),
            }
        }
    }
}
