# Trabas Config
Trabas uses file based `.env` to the config locally inside the binary file location in `trabas_config/`.
This config file shares for both server service and client service if you use the same binary file to run both services.

## Server Service
**SV_SECRET**

A secret key shared with client service connection for HMAC validation:
```console
foo@bar:~$ trabas server set-config --gen-key
```
this will generate the secret for the first time. You may regenerate later using `--force` option.

**SV_REDIS_HOST**

A redis host:
```console
foo@bar:~$ trabas server set-config --redis-host [value goes here]
```

**SV_REDIS_PORT**

A redis port:
```console
foo@bar:~$ trabas server set-config --redis-port [value goes here]
```

**SV_REDIS_PASS**

A redis pass (it's required for now):
```console
foo@bar:~$ trabas server set-config --redis-pass [value goes here]
```

### Run at once
You may also run the command at once:
```console
foo@bar:~$ trabas server set-config --gen-key --redis-host [value] --redis-port [value] --redis-pass [value] 
```

## Client Service
**CL_ID**

A client id is used for indentification on the server. You can generate one:
```console
foo@bar:~$ trabas server set-config --client-id
```
You may also want to specific the value, but ensure your id is unique accross registered clients:
```console
foo@bar:~$ trabas server set-config --client-id [value goes here]
```
**CL_SERVER_HOST**

A server host:
```console
foo@bar:~$ trabas server set-config --server-host [value goes here]
```

**CL_SERVER_HOST**

A server host:
```console
foo@bar:~$ trabas server set-config --server-host [value goes here]
```

### Run at once
You may also run the command at once:
```console
foo@bar:~$ trabas server set-config --client-id --server-host [value] --server-port [value]
```

### Note
Trabas also provides `--force` option to replace an existing config value as mentioned earlier:
```console
foo@bar:~$ trabas server set-config --gen-key --force
```

# Other Configs
Beside configs defined in `trabas_config/.env`, some are stored in the `Redis` i.e: Request Cache.
## Request Cache
Trabas provides a caching layer for a particular HTTP request. The cache is unique by **Client ID**, **Method**, **URI**, and **Body**. This is reliable when the request headers is insignificant to the result (Some ID spefic request by headers might not use this config).
### Manage Cache Rule
This config will store **Client ID**, **Method**, **Path**, and **Expiry Duration in Seconds**. You easily set while the **Redis** is ready as follows:
```console
foo@bar:~$ trabas server cache-config set --client-id client1 --method POST --path /ping --exp-duration 10
```
If you want to disable the rule, just remove it with this command:
```console
foo@bar:~$ trabas server cache-config remove --client-id client1 --method POST --path /ping
```
*change the values with yours.
### Show Cache Rules
You may see all the configs you set as follows:
```console
foo@bar:~$ trabas server cache-config list
```
This will show all rules you have added:
```console
Request Cache Configurations:
+-----------+--------+-------+---------------------------+
| Client ID | Method | Path  | Expiry Duration (Seconds) |
+-----------+--------+-------+---------------------------+
| client1   | GET    | /ping |            10             |
+-----------+--------+-------+---------------------------+
| client1   | POST   | /ping |            10             |
+-----------+--------+-------+---------------------------+
```
Worth noting that if you the rule/config is unique by `Client ID`, `Method`, and `Path`. Setting the existing one will only replace the `Expiry Duration` value.
