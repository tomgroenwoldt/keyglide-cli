use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info};
use uuid::Uuid;

use common::BackendMessage;

use super::App;
use crate::lobby::Player;

pub enum AppMessage {
    /// Broadcasts all already connected players provided lobby to provided
    /// player.
    CurrentPlayers { lobby_id: Uuid, player: Player },
    /// Creates a new lobby and adds provided player.
    CreateLobbyAndAddPlayer { player: Player },
    /// Adds provided player to an already existing lobby or creates a new lobby
    /// if non exist.
    AddPlayerViaQuickplay { player: Player },
    /// Adds provided player to the lobby and broadcasts this information to
    /// already connected players.
    AddPlayerToLobby { lobby_id: Uuid, player: Player },
    /// Broadcasts a message of provided player to all connected players.
    SendMessage { player: Player, message: String },
    /// Removes a player from the lobby and broadcasts this information to
    /// already connected players.
    RemovePlayer { player: Player },
    /// Tells a player that the lobby he is trying to connect to is already
    /// full.
    LobbyFull {
        player_tx: UnboundedSender<BackendMessage>,
    },

    /// Broadcasts all existing lobbies to a freshly connected client.
    CurrentLobbies { client_id: Uuid },
    /// Broadcasts name and player count of a lobby to all connected clients.
    SendLobbyInformation { lobby_id: Uuid },
    /// Removes an existing lobby.
    RemoveLobby { lobby_id: Uuid },
    /// Broadcasts the current amount of connected clients and players to all
    /// connected clients.
    SendConnectionCounts,
    /// Adds a new client.
    AddClient {
        client_id: Uuid,
        client_tx: UnboundedSender<BackendMessage>,
    },
    /// Removes an existing client.
    RemoveClient { client_id: Uuid },
}

/// # Handle app message
///
/// Manages the app based on received `AppMessage`.
pub async fn handle_app_message(mut app: App) -> Result<()> {
    while let Some(msg) = app.rx.recv().await {
        match msg {
            AppMessage::CurrentPlayers { lobby_id, player } => {
                if let Some(lobby) = app.lobbies.get(&lobby_id) {
                    lobby.send_current_players(player)?;
                } else {
                    error!("Lobby with ID {} was not found.", lobby_id);
                }
            }
            AppMessage::CreateLobbyAndAddPlayer { player } => {
                app.add_player_to_new_lobby(player)?;
            }
            AppMessage::AddPlayerViaQuickplay { player } => {
                app.add_player_via_quickplay(player)?;
            }
            AppMessage::AddPlayerToLobby { lobby_id, player } => {
                if let Some(lobby) = app.lobbies.get_mut(&lobby_id) {
                    lobby.add_player(player, &app.tx)?;
                } else {
                    error!("Lobby with ID {} was not found.", lobby_id);
                }
            }
            AppMessage::SendMessage { player, message } => {
                if let Some(lobby) = app
                    .lobbies
                    .values_mut()
                    .find(|lobby| lobby.players.contains_key(&player.id))
                {
                    lobby.send_message(player, message.clone())?;
                } else {
                    error!(
                        "No lobby has player {}. Unable to send message to the rest of the lobby members.",
                        player.name
                    );
                }
            }
            AppMessage::RemovePlayer { player } => {
                if let Some(lobby) = app
                    .lobbies
                    .values_mut()
                    .find(|lobby| lobby.players.contains_key(&player.id))
                {
                    lobby.remove_player(player, &app.tx)?;
                } else {
                    error!(
                        "No lobby has player {}. Unable to delete the player.",
                        player.name
                    );
                }
            }
            AppMessage::LobbyFull { player_tx } => {
                let message = BackendMessage::LobbyFull;
                player_tx.send(message)?;
            }

            AppMessage::CurrentLobbies { client_id } => {
                if let Some(client) = app.clients.get(&client_id) {
                    let lobbies = app.get_current_lobbies();
                    let message = BackendMessage::CurrentLobbies(lobbies);
                    client.send(message)?;
                } else {
                    error!("Client with ID {} was not found.", client_id);
                }
            }
            AppMessage::SendLobbyInformation { lobby_id } => {
                app.send_lobby_information(lobby_id)?;
            }
            AppMessage::RemoveLobby { lobby_id } => {
                app.remove_lobby(lobby_id)?;
            }

            AppMessage::AddClient {
                client_id,
                client_tx,
            } => {
                app.clients.insert(client_id, client_tx);
                app.tx.send(AppMessage::SendConnectionCounts)?;
                info!(
                    "Added client with ID {}. Client count is {}.",
                    client_id,
                    app.clients.len()
                );
            }
            AppMessage::RemoveClient { client_id } => {
                app.clients.remove(&client_id);
                app.tx.send(AppMessage::SendConnectionCounts)?;
                info!(
                    "Removed client with ID {}. Client count is {}.",
                    client_id,
                    app.clients.len()
                );
            }
            AppMessage::SendConnectionCounts => {
                let clients = app.clients.len();
                let players = app.lobbies.values().map(|lobby| lobby.players.len()).sum();
                let message = BackendMessage::ConnectionCounts { clients, players };
                for client in app.clients.values() {
                    client.send(message.clone())?;
                }
            }
        }
    }
    Ok(())
}
