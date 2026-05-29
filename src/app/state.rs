use std::collections::VecDeque;

use chrono::{DateTime, Local};
use ratatui::style::Color;
use tokio::sync::mpsc::UnboundedSender;

use crate::mqtt::{MqttCmd, MqttEvent};

pub const MAX_MESSAGES_PER_TOPIC: usize = 500;

const TOPIC_COLORS: [Color; 12] = [
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::LightRed,
    Color::LightGreen,
    Color::LightYellow,
    Color::LightBlue,
    Color::LightMagenta,
    Color::LightCyan,
];

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Focus {
    Host,
    Port,
    ClientId,
    Username,
    Password,
    SubscribeTopic,
    SubscribeQos,
    PublishTopic,
    PublishQos,
    PublishPayload,
}

#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub host: String,
    pub port: u16,
    pub use_websockets: bool,
    pub client_id: String,
    pub username: String,
    pub password: String,
    pub clean_session: bool,
    pub keep_alive: u16,
}

#[derive(Debug, Clone)]
pub struct MqttMessage {
    pub payload: Vec<u8>,
    pub retain: bool,
    pub timestamp: DateTime<Local>,
}

#[derive(Debug, Clone)]
pub struct Subscription {
    pub topic: String,
    pub qos: u8,
    pub color: Color,
    pub messages: VecDeque<MqttMessage>,
    pub scroll_offset: usize,
}

#[derive(Debug, Clone)]
pub struct PublishForm {
    pub topic: String,
    pub qos: u8,
    pub retain: bool,
    pub is_json: bool,
    pub payload_lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
}

pub struct App {
    pub connection_config: ConnectionConfig,
    pub connection_state: ConnectionState,
    pub subscriptions: Vec<Subscription>,
    pub focused_subscription: usize,
    pub publish_form: PublishForm,
    pub focus: Focus,
    pub status_message: String,
    pub should_quit: bool,
    pub cursor_visible: bool,
    pub blink_tick: u64,
    pub cmd_tx: Option<UnboundedSender<MqttCmd>>,
    pub new_sub_topic: String,
    pub new_sub_topic_cursor: usize,
    pub new_sub_qos: u8,
    pub host_cursor: usize,
    pub port_cursor: usize,
    pub client_id_cursor: usize,
    pub username_cursor: usize,
    pub password_cursor: usize,
    pub publish_topic_cursor: usize,
    pub publish_qos_idx: usize,
}

impl App {
    pub fn new(cmd_tx: UnboundedSender<MqttCmd>) -> Self {
        let client_id = format!(
            "mqtt_tui_{}",
            uuid::Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("unknown")
        );

        Self {
            connection_config: ConnectionConfig {
                host: String::new(),
                port: 1883,
                use_websockets: false,
                client_id: client_id.clone(),
                username: String::new(),
                password: String::new(),
                clean_session: true,
                keep_alive: 60,
            },
            connection_state: ConnectionState::Disconnected,
            subscriptions: Vec::new(),
            focused_subscription: 0,
            publish_form: PublishForm {
                topic: String::new(),
                qos: 0,
                retain: false,
                is_json: false,
                payload_lines: vec![String::new()],
                cursor_line: 0,
                cursor_col: 0,
            },
            focus: Focus::Host,
            status_message: String::from("Disconnected | ^C: Connect  ^Q: Quit"),
            should_quit: false,
            cursor_visible: true,
            blink_tick: 0,
            cmd_tx: Some(cmd_tx),
            new_sub_topic: String::new(),
            new_sub_topic_cursor: 0,
            new_sub_qos: 0,
            host_cursor: 0,
            port_cursor: 0,
            client_id_cursor: client_id.len().saturating_sub(1),
            username_cursor: 0,
            password_cursor: 0,
            publish_topic_cursor: 0,
            publish_qos_idx: 0,
        }
    }

    pub fn tick(&mut self) {
        self.blink_tick = self.blink_tick.wrapping_add(1);
        if self.blink_tick.is_multiple_of(30) {
            self.cursor_visible = !self.cursor_visible;
        }
    }

