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