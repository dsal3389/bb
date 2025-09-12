use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use ratatui::TerminalOptions;
use ratatui::Viewport;
use ratatui::prelude::*;

use threet_storage::models::User;

use tokio::sync::Mutex;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::channel;
use tokio::time::MissedTickBehavior;
use tokio::time::interval;

use crate::combo::ComboCallback;
use crate::combo::ComboRecorder;
use crate::compositor::Compositor;
use crate::compositor::Layout;
use crate::event::Event;
use crate::event::Key;
use crate::event::KeyCode;
use crate::views::AuthenticateView;
use crate::views::HandlekeysResults;

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Insert,
    Normal,
}

/// context is used to passed to the compositor, and the
/// compositor will pass the app context to the currently focused view
pub struct Context<'a> {
    pub compositor: &'a mut Compositor,
    pub dispatcher: Sender<Event>,
    pub user: Option<&'a User>,
    pub mode: Mode,
}

pub struct App<W: Write> {
    events: Receiver<Event>,
    events_sender: Sender<Event>,
    terminal: Terminal<CrosstermBackend<W>>,
    mode: Mode,

    compositor: Compositor,

    /// vector of the current keys pressed by the user
    /// to match with the combo, this vector is filled when
    /// the app mode is in `Normal` and the vector is emptied
    /// when a `ESC` key is recieved
    recorder: ComboRecorder,

    /// defines the authenticated user for the current app
    user: Option<User>,
}

impl<W: Write> App<W> {
    /// creates a new application instance that will write to the
    /// given stdout buffer, the returned value includes a channel sender
    /// to insert events to the app from outside
    pub fn new(stdout: W, size: (u16, u16)) -> (Self, Sender<Event>) {
        let area = Rect::new(0, 0, size.0, size.1);
        let (app_tx, app_rx) = channel(1);
        let terminal = Terminal::with_options(
            CrosstermBackend::new(stdout),
            TerminalOptions {
                viewport: Viewport::Fixed(area),
            },
        )
        .unwrap();

        let mut compositor = Compositor::new(area);

        compositor.split_view(
            Box::new(AuthenticateView::new(app_tx.clone())),
            Layout::Vertical,
        );

        let app = App {
            events: app_rx,
            events_sender: app_tx.clone(),
            recorder: ComboRecorder::new(),
            user: None,
            mode: Mode::Normal,
            compositor,
            terminal,
        };
        (app, app_tx)
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        // initial unconditiond application render
        self.terminal.clear().unwrap();
        self.render();

        // a boolean value indicating if the tick event was comsumed, the tick
        // event task won't place more `Tick` events on the channel
        // if the last `Tick` event was not consumed
        // TODO: maybe should this be atomic bool?
        let tick_consumed = Arc::new(Mutex::new(true));

        tokio::spawn({
            let tick_consumed = tick_consumed.clone();
            let app_tx = self.events_sender.clone();

            async move {
                let mut interval_ = interval(Duration::from_millis(350));
                interval_.set_missed_tick_behavior(MissedTickBehavior::Skip);

                // FIXME!: need to kill that loop when app instance
                // is dropped!!!
                loop {
                    interval_.tick().await;

                    let mut tick_consumed = tick_consumed.lock().await;

                    if *tick_consumed {
                        // FIXME: this will break if the app drop
                        app_tx.send(Event::Tick).await.unwrap();
                        *tick_consumed = false;
                    }
                }
            }
        });

        while let Some(event) = self.events.recv().await {
            match event {
                Event::Stdin(bytes) => {
                    let Some(key) = Key::from_bytes(bytes.as_slice()) else {
                        continue;
                    };
                    self.recorder.extend([key; 1]);

                    let view = self.compositor.current_view_mut();

                    match view.handle_keys(self.recorder.as_ref()).await {
                        HandlekeysResults::Callback(callback) => {
                            let cx = Context {
                                dispatcher: self.events_sender.clone(),
                                compositor: &mut self.compositor,
                                user: self.user.as_ref(),
                                mode: self.mode,
                            };
                            callback(cx).await;
                        }
                        _ => {}
                    }
                }
                Event::Resize(mut size) => {
                    self.terminal
                        .resize(Rect::new(0, 0, size.0, size.1))
                        .unwrap();

                    // reduce 1 from the area hight because the app will use that line
                    // to render the status bar
                    size.1 -= 1;

                    // resize the compositor which wil trigger a recalculation
                    // and unconditional render
                    self.compositor.resize(size);
                    self.render();
                }
                Event::Render => self.render(),
                _ => {}
            };
        }
        Ok(())
    }

    #[inline]
    fn render(&mut self) {
        self.terminal
            .draw(|frame| self.compositor.render(frame.buffer_mut()))
            .unwrap();
    }
}
