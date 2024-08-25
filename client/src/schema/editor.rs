use std::{fs, path::Path};

use anyhow::Result;
use log::warn;
use portable_pty::{Child, CommandBuilder};
use ratatui::layout::Size;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use super::terminal::Terminal;
use crate::schema::lobby::LobbyMessage;

pub struct Editor {
    pub terminal: Terminal,
}

impl Editor {
    /// # Create a new editor
    ///
    /// Starts a new editor inside a PTY instance that opens up the start file
    /// of the current lobby.
    pub fn new(
        app_size: Size,
        lobby_tx: UnboundedSender<LobbyMessage>,
        start_file: Vec<u8>,
    ) -> Result<Self> {
        // Write the start file bytes to a file.
        let file_name = Uuid::new_v4();
        let file_path = format!("/tmp/{}", file_name);
        fs::write(&file_path, start_file)?;

        // Build the command that opens the new start file.
        let mut cmd = CommandBuilder::new("helix");
        let path = Path::new(&file_path);
        cmd.arg(path);

        // Build the terminal and resize it directly.
        let (mut terminal, child) = Terminal::new(app_size, cmd)?;
        terminal.resize(app_size.height, app_size.width)?;

        // Spawn a task that messages the application after our editor instance
        // terminates and kills the terminal process on app close.
        tokio::spawn(Editor::handle_termination(child, lobby_tx));

        Ok(Self { terminal })
    }

    /// # Handle termination
    ///
    /// Waits for the child process to finish. After finish, message the lobby
    /// and trigger a restart.
    pub async fn handle_termination(
        mut child: Box<dyn Child + Send + Sync>,
        lobby_tx: UnboundedSender<LobbyMessage>,
    ) -> Result<()> {
        child.wait()?;
        warn!("The editor process terminated.");
        lobby_tx.send(LobbyMessage::EditorTerminated)?;
        Ok(())
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.terminal.resize(rows, cols)?;
        Ok(())
    }
}
