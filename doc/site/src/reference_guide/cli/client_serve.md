## `trabas client serve`
Connect to the server service and serve the underlying service.
#### Options
Option | Type | Description |
--- | --- | --- |
`--host` | String [Optional] | Target host of the underlying service i.e: `localhost` |
`--port` | Integer | Target port of the underlying service |
`--tls` | No value [Optional] | Enable TLS connection to the server service |
#### Example
```bash
trabas client serve --host localhost --port 8001 --tls
```