# TeleMQ configuration TOML file

### `broker_id`

**`broker_id`** - a string which will be used as a broker ID. If not provided default value `<undefined>` will be used.

Example:

```toml
broker_id = "broker_eJa9C5VGWuGryALX"
```

### `max_connections`

**`max_connections`** - a maximal number of concurent connections allowed by TeleMQ server. It includes all types of connections - plain TCP, TLS, Websocket connections. If a `max_connections` reached no new connection will be accepted. Default value 10,000 connections.

Example:

```toml
max_connections = 12000
```

### `tcp_port`

**`tcp_port`** - a port which will be used by TeleMQ listener to accept plain TCP connections. Default value - 1883 (standard port accoring to the MQTT spec).

Example:

```toml
tcp_port = 8888
```

### `tls_port`

**`tls_port`** - a port which will be used by TeleMQ listener to accept TLS connections. <u>Important:</u> in order to make TLS listener working you have additionaly to provide a certificate as a `cert_file` config property. If it won't be provided, `tls_port` will be ignored and a TLS listener won't be created. If `cert_file` is provided and `tls_port` not, then a default 8883 port will be used.

Example:

```toml
tls_port = 8883
cert_file = "./server.crt"
```

### `ws_port`

**`ws_port`** - a port which will be used by TeleMQ listener to accept Websocket connections. No default value - Websocket connections are disabled by default.

Example:

```toml
ws_port = 1880
```

### `keep_alive`

**`keep_alive`** - a keep alive interval (in seconds). A connection should send at least one control packet during this interval, otherwise TeleMQ will close a connection due to inactivity. According to MQTT spec `PINGREQ` packets should be used by a client to indicate that it is still alive to prolongue a connection. Default value is 120 seconds.

Example:

```toml
keep_alive = 60
```

### `log_dest`

**`log_dest`** - a logs destination. TeleMQ has three possible logs desitnations:

- `stdout` - logs will be written to an _stdout_ file of TeleMQ process.
- `stderr` - logs will be written to an _stderr_ file of TeleMQ process.
- `file:<PATH_TO_FILE>` - logs will be written to a file with a give path (if not exist, a file will be created automatically).

**`log_level`** - TeleMQ defines several log levels which are used in different situations, and this property defines a minimal log level which will be captured.

Log levels:

_To Be Defined_

### `anonymous_allowed`

**`anonymous_allowed`** - a boolean value which defines if an anonymous clients (the ones which don't provide neither `username` nor `password` in a `CONNECT` control packet) are allowed by a TeleMQ server. Default value - `true`. <u>Important:</u> if `false` is provided then one should provide `auth_file` (a path to an [authentication file TOML file](./auth-file.md)).

Example:

```toml
anonymous_allowed = true
```

### `auth_file`

**`auth_file`** - a path to an [authentication file TOML file](./auth-file.md).

Example:

```toml
auth_file = "./auth_file.toml"
```

### `sys_topics_update_interval`

**`sys_topics_update_interval`** is a time interval in seconds after which $SYS-topic messages are published by a broker. If `0` is provided, $SYS-topics are disabled. Default value - `30` (30 seconds).

Example:

```toml
sys_topics_update_interval = 300
```
