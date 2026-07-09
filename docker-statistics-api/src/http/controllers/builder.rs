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

    // exec permission — time-limited unlock of the exec_in_container MCP tool

    result.register_get_action(Arc::new(
        super::exec_permission::GetExecPermissionAction::new(app.clone()),
    ));

    result.register_post_action(Arc::new(
        super::exec_permission::EnableExecPermissionAction::new(app.clone()),
    ));

    result.register_post_action(Arc::new(
        super::exec_permission::DisableExecPermissionAction::new(app.clone()),
    ));

    result
}
