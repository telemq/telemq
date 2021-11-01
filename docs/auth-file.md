# TeleMQ authentication and authorization TOML file

Auth file is a TOML file which defines which topics are allowed to publish and/or subscribe both for anonymous (make sure the config file allows anonymous connections) and non-anonymous clients.

It provides a list of credentials for non-anonymous clients so that TeleMQ will check credentials provided by a client in a CONNECT packet against this list. Passwords should be provided as SHA-256 hash of an original passwords which will provided in CONNECT packets.

And finally, it defines white- and blacklist of IP addresses. So if an IP whitelist is defined any new connection should have an IP address which matches the range. And otherwise, if blacklist is provided each new connection IP should not be from the provided IP blacklist range.

---

## `topic_all_rules`

**`topic_all_rules`** is a list of objects. Each object from this list describes access rules for a given topic. It will be used for a check when an anonymous client tries to subscribe or to publish. There are several access types which could be used to define an access level:

- `Read` - a client is allowed to subscribe to a topic.
- `Write` - a client is allowed to publish to a topic.
- `ReadWrite` - a client is allowed both to subscribe and to publish to a topic.
- `Deny` - both publishing and subscribing is forbidden for a client.

Topic name supports all wildcards defined in MQTT standard. Namely,

- `#` - a multiple levels wildcard (levels are separated by `/` symbol). It will match anything which goes after this symbol. For example, `a/#` will match `a/b` and `a/b/c`, but not `b/c`.
- `+` - a single level wildcard. It will pass only a single level. For example, `a/+` will pass `a/b` and `a/c`, but not `a/b/c`. Similarly, `a/+/c` will pass `a/b/c`, but neither `a/b` nor `a/b/d`.

Example:

```toml
topic_all_rules = [
  {access = "Read", topic = "#"},
  {access = "Write", topic = "some_topic"}
]

```

Or equivalently

```toml
[[topic_all_rules]]
topic = "#"
access = "Read"

[[topic_all_rules]]
topic = "some_topic"
access = "Write"
```

## `topic_client_rules`

**`topic_client_rules`** - similarly to `topic_all_rules` it defines a level of an access to a topic by a particular client. Here, different clients are defined by different client IDs.

Example:

```toml
topic_client_rules = [
  {client_id = "ADMIN", topic_rules = [{access = "ReadWrite", topic = "#"}]},
  {client_id = "DEVICE_1", topic_rules = [{access = "ReadWrite", topic = "device/#"}]}
]

```

Or equivalently,

```toml
[[topic_client_rules]]
client_id = "ADMIN"
topic_rules = [{access = "ReadWrite", topic = "#"}]

[[topic_client_rules]]
client_id = "DEVICE_1"
topic_rules = [{access = "ReadWrite", topic = "device/#"}]
```

## `credentials`

Credentials section contains a list of credentials - one entry per client. Each credentials entry should contain following information:

- `client_id` - unique client ID.
- `username` - optional string which represents a username associated with a client.
- `password` - mandatory string which contains an [SHA-256 hash](https://en.wikipedia.org/wiki/SHA-2) of an original password. (An original password should be send in a `password` field in a CONNECT packet).

Example:

```toml
[[credentials]]
client_id = "DEVICE_1"
username = "device_1"
password = "8ee1cb7bc9ab43894d0bbdad6ef11a9cb3f9aee853b095b26b4e126a5976f555"

[[credentials]]
client_id = "DEVICE_2"
username = "device_2"
password = "8ee1cb7bc9ab43894d0bbdad6ef11a9cb3f9aee853b095b26b4e126a5976f555"
```

## `ip_blacklist`

`ip_blacklist` is a list of client IP ranges which are forbidden by a TeleMQ server. For more information, please refer to the origin library [documentation](https://docs.rs/ipnet/2.3.1/ipnet/enum.IpNet.html). During the login an authenticator will iterate through the list and use [`contains`](https://docs.rs/ipnet/2.3.1/ipnet/enum.IpNet.html#method.contains) method to define if a client IP address is in a given range. If there was found a range from `ip_blacklist` which contains a client IP address, such connection will be disallowed. <u>Important:</u> this check takes place when a client sends CONNECT packet to TeleMQ.

Example:

```toml
ip_blacklist = ["192.168.0.0/24"]
```

## `ip_whitelist`

If `ip_whitelist` property is defined TeleMQ will allow clients only if a client IP is within at least one of ranges provided in a list. Similarly to `ip_blacklist` TeleMQ will use [`contains`](https://docs.rs/ipnet/2.3.1/ipnet/enum.IpNet.html#method.contains) method to define if a client IP address is in a given range. <u>Important:</u> this check takes place when a client sends CONNECT packet to TeleMQ.

Example:

```toml
ip_whitelist = ["192.168.0.0/24"]
```
