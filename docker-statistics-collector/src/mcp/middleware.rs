use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use http::Response;
use http_body_util::{BodyExt, Full};
use my_http_server::{
    HttpContext, HttpFailResult, HttpOkResult, HttpOutput, HttpServerMiddleware, MyHyperHttpRequest,
};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, tower::StreamableHttpServerConfig, StreamableHttpService,
};

use crate::app::AppContext;

use super::server::DockerMcpServer;

const MCP_PATH: &str = "/mcp";

pub struct McpMiddleware {
    inner: StreamableHttpService<DockerMcpServer, LocalSessionManager>,
}

impl McpMiddleware {
    pub fn new(app: Arc<AppContext>) -> Self {
        let app_for_factory = app.clone();
        let inner = StreamableHttpService::new(
            move || Ok(DockerMcpServer::new(app_for_factory.clone())),
            Arc::new(LocalSessionManager::default()),
            StreamableHttpServerConfig::default()
                .with_sse_keep_alive(Some(Duration::from_secs(15)))
                .with_stateful_mode(true),
        );

        Self { inner }
    }
}

#[async_trait]
impl HttpServerMiddleware for McpMiddleware {
    async fn handle_request(
        &self,
        ctx: &mut HttpContext,
    ) -> Option<Result<HttpOkResult, HttpFailResult>> {
        if !ctx.request.get_path().equals_to(MCP_PATH) {
            return None;
        }

        let hyper_req = ctx.request.take_my_hyper_http_request();
        let response = match hyper_req {
            MyHyperHttpRequest::Incoming(req) => self.inner.handle(req).await,
            MyHyperHttpRequest::Full(req) => self.inner.handle(req).await,
        };

        // BoxBody<Bytes, Infallible> -> BoxBody<Bytes, String> required by MyHttpResponse.
        let (parts, body) = response.into_parts();
        let body = body.map_err(|never| match never {}).boxed();
        let response = Response::from_parts(parts, body);

        Some(Ok(HttpOkResult {
            write_telemetry: false,
            output: HttpOutput::Raw(response),
        }))
    }
}

// Some helpers to silence unused imports if rmcp's body type changes.
#[allow(dead_code)]
fn _empty_body() -> http_body_util::combinators::BoxBody<Bytes, String> {
    Full::new(Bytes::new())
        .map_err(|never: std::convert::Infallible| match never {})
        .boxed()
}
