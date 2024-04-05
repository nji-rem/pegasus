use std::sync::Arc;


use axum::routing::{get, post};
use tower_http::cors::CorsLayer;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::client::hotel;
use crate::core::taskmgr::task;

use crate::webapi::actions::web::WebService;

mod webapi;
mod app_config;
mod client;
mod communication;
mod retro;
pub mod connection;
mod core;
pub mod event;
pub mod room;
mod user;

#[derive(Clone)]
struct AppState {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ascii_art = r#" _  (`-') (`-')  _           (`-')  _  (`-').->            (`-').->
    \-.(OO ) ( OO).-/    .->    (OO ).-/  ( OO)_       .->    ( OO)_
    _.'    \(,------. ,---(`-') / ,---.  (_)--\_) ,--.(,--.  (_)--\_)
   (_...--'' |  .---''  .-(OO ) | \ /`.\ /    _ / |  | |(`-')/    _ /
   |  |_.' |(|  '--. |  | .-, \ '-'|_.' |\_..`--. |  | |(OO )\_..`--.
   |  .___.' |  .--' |  | '.(_/(|  .-.  |.-._)   \|  | | |  \.-._)   \
   |  |      |  `---.|  '-'  |  |  | |  |\       /\  '-'(_ .'\       /
   `--'      `------' `-----'   `--' `--' `-----'  `-----'    `-----'  "#;
    println!("{}", ascii_art);
    println!("Habbo Load Tester");
    println!("Developed by Merijn (Discord: merijnn)");
    println!("-------------------------------------------------------------------------------");

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();

    let retro_manager = Arc::new(retro::Manager::new());
    let task_manager = Arc::new(task::Manager::new());

    let app_config = app_config::load().unwrap();

    let mut web_service = WebService::new();

    if app_config.use_default_handlers {
        for handler in app_config.handlers {
            let name = handler.name.clone();
            let ws_link = handler.ws_link.clone();
            let origin = handler.origin.clone();

            let retro_manager_clone = retro_manager.clone();

            tokio::spawn(async move {
                let connector = Arc::new(connection::Connector::new(ws_link, origin));
                let manager = Arc::new(hotel::Manager::new(connector));

                info!("Added hotel");

                retro_manager_clone.add_hotel(name, manager).await.unwrap();
            });
        }
    }

    // Configure routes
    web_service.configure_routes(|router| {
        // Health
        let router = router.route("/api/health", get(webapi::health::index));

        // Task manager
        let router = router.route("/api/tasks/delete", post(webapi::task::kill_task));

        // Bot actions
        let router = router
            .route("/api/bots/available", post(webapi::bot::available))
            .route(
                "/api/bots/broadcast/message",
                post(webapi::bot::broadcast_message),
            )
            .route(
                "/api/bots/broadcast/enter_room",
                post(webapi::bot::broadcast_enter_room),
            )
            .route("/api/bots/broadcast/walk", post(webapi::bot::broadcast_walk))
            .route("/api/bots/broadcast/cfh_abuse", post(webapi::bot::broadcast_cfh_abuse));

        // Connection actions
        let router = router
            .route("/api/hotels", post(webapi::hotel::list));
            // .route("/webapi/add_hotel", post(webapi::hotel::add_hotel));

        // Session actions
        let router = router
            .route("/api/sessions/add", post(webapi::api_session::add))
            .route("/api/sessions/add_many", post(webapi::api_session::add_many));

        return router;
    })?;

    // Configure webservice extensions
    web_service.configure_extensions(|router| {
        let app_state = webapi::app_state::AppState{
            auth_token: app_config.auth_token.clone(),
        };

        info!("Using auth token {}", &app_state.auth_token);

        return router
            .layer(axum::Extension(retro_manager))
            .layer(axum::Extension(task_manager))
            .layer(CorsLayer::permissive())
            .route_layer(axum::middleware::from_fn_with_state(app_state.clone(), webapi::middleware::auth::handle));
    })?;

    web_service.start(app_config.webserver.clone()).await?;

    Ok(())
}
