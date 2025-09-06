## `trabas client set-config`
Set client service configuration.
#### Options
Option | Type | Description |
--- | --- | --- |
`--client-id` | No value/String [Optional] | Specify Client ID or Generate it if no value is passed |
`--tls-tofu-enable` | Boolean [Optional] | Enable TOFU (Trust On First Use) for TLS connection |
`--server-host` | String [Optional] | Server service host |
`--server-port` | Integer | Server service port |
`--server-signing-key` | String | Server secret for server authentication |
`--force` | No value [Optional] | Force rewrite all configs that has been set |
#### Example
```bash
trabas client set-config --server-port 8000 --server-host myhost.com --client-id client1 --server-signing-key 234523  --force
```