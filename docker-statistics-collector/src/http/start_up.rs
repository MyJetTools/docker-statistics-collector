use std::{net::SocketAddr, sync::Arc};

use mcp_server_middleware::McpMiddleware;
use my_http_server::{controllers::swagger::SwaggerMiddleware, MyHttpServer};

use crate::app::AppContext;
use crate::mcp::{FindContainersHandler, GetContainerLogsHandler};

pub async fn start_http_server(app: &Arc<AppContext>) {
    let mut http_server = MyHttpServer::new(SocketAddr::from(([0, 0, 0, 0], 8000)));

    let controllers = super::build_controllers::build_controllers(app);
    let controllers = Arc::new(controllers);

    let mut mcp = McpMiddleware::new(
        "/mcp",
        crate::app::APP_NAME,
        crate::app::APP_VERSION,
        "Tools to inspect Docker containers across this instance and federated peers.",
    );

    mcp.register_tool_call(Arc::new(FindContainersHandler::new(app.clone())))
        .await;
    mcp.register_tool_call(Arc::new(GetContainerLogsHandler::new(app.clone())))
        .await;

    http_server.add_middleware(Arc::new(mcp));

    http_server.add_middleware(Arc::new(SwaggerMiddleware::new(
        controllers.clone(),
        crate::app::APP_NAME.to_string(),
        crate::app::APP_VERSION.to_string(),
    )));
    http_server.add_middleware(controllers.clone());

    http_server.start(app.states.clone(), my_logger::LOGGER.clone());
}
