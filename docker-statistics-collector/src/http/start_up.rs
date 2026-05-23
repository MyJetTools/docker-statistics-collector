use std::{net::SocketAddr, sync::Arc};

use mcp_server_middleware::McpMiddleware;
use my_http_server::{
    controllers::swagger::SwaggerMiddleware, web_sockets::MyWebsocketMiddleware, MyHttpServer,
};

use crate::app::AppContext;
use crate::mcp::{
    FindApplicationHandler, FindContainersHandler, GetContainerLogsHandler,
    HowToUseItPromptHandler, ListServersAndServicesHandler,
};
use crate::ws::LogsWsCallback;

/// Sent to MCP clients on `initialize` as ServerInfo.instructions. Loaded at
/// compile time from MCP_INSTRUCTION.md so the document can be edited as
/// markdown without touching Rust source. Re-build the binary after editing.
const MCP_INSTRUCTIONS: &str = include_str!("../../MCP_INSTRUCTION.md");

pub async fn start_http_server(app: &Arc<AppContext>) {
    let mut http_server = MyHttpServer::new(SocketAddr::from(([0, 0, 0, 0], 8000)));

    let controllers = super::build_controllers::build_controllers(app);
    let controllers = Arc::new(controllers);

    let mut mcp = McpMiddleware::new(
        "/mcp",
        crate::app::APP_NAME,
        crate::app::APP_VERSION,
        MCP_INSTRUCTIONS,
    );

    mcp.register_tool_call(Arc::new(ListServersAndServicesHandler::new(app.clone())));
    mcp.register_tool_call(Arc::new(FindApplicationHandler::new(app.clone())));
    mcp.register_tool_call(Arc::new(FindContainersHandler::new(app.clone())));
    mcp.register_tool_call(Arc::new(GetContainerLogsHandler::new(app.clone())));

    mcp.register_prompt(Arc::new(HowToUseItPromptHandler));

    http_server.add_middleware(Arc::new(mcp));

    http_server.add_middleware(Arc::new(MyWebsocketMiddleware::new(
        "/ws/logs",
        Arc::new(LogsWsCallback::new(app.clone())),
        my_logger::LOGGER.clone(),
    )));

    http_server.add_middleware(Arc::new(SwaggerMiddleware::new(
        controllers.clone(),
        crate::app::APP_NAME.to_string(),
        crate::app::APP_VERSION.to_string(),
    )));
    http_server.add_middleware(controllers.clone());

    http_server.start(app.states.clone(), my_logger::LOGGER.clone());
}
