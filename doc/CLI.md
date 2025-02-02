# Trabas CLI Documentation

## Introduction
`trabas` (the name of the binary file) is a command-line interface (CLI) tool designed for All-in-One functionality for HTTP tunneling. This guide provides detailed documentation for all available commands and options.

---

## Usage
The basic syntanx for using `trabas` is:

```console
foo@bar:~$ trabas [COMMAND] [SUBCOMMANDS if any] [OPTIONS]
```

Run the following for the help on specific command:
```console
foo@bar:~$ trabas [COMMAND] [SUBCOMMANDS if any] --help
```

## Commands
### `trabas server`
Manage server service.
### Sub commands
#### `trabas server run`
Run server service.
#### Options
Option | Type | Description |
--- | --- | --- |
`--host` | String [Optional] | Target host where service will be listening on
`--public_port` | Integer | Port for end user access |
`--client_port` | Integer | Port for client service access |
`--client-request-limit` | Integer [Optional] | The request limit for each client service |
`--cache-client-id` | No value [Optional] | Allow client id to pass through cookie header `trabas_client_id`. This also caches the client id passed by request path using `Set-Cookie` response header. |
#### Example
```console
foo@bar:~$ trabas server run --public-port 8001 --client-port 8002
```
#### `trabas server set-config`
Set server service configuration.
#### Options
Option | Type | Description |
--- | --- | --- |
`--gen-key` | No value [Optional] | Generate server secret |
`--key` | String [Optional] | Manual set server secret |
`--redis-enable` | String | Enable flag whether to use redis for temporary transfer store. The value is either `true` or `false` |
`--redis-host` | String | Host for redis |
`--redis-port` | String | Port for redis |
`--redis-pass` | String | Password redis |
`--force` | No value [Optional] | Force rewrite all configs that has been set |
#### Example
```console
foo@bar:~$ trabas server set-config --get-key --redis-host localhost --redis-port 6379 --redis-pass mypass --force
```
#### `trabas server cache-config set`
Setting cache configuration for specific requests.

NOTE: This is only available when Redis is enabled.
#### Options
Option | Type | Description |
--- | --- | --- |
`--client-id` | String | Client ID
`--method` | String | Request method |
`--path` | String | Request path |
`--exp-duration` | Integer | Request Cache duration in seconds |
#### Example
```console
foo@bar:~$ trabas server cache-config set --client-id client1 --method GET --path /ping --exp-duration 60
```
#### `trabas server cache-config list`
Show all cache configurations for specific requests.

NOTE: This is only available when Redis is enabled.
### `trabas client`
Manage client service.
### Sub commands
#### `trabas client serve`
Connect to the server service and serve the underlying service.
#### Options
Option | Type | Description |
--- | --- | --- |
`--host` | String [Optional] | Target host of the underlying service i.e: `localhost` |
`--port` | Integer | Target port of the underlying service |
`--tls` | No value [Optional] | Enable TLS connection to the server service |
#### Example
```console
foo@bar:~$ trabas client serve --host localhost --port 8001 --tls
```
#### `trabas client set-config`
Set clinet service configuration.
#### Options
Option | Type | Description |
--- | --- | --- |
`--client-id` | No value/String [Optional] | Specify Client ID or Generate it if no value is passed |
`--server-host` | String [Optional] | Server service host |
`--server-port` | Integer | Server service port |
`--server-signing-key` | Integer | Server secret for server authentication |
`--force` | No value [Optional] | Force rewrite all configs that has been set |
#### Example
```console
foo@bar:~$ trabas client set-config --server-port 8000 --server-host myhost.com --client-id client1 --server-signing-key 234523  --force
```
### `trabas version`
Show current version of the tool.