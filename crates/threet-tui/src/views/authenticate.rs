use std::pin::Pin;
use std::sync::LazyLock;
use std::time::Duration;

use async_trait::async_trait;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::prelude::*;
use ratatui::text::ToLine;
use ratatui::widgets::Block;
use ratatui::widgets::Padding;
use ratatui::widgets::Paragraph;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

use threet_storage::get_database;
use threet_storage::models::User;

use crate::app::Context;
use crate::combo::ComboRegister;
use crate::event::Event;
use crate::event::Key;
use crate::notifications::Notification;
use crate::utils::get_middle_area;
use crate::widgets::ButtonWidget;
use crate::widgets::Field;
use crate::widgets::FieldBuilder;
use crate::widgets::FieldKind;

use super::Focuse;
use super::FocuseIterator;
use super::HandlekeysResults;
use super::View;

static COMBOS: LazyLock<ComboRegister> = LazyLock::new(|| {
    let mut combos = ComboRegister::new();
    combos.add(vec![Key::from_utf8(&[0x61])], add_window);
    combos
});

fn add_window<'a>(cx: Context<'a>) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        cx.compositor.split_view(
            Box::new(AuthenticateView::new(cx.dispatcher.clone())),
            crate::compositor::Layout::Vertical,
        );
        cx.dispatcher.send(Event::Render).await.unwrap();
    })
}

#[derive(Default, Clone)]
enum FocuseArea {
    #[default]
    UsernameField,
    PasswordField,
    AuthenticateButton,
}

impl FocuseArea {
    #[inline]
    fn is_username_field(&self) -> bool {
        matches!(self, FocuseArea::UsernameField)
    }

    #[inline]
    fn is_password_field(&self) -> bool {
        matches!(self, FocuseArea::PasswordField)
    }

    #[inline]
    fn is_authenticate_button(&self) -> bool {
        matches!(self, FocuseArea::AuthenticateButton)
    }
}

impl FocuseIterator for FocuseArea {
    fn previous(&mut self) -> Self {
        match self {
            FocuseArea::AuthenticateButton => FocuseArea::PasswordField,
            FocuseArea::PasswordField => FocuseArea::UsernameField,
            FocuseArea::UsernameField => FocuseArea::AuthenticateButton,
        }
    }

    fn next(&mut self) -> Self {
        match self {
            FocuseArea::UsernameField => FocuseArea::PasswordField,
            FocuseArea::PasswordField => FocuseArea::AuthenticateButton,
            FocuseArea::AuthenticateButton => FocuseArea::UsernameField,
        }
    }
}

pub struct AuthenticateView {
    app_tx: Sender<Event>,
    focuse: Focuse<FocuseArea>,

    // authentication task will contain the task handler for the
    // authentication, it is a separate task to not block the processor
    // and if we see we have a authentication task that is not
    // done, we display a loading screen
    authentication_task: Option<JoinHandle<()>>,
    username: Field,
    password: Field,
}

impl AuthenticateView {
    pub fn new(app_tx: Sender<Event>) -> Self {
        let username = FieldBuilder::default()
            .min(2)
            .max(16)
            .kind(FieldKind::String)
            .build();
        let password = FieldBuilder::default()
            .min(2)
            .max(32)
            .kind(FieldKind::Secret)
            .build();

        AuthenticateView {
            app_tx,
            username,
            password,
            authentication_task: None,
            focuse: Focuse::default(),
        }
    }

    #[inline]
    fn start_authentication_task(&mut self) {
        self.authentication_task = Some(tokio::spawn({
            let username = self.username.value().to_string();
            let password = self.password.value().to_string();
            let app_tx = self.app_tx.clone();

            async move {
                match User::by_username_password(get_database(), &username, &password).await {
                    Some(user) => {
                        app_tx.send(Event::SetUser(user)).await.unwrap();
                    }
                    None => {
                        let notification = Notification::error(
                            "authentiation error".to_string(),
                            "couldn't authentication with given credentials".to_string(),
                        );
                        // the notification message should also trigger an unconditional
                        // render to display the notification
                        app_tx
                            .send(Event::Notification((notification, Duration::from_secs(5))))
                            .await
                            .unwrap();
                    }
                }
            }
        }));
    }

    #[inline]
    fn is_authentication_task_running(&self) -> bool {
        self.authentication_task
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
    }
}

#[async_trait]
impl View for AuthenticateView {
    #[inline]
    fn name(&self) -> &str {
        "Authentication"
    }

    async fn handle_keys<'a>(&mut self, keys: &[Key]) -> HandlekeysResults<'a> {
        // if authentication task is running we should not handle
        // any new key event and we don't need to rerender the screen
        if self.is_authentication_task_running() {
            return HandlekeysResults::None;
        }
        match COMBOS.get(keys) {
            Some(callback) => HandlekeysResults::Callback(callback),
            None => HandlekeysResults::None,
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        // if the authentication task is not done
        // we should show a loading page
        if self.is_authentication_task_running() {
            Paragraph::new("loading").centered().render(area, buf);
            return;
        }

        let middle = get_middle_area((60, 13), area);
        let container = Block::bordered()
            .padding(Padding::symmetric(2, 1))
            .border_type(ratatui::widgets::BorderType::Thick)
            .title_top("[ Authenticate ]".to_line().style(Style::new().bold()))
            .style(Style::new().dark_gray());
        let [username_area, password_area, btn_area] =
            Layout::vertical([Constraint::Length(3); 3]).areas(container.inner(middle));

        container.render(middle, buf);

        let (username_widget, password_widget, btn_widget) = match self.focuse.current() {
            FocuseArea::UsernameField => (
                self.username.widget().focused(),
                self.password.widget(),
                ButtonWidget::new("LOGIN"),
            ),
            FocuseArea::PasswordField => (
                self.username.widget(),
                self.password.widget().focused(),
                ButtonWidget::new("LOGIN"),
            ),
            FocuseArea::AuthenticateButton => (
                self.username.widget(),
                self.password.widget(),
                ButtonWidget::new("LOGIN").focused(),
            ),
        };

        username_widget
            .placeholder("username...")
            .render(username_area, buf);
        password_widget
            .placeholder("password...")
            .render(password_area, buf);
        btn_widget.render(btn_area, buf);
    }
}
