## `trabas server set-config`
Set server service configuration.
#### Options
Option | Type | Description |
--- | --- | --- |
`--gen-key` | No value [Optional] | Generate server secret |
`--key` | String [Optional] | Manual set server secret |
`--public-endpoint` | String | A public endpoint host will be returned to the client |
`--redis-enable` | String | Enable flag whether to use redis for temporary transfer store. The value is either `true` or `false` |
`--redis-host` | String | Host for redis |
`--redis-port` | String | Port for redis |
`--redis-pass` | String | Password redis |
`--force` | No value [Optional] | Force rewrite all configs that has been set |
#### Example
```bash
trabas server set-config --gen-key --redis-host localhost --redis-port 6379 --redis-pass mypass --force
```

#### `trabas server ssl-config generate-keys`
Setting SSL configuration for the server.
#### Options
Option | Type | Description |
--- | --- | --- |
`--server_conf_path` | String | Server configuration file path |
`--host` | String | Host for SANs |
`--ip` | String | IP for SANs |
`--force` | No value [Optional] | Force rewrite all configs that has been set |
#### Example
If we have our server.conf file as follows:
```bash
trabas server ssl-config generate-keys --server_conf_path /path/to/server.conf
```
Or if simply provide the host and/or ip as follows:
```bash
trabas server ssl-config generate-keys --host localhost --ip 127.0.0.1
```

