
## `trabas server cache-config set`
Setting cache configuration for specific requests.

NOTE: This is only available when Redis is enabled.
#### Options
Option | Type | Description |
--- | --- | --- |
`--client-id` | String | Client ID |
`--method` | String | Request method |
`--path` | String | Request path |
`--exp-duration` | Integer | Request Cache duration in seconds |
#### Example
```bash
trabas server cache-config set --client-id client1 --method GET --path /ping --exp-duration 60
```
## `trabas server cache-config remove`
Remove cache configuration for specific requests.

NOTE: This is only available when Redis is enabled.
#### Options
Option | Type | Description |
--- | --- | --- |
`--client-id` | String | Client ID
`--method` | String | Request method |
`--path` | String | Request path |
#### Example
```bash
trabas server cache-config remove --client-id client1 --method GET --path /ping
```
## `trabas server cache-config list`
Show all cache configurations for specific requests.

NOTE: This is only available when Redis is enabled.