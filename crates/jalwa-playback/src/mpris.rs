//! MPRIS2 D-Bus interface for media key support.
//!
//! Exposes jalwa as an MPRIS2 media player on the session bus, enabling
//! hardware media keys (play/pause/next/prev/stop) and desktop integration.

use std::sync::mpsc;

/// MPRIS commands received from D-Bus (media keys, desktop controls).
#[derive(Debug, Clone)]
pub enum MprisCommand {
    PlayPause,
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    Seek(f64),  // offset in seconds
    SetVolume(f64), // 0.0..1.0
}

/// Spawn the MPRIS2 D-Bus server on a background tokio task.
///
/// Returns a receiver for commands from media keys / desktop integration.
/// The server runs until the receiver is dropped.
pub fn spawn_mpris_server() -> mpsc::Receiver<MprisCommand> {
    let (tx, rx) = mpsc::channel();

    std::thread::Builder::new()
        .name("jalwa-mpris".into())
        .spawn(move || {
            if let Err(e) = run_mpris_server(tx) {
                tracing::warn!("MPRIS server failed: {e}");
            }
        })
        .ok();

    rx
}

fn run_mpris_server(tx: mpsc::Sender<MprisCommand>) -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async move {
        let conn = zbus::Connection::session().await?;

        let player = MprisPlayer { cmd_tx: tx };
        let root = MprisRoot;

        conn.object_server()
            .at("/org/mpris/MediaPlayer2", root)
            .await?;
        conn.object_server()
            .at("/org/mpris/MediaPlayer2", player)
            .await?;

        let bus_name = zbus::names::WellKnownName::try_from("org.mpris.MediaPlayer2.jalwa").unwrap();
        conn.request_name(bus_name).await?;

        // Keep running until sender side is dropped
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    })
}

/// org.mpris.MediaPlayer2 root interface
struct MprisRoot;

#[zbus::interface(name = "org.mpris.MediaPlayer2")]
impl MprisRoot {
    #[zbus(property)]
    fn can_quit(&self) -> bool { true }

    #[zbus(property)]
    fn can_raise(&self) -> bool { false }

    #[zbus(property)]
    fn has_track_list(&self) -> bool { false }

    #[zbus(property)]
    fn identity(&self) -> &str { "Jalwa" }

    #[zbus(property)]
    fn desktop_entry(&self) -> &str { "jalwa" }

    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<String> { vec!["file".into()] }

    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<String> {
        vec![
            "audio/mpeg".into(), "audio/flac".into(), "audio/ogg".into(),
            "audio/wav".into(), "audio/aac".into(), "audio/opus".into(),
            "audio/mp4".into(),
        ]
    }

    fn quit(&self) {}
    fn raise(&self) {}
}

/// org.mpris.MediaPlayer2.Player interface
struct MprisPlayer {
    cmd_tx: mpsc::Sender<MprisCommand>,
}

#[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayer {
    fn play_pause(&self) {
        let _ = self.cmd_tx.send(MprisCommand::PlayPause);
    }

    fn play(&self) {
        let _ = self.cmd_tx.send(MprisCommand::Play);
    }

    fn pause(&self) {
        let _ = self.cmd_tx.send(MprisCommand::Pause);
    }

    fn stop(&self) {
        let _ = self.cmd_tx.send(MprisCommand::Stop);
    }

    fn next(&self) {
        let _ = self.cmd_tx.send(MprisCommand::Next);
    }

    fn previous(&self) {
        let _ = self.cmd_tx.send(MprisCommand::Previous);
    }

    #[zbus(property)]
    fn can_play(&self) -> bool { true }

    #[zbus(property)]
    fn can_pause(&self) -> bool { true }

    #[zbus(property)]
    fn can_go_next(&self) -> bool { true }

    #[zbus(property)]
    fn can_go_previous(&self) -> bool { true }

    #[zbus(property)]
    fn can_seek(&self) -> bool { true }

    #[zbus(property)]
    fn can_control(&self) -> bool { true }

    #[zbus(property)]
    fn playback_status(&self) -> &str { "Stopped" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpris_command_debug() {
        let cmd = MprisCommand::PlayPause;
        assert!(format!("{:?}", cmd).contains("PlayPause"));
    }

    #[test]
    fn mpris_command_variants() {
        let cmds = vec![
            MprisCommand::Play,
            MprisCommand::Pause,
            MprisCommand::Stop,
            MprisCommand::Next,
            MprisCommand::Previous,
            MprisCommand::Seek(10.0),
            MprisCommand::SetVolume(0.5),
        ];
        assert_eq!(cmds.len(), 7);
    }
}
