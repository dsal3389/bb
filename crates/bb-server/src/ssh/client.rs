use anyhow::Context;
use russh::keys::PublicKey;
use russh::server::{Auth, Handler, Msg, Session};
use russh::{Channel, ChannelId, Pty};

use super::channel::AppChannel;

/// a simple macro that will return to the client the action status
/// if it failed or succeed based on the action result
macro_rules! channel_action_with_state {
    ($action: expr, $session: expr, $channel_id: expr) => {
        match $action {
            Ok(_) => $session.channel_success($channel_id)?,
            Err(_) => $session.channel_failure($channel_id)?,
        }
    };
}

/// represent a new client connection with a single application channel
/// the client will create a new AppChannel when the remote requests
/// a new session channel
///
/// the `AppClient` will forward the connection events to the correct
/// channel methods for the channel to handle correctly
#[derive(Default)]
pub struct AppClient {
    app_channel: Option<AppChannel>,
}

impl Handler for AppClient {
    type Error = anyhow::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        _: &mut Session,
    ) -> anyhow::Result<bool> {
        if self.app_channel.is_some() {
            anyhow::bail!("only a single session channel can be created");
        }

        self.app_channel = Some(AppChannel::new(channel.id()));
        Ok(true)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> anyhow::Result<()> {
        let app_channel = self
            .app_channel
            .as_mut()
            .context("expected `channel_open_session` to already be called")?;
        channel_action_with_state!(app_channel.stdin(data).await, session, channel);
        Ok(())
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _: &str,
        col_width: u32,
        row_height: u32,
        _: u32,
        _: u32,
        _: &[(Pty, u32)],
        session: &mut Session,
    ) -> anyhow::Result<()> {
        let app_channel = self
            .app_channel
            .as_mut()
            .context("expected `channel_open_session` to already be called")?;
        channel_action_with_state!(
            app_channel
                .create_pty(session.handle(), col_width as u16, row_height as u16)
                .await,
            session,
            channel
        );
        Ok(())
    }

    async fn window_change_request(
        &mut self,
        channel: ChannelId,
        col_width: u32,
        row_height: u32,
        _: u32,
        _: u32,
        session: &mut Session,
    ) -> anyhow::Result<()> {
        let app_channel = self
            .app_channel
            .as_ref()
            .context("expected `channel_open_session` to already be called")?;
        channel_action_with_state!(
            app_channel.resize(col_width as u16, row_height as u16),
            session,
            channel
        );
        Ok(())
    }

    async fn auth_none(&mut self, _: &str) -> anyhow::Result<Auth> {
        Ok(Auth::Accept)
    }

    async fn auth_password(&mut self, _: &str, _: &str) -> anyhow::Result<Auth> {
        Ok(Auth::Accept)
    }

    async fn auth_publickey(&mut self, _: &str, _: &PublicKey) -> anyhow::Result<Auth> {
        Ok(Auth::Accept)
    }
}
