use std::sync::Arc;

use my_http_server::controllers::ControllersMiddleware;

use crate::app::AppContext;

pub fn build_controllers(app: &Arc<AppContext>) -> ControllersMiddleware {
    let mut controllers_middleware = ControllersMiddleware::new(None, None);

    controllers_middleware.register_get_action(Arc::new(
        super::controllers::containers_controller::GetContainersAction::new(app.clone()),
    ));

    controllers_middleware.register_get_action(Arc::new(
        super::controllers::containers_controller::GetRunningContainersAction::new(app.clone()),
    ));

    controllers_middleware.register_get_action(Arc::new(
        super::controllers::containers_controller::GetLogsAction::new(app.clone()),
    ));

    // metrics

    controllers_middleware.register_get_action(Arc::new(
        super::controllers::metrics_controller::GetListOfServicesWithMetrics::new(app.clone()),
    ));

    controllers_middleware
}
