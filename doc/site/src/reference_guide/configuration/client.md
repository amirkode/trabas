# Client
Configuration for the client service.

### **CL_ID**

A client id is used for indentification on the server. You can generate one:
```bash
trabas client set-config --client-id
```
You may also want to specific the value, but ensure your id is unique accross registered clients:
```bash
trabas client set-config --client-id [value goes here]
```
### **CL_TLS_TOFU_ENABLE**

A flag indicating whether to enable TOFU (Trust On First Use) for TLS connections (the value `true` or `false`):
```bash
trabas client set-config --tls-tofu-enable [value goes here]
```

### **CL_SERVER_HOST**

A server host:
```bash
trabas client set-config --server-host [value goes here]
```

### **CL_SERVER_PORT**

A server port:
```bash
trabas client set-config --server-port [value goes here]
```

### **CL_SERVER_SIGNING_KEY**
A server signing key, used for server authentication.
```bash
trabas client set-config --server-signing-key [value goes here]
```

### Run at once
You may also run the command at once:
```bash
trabas client set-config --client-id --server-host [value] --server-port [value]
```

### **CL_SERVER_FINGERPRINT**
A server fingerprint, used for verifying the server's identity.
The value will be generated automatically when the first time handshake occurs.


## Note
Trabas also provides `--force` option to replace an existing config value as mentioned :
```bash
trabas client set-config --gen-key --force
```