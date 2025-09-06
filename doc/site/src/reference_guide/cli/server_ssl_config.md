## `trabas server ssl-config generate-keys`
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