use poem::Route;
use poem_openapi::OpenApi;

use crate::capabilities::background;

#[async_trait::async_trait]
pub trait Service {
    fn register_routes() -> Option<impl OpenApi> {
        None::<()>
    }
    fn register_health_check(_: Route) -> Option<Route> {
        None
    }
    fn register_private_routes() -> Option<impl OpenApi> {
        None::<()>
    }
    async fn register_background(
        runner: background::orchestrator::BackgroundOrchestrator,
    ) -> background::orchestrator::BackgroundOrchestrator {
        runner
    }
}
