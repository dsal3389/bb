use std::sync::LazyLock;

use async_trait::async_trait;

use super::HandlekeysResults;
use super::View;

use crate::combo::ComboRegister;
use crate::event::Key;

pub struct ChatView {}

impl ChatView {
    pub fn new() -> Self {
        ChatView {}
    }
}

#[async_trait]
impl View for ChatView {
    fn name(&self) -> &str {
        "chat"
    }

    fn render(&self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {}

    async fn handle_keys<'a>(&mut self, key: &[Key]) -> HandlekeysResults<'a> {
        HandlekeysResults::None
    }
}
