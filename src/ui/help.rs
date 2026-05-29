use crate::app::{App, Focus};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = match app.focus {
        Focus::Host => "Host: broker address | Tab: next field | ^C: Connect",
        Focus::Port => "Port: broker port (default 1883 TCP, 8083 WS) | Tab: next field",
        Focus::ClientId => "Client ID: unique MQTT client identifier | Tab: next field",
        Focus::Username => "Username: optional MQTT credentials | Tab: next field",
        Focus::Password => "Password: optional MQTT credentials (hidden) | Tab: next field",
        Focus::SubscribeTopic => "Subscribe topic: e.g. sensors/# | ^S: Subscribe | Tab: next",
        Focus::SubscribeQos => "Subscription QoS: 0=fire & forget, 1=at least once, 2=exactly once | Space: cycle",
        Focus::PublishTopic => "Publish topic: target topic | ^P: Publish | Tab: next",
        Focus::PublishQos => "Publish QoS: 0, 1, or 2 | Space: cycle | ^P: Publish",
        Focus::PublishPayload => {
            "Payload: Enter for newline | ^J: JSON/Text | ^R: Retain | ^P: Publish"
        }
    };

    let paragraph = Paragraph::new(Line::from(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )));

    frame.render_widget(paragraph, area);
}
