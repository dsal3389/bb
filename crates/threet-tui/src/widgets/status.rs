use ratatui::prelude::*;
use ratatui::text::Line;
use ratatui::text::Span;

pub struct StatusWidget<'a> {
    view_name: &'a str,
}

impl<'a> StatusWidget<'a> {
    pub fn new(view_name: &'a str) -> Self {
        StatusWidget { view_name }
    }

    fn view_spans(&self) -> [Span; 2] {
        [
            Span::from(format!(" {} ", self.view_name)).style(Style::new().italic().on_blue()),
            Span::from("\u{e0b0}").style(Style::new().blue()),
        ]
    }
}

impl Widget for StatusWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let mut spans = Vec::with_capacity(4);
        spans.extend(self.view_spans());
        Line::from_iter(spans).render(area, buf);
    }
}
