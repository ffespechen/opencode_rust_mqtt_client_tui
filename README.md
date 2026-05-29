# Rust MQTT Client TUI

Cliente MQTT 3.1.1 con interfaz de terminal (TUI) construido en Rust con Ratatui y
Crossterm. Soporta conexiones TCP y WebSocket, suscripciones a topicos con mensajes
diferenciados por color, y publicacion en texto plano y formato JSON con niveles de
QoS configurables.

## Caracteristicas

- **Protocolo MQTT 3.1.1** — Implementacion completa del estandar via [rumqttc](https://github.com/bytebeamio/rumqtt).
- **Transporte TCP y WebSocket** — Alternar entre TCP (puerto por defecto 1883) y WebSocket (puerto 8083) con `^W`.
- **Autenticacion** — Campos de usuario y contraseña con credenciales opcionales.
- **Suscripciones a topicos** — Suscribirse a multiples topicos con QoS independientes (0, 1, 2). Cada topico recibe un color unico para distincion visual.
- **Visualizacion de mensajes entrantes** — Los mensajes recibidos se muestran en tiempo real bajo su topico correspondiente, con marca de tiempo, flag de retencion y payload.
- **Publicacion de mensajes** — Publicar en texto plano o JSON con QoS (0, 1, 2) y flag de retencion configurables.
- **Validacion JSON** — Los payloads marcados como JSON se validan antes de enviar, evitando datos mal formados.
- **Ayuda contextual** — La barra inferior muestra ayuda segun el campo que tenga el foco.
- **Navegacion por teclado** — Control completo: Tab/Shift+Tab para navegar campos, combinaciones Ctrl para acciones.
- **Soporte Docker** — Dockerfile multi-etapa para construir una imagen contenedora lista para DockerHub.

## Atajos de Teclado

### Acciones globales

| Tecla         | Accion                        |
|---------------|-------------------------------|
| `Ctrl + C`    | Conectar al broker            |
| `Ctrl + D`    | Desconectar del broker        |
| `Ctrl + S`    | Suscribirse a un topico       |
| `Ctrl + U`    | Cancelar suscripcion          |
| `Ctrl + P`    | Publicar mensaje              |
| `Ctrl + W`    | Alternar TCP / WebSocket      |
| `Ctrl + R`    | Alternar flag Retain          |
| `Ctrl + J`    | Alternar formato JSON / Texto |
| `Ctrl + Q`    | Salir de la aplicacion        |
| `Tab`         | Siguiente campo               |
| `Shift + Tab` | Campo anterior                |
| `Espacio`     | Ciclar valor (QoS) / insertar espacio en payload |

### Edicion de campos

| Tecla              | Accion                                   |
|--------------------|------------------------------------------|
| `Flechas Izq/Der`  | Mover cursor                             |
| `Flechas Arr/Aba`  | Navegar topicos / lineas del payload     |
| `Retroceso`        | Borrar antes del cursor                  |
| `Suprimir`         | Borrar despues del cursor                |
| `Inicio`           | Ir al principio del campo                |
| `Fin`              | Ir al final del campo                    |
| `Enter`            | Nueva linea (solo en editor de payload)  |
| `RePag / AvPag`    | Desplazar mensajes                       |

### Orden de navegacion de campos

`Host` → `Puerto` → `Client ID` → `Usuario` → `Contraseña` → `Topico suscripcion` → `QoS suscripcion` → `Topico publicacion` → `QoS publicacion` → `Payload`

Al tener el foco en QoS de suscripcion o QoS de publicacion, presionar `Espacio` cicla el valor (0→1→2).

## Requisitos Previos

- **Rust** 1.88.0+ (toolchain estable)
- **OpenSSL** — bibliotecas de desarrollo (para soporte TLS en Linux)

### Instalacion de OpenSSL en Linux

```bash
# Debian / Ubuntu
sudo apt-get install pkg-config libssl-dev

# Fedora
sudo dnf install openssl-devel

# Arch Linux
sudo pacman -S openssl
```

## Inicio Rapido

### Compilar desde el codigo fuente

```bash
git clone https://github.com/ffespechen/rust_mqtt_client_tui.git
cd rust_mqtt_client_tui
cargo build --release
./target/release/rust_mqtt_client_tui
```

### Ejecutar con Cargo

```bash
cargo run --release
```

## Uso

1. Iniciar la aplicacion. El foco inicial esta en el campo **Host**.
2. Ingresar la direccion del broker MQTT (ej: `localhost` o `broker.emqx.io`).
3. Usar Tab para navegar a **Puerto** y ajustar si es necesario (por defecto: 1883 TCP, 8083 WebSocket).
4. Opcionalmente ingresar **Client ID**, **Usuario** y **Contraseña**.
5. Presionar `^W` para alternar entre TCP y WebSocket si se requiere.
6. Presionar `^C` para conectar. La barra de estado mostrara el progreso.
7. Una vez conectado, navegar al campo **Topico suscripcion**, ingresar un topico (ej: `sensores/#`) y presionar `^S` para suscribirse.
8. Los mensajes entrantes aparecen en el panel **Messages** a la izquierda, coloreados por topico.
9. Para publicar, navegar a los campos **Topico publicacion** y **Payload** a la derecha, ingresar el mensaje y presionar `^P`.
10. Alternar formato JSON con `^J` — el payload se validara como JSON antes de publicar.

## Arquitectura

Para un analisis detallado de las decisiones arquitectonicas, modelo de concurrencia,
flujo de datos y compromisos de diseño, consultar **[ARCHITECTURE.md](ARCHITECTURE.md)**.

```
src/
├── main.rs              # Punto de entrada: init del terminal, bucle de eventos
├── app/
│   ├── mod.rs
│   └── state.rs         # Struct App, logica de negocio, gestion de estado
├── events/
│   ├── mod.rs
│   └── handler.rs       # KeyEvent → Action, mapeo de atajos de teclado
├── ui/
│   ├── mod.rs           # Orquestacion del layout (filas: top/contenido/bottom)
│   ├── top_bar.rs       # Barra de acciones con atajos Ctrl+letra
│   ├── connection.rs    # Panel de conexion: host, puerto, usuario, contraseña
│   ├── subscriptions.rs # Panel izquierdo: formulario suscripcion, topicos, mensajes
│   ├── publish.rs       # Panel derecho: formulario de publicacion, editor de payload
│   └── help.rs          # Barra inferior: ayuda contextual
└── mqtt/
    ├── mod.rs
    └── client.rs        # Tarea MQTT asincrona: bucle de eventos, comandos
```

- **`src/app`** — Logica de negocio pura. El struct `App` contiene todo el estado. Sin codigo de renderizado.
- **`src/ui`** — Renderizado de solo lectura. Recibe `&App` y dibuja con widgets de Ratatui. Nunca muta el estado.
- **`src/events`** — Traduce `KeyEvent` de Crossterm a enums `Action` del dominio. Sensible al contexto (Tab siempre avanza foco, Espacio cicla QoS o inserta espacio).
- **`src/mqtt`** — Integracion MQTT. Se ejecuta como tarea de Tokio; se comunica con el hilo principal mediante canales MPSC (`MqttCmd` → tarea, `MqttEvent` → UI).

## Docker

### Construir la Imagen

```bash
docker build -t rust-mqtt-client-tui:latest .
```

### Ejecutar el Contenedor

```bash
docker run -it --rm \
  -p 1883:1883 \
  -p 9001:9001 \
  rust-mqtt-client-tui
```

Los flags `-it` son **obligatorios** — la TUI requiere una terminal interactiva con TTY.
Sin ellos, la aplicacion muestra `Fatal error: Failed to enable raw mode — is a TTY available?`
y finaliza con codigo de error.

**Mapeo de puertos:**

| Puerto Host | Puerto Contenedor | Protocolo | Proposito                    |
|-------------|-------------------|-----------|------------------------------|
| 1883        | 1883              | TCP       | MQTT sin encriptar           |
| 9001        | 9001              | TCP       | MQTT WebSocket sin encriptar |

Si tu broker utiliza puertos diferentes, ajusta los flags `-p`. Alternativas comunes:
- EMQX: 1883 (TCP), 8083 (WS)
- HiveMQ: 1883 (TCP), 8000 (WS)
- Mosquitto: 1883 (TCP), 9001 (WS)

### Publicar en DockerHub

```bash
docker login
docker tag rust-mqtt-client-tui:latest ffespechen/rust-mqtt-client-tui:0.1.0
docker push ffespechen/rust-mqtt-client-tui:latest
docker push ffespechen/rust-mqtt-client-tui:0.1.0
```

## Licencia

MIT

## Dependencias

| Crate        | Proposito                                  |
|--------------|--------------------------------------------|
| `ratatui`    | Framework de interfaz de terminal (TUI)    |
| `crossterm`  | Manejo de terminal y eventos de teclado    |
| `rumqttc`    | Cliente MQTT 3.1.1 asincrono              |
| `tokio`      | Runtime asincrono                          |
| `serde_json` | Parseo y validacion JSON                   |
| `chrono`     | Marcas de tiempo en mensajes               |
| `uuid`       | Generacion de IDs de cliente y mensajes    |
| `anyhow`     | Manejo de errores                          |
