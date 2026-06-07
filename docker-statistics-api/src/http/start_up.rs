use std::{net::SocketAddr, sync::Arc};

use my_http_server::controllers::swagger::SwaggerMiddleware;
use my_http_server::web_sockets::MyWebsocketMiddleware;
use my_http_server::MyHttpServer;

use crate::app::AppCtx;
use crate::ws::{ExecWsCallback, LogsWsCallback};

pub fn setup_server(app: &Arc<AppCtx>) {
    let mut http_server = MyHttpServer::new(SocketAddr::from(([0, 0, 0, 0], 8000)));

    let controllers = Arc::new(crate::http::controllers::builder::build(app));

    let swagger_middleware = SwaggerMiddleware::new(
        controllers.clone(),
        "docker-statistics-api".to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
    );

    http_server.add_middleware(Arc::new(MyWebsocketMiddleware::new(
        "/ws/logs",
        Arc::new(LogsWsCallback::new(app.clone())),
        my_logger::LOGGER.clone(),
    )));

    http_server.add_middleware(Arc::new(MyWebsocketMiddleware::new(
        "/ws/exec",
        Arc::new(ExecWsCallback::new(app.clone())),
        my_logger::LOGGER.clone(),
    )));

    http_server.add_middleware(Arc::new(swagger_middleware));
    http_server.add_middleware(controllers);

    http_server.start(app.app_states.clone(), my_logger::LOGGER.clone());
}
