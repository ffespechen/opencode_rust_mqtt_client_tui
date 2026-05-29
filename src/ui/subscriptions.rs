use crate::app::{App, Focus};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

    render_subscribe_form(frame, app, chunks[0]);
    render_topic_list(frame, app, chunks[1]);
    render_messages(frame, app, chunks[2]);
}

fn render_subscribe_form(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    let topic_label = if app.focus == Focus::SubscribeTopic {
        Span::styled("Topic: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("Topic: ", Style::default().fg(Color::Gray))
    };

    let topic_text = Span::styled(
        format!("{}{}", app.new_sub_topic, cursor_char(app, app.focus == Focus::SubscribeTopic, app.new_sub_topic_cursor, &app.new_sub_topic)),
        Style::default().fg(Color::White),
    );

    let topic_line = Line::from(vec![topic_label, topic_text]);
    let topic_para = Paragraph::new(topic_line);
    frame.render_widget(topic_para, chunks[0]);

    let qos_label = if app.focus == Focus::SubscribeQos {
        Span::styled("QoS: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("QoS: ", Style::default().fg(Color::Gray))
    };

    let qos_value = Span::styled(
        format!("{} [Spc] ", app.new_sub_qos),
        Style::default().fg(Color::Yellow),
    );

    let qos_line = Line::from(vec![qos_label, qos_value]);
    let qos_para = Paragraph::new(qos_line);
    frame.render_widget(qos_para, chunks[1]);

    let btn_style = if app.focus == Focus::SubscribeTopic || app.focus == Focus::SubscribeQos {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let btn = Paragraph::new(Line::from(Span::styled(" ^S Subscribe ", btn_style)));
    frame.render_widget(btn, chunks[2]);
}

fn render_topic_list(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .subscriptions
        .iter()
        .enumerate()
        .map(|(i, sub)| {
            let prefix = if i == app.focused_subscription {
                "▸ "
            } else {
                "  "
            };
            let count = sub.messages.len();
            let content = format!("{} {} (QoS:{}) [{}]", prefix, sub.topic, sub.qos, count);
            let style = if i == app.focused_subscription {
                Style::default()
                    .fg(sub.color)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default().fg(sub.color)
            };
            ListItem::new(Line::from(Span::styled(content, style)))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Subscriptions ")
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(list, area);
}

fn render_messages(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    if let Some(sub) = app.subscriptions.get(app.focused_subscription) {
        let start = sub.scroll_offset;
        let visible_height = area.height.saturating_sub(2) as usize;

        for msg in sub.messages.iter().rev().skip(start).take(visible_height) {
            let ts = msg.timestamp.format("%H:%M:%S").to_string();

            let time_span = Span::styled(
                format!("{} ", ts),
                Style::default().fg(Color::DarkGray),
            );

            let retain_span = if msg.retain {
                Span::styled("[R] ", Style::default().fg(Color::Yellow))
            } else {
                Span::raw("")
            };

            let payload_str = String::from_utf8_lossy(&msg.payload);
            let truncated: String = payload_str
                .chars()
                .take(area.width.saturating_sub(20) as usize)
                .collect();

            let payload_span = Span::styled(truncated, Style::default().fg(sub.color));

            lines.push(Line::from(vec![time_span, retain_span, payload_span]));
        }
    } else if !app.subscriptions.is_empty() {
        lines.push(Line::from(Span::styled(
            "No subscription selected",
            Style::default().fg(Color::Gray),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "No subscriptions. Use ^S to subscribe to a topic.",
            Style::default().fg(Color::Gray),
        )));
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Messages ")
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(paragraph, area);
}

fn cursor_char(_app: &App, focused: bool, cursor: usize, text: &str) -> &'static str {
    if focused && cursor < text.len() {
        "|"
    } else if focused {
        "▌"
    } else {
        ""
    }
}
