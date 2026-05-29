mod connection;
mod help;
mod publish;
mod subscriptions;
mod top_bar;

use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(5),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(frame.area());

    top_bar::render(frame, app, main_chunks[0]);

    connection::render(frame, app, main_chunks[1]);

    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[2]);

    subscriptions::render(frame, app, content[0]);
    publish::render(frame, app, content[1]);

    help::render(frame, app, main_chunks[3]);
}
