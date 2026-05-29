use std::time::Duration;

use anyhow::Context;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use tokio::sync::mpsc::unbounded_channel;

mod app;
mod events;
mod mqtt;
mod ui;

use app::App;
use events::{key_to_action, Action};

fn main() {
    let result = run();
    if let Err(e) = result {
        eprintln!("Fatal error: {e:#}");
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen, crossterm::cursor::Show);
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let (mqtt_event_tx, mqtt_event_rx) = unbounded_channel();
    let (cmd_tx, cmd_rx) = unbounded_channel();

    let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
    rt.spawn(mqtt::run(cmd_rx, mqtt_event_tx));

    enable_raw_mode().context("Failed to enable raw mode — is a TTY available?")?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::cursor::Hide)
        .context("Failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to initialize terminal")?;

    let mut app = App::new(cmd_tx);

    let res = run_app(&mut terminal, &mut app, mqtt_event_rx);

    let _ = disable_raw_mode();
    let _ = execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::cursor::Show
    );
    let _ = terminal.show_cursor();

    res
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    mut mqtt_event_rx: tokio::sync::mpsc::UnboundedReceiver<mqtt::MqttEvent>,
) -> anyhow::Result<()> {
    loop {
        if app.should_quit {
            break;
        }

        app.tick();

        if event::poll(Duration::from_millis(16)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                    if let Some(action) = key_to_action(key, app) {
                        handle_action(app, action);
                    }
                }
            }
        }

        while let Ok(mqtt_event) = mqtt_event_rx.try_recv() {
            app.handle_mqtt_event(mqtt_event);
        }

        if let Err(e) = terminal.draw(|f| ui::render(f, app)) {
            eprintln!("Draw error (non-fatal): {e}");
        }
    }

    drop(app.cmd_tx.take());

    Ok(())
}

fn handle_action(app: &mut App, action: Action) {
    match action {
        Action::Quit => {
            app.should_quit = true;
            if let Some(tx) = &app.cmd_tx {
                let _ = tx.send(mqtt::MqttCmd::Disconnect);
            }
        }
        Action::Connect => app.connect(),
        Action::Disconnect => app.disconnect(),
        Action::Subscribe => app.subscribe(),
        Action::Unsubscribe => app.unsubscribe(),
        Action::Publish => app.publish(),
        Action::NextFocus => app.next_focus(),
        Action::PrevFocus => app.prev_focus(),
        Action::Char(c) => app.insert_char(c),
        Action::Backspace => app.backspace(),
        Action::Delete => app.delete(),
        Action::Newline => app.insert_newline(),
        Action::CursorLeft => app.move_cursor_left(),
        Action::CursorRight => app.move_cursor_right(),
        Action::CursorUp => app.move_cursor_up(),
        Action::CursorDown => app.move_cursor_down(),
        Action::Home => app.move_to_home(),
        Action::End => app.move_to_end(),
        Action::ToggleWebSockets => {
            app.connection_config.use_websockets = !app.connection_config.use_websockets;
            if app.connection_config.port == 1883 {
                app.connection_config.port = 8083;
            } else if app.connection_config.port == 8083 {
                app.connection_config.port = 1883;
            }
            app.status_message = format!(
                "Transport: {}",
                if app.connection_config.use_websockets {
                    "WebSocket"
                } else {
                    "TCP"
                }
            );
        }
        Action::ToggleRetain => {
            app.publish_form.retain = !app.publish_form.retain;
        }
        Action::ToggleJson => {
            app.publish_form.is_json = !app.publish_form.is_json;
        }
        Action::CycleValue => match app.focus {
            app::Focus::SubscribeQos => app.cycle_qos_sub(),
            app::Focus::PublishQos => app.cycle_qos_publish(),
            _ => {}
        },
        Action::TopicUp => {
            if app.focused_subscription > 0 {
                app.focused_subscription -= 1;
            }
        }
        Action::TopicDown => {
            if app.focused_subscription + 1 < app.subscriptions.len() {
                app.focused_subscription += 1;
            }
        }
        Action::ScrollUp => {
            if let Some(sub) = app.subscriptions.get_mut(app.focused_subscription) {
                sub.scroll_offset = sub.scroll_offset.saturating_add(1);
            }
        }
        Action::ScrollDown => {
            if let Some(sub) = app.subscriptions.get_mut(app.focused_subscription) {
                sub.scroll_offset = sub.scroll_offset.saturating_sub(1);
            }
        }
        Action::Noop => {}
    }
}
