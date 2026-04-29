use std::{net::SocketAddr, sync::Arc};

use my_http_server::{controllers::swagger::SwaggerMiddleware, MyHttpServer};

use crate::app::AppContext;
use crate::mcp::McpMiddleware;

pub async fn start_http_server(app: &Arc<AppContext>) {
    let mut http_server = MyHttpServer::new(SocketAddr::from(([0, 0, 0, 0], 8000)));

    let controllers = super::build_controllers::build_controllers(app);
    let controllers = Arc::new(controllers);

    // /mcp is handled by the MCP streamable-HTTP middleware. Register it first so
    // the request never falls through to the controllers/swagger pipeline.
    http_server.add_middleware(Arc::new(McpMiddleware::new(app.clone())));

    http_server.add_middleware(Arc::new(SwaggerMiddleware::new(
        controllers.clone(),
        crate::app::APP_NAME.to_string(),
        crate::app::APP_VERSION.to_string(),
    )));
    http_server.add_middleware(controllers.clone());

    http_server.start(app.states.clone(), my_logger::LOGGER.clone());
}
