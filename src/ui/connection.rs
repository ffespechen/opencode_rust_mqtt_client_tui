use crate::app::{App, ConnectionState, Focus};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let inner = Block::default()
        .borders(Borders::ALL)
        .title(" Connection ")
        .border_style(Style::default().fg(Color::DarkGray));
    let inner_area = inner.inner(area);
    frame.render_widget(inner, area);

    let rows = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner_area);

    let top = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(35),
        ])
        .split(rows[0]);

    render_input(frame, app, top[0], "Host", &app.connection_config.host, app.host_cursor, Focus::Host);
    render_input(frame, app, top[1], "Port", &app.connection_config.port.to_string(), app.port_cursor, Focus::Port);
    render_transport(frame, app, top[2]);
    render_input(frame, app, top[3], "Client ID", &app.connection_config.client_id, app.client_id_cursor, Focus::ClientId);

    let bottom = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Percentage(35),
            Constraint::Percentage(30),
        ])
        .split(rows[1]);

    render_input(frame, app, bottom[0], "User", &app.connection_config.username, app.username_cursor, Focus::Username);
    render_password(frame, app, bottom[1]);
    render_status(frame, app, bottom[2]);
}

fn render_input(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    label: &str,
    value: &str,
    cursor: usize,
    focus: Focus,
) {
    let is_focused = app.focus == focus;

    let label_style = if is_focused {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let value_style = if is_focused {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let display = build_cursor_text(value, cursor, is_focused && app.cursor_visible);

    let line = Line::from(vec![
        Span::styled(format!("{}: ", label), label_style),
        Span::styled(display, value_style),
    ]);

    let inner = Block::default()
        .borders(Borders::NONE)
        .style(if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        });
    let para = Paragraph::new(line).block(inner);
    frame.render_widget(para, area);
}

fn render_password(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::Password;
    let pwd = &app.connection_config.password;

    let label_style = if is_focused {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let value_style = if is_focused {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    let display = if pwd.is_empty() {
        build_cursor_text("", app.password_cursor, is_focused && app.cursor_visible)
    } else {
        let masked: String = pwd.chars().enumerate().map(|(i, _)| {
            if is_focused && i == app.password_cursor && app.cursor_visible {
                '▌'
            } else {
                '•'
            }
        }).collect();
        let cursor_after = if is_focused && app.password_cursor >= pwd.len() && app.cursor_visible {
            "▌".to_string()
        } else {
            String::new()
        };
        format!("{}{}", masked, cursor_after)
    };

    let line = Line::from(vec![
        Span::styled("Pass: ", label_style),
        Span::styled(display, value_style),
    ]);

    let para = Paragraph::new(line);
    frame.render_widget(para, area);
}

fn render_transport(frame: &mut Frame, app: &App, area: Rect) {
    let ws = app.connection_config.use_websockets;
    let label = Span::styled(
        "^W: ",
        Style::default().fg(Color::DarkGray),
    );
    let value = Span::styled(
        if ws { "WS " } else { "TCP" },
        Style::default()
            .fg(if ws { Color::Yellow } else { Color::White })
            .add_modifier(Modifier::BOLD),
    );
    let para = Paragraph::new(Line::from(vec![label, value]));
    frame.render_widget(para, area);
}

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let transport = if app.connection_config.use_websockets { "WS" } else { "TCP" };
    let port = app.connection_config.port;
    let (text, color) = match app.connection_state {
        ConnectionState::Connected => {
            (format!("Connected to {}:{}", app.connection_config.host, port), Color::Green)
        }
        ConnectionState::Connecting => {
            ("Connecting...".to_string(), Color::Yellow)
        }
        ConnectionState::Disconnected => {
            (format!("{}:{}", transport, port), Color::Gray)
        }
    };
    let para = Paragraph::new(Line::from(Span::styled(text, Style::default().fg(color))));
    frame.render_widget(para, area);
}

fn build_cursor_text(value: &str, cursor: usize, show_cursor: bool) -> String {
    if value.is_empty() {
        return if show_cursor { "▌".into() } else { " ".into() };
    }
    let c = cursor.min(value.len());
    let before = &value[..c];
    let cursor_char = if show_cursor { "▌" } else { " " };
    let after = &value[c..];
    format!("{}{}{}", before, cursor_char, after)
}
