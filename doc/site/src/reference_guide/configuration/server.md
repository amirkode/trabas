# Server
Configuration for the server service.

### **SV_SECRET**

A secret key shared with client service connection for HMAC validation:
```bash
trabas server set-config --gen-key
```
this will generate the secret for the first time. You may regenerate later using `--force` option.

### **SV_PUBLIC_ENDPOINT**

A public endpoint host will be returned to the client:
```bash
trabas server set-config --public-endpoint https://yourendpoint.com
```

### **SV_PUBLIC_REQUEST_TIMEOUT**
A public request timeout in seconds:
```bash
trabas server set-config --public-request-timeout 10
```

### **SV_CACHE_CONFIGS**

```bash
trabas server set-config --cache-configs [value goes here]
```
Trabas provides a caching layer for a particular HTTP request. The cache is unique by **Client ID**, **Method**, **URI**, and **Body**. This is reliable when the request headers is insignificant to the result (Some ID spefic request by headers might not use this config).
#### Manage Cache Rule
This config will store **Client ID**, **Method**, **Path**, and **Expiry Duration in Seconds**. You easily set while the **Redis** is ready as follows:
```bash
trabas server cache-config set --client-id client1 --method POST --path /ping --exp-duration 10
```
If you want to disable the rule, just remove it with this command:
```bash
trabas server cache-config remove --client-id client1 --method POST --path /ping
```
*note: change the values with yours.
#### Show Cache Rules
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

### **SV_REDIS_ENABLE**

If redis is preferred for the request queue (the value `true` or `false`):
```bash
trabas server set-config --redis-enable [value goes here]
```

### **SV_REDIS_HOST**
A redis host:
```bash
trabas server set-config --redis-host [value goes here]
```

### **SV_REDIS_PORT**

A redis port:
```bash
trabas server set-config --redis-port [value goes here]
```

### **SV_REDIS_PASS**

A redis pass (it's required for now):
```bash
trabas server set-config --redis-pass [value goes here]
```

### Run at once
You may also run the command at once:
```bash
trabas server set-config --gen-key --redis-host [value] --redis-port [value] --redis-pass [value]
```