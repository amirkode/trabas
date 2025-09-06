## `trabas server run`
Run server service.
#### Options
Option | Type | Description |
--- | --- | --- |
`--host` | String [Optional] | Target host where service will be listening on
`--public_port` | Integer | Port for end user access |
`--client_port` | Integer | Port for client service access |
`--client-request-limit` | Integer [Optional] | The request limit for each client service |
`--cache-client-id` | No value [Optional] | Allow client id to pass through cookie header `trabas_client_id`. This also caches the client id passed by request path using `Set-Cookie` response header. |
`--return-tunnel-id` | No value [Optional] | Return tunnel ID to the response headers with key `trabas_tunnel_id` |
`--tls` | No value [Optional] | Enable TLS for the server |
#### Example
```bash
trabas server run --public-port 8001 --client-port 8002
```