    pub fn connect(&mut self) {
        if self.connection_state == ConnectionState::Connected {
            self.status_message = String::from("Already connected");
            return;
        }

        let host = self.connection_config.host.trim().to_string();
        if host.is_empty() {
            self.status_message = String::from("Host is required");
            return;
        }

        if let Some(tx) = &self.cmd_tx {
            self.connection_state = ConnectionState::Connecting;
            self.status_message =
                format!("Connecting to {}:{}...", host, self.connection_config.port);
            let _ = tx.send(MqttCmd::Connect(self.connection_config.clone()));
        }
    }

    pub fn disconnect(&mut self) {
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(MqttCmd::Disconnect);
        }
    }

    pub fn subscribe(&mut self) {
        let topic = self.new_sub_topic.trim().to_string();
        if topic.is_empty() {
            self.status_message = String::from("Topic is required for subscription");
            return;
        }

        if self.connection_state != ConnectionState::Connected {
            self.status_message = String::from("Not connected to a broker");
            return;
        }

        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(MqttCmd::Subscribe {
                topic: topic.clone(),
                qos: self.new_sub_qos,
            });
        }

        if !self.subscriptions.iter().any(|s| s.topic == topic) {
            let color = TOPIC_COLORS[self.subscriptions.len() % TOPIC_COLORS.len()];
            self.subscriptions.push(Subscription {
                topic: topic.clone(),
                qos: self.new_sub_qos,
                color,
                messages: VecDeque::new(),
                scroll_offset: 0,
            });
            self.focused_subscription = self.subscriptions.len().saturating_sub(1);
            self.status_message = format!("Subscribed to '{}' (QoS {})", topic, self.new_sub_qos);
        } else if let Some(sub) = self.subscriptions.iter_mut().find(|s| s.topic == topic) {
            sub.qos = self.new_sub_qos;
            self.status_message =
                format!("Updated subscription '{}' (QoS {})", topic, self.new_sub_qos);
        }
    }

    pub fn unsubscribe(&mut self) {
        if self.subscriptions.is_empty() {
            return;
        }

        let idx = self
            .focused_subscription
            .min(self.subscriptions.len().saturating_sub(1));
        let topic = self.subscriptions[idx].topic.clone();

        if self.connection_state == ConnectionState::Connected {
            if let Some(tx) = &self.cmd_tx {
                let _ = tx.send(MqttCmd::Unsubscribe {
                    topic: topic.clone(),
                });
            }
        }

        self.subscriptions.remove(idx);
        if self.subscriptions.is_empty() {
            self.focused_subscription = 0;
        } else if self.focused_subscription >= self.subscriptions.len() {
            self.focused_subscription = self.subscriptions.len().saturating_sub(1);
        }
        self.status_message = format!("Unsubscribed from '{}'", topic);
    }

    pub fn publish(&mut self) {
        let topic = self.publish_form.topic.trim().to_string();
        if topic.is_empty() {
            self.status_message = String::from("Topic is required for publishing");
            return;
        }

        let payload = self.publish_form.payload_lines.join("\n");

        if self.publish_form.is_json && !payload.trim().is_empty()
            && serde_json::from_str::<serde_json::Value>(&payload).is_err()
        {
            self.status_message = String::from("Invalid JSON payload");
            return;
        }

        if self.connection_state != ConnectionState::Connected {
            self.status_message = String::from("Not connected to a broker");
            return;
        }

        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(MqttCmd::Publish {
                topic: topic.clone(),
                qos: self.publish_form.qos,
                retain: self.publish_form.retain,
                payload: payload.into_bytes(),
            });
            self.status_message = format!("Published to '{}' (QoS {})", topic, self.publish_form.qos);
        }
    }

    pub fn handle_mqtt_event(&mut self, event: MqttEvent) {
        match event {
            MqttEvent::Connected => {
                self.connection_state = ConnectionState::Connected;
                self.status_message = String::from("Connected");
            }
            MqttEvent::Disconnected => {
                self.connection_state = ConnectionState::Disconnected;
                self.status_message = String::from("Disconnected");
            }
            MqttEvent::MessageReceived {
                topic,
                payload,
                retain,
            } => {
                let msg = MqttMessage {
                    payload,
                    retain,
                    timestamp: Local::now(),
                };

                if let Some(sub) = self.subscriptions.iter_mut().find(|s| s.topic == topic) {
                    if sub.messages.len() >= MAX_MESSAGES_PER_TOPIC {
                        sub.messages.pop_front();
                    }
                    sub.messages.push_back(msg);
                }
            }
            MqttEvent::Error(e) => {
                self.connection_state = ConnectionState::Disconnected;
                self.status_message = format!("Error: {}", e);
            }
        }
    }

    pub fn insert_char(&mut self, c: char) {
        match self.focus {
            Focus::Host => {
                self.connection_config.host.insert(self.host_cursor, c);
                self.host_cursor = (self.host_cursor + 1).min(self.connection_config.host.len());
            }
            Focus::Port => {
                if c.is_ascii_digit() {
                    let mut s = self.connection_config.port.to_string();
                    s.insert(self.port_cursor, c);
                    self.connection_config.port = s.parse().unwrap_or(self.connection_config.port);
                    self.port_cursor = (self.port_cursor + 1).min(s.len());
                }
            }
            Focus::ClientId => {
                self.connection_config
                    .client_id
                    .insert(self.client_id_cursor, c);
                self.client_id_cursor =
                    (self.client_id_cursor + 1).min(self.connection_config.client_id.len());
            }
            Focus::Username => {
                self.connection_config
                    .username
                    .insert(self.username_cursor, c);
                self.username_cursor =
                    (self.username_cursor + 1).min(self.connection_config.username.len());
            }
            Focus::Password => {
                self.connection_config
                    .password
                    .insert(self.password_cursor, c);
                self.password_cursor =
                    (self.password_cursor + 1).min(self.connection_config.password.len());
            }
            Focus::SubscribeTopic => {
                self.new_sub_topic.insert(self.new_sub_topic_cursor, c);
                self.new_sub_topic_cursor =
                    (self.new_sub_topic_cursor + 1).min(self.new_sub_topic.len());
            }
            Focus::PublishTopic => {
                self.publish_form
                    .topic
                    .insert(self.publish_topic_cursor, c);
                self.publish_topic_cursor =
                    (self.publish_topic_cursor + 1).min(self.publish_form.topic.len());
            }
            Focus::PublishPayload => {
                let line = &mut self.publish_form.payload_lines[self.publish_form.cursor_line];
                line.insert(self.publish_form.cursor_col, c);
                self.publish_form.cursor_col += 1;
            }
            _ => {}
        }
    }

    pub fn backspace(&mut self) {
        match self.focus {
            Focus::Host if self.host_cursor > 0 => {
                self.connection_config.host.remove(self.host_cursor - 1);
                self.host_cursor -= 1;
            }
            Focus::Port if self.port_cursor > 0 => {
                let mut s = self.connection_config.port.to_string();
                if !s.is_empty() && self.port_cursor <= s.len() {
                    s.remove(self.port_cursor - 1);
                    self.connection_config.port = s.parse().unwrap_or(0);
                    self.port_cursor -= 1;
                }
            }
            Focus::ClientId if self.client_id_cursor > 0 => {
                self.connection_config
                    .client_id
                    .remove(self.client_id_cursor - 1);
                self.client_id_cursor -= 1;
            }
            Focus::Username if self.username_cursor > 0 => {
                self.connection_config
                    .username
                    .remove(self.username_cursor - 1);
                self.username_cursor -= 1;
            }
            Focus::Password if self.password_cursor > 0 => {
                self.connection_config
                    .password
                    .remove(self.password_cursor - 1);
                self.password_cursor -= 1;
            }
            Focus::SubscribeTopic if self.new_sub_topic_cursor > 0 => {
                self.new_sub_topic
                    .remove(self.new_sub_topic_cursor - 1);
                self.new_sub_topic_cursor -= 1;
            }
            Focus::PublishTopic if self.publish_topic_cursor > 0 => {
                self.publish_form.topic.remove(self.publish_topic_cursor - 1);
                self.publish_topic_cursor -= 1;
            }
            Focus::PublishPayload if self.publish_form.cursor_col > 0 => {
                let line = &mut self.publish_form.payload_lines[self.publish_form.cursor_line];
                line.remove(self.publish_form.cursor_col - 1);
                self.publish_form.cursor_col -= 1;
            }
            _ => {}
        }
    }

    pub fn delete(&mut self) {
        match self.focus {
            Focus::Host if self.host_cursor < self.connection_config.host.len() => {
                self.connection_config.host.remove(self.host_cursor);
            }
            Focus::Port => {
                let s = self.connection_config.port.to_string();
                if self.port_cursor < s.len() {
                    let mut new_s = s;
                    new_s.remove(self.port_cursor);
                    self.connection_config.port = new_s.parse().unwrap_or(0);
                }
            }
            Focus::ClientId
                if self.client_id_cursor < self.connection_config.client_id.len() =>
            {
                self.connection_config.client_id.remove(self.client_id_cursor);
            }
            Focus::Username
                if self.username_cursor < self.connection_config.username.len() =>
            {
                self.connection_config.username.remove(self.username_cursor);
            }
            Focus::Password
                if self.password_cursor < self.connection_config.password.len() =>
            {
                self.connection_config.password.remove(self.password_cursor);
            }
            Focus::SubscribeTopic
                if self.new_sub_topic_cursor < self.new_sub_topic.len() =>
            {
                self.new_sub_topic.remove(self.new_sub_topic_cursor);
            }
            Focus::PublishTopic
                if self.publish_topic_cursor < self.publish_form.topic.len() =>
            {
                self.publish_form.topic.remove(self.publish_topic_cursor);
            }
            Focus::PublishPayload
                if self.publish_form.cursor_col
                    < self.publish_form.payload_lines[self.publish_form.cursor_line].len() =>
            {
                let line = &mut self.publish_form.payload_lines[self.publish_form.cursor_line];
                line.remove(self.publish_form.cursor_col);
            }
            _ => {}
        }
    }

    pub fn move_cursor_left(&mut self) {
        match self.focus {
            Focus::Host if self.host_cursor > 0 => self.host_cursor -= 1,
            Focus::Port if self.port_cursor > 0 => self.port_cursor -= 1,
            Focus::ClientId if self.client_id_cursor > 0 => self.client_id_cursor -= 1,
            Focus::Username if self.username_cursor > 0 => self.username_cursor -= 1,
            Focus::Password if self.password_cursor > 0 => self.password_cursor -= 1,
            Focus::SubscribeTopic if self.new_sub_topic_cursor > 0 => {
                self.new_sub_topic_cursor -= 1
            }
            Focus::PublishTopic if self.publish_topic_cursor > 0 => self.publish_topic_cursor -= 1,
            Focus::PublishPayload if self.publish_form.cursor_col > 0 => {
                self.publish_form.cursor_col -= 1;
            }
            _ => {}
        }
    }

    pub fn move_cursor_right(&mut self) {
        match self.focus {
            Focus::Host => {
                self.host_cursor =
                    (self.host_cursor + 1).min(self.connection_config.host.len());
            }
            Focus::Port => {
                let len = self.connection_config.port.to_string().len();
                self.port_cursor = (self.port_cursor + 1).min(len);
            }
            Focus::ClientId => {
                self.client_id_cursor = (self.client_id_cursor + 1)
                    .min(self.connection_config.client_id.len());
            }
            Focus::Username => {
                self.username_cursor = (self.username_cursor + 1)
                    .min(self.connection_config.username.len());
            }
            Focus::Password => {
                self.password_cursor = (self.password_cursor + 1)
                    .min(self.connection_config.password.len());
            }
            Focus::SubscribeTopic => {
                self.new_sub_topic_cursor =
                    (self.new_sub_topic_cursor + 1).min(self.new_sub_topic.len());
            }
            Focus::PublishTopic => {
                self.publish_topic_cursor =
                    (self.publish_topic_cursor + 1).min(self.publish_form.topic.len());
            }
            Focus::PublishPayload => {
                let max_col =
                    self.publish_form.payload_lines[self.publish_form.cursor_line].len();
                self.publish_form.cursor_col = (self.publish_form.cursor_col + 1).min(max_col);
            }
            _ => {}
        }
    }

    pub fn move_cursor_up(&mut self) {
        if self.focus == Focus::PublishPayload && self.publish_form.cursor_line > 0 {
            self.publish_form.cursor_line -= 1;
            let line_len = self.publish_form.payload_lines[self.publish_form.cursor_line].len();
            self.publish_form.cursor_col = self.publish_form.cursor_col.min(line_len);
        }
    }

    pub fn move_cursor_down(&mut self) {
        if self.focus == Focus::PublishPayload
            && self.publish_form.cursor_line + 1 < self.publish_form.payload_lines.len()
        {
            self.publish_form.cursor_line += 1;
            let line_len = self.publish_form.payload_lines[self.publish_form.cursor_line].len();
            self.publish_form.cursor_col = self.publish_form.cursor_col.min(line_len);
        }
    }

    pub fn insert_newline(&mut self) {
        if self.focus == Focus::PublishPayload {
            let current_line = &self.publish_form.payload_lines[self.publish_form.cursor_line];
            let rest = current_line[self.publish_form.cursor_col..].to_string();
            self.publish_form.payload_lines[self.publish_form.cursor_line]
                .truncate(self.publish_form.cursor_col);
            self.publish_form
                .payload_lines
                .insert(self.publish_form.cursor_line + 1, rest);
            self.publish_form.cursor_line += 1;
            self.publish_form.cursor_col = 0;
        }
    }

    pub fn move_to_home(&mut self) {
        match self.focus {
            Focus::Host => self.host_cursor = 0,
            Focus::Port => self.port_cursor = 0,
            Focus::ClientId => self.client_id_cursor = 0,
            Focus::Username => self.username_cursor = 0,
            Focus::Password => self.password_cursor = 0,
            Focus::SubscribeTopic => self.new_sub_topic_cursor = 0,
            Focus::PublishTopic => self.publish_topic_cursor = 0,
            Focus::PublishPayload => self.publish_form.cursor_col = 0,
            _ => {}
        }
    }

    pub fn move_to_end(&mut self) {
        match self.focus {
            Focus::Host => self.host_cursor = self.connection_config.host.len(),
            Focus::Port => self.port_cursor = self.connection_config.port.to_string().len(),
            Focus::ClientId => self.client_id_cursor = self.connection_config.client_id.len(),
            Focus::Username => self.username_cursor = self.connection_config.username.len(),
            Focus::Password => self.password_cursor = self.connection_config.password.len(),
            Focus::SubscribeTopic => self.new_sub_topic_cursor = self.new_sub_topic.len(),
            Focus::PublishTopic => {
                self.publish_topic_cursor = self.publish_form.topic.len();
            }
            Focus::PublishPayload => {
                self.publish_form.cursor_col =
                    self.publish_form.payload_lines[self.publish_form.cursor_line].len();
            }
            _ => {}
        }
    }

    pub fn next_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Host => Focus::Port,
            Focus::Port => Focus::ClientId,
            Focus::ClientId => Focus::Username,
            Focus::Username => Focus::Password,
            Focus::Password => Focus::SubscribeTopic,
            Focus::SubscribeTopic => Focus::SubscribeQos,
            Focus::SubscribeQos => Focus::PublishTopic,
            Focus::PublishTopic => Focus::PublishQos,
            Focus::PublishQos => Focus::PublishPayload,
            Focus::PublishPayload => Focus::Host,
        };
    }

    pub fn prev_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Host => Focus::PublishPayload,
            Focus::Port => Focus::Host,
            Focus::ClientId => Focus::Port,
            Focus::Username => Focus::ClientId,
            Focus::Password => Focus::Username,
            Focus::SubscribeTopic => Focus::Password,
            Focus::SubscribeQos => Focus::SubscribeTopic,
            Focus::PublishTopic => Focus::SubscribeQos,
            Focus::PublishQos => Focus::PublishTopic,
            Focus::PublishPayload => Focus::PublishQos,
        };
    }

    pub fn cycle_qos_sub(&mut self) {
        self.new_sub_qos = (self.new_sub_qos + 1) % 3;
    }

    pub fn cycle_qos_publish(&mut self) {
        self.publish_qos_idx = (self.publish_qos_idx + 1) % 3;
        self.publish_form.qos = self.publish_qos_idx as u8;
    }
}
