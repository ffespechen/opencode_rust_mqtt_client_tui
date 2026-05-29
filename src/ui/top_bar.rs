use crate::app::{App, ConnectionState, Focus};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let connected = app.connection_state == ConnectionState::Connected;

    let spans = vec![
        styled_shortcut("^C:Connect", connected, Color::Green),
        styled_shortcut("^D:Disconnect", false, Color::Red),
        styled_shortcut(
            "^S:Subscribe",
            Focus::SubscribeTopic == app.focus || Focus::SubscribeQos == app.focus,
            Color::Cyan,
        ),
        styled_shortcut("^U:Unsubscribe", false, Color::Yellow),
        styled_shortcut(
            "^P:Publish",
            Focus::PublishTopic == app.focus
                || Focus::PublishQos == app.focus
                || Focus::PublishPayload == app.focus,
            Color::Magenta,
        ),
        styled_shortcut("^W:WS/TCP", false, Color::Blue),
        styled_shortcut(
            "Spc:Cycle",
            Focus::SubscribeQos == app.focus || Focus::PublishQos == app.focus,
            Color::White,
        ),
        styled_shortcut("^Q:Quit", false, Color::Gray),
    ];

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}

fn styled_shortcut(label: &str, active: bool, color: Color) -> Span<'_> {
    if active {
        Span::styled(
            format!(" {} ", label),
            Style::default()
                .fg(Color::Black)
                .bg(color)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            format!(" {} ", label),
            Style::default().fg(Color::DarkGray),
        )
    }
}
