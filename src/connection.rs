use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock, watch};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_tungstenite::tungstenite::Message;
use crate::{client, session};
use crate::session::Session;
use anyhow::{Result};

pub struct Config {
    pub(crate) ws_link: String,
    pub(crate) origin: String,
}

pub struct Service {
    config: Config,
    session_service: Arc<Mutex<session::Service>>
}

impl Service {
    pub fn new(
        config: Config,
        session_service: Arc<Mutex<session::Service>>,
    ) -> Self {
        Service { config, session_service }
    }

    pub async fn new_client(&self, auth_ticket: String) -> Result<Arc<Session>> {
        // Create a new channel for packet communication.
        let (tx, rx): (Sender<Message>, Receiver<Message>) = mpsc::channel(1);

        // Create a cancellation token for the threads that are related to this connection. Calling
        // this token will kill every thread that's related to this client.
        let (kill_sig_tx, kill_sig_rx) = watch::channel(false);

        // Create a new session object for this auth ticket.
        let session = Arc::new(Session{
            ticket: auth_ticket.clone(),
            tx: tx.clone(),
            kill_sig_rx,
            kill_sig_tx
        });

        let ws_link = self.config.ws_link.clone();
        let origin = self.config.origin.clone();
        let current_session = session.clone();
        let session_service = self.session_service.clone();

        session_service
            .lock()
            .await
            .add_session(current_session.clone())
            .await;

        tokio::spawn(async move {
            // Establish a connection with the server
            let client = client::connect(ws_link, origin).await?;

            println!("Connection created for auth ticket {}", &current_session.ticket);

            // Handle the connection.
            let result = client::handle(client, rx, tx, current_session.clone()).await;

            // Clean up when the connection just closed or when it has returned an error.
            let mut write_lock = session_service.lock().await;
            write_lock.delete(&current_session.ticket).await;

            println!("Session with auth ticket {} dropped", &current_session.ticket);

            return result
        });

        Ok(session)
    }
}