use crate::app::{App, Focus};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(area);

    render_topic_field(frame, app, chunks[0]);
    render_qos_field(frame, app, chunks[1]);
    render_options_row(frame, app, chunks[2]);
    render_publish_button(frame, app, chunks[3]);
    render_payload(frame, app, chunks[4]);
    render_payload_help(frame, app, chunks[5]);
}

fn render_topic_field(frame: &mut Frame, app: &App, area: Rect) {
    let label = if app.focus == Focus::PublishTopic {
        Span::styled("Topic: ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("Topic: ", Style::default().fg(Color::Gray))
    };

    let text = Span::styled(
        format!(
            "{}{}",
            app.publish_form.topic,
            if app.focus == Focus::PublishTopic {
                render_cursor(app, app.publish_form.topic.len(), &app.publish_form.topic)
            } else {
                String::new()
            }
        ),
        Style::default().fg(Color::White),
    );

    let para = Paragraph::new(Line::from(vec![label, text]));
    frame.render_widget(para, area);
}

fn render_qos_field(frame: &mut Frame, app: &App, area: Rect) {
    let label = if app.focus == Focus::PublishQos {
        Span::styled("QoS: ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("QoS: ", Style::default().fg(Color::Gray))
    };

    let qos_values = ["0 - At most once", "1 - At least once", "2 - Exactly once"];
    let qos_str = qos_values[app.publish_form.qos as usize];
    let text = Span::styled(
        format!("{}  [Spc] to cycle", qos_str),
        Style::default().fg(Color::Yellow),
    );

    let para = Paragraph::new(Line::from(vec![label, text]));
    frame.render_widget(para, area);
}

fn render_options_row(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let retain_style = if app.publish_form.retain {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let retain_text = format!("Retain: {} (^R)", if app.publish_form.retain { "ON " } else { "OFF" });
    let retain = Paragraph::new(Line::from(Span::styled(retain_text, retain_style)));
    frame.render_widget(retain, chunks[0]);

    let json_style = if app.publish_form.is_json {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let json_text = format!("Format: {} (^J)", if app.publish_form.is_json { "JSON" } else { "Text" });
    let json = Paragraph::new(Line::from(Span::styled(json_text, json_style)));
    frame.render_widget(json, chunks[1]);
}

fn render_publish_button(frame: &mut Frame, _app: &App, area: Rect) {
    let btn = Paragraph::new(Line::from(Span::styled(
        " ^P Publish ",
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(btn, area);
}

fn render_payload(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    let focused = app.focus == Focus::PublishPayload;

    for (i, payload_line) in app.publish_form.payload_lines.iter().enumerate() {
        if focused && i == app.publish_form.cursor_line {
            let before = &payload_line[..app.publish_form.cursor_col.min(payload_line.len())];
            let cursor = if app.cursor_visible { "▌" } else { " " };
            let after = &payload_line[app.publish_form.cursor_col.min(payload_line.len())..];
            lines.push(Line::from(vec![
                Span::styled(before.to_string(), Style::default().fg(Color::White)),
                Span::styled(cursor, Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
                Span::styled(after.to_string(), Style::default().fg(Color::White)),
            ]));
        } else {
            lines.push(Line::from(Span::styled(
                payload_line.clone(),
                Style::default().fg(Color::White),
            )));
        }
    }

    let border_style = if focused {
        Style::default().fg(Color::Magenta)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Payload ")
            .border_style(border_style),
    );

    frame.render_widget(para, area);
}

fn render_payload_help(frame: &mut Frame, _app: &App, area: Rect) {
    let help = Paragraph::new(Line::from(Span::styled(
        "Enter: newline | ^P: Publish",
        Style::default().fg(Color::DarkGray),
    )));
    frame.render_widget(help, area);
}

fn render_cursor(app: &App, cursor: usize, _text: &str) -> String {
    if app.cursor_visible {
        "▌".to_string()
    } else if cursor == 0 {
        String::new()
    } else {
        " ".to_string()
    }
}
