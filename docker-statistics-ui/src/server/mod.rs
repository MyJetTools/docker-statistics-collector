mod app;
mod background;
pub mod http_client;
mod settings;

lazy_static::lazy_static! {
    pub static ref APP_CTX:  app::AppCtx = {
        app::AppCtx::new()
    };
}
