# ARCHITECTURE.md — Rust MQTT Client TUI

Guia de referencia arquitectonica para el cliente MQTT TUI. Documenta cada decision de diseño,
patron, y la estructura de modulos con el proposito de servir como material de aprendizaje sobre
como construir aplicaciones TUI en Rust con concurrencia, canales de comunicacion, y separacion
estricta de responsabilidades.

---

## Tabla de Contenidos

1. [Vision General](#1-visi%C3%B3n-general)
2. [Arquitectura de Alto Nivel](#2-arquitectura-de-alto-nivel)
3. [Flujo de Datos](#3-flujo-de-datos)
4. [Modelo de Concurrencia](#4-modelo-de-concurrencia)
5. [Gestion de Estado](#5-gesti%C3%B3n-de-estado)
6. [Sistema de Eventos y Acciones](#6-sistema-de-eventos-y-acciones)
7. [Composicion de la UI](#7-composici%C3%B3n-de-la-ui)
8. [Integracion MQTT](#8-integraci%C3%B3n-mqtt)
9. [Manejo de Errores](#9-manejo-de-errores)
10. [Decisiones de Disegno y Trade-offs](#10-decisiones-de-dise%C3%B1o-y-trade-offs)
11. [Estructura de Archivos](#11-estructura-de-archivos)
12. [Dependencias Clave](#12-dependencias-clave)
13. [Build y Despliegue (Docker)](#13-build-y-despliegue-docker)
14. [Lecciones Aprendidas y Patrones Reutilizables](#14-lecciones-aprendidas-y-patrones-reutilizables)

---

## 1. Vision General

**rust_mqtt_client_tui** es una aplicacion TUI (Terminal User Interface) escrita en Rust que
implementa un cliente interactivo para el protocolo MQTT 3.1.1. Permite conectarse a brokers
MQTT via TCP o WebSocket, suscribirse a topicos, recibir mensajes en tiempo real y publicar
mensajes en formato texto plano o JSON con niveles de QoS configurables.

### Objetivos de disegno

1. **Separacion estricta** entre logica de negocio (`app`), renderizado (`ui`), manejo de
   entrada (`events`) y comunicacion de red (`mqtt`).
2. **Single-threaded render loop** con Ratatui, delegando operaciones de I/O a tareas
   asincronas de Tokio.
3. **Comunicacion por canales** entre la UI y el subsistema MQTT, evitando estados
   compartidos con Mutex/Arc.
4. **Cero `unwrap()` en produccion** — propagacion de errores con `anyhow::Result` y `?`.
5. **Cumplimiento de Clippy** con `-D warnings` — codigo limpio sin avisos.

---

## 2. Arquitectura de Alto Nivel

El proyecto sigue una **arquitectura en capas** con cuatro modulos de dominio mas el punto de
entrada:

```
┌─────────────────────────────────────────────────────┐
│                     main.rs                         │
│  ┌───────────┐  ┌───────────┐  ┌────────────────┐  │
│  │ Terminal  │  │ Tokio RT  │  │ Channel Setup  │  │
│  │ (Crossterm│  │  spawn()  │  │ unbounded_pair │  │
│  │  Ratatui) │  │           │  │                │  │
│  └───────────┘  └───────────┘  └────────────────┘  │
│                                                     │
│  ┌─────────────────────────────────────────────┐    │
│  │              Event Loop                      │    │
│  │  poll(input) → Action → update(state)       │    │
│  │  try_recv(MqttEvent) → update(state)        │    │
│  │  draw(Frame, &state)                        │    │
│  └─────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────┘
          │                │              │
          ▼                ▼              ▼
   ┌──────────┐   ┌──────────────┐  ┌──────────┐
   │ events/  │   │    app/      │  │   ui/    │
   │ handler  │   │   state      │  │renderizar│
   │ Key→Act  │   │ logica pura  │  │solo lect │
   └──────────┘   └──────────────┘  └──────────┘
                         │
                         │ MqttCmd (UnboundedSender)
                         ▼
                  ┌──────────────┐
                  │   mqtt/      │
                  │ Tokio task   │
                  │ EventLoop    │←→ Red (rumqttc)
                  └──────────────┘
                         │
                         │ MqttEvent (UnboundedSender)
                         ▼
                  ┌──────────────┐
                  │ Event Loop   │
                  │ try_recv()   │
                  └──────────────┘
```

### Principio fundamental: UI no muta estado

El modulo `ui/` recibe `&App` (referencia inmutable) y jamas modifica el estado. El estado
solo cambia en dos lugares:

1. **`handle_action(app, action)`** en `main.rs:89` — cambios iniciados por el usuario.
2. **`app.handle_mqtt_event(event)`** en `main.rs:78` — cambios iniciados por la red.

Esta separacion evita condiciones de carrera, facilita el razonamiento sobre el codigo, y
permite testear la UI con snapshots predecibles.

---

## 3. Flujo de Datos

### 3.1 Ciclo de vida de una pulsacion de tecla

```
Usuario presiona ^C
        │
        ▼
crossterm::event::poll()   →   Event::Key(key)
        │
        ▼
events::handler::key_to_action(key, &app)
        │  Context-aware: Tab en SubscribeQos → CycleQosSub
        │                 Tab en Host → NextFocus
        ▼
Action::Connect
        │
        ▼
handle_action(app, action)   en main.rs:89
        │  app.connect()
        │  app.cmd_tx.send(MqttCmd::Connect(config))
        ▼
mqtt::client::run()         tarea Tokio
        │  recibe MqttCmd::Connect
        │  crea AsyncClient + EventLoop
        │  spawn tarea para poll del EventLoop
        ▼
EventLoop devuelve Incoming::ConnAck
        │
        ▼
event_tx.send(MqttEvent::Connected)
        │
        ▼
Event Loop (main thread)    main.rs:77
        │  try_recv() → MqttEvent::Connected
        │  app.handle_mqtt_event(event)
        │  state.connection_state = Connected
        ▼
terminal.draw(|f| ui::render(f, &app))
        │  top_bar muestra "[Connected]"
        │  help muestra contexto actualizado
```

### 3.2 Ciclo de vida de un mensaje entrante

```
Red MQTT → rumqttc EventLoop
        │
        ▼
mqtt::client::run() — spawned task
        │  event_tx.send(MqttEvent::MessageReceived { topic, payload, retain })
        ▼
main event loop — try_recv()
        │  app.handle_mqtt_event(event)
        │  busqueda de Subscription por topic
        │  push a VecDeque<MqttMessage> con timestamp Local::now()
        ▼
ui::subscriptions::render_messages()
        │  iter().rev() sobre mensajes, aplica scroll_offset
        │  renderiza con color del topic, timestamp, flag [R]
```

### 3.3 Ciclo de vida de una publicacion

```
Usuario llena PublishForm y presiona ^P
        │
        ▼
Action::Publish → app.publish()
        │  valida topic no vacio
        │  valida JSON si is_json == true
        │  valida ConnectionState::Connected
        │  cmd_tx.send(MqttCmd::Publish { ... })
        ▼
mqtt::client::run() — recibe Publish
        │  u8_to_qos(qos) → rumqttc::QoS
        │  client.publish(topic, qos, retain, payload).await
        ▼
Red MQTT → broker
```

---

## 4. Modelo de Concurrencia

### 4.1 Por que dos canales?

Elegimos **dos canales unidireccionales** en lugar de un solo canal bidireccional o un
`Arc<Mutex<App>>` compartido:

```
main thread (sync)              mqtt task (async)
     │                                │
     │── cmd_tx ──────────────────→ cmd_rx
     │   (MqttCmd)                    │
     │                                │
     │←─ event_tx ───────────────── event_tx.clone()
     │   (MqttEvent)                  │
```

**Ventajas de este disegno:**

- **Sin locks:** El main thread tiene ownership exclusivo de `App`. La tarea MQTT tiene
  ownership exclusivo del `AsyncClient` y `EventLoop`. No hay estructuras compartidas que
  requieran sincronizacion.
- **Backpressure implicito:** `unbounded_channel` nunca bloquea al emisor. Para una TUI
  con volumen de mensajes moderado, esto es aceptable. Si se necesitara backpressure, se
  puede migrar a `mpsc::channel(capacity)`.
- **Tipado fuerte:** `MqttCmd` y `MqttEvent` son enums con variantes especificas. El
  compilador garantiza que todos los casos estan cubiertos en los `match`.

### 4.2 El EventLoop de MQTT como tarea separada

`rumqttc::AsyncClient::new()` devuelve una tupla `(AsyncClient, EventLoop)`. El `AsyncClient`
implementa `Clone + Send + Sync` y se usa para publicar/suscribir. El `EventLoop` **debe ser
polleado continuamente** para mantener la conexion y recibir mensajes.

En lugar de integrar el EventLoop en el `tokio::select!` del comando (lo que causaria
conflictos de borrow con `&mut self`), **spawneamos una tarea dedicada** que solo pollea el
EventLoop:

```rust
// src/mqtt/client.rs:55
let handle = tokio::spawn(async move {
    loop {
        match ev.poll().await {
            Ok(Event::Incoming(Incoming::Publish(p))) => {
                let _ = tx.send(MqttEvent::MessageReceived { ... });
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
```

**Consecuencia:** La tarea de comandos (`mqtt::run`) solo se encarga de recibir `MqttCmd` y
ejecutar operaciones sobre el `AsyncClient`. La tarea spawneada solo se encarga de pollear el
EventLoop y enviar eventos. Esta separacion evita el problema de borrow mutuo en `select!`.

### 4.3 Desconexion limpia

Para desconectar, se dropean el `AsyncClient` y el `JoinHandle`:

```rust
// src/mqtt/client.rs:81-84
Some(MqttCmd::Disconnect) => {
    client = None;             // Dropea AsyncClient → cierra conexion TCP/WS
    eventloop_handle = None;   // Dropea JoinHandle (la tarea ya habra salido o saldra pronto)
    let _ = event_tx.send(MqttEvent::Disconnected);
}
```

Al dropear `client`, la conexion se cierra. El `EventLoop::poll()` en la tarea spawneada
devolvera `Err(...)`, rompiendo el loop y enviando un `MqttEvent::Disconnected` adicional.
La UI maneja eventos `Disconnected` duplicados sin problemas (idempotencia).

### 4.4 El bucle principal es sincrono

El `main thread` ejecuta un loop sincrono que:

1. **Pollea input** con `event::poll(Duration::from_millis(16))` — aproximadamente 60 FPS.
2. **Drena eventos MQTT** con `while let Ok(event) = mqtt_event_rx.try_recv()` — no bloqueante.
3. **Renderiza** con `terminal.draw(|f| ui::render(f, app))`.

Este patron es el recomendado por la documentacion de Ratatui: el render loop es
single-threaded, y las operaciones de I/O se delegan a tareas asincronas que se comunican
mediante canales.

### 4.5 Reset del cursor y Tick

El cursor parpadea cada 30 frames (~0.5s a 60 FPS) usando un contador wrapping:

```rust
// src/app/state.rs:163
pub fn tick(&mut self) {
    self.blink_tick = self.blink_tick.wrapping_add(1);
    if self.blink_tick.is_multiple_of(30) {
        self.cursor_visible = !self.cursor_visible;
    }
}
```

`wrapping_add` evita overflow sin panic. `is_multiple_of(30)` es mas legible y eficiente
que `% 30 == 0`.

---

## 5. Gestion de Estado

### 5.1 El struct `App`

`App` (`src/app/state.rs:86`) es el **unico owner del estado** de la aplicacion. Centraliza:

| Categoria         | Campos                                                           |
|-------------------|------------------------------------------------------------------|
| Conexion          | `connection_config`, `connection_state`                          |
| Subscripciones    | `subscriptions: Vec<Subscription>`, `focused_subscription`       |
| Publicacion       | `publish_form: PublishForm`                                      |
| Navegacion        | `focus: Focus`                                                   |
| UI auxiliar       | `cursor_visible`, `blink_tick`, `status_message`, `should_quit` |
| Canales           | `cmd_tx: Option<UnboundedSender<MqttCmd>>`                       |
| Cursores de texto | `host_cursor`, `port_cursor`, ..., `publish_topic_cursor`        |

### 5.2 Estados de conexion

```rust
enum ConnectionState { Disconnected, Connecting, Connected }
```

La UI reacciona a estos estados en tiempo real:

- `Disconnected` → boton "^C:Connect" en verde, status en rojo.
- `Connecting` → status muestra "Connecting to host:port...".
- `Connected` → indicador "[Connected]" en verde, se habilitan Subscribe/Publish.

### 5.3 Sistema de Foco

El `enum Focus` (`src/app/state.rs:34`) define los 10 campos navegables. Es `Copy + Clone +
PartialEq`, lo que permite comparaciones directas y paso por valor.

**Orden de tabulacion lineal:**

```
Host → Port → ClientId → Username → Password →
SubscribeTopic → SubscribeQos → PublishTopic → PublishQos → PublishPayload → (ciclo a Host)
```

Cada campo de texto tiene su propio cursor independiente (ej: `host_cursor: usize`). Esto
permite preservar la posicion del cursor al cambiar de foco y volver.

### 5.4 Gestion de mensajes por topico

Cada `Subscription` contiene un `VecDeque<MqttMessage>` con capacidad maxima configurable:

```rust
pub const MAX_MESSAGES_PER_TOPIC: usize = 500;
```

Se usa `VecDeque` porque insertamos al final (`push_back`) y removemos del frente
(`pop_front`) cuando se excede el limite — operaciones O(1) en ambas direcciones.

### 5.5 Colores de topicos

12 colores definidos en `TOPIC_COLORS` y asignados con round-robin:

```rust
let color = TOPIC_COLORS[self.subscriptions.len() % TOPIC_COLORS.len()];
```

Los topicos mantienen su color aun si otros son eliminados (no se reasignan), garantizando
consistencia visual durante la sesion.

---

## 6. Sistema de Eventos y Acciones

### 6.1 El patron Action

Inspirado en Elm/Redux, pero sin reducers inmutables. El flujo es:

```
KeyEvent → Action → App.mutacion
```

El enum `Action` (`src/events/handler.rs:5`) tiene 34 variantes que cubren todas las
operaciones posibles. La traduccion de teclas a acciones es **pura y testeable**: recibe
`KeyEvent` y `&App` (para contexto) y devuelve `Option<Action>`.

### 6.2 Despacho context-aware

Algunas teclas cambian de significado segun el foco activo:

```rust
// Tab: ciclo de QoS en campos QoS, navegacion en el resto
(KeyModifiers::NONE, KeyCode::Tab) => match app.focus {
    Focus::SubscribeQos => Some(Action::CycleQosSub),
    Focus::PublishQos   => Some(Action::CycleQosPublish),
    _                   => Some(Action::NextFocus),
},

// Up/Down: navegar payload en editor, topicos en la lista
(KeyModifiers::NONE, KeyCode::Up) => match app.focus {
    Focus::PublishPayload => Some(Action::CursorUp),
    _                     => Some(Action::TopicUp),
},
```

Esto crea una experiencia de usuario coherente: Tab siempre "activa" el elemento actual. Si
el elemento es un selector de QoS, Tab lo cicla. Si es un campo de texto, Tab avanza al
siguiente campo.

### 6.3 La funcion handle_action

`handle_action(app, action)` en `main.rs:89` es un `match` exhaustivo que traduce cada
`Action` en mutaciones concretas del estado. Es el **unico punto del codigo donde `main.rs`
muta `App`**. Ninguna accion realiza I/O directamente; si se necesita comunicacion MQTT,
se envia un `MqttCmd` por el canal.

---

## 7. Composicion de la UI

### 7.1 Estructura de Layout

```
┌─────────────────────────────────────────────┐
│  top_bar: Actions (Ctrl+letter shortcuts)   │  Constraint::Length(3)
├──────────────────────┬──────────────────────┤
│  subscriptions       │  publish             │  Constraint::Min(1)
│  ┌────────────────┐  │  ┌────────────────┐  │  50% / 50%
│  │ Subscribe form │  │  │ Topic field    │  │
│  │ Topic list     │  │  │ QoS selector   │  │
│  │ Messages       │  │  │ Options row    │  │
│  │                │  │  │ Publish btn    │  │
│  │                │  │  │ Payload editor │  │
│  └────────────────┘  │  └────────────────┘  │
├──────────────────────┴──────────────────────┤
│  help: Contextual hints                     │  Constraint::Length(3)
└─────────────────────────────────────────────┘
```

Definido en `src/ui/mod.rs:12`. Ratatui usa `Layout::split()` para dividir el area en chunks
rectangulares. Las constraints:

- `Length(n)` — tamagno fijo en lineas.
- `Min(n)` — ocupa el espacio restante.
- `Percentage(n)` — porcentaje del area disponible.

### 7.2 Funciones de renderizado

Cada panel de la UI recibe `(frame: &mut Frame, app: &App, area: Rect)`:

```rust
// src/ui/mod.rs:22
top_bar::render(frame, app, main_chunks[0]);
subscriptions::render(frame, app, content[0]);
publish::render(frame, app, content[1]);
help::render(frame, app, main_chunks[2]);
```

**Regla arquitectonica:** Ninguna funcion en `ui/` muta `app`. Solo leen `&App` y escriben
en `&mut Frame`. Esto hace que el renderizado sea determinista: dado el mismo `&App`, la
misma salida visual.

### 7.3 Resaltado condicional

El `top_bar` usa una funcion `highlight_if(active, focused, color)` (`src/ui/top_bar.rs:78`):

```rust
fn highlight_if(active: bool, focused: bool, color: Color) -> Style {
    if active {
        Style::default().fg(Color::Black).bg(color).add_modifier(Modifier::BOLD)
    } else if focused {
        Style::default().fg(color).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(Color::White)
    }
}
```

- **active** (verde con fondo) cuando la accion esta en curso (Connect ya conectado).
- **focused** (color con underline) cuando el campo relacionado tiene el foco.
- **default** (blanco) en caso contrario.

### 7.4 Renderizado del cursor

Ratatui no tiene widgets de entrada de texto nativos. Implementamos cursores manualmente con
`Span`s:

```rust
// En render_payload (src/ui/publish.rs:114)
if focused && i == app.publish_form.cursor_line {
    let before = &payload_line[..cursor_col];
    let cursor = if app.cursor_visible { "▌" } else { " " };
    let after = &payload_line[cursor_col..];
    // Se renderiza como tres Spans concatenados
}
```

El cursor parpadeante se controla con `app.cursor_visible`, que alterna cada ~0.5s via
`app.tick()`.

### 7.5 Ayuda contextual

`src/ui/help.rs` muestra un mensaje diferente segun `app.focus`:

```rust
Focus::SubscribeTopic => "Subscribe topic: e.g. sensors/# | ^S: Subscribe | Tab: cycle QoS",
Focus::PublishPayload => "Payload: Enter for newline | ^J: JSON/Text | ^R: Retain | ^P: Publish",
```

Esto guia al usuario sobre que teclas estan disponibles en cada contexto, reduciendo la
necesidad de documentacion externa.

---

## 8. Integracion MQTT

### 8.1 Dependencia: rumqttc 0.24

`rumqttc` es una implementacion pura en Rust del protocolo MQTT. Soporta:

- **MQTT 3.1.1** (el estandar mas compatible).
- **Transporte TCP** nativo.
- **Transporte WebSocket** via feature flag `websocket` (agrega `async-tungstenite`).
- **TLS** via `rustls` (feature por defecto `use-rustls`).

En `Cargo.toml`:

```toml
rumqttc = { version = "0.24", features = ["websocket"] }
```

### 8.2 Construccion de MqttOptions

`build_client_options()` (`src/mqtt/client.rs:135`) convierte `ConnectionConfig` en las
opciones que rumqttc necesita:

```rust
let mut options = MqttOptions::new(&client_id, host, port);
options.set_keep_alive(Duration::from_secs(config.keep_alive as u64));
options.set_clean_session(config.clean_session);
if !config.username.is_empty() {
    options.set_credentials(&config.username, &config.password);
}
if config.use_websockets {
    options.set_transport(Transport::Ws);
}
```

Puntos clave:

- Si `host` esta vacio, no se intenta conexion (retorna `None`, se usa fallback a localhost).
- Si `port` es 0, se usa default segun transporte (1883 TCP, 8083 WS).
- Si `client_id` esta vacio, se genera uno aleatorio con `uuid::Uuid::new_v4()`.
- `Transport::Ws` requiere el feature `websocket` activado en rumqttc.

### 8.3 Conversion de QoS

rumqttc define su propio enum `QoS` (AtMostOnce, AtLeastOnce, ExactlyOnce). La UI maneja
QoS como `u8` (0, 1, 2) por simplicidad. La conversion se hace en el boundary de MQTT:

```rust
fn u8_to_qos(qos: u8) -> rumqttc::QoS {
    match qos {
        0 => rumqttc::QoS::AtMostOnce,
        1 => rumqttc::QoS::AtLeastOnce,
        _ => rumqttc::QoS::ExactlyOnce,
    }
}
```

### 8.4 Capacidad del canal interno de rumqttc

`AsyncClient::new(options, 100)` — el segundo argumento (100) es la capacidad del canal
interno entre el cliente y el EventLoop. 100 es suficiente para una TUI interactiva; valores
mayores consumirian mas memoria sin beneficio.

---

## 9. Manejo de Errores

### 9.1 Estrategia general

| Contexto          | Estrategia                                                 |
|-------------------|------------------------------------------------------------|
| `main()`          | `anyhow::Result<()>` — errores fatales terminan el proceso |
| Conexion MQTT     | `MqttEvent::Error(String)` — errores no fatales notificados al usuario |
| Validacion de UI  | `status_message` actualizado con mensaje descriptivo       |
| Operaciones async | `if let Err(e) = client.subscribe(...)` — log a `event_tx` |
| Parsing de puerto | `.parse().unwrap_or(prev_value)` — fallback seguro         |

### 9.2 El canal como mecanismo de error

Los errores en la tarea MQTT (conexion fallida, subscribe rechazado, etc.) no se propagan
con `?` hacia el main thread. En su lugar, se empaquetan en `MqttEvent::Error(String)` y se
envian por el canal. La UI los muestra en `status_message` y transiciona a `Disconnected`.

### 9.3 Validacion pre-publicacion

Antes de enviar un `MqttCmd::Publish`, `app.publish()` valida:

1. Topic no vacio.
2. Si `is_json`, el payload debe parsear como JSON valido (`serde_json::from_str`).
3. Estado de conexion `Connected`.

Esto evita enviar comandos invalidos al broker y proporciona feedback inmediato al usuario.

### 9.4 Cero unwrap() en produccion

El unico `.unwrap_or()` en el codigo esta en `App::new()` para generar el client_id por
defecto (sobre un UUID que siempre es valido). En `backspace` para el puerto, se usa
`.parse().unwrap_or(0)` como fallback seguro.

---

## 10. Decisiones de Disegno y Trade-offs

### 10.1 ¿Por que `unbounded_channel` en vez de `mpsc::channel`?

**Trade-off:** Unbounded no aplica backpressure — si la tarea MQTT produce eventos mas
rapido de lo que el main loop los consume, la memoria crece.

**Justificacion:** En una TUI interactiva con ~60 FPS y volumen de mensajes MQTT moderado
(cientos por segundo como maximo), el riesgo de acumulacion es bajo. Si se necesitara
backpressure, se puede migrar a `mpsc::channel(1024)` con `try_send`.

### 10.2 ¿Por que no usar `Arc<Mutex<App>>`?

**Alternativa considerada:** Compartir `App` entre el main thread y la tarea MQTT con
`Arc<Mutex<App>>`. La tarea MQTT mutaria el estado directamente al recibir mensajes.

**Rechazada porque:**
- Introduce contención de locks que puede causar frames perdidos en el render.
- El main thread tendria que lockear para leer durante `draw()`, y la tarea MQTT para
  escribir al recibir mensajes — deadlock potencial.
- Rompe la separacion de responsabilidades: la tarea MQTT necesitaria conocer la estructura
  interna de `App`.

**El disegno con canales** mantiene ownership exclusivo y separacion clara.

### 10.3 ¿Por que no usar el pattern Elm/Redux completo?

**Alternativa considerada:** `App::update(&mut self, action: Action)` que devuelve efectos
secundarios, y un runtime que los ejecuta.

**Rechazada porque:**
- Agregaria complejidad innecesaria para una aplicacion de este tamagno.
- Rust no tiene un runtime de efectos como Elm — requeriria construir uno.
- El despacho directo (`handle_action`) es mas simple de entender y debuggear.

### 10.4 Cursores por campo vs. cursor unico

**Decision:** Cada campo tiene su propio `*_cursor: usize`.

**Justificacion:** Preserva la posicion del cursor al navegar entre campos con Tab/Shift+Tab.
La alternativa (un solo cursor que se resetea) seria frustrante para el usuario.

**Costo:** 10 campos `usize` en `App` (~80 bytes en 64-bit). Despreciable.

### 10.5 VecDeque para mensajes

**Alternativa considerada:** `Vec<MqttMessage>` con `remove(0)` al exceder el limite.

**Rechazada porque:** `remove(0)` es O(n) — desplaza todos los elementos. `VecDeque` permite
`pop_front()` O(1). Para buffers de mensajes donde siempre insertamos al final y removemos
del principio, `VecDeque` es la estructura de datos correcta.

### 10.6 No TLS en esta version

**Decision:** No se implemento TLS/WSS en la version inicial.

**Justificacion:**
- Anadiria complejidad de configuracion (certificados, CA bundles).
- `Transport::Tls` requiere `TlsConfiguration` que varia entre plataformas.
- El usuario puede usar un proxy TLS (ej: nginx, HAProxy) frente al broker MQTT.
- Se puede agregar en una version futura sin cambios arquitectonicos (solo en
  `build_client_options`).

---

## 11. Estructura de Archivos

```
rust_mqtt_client_tui/
├── Cargo.toml                 # Manifiesto con 9 dependencias
├── Dockerfile                 # Build multi-stage (rust → debian-slim)
├── .dockerignore              # Excluye target/, .git/, docs
├── README.md                  # Documentacion de usuario
├── ARCHITECTURE.md            # Este documento
└── src/
    ├── main.rs                # (158 lineas) Entry point, event loop, handle_action
    ├── app/
    │   ├── mod.rs             # Re-exporta state::*
    │   └── state.rs           # (642 lineas) App, ConnectionConfig, Subscription,
    │                          #   PublishForm, MqttMessage, Focus, toda la logica
    ├── events/
    │   ├── mod.rs             # Re-exporta handler::*
    │   └── handler.rs         # (74 lineas) KeyEvent → Action, context-aware
    ├── ui/
    │   ├── mod.rs             # (33 lineas) Layout maestro (vertical/horizontal splits)
    │   ├── top_bar.rs         # (91 lineas) Barra de acciones con shortcuts
    │   ├── subscriptions.rs   # (169 lineas) Panel izquierdo: form, topics, mensajes
    │   ├── publish.rs         # (168 lineas) Panel derecho: form de publicacion
    │   └── help.rs            # (32 lineas) Barra inferior de ayuda contextual
    └── mqtt/
        ├── mod.rs             # Re-exporta client::*
        └── client.rs          # (198 lineas) MqttCmd, MqttEvent, run(), build_options
```

### Metricas

| Metrica               | Valor   |
|-----------------------|---------|
| Total de archivos     | 17      |
| Lineas de codigo Rust | ~1,500  |
| Dependencias directas | 9       |
| Dependencias totales  | ~150    |
| Tiempo de build (rel) | ~3.5 min (primera vez) |
| Binario final (rel)   | ~7 MB   |

---

## 12. Dependencias Clave

| Crate       | Version | Proposito                                             |
|-------------|---------|-------------------------------------------------------|
| `ratatui`   | 0.28    | Framework TUI: widgets, layout, styling               |
| `crossterm` | 0.28    | Terminal backend: raw mode, eventos, alternate screen |
| `rumqttc`   | 0.24    | Cliente MQTT 3.1.1 async (AsyncClient + EventLoop)    |
| `tokio`     | 1.x     | Runtime asyncrono, canales MPSC, spawn                |
| `serde_json`| 1.x     | Validacion de payloads JSON antes de publicar         |
| `chrono`    | 0.4     | Timestamps para mensajes recibidos                    |
| `uuid`      | 1.x     | Generacion de client_id aleatorio                     |
| `anyhow`    | 1.x     | Manejo flexible de errores en el binary crate         |

---

## 13. Build y Despliegue (Docker)

### 13.1 Build multi-stage (version final)

```dockerfile
FROM rust:slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 1. Copiar solo el manifiesto de dependencias
COPY Cargo.toml Cargo.lock* ./

# 2. Crear estructura dummy que importa TODAS las dependencias
RUN mkdir -p src/app src/events src/ui src/mqtt \
    && echo "" > src/app/mod.rs \
    && echo "" > src/events/mod.rs \
    && echo "" > src/ui/mod.rs \
    && echo "" > src/mqtt/mod.rs \
    && printf '#![allow(unused_imports)]\nmod app {} mod events {} mod ui {} mod mqtt {}\nuse ratatui::style::Color;\nuse crossterm::event::KeyEvent;\nuse rumqttc::AsyncClient;\nuse tokio::sync::mpsc::unbounded_channel;\nuse serde_json;\nuse anyhow::Result;\nuse chrono::Local;\nuse uuid::Uuid;\nfn main() {}\n' > src/main.rs

# 3. Compilar dependencias (cache Docker efectivo)
RUN cargo build --release

# 4. Copiar codigo real y compilar aplicacion
COPY src/ src/
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

ENV TERM=xterm-256color

EXPOSE 1883/tcp
EXPOSE 9001/tcp

COPY --from=builder /app/target/release/rust_mqtt_client_tui /usr/local/bin/rust-mqtt-client-tui

ENTRYPOINT ["rust-mqtt-client-tui"]
```

### 13.6 El problema del TTY y la solucion con `script`

Las TUIs requieren un terminal real (TTY/PTY). Crossterm necesita:

1. `tcgetattr`/`tcsetattr` en stdin para activar raw mode
2. ANSI escape codes en stdout para la alternate screen y el renderizado
3. Propagacion de eventos de teclado desde stdin

En Docker, `-it` asigna un pseudo-terminal. Pero en ciertos entornos (CI, IDEs,
terminales no estandar) la asignacion puede fallar o ser insuficiente, causando
que `enable_raw_mode()` falle y la aplicacion termine.

**Solucion:** `script` (de `bsdextrautils`, ~150 KB) crea un PTY garantizado:

```
User terminal → Docker PTY (-it) → script PTY → TUI (Crossterm raw mode)
```

`script` actua como puente: asigna un nuevo PTY, ejecuta la TUI dentro de el, y
copia bidireccionalmente los datos entre el PTY interno y el externo. La TUI
siempre ve un terminal real, independientemente de como Docker configure el suyo.

El flag `-q` suprime mensajes de inicio/fin, `-c` especifica el comando, y
`/dev/null` descarta el archivo typescript (no necesario para TUIs).

Para mayor robustez, `src/main.rs` fue refactorizado para:

- Manejar errores de `terminal.draw()` como no fatales
- Usar `event::poll().unwrap_or(false)` en lugar de `?`
- Garantizar que el cleanup (`disable_raw_mode`, `LeaveAlternateScreen`)
  siempre se ejecute incluso si `run()` retorna error
- Imprimir mensajes de diagnostico claros en stderr

### 13.7 Ejecucion del contenedor

```bash
docker run -it --rm -p 1883:1883 -p 9001:9001 mqtt-tui
```

Los flags `-it` son **obligatorios**: la TUI requiere una terminal interactiva con TTY.
Sin ellos, Crossterm no puede inicializar raw mode y `event::poll()` devuelve error,
lo que causaria que la aplicacion termine inmediatamente.

Para mayor robustez, `event::poll()` en `main.rs:67` usa `.unwrap_or(false)` y
`event::read()` usa `if let Ok(...)` — si el terminal no esta disponible, el loop
simplemente no procesa eventos de teclado pero sigue renderizando y recibiendo
mensajes MQTT. Esto evita crashes en entornos sin TTY aunque la experiencia de
usuario se degradaria (sin input posible).

**Puertos expuestos:**

| Puerto | Protocolo | Proposito                           |
|--------|-----------|-------------------------------------|
| 1883   | TCP       | MQTT sin encriptar (estandar)       |
| 9001   | TCP       | MQTT WebSocket sin encriptar (Mosquitto) |

Si el broker usa otros puertos, ajustar con `-p <host>:<container>`. Alternativas
comunes: EMQX usa 8083 para WS, HiveMQ usa 8000.

---

## 14. Lecciones Aprendidas y Patrones Reutilizables

### 14.1 Patron: State-Owned TUI

```
main thread owns App ────► ui::render(&App)  (inmutable)
    ▲                          ▲
    │ handle_action()          │ try_recv()
    │                          │
 events::handler            mqtt::client (Tokio task)
 KeyEvent → Action          EventLoop → MqttEvent
```

Este patron es aplicable a **cualquier TUI en Rust** que necesite comunicarse con un
subsistema externo (red, base de datos, archivos). Las claves son:

1. **`App` es owned por el main thread** — sin `Arc<Mutex<>>`.
2. **Comunicacion por canales** — `Sender` en la tarea, `Receiver.try_recv()` en el loop.
3. **UI de solo lectura** — `&App`, nunca `&mut App` en funciones de renderizado.
4. **Acciones como intermediate representation** — desacopla teclas de mutaciones.

### 14.2 Patron: Context-Aware Key Dispatch

```rust
(KeyModifiers::NONE, KeyCode::Tab) => match app.focus {
    Focus::SubscribeQos => Some(Action::CycleQosSub),
    Focus::PublishQos   => Some(Action::CycleQosPublish),
    _                   => Some(Action::NextFocus),
},
```

En lugar de tener un mapping fijo de teclas, el handler inspecciona `app.focus` para decidir
que accion generar. Esto permite que una misma tecla tenga comportamientos distintos segun
el contexto sin complicar la UI.

### 14.3 Patron: Cursor por campo con navegacion lineal

Cada campo de entrada tiene su propio cursor. El foco define cual campo recibe input. La
navegacion es un ciclo lineal definido por `next_focus()` y `prev_focus()`. Este disegno
es mas simple que un arbol de widgets y suficiente para formularios.

### 14.4 Patron: Tarea spawneada para EventLoop con borrow problem

Cuando una API async requiere `&mut self` para un loop de polling (como `EventLoop::poll()`)
y otro componente necesita `&self` para comandos (como `AsyncClient::subscribe()`), separar
ambos en tareas distintas con canales evita el problema de borrow mutuo en `select!`.

### 14.5 Regla de Oro: Validacion temprana, publicacion tardia

La app valida inputs en la capa de UI/logica (`app.publish()`) **antes** de enviar comandos
al subsistema MQTT. Esto:
- Evita enviar datos invalidos por la red.
- Proporciona feedback inmediato (sin latencia de red).
- Mantiene la tarea MQTT simple (solo ejecuta, no valida).

---

## Referencias

- [Ratatui Documentation](https://docs.rs/ratatui/)
- [Crossterm Documentation](https://docs.rs/crossterm/)
- [rumqttc Documentation](https://docs.rs/rumqttc/)
- [Tokio Channels](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html)
- [Apollo Rust Best Practices](https://github.com/apollographql/rust-best-practices)

---

*Documento generado a partir del codigo en `src/` con propositos de documentacion y
aprendizaje. Ultima actualizacion: Mayo 2026.*
