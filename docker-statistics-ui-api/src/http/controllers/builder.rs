use std::sync::Arc;

use my_http_server::controllers::ControllersMiddleware;

use crate::app::AppCtx;

pub fn build(app: &Arc<AppCtx>) -> ControllersMiddleware {
    let mut result = ControllersMiddleware::new(None, None);

    result.register_get_action(Arc::new(super::envs::GetEnvsAction::new(app.clone())));

    result.register_get_action(Arc::new(super::metrics::GetVmCpuAndMemAction::new(
        app.clone(),
    )));

    result.register_get_action(Arc::new(super::logs::GetLogsAction::new(app.clone())));

    result.register_get_action(Arc::new(super::processes::GetProcessesAction::new(
        app.clone(),
    )));

    result.register_post_action(Arc::new(super::ssh::ApplyPassPhraseAction::new(
        app.clone(),
    )));

    result
}
