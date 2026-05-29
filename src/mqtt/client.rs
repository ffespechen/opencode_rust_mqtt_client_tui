use crate::app::ConnectionConfig;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, Transport};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Debug, Clone)]
pub enum MqttCmd {
    Connect(ConnectionConfig),
    Disconnect,
    Subscribe {
        topic: String,
        qos: u8,
    },
    Unsubscribe {
        topic: String,
    },
    Publish {
        topic: String,
        qos: u8,
        retain: bool,
        payload: Vec<u8>,
    },
}

#[derive(Debug, Clone)]
pub enum MqttEvent {
    Connected,
    Disconnected,
    MessageReceived {
        topic: String,
        payload: Vec<u8>,
        retain: bool,
    },
    Error(String),
}

pub async fn run(mut cmd_rx: UnboundedReceiver<MqttCmd>, event_tx: UnboundedSender<MqttEvent>) {
    let mut client: Option<AsyncClient> = None;
    let mut eventloop_handle: Option<tokio::task::JoinHandle<()>> = None;

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            MqttCmd::Connect(config) => {
                if eventloop_handle.is_some() {
                    drop(client.take());
                    drop(eventloop_handle.take());
                }

                let (c, mut ev) = build_client_options(&config)
                    .map(|opts| AsyncClient::new(opts, 100))
                    .unwrap_or_else(|| AsyncClient::new(default_options(), 100));

                let tx = event_tx.clone();
                let handle = tokio::spawn(async move {
                    loop {
                        match ev.poll().await {
                            Ok(Event::Incoming(Incoming::Publish(p))) => {
                                let _ = tx.send(MqttEvent::MessageReceived {
                                    topic: p.topic,
                                    payload: p.payload.to_vec(),
                                    retain: p.retain,
                                });
                            }
                            Ok(Event::Incoming(Incoming::ConnAck(_))) => {
                                let _ = tx.send(MqttEvent::Connected);
                            }
                            Err(e) => {
                                let _ = tx.send(MqttEvent::Error(e.to_string()));
                                break;
                            }
                            _ => {}
                        }
                    }
                    let _ = tx.send(MqttEvent::Disconnected);
                });

                eventloop_handle = Some(handle);
                client = Some(c);
            }
            MqttCmd::Disconnect => {
                client = None;
                eventloop_handle = None;
                let _ = event_tx.send(MqttEvent::Disconnected);
            }
            MqttCmd::Subscribe { topic, qos } => {
                if let Some(c) = &client {
                    let qos = u8_to_qos(qos);
                    if let Err(e) = c.subscribe(&topic, qos).await {
                        let _ = event_tx.send(MqttEvent::Error(format!(
                            "Subscribe failed for '{}': {}",
                            topic, e
                        )));
                    }
                }
            }
            MqttCmd::Unsubscribe { topic } => {
                if let Some(c) = &client {
                    if let Err(e) = c.unsubscribe(&topic).await {
                        let _ = event_tx.send(MqttEvent::Error(format!(
                            "Unsubscribe failed for '{}': {}",
                            topic, e
                        )));
                    }
                }
            }
            MqttCmd::Publish {
                topic,
                qos,
                retain,
                payload,
            } => {
                if let Some(c) = &client {
                    let qos = u8_to_qos(qos);
                    if let Err(e) = c.publish(&topic, qos, retain, payload).await {
                        let _ = event_tx.send(MqttEvent::Error(format!(
                            "Publish failed to '{}': {}",
                            topic, e
                        )));
                    }
                }
            }
        }
    }

    drop(client);
    drop(eventloop_handle);
}

fn build_client_options(config: &ConnectionConfig) -> Option<MqttOptions> {
    let host = if config.host.is_empty() {
        return None;
    } else {
        &config.host
    };

    let port = if config.port == 0 {
        if config.use_websockets {
            8083
        } else {
            1883
        }
    } else {
        config.port
    };

    let client_id = if config.client_id.is_empty() {
        format!(
            "mqtt_tui_{}",
            uuid::Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("unknown")
        )
    } else {
        config.client_id.clone()
    };

    let mut options = MqttOptions::new(&client_id, host, port);
    options.set_keep_alive(std::time::Duration::from_secs(config.keep_alive as u64));
    options.set_clean_session(config.clean_session);

    if !config.username.is_empty() {
        options.set_credentials(&config.username, &config.password);
    }

    if config.use_websockets {
        options.set_transport(Transport::Ws);
    }

    Some(options)
}

fn u8_to_qos(qos: u8) -> rumqttc::QoS {
    match qos {
        0 => rumqttc::QoS::AtMostOnce,
        1 => rumqttc::QoS::AtLeastOnce,
        _ => rumqttc::QoS::ExactlyOnce,
    }
}

fn default_options() -> MqttOptions {
    let id = format!(
        "mqtt_tui_{}",
        uuid::Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("unknown")
    );
    MqttOptions::new(&id, "localhost", 1883)
}
