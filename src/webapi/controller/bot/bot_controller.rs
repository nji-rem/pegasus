use std::sync::Arc;
use axum::{debug_handler, Extension, Json};
use axum::extract::Path;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use crate::retro;
use axum;
use tokio_tungstenite::tungstenite::connect;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Status;
use crate::communication::outgoing::composer;
use crate::communication::outgoing::composer::Composable;
use crate::connection::session::Session;

#[derive(Serialize)]
pub struct AvailableBots {
    n: usize,
    bots: Option<Vec<BotInfo>>
}

#[derive(Serialize)]
pub struct BotInfo {
    pub user_id: u32,
    pub username: String,
    pub motto: String,
    pub figure: String,
    pub gender: String,
    pub sso_ticket: String
}
#[derive(Deserialize)]
pub struct BotsRequest {
    hotel: String,
}

pub async fn index(
    connection_service: Extension<Arc<retro::Manager>>,
    Json(payload): Json<BotsRequest>
) -> Result<Json<AvailableBots>, StatusCode> {
    match connection_service.get_hotel_connection_handler(payload.hotel) {
        Ok(handler) => {
            let mut response = AvailableBots{
                n: handler.get_session_service().online_bots(),
                bots: None,
            };

            let user_manager = handler.global_state().user_manager.clone();
            let bots: Vec<BotInfo> = user_manager.users().iter().map(|entry| {
                BotInfo{
                    user_id: entry.user_id.clone(),
                    username: entry.username.clone(),
                    motto: entry.motto.clone(),
                    figure: entry.figure.clone(),
                    gender: entry.gender.clone(),
                    sso_ticket: entry.sso_ticket.clone()
                }
            }).collect();

            if bots.len() > 0 {
                response.bots = Some(bots);
            }

            Ok(Json(response))
        },

        Err(_err) => {
            Err(StatusCode::NOT_FOUND)
        },
    }
}

#[derive(Deserialize)]
pub struct ShowBotRequest {
    hotel: String,
}

#[debug_handler]
pub async fn show(
    ticket: Path<String>,
    connection_service: Extension<Arc<retro::Manager>>,
    Json(payload): Json<ShowBotRequest>
) -> Result<Json<BotInfo>, StatusCode> {
    return match connection_service.get_hotel_connection_handler(payload.hotel) {
        Ok(handler) => {
            let user_manager = handler.global_state().user_manager.clone();

            if let Some(bot) = user_manager.get_user(ticket.clone()) {
                Ok(Json(BotInfo {
                    user_id: bot.user_id.clone(),
                    username: bot.username.clone(),
                    motto: bot.motto.clone(),
                    figure: bot.figure.clone(),
                    gender: bot.gender.clone(),
                    sso_ticket: bot.sso_ticket.clone()
                }))
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        },

        Err(_) => Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Deserialize)]
pub struct UpdateBot {
    hotel: String,
    motto: Option<String>,
    appearance: Option<UpdateAppearance>
}

#[derive(Deserialize)]
pub struct UpdateAppearance {
    gender: String,
    figure: String
}

#[debug_handler]
pub async fn update(
    ticket: Path<String>,
    connection_service: Extension<Arc<retro::Manager>>,
    Json(payload): Json<UpdateBot>
) -> Result<(), StatusCode> {
    match connection_service.get_hotel_connection_handler(payload.hotel) {
        Ok(handler) => {
            let session_service = handler.get_session_service();

            if let Some(session) = session_service.get(&ticket) {
                if let Some(motto) = payload.motto {
                    let _ = session_service.send(&session, composer::UpdateMotto { motto}.compose()).await;
                }

                if let Some(appearance) = payload.appearance {
                    let _ = session_service.send(&session, composer::UpdateLook { figure: appearance.figure, gender: appearance.gender }.compose()).await;
                }

                Ok(())
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        },

        Err(_) => {
            Err(StatusCode::NOT_FOUND)
        }
    }
}