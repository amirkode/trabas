# Server Setup
This guide will cover remote server setup especially with **TLS** for client-server connection. For simplicity, we will focus on **Docker** setup.

## Proxy
Trabas utilizes standard TCP Connection for data sharing. It has its own data format (not tied to a particular protocol, i.e: HTTP) for client-server data sharing. If you use a reverse proxy, ensure it forwards the data packet **without protocol specific validation**. You may use NGINX with stream enabled or other tools that offer such feature.

## Setting up TLS

To establish connection via TLS, we have two options:
- Behind Reverse Proxy (e.g: NGINX, HAProxy, etc.)
  - The server service will not handle TLS directly, but rather the reverse proxy will handle it.
- Direct TLS Connection (supported since `v0.2.0`)

### **Generate CA and Server Certification**
You may generate these certificates using trusted issuers e.g: `Let's Encrypt, DigiCert, etc`. But, if your prefer self-signed certificates, you can use the following methods:

### A. Using `trabas` CLI:
Since `v0.2.0`, `trabas` CLI supports generating self-signed certificates for server service.
You can generate the CA and server certificates using `trabas` CLI:
```console
foo@bar:~$ trabas server ssl-config generate-keys --host localhost --ip 127.0.0.1
```
This command will generate the CA and server certificates in the `trabas_config` directory. The generated files will be:
- `ca.crt`: The CA certificate.
- `ca.key`: The CA private key.
- `server.crt`: The server certificate signed by the CA.
- `server.csr`: The server certificate signing request.
- `server.key`: The server private key.

### B. Manual Generation with `openssl`:
In this case, we try to generate certificates for `localhost` (You should change some details for a real server deployment).

_Generate CA Certificate_

Create a private key:
```console
foo@bar:~$ openssl genpkey -algorithm RSA -out ca.key -pkeyopt rsa_keygen_bits:2048
```
Create a self-signed certificate:
```console
foo@bar:~$ openssl req -x509 -new -nodes -key ca.key -sha256 -days 3650 -out ca.crt -subj "/C=US/ST=State/L=City/O=Organization/OU=OrgUnit/CN=Example CA"
```

_Generate Server Certificate signed by the CA_

Create a server private key:
```console
foo@bar:~$ openssl genpkey -algorithm RSA -out server.key -pkeyopt rsa_keygen_bits:2048
```
Create a certificate signing request (CSR):
```console
foo@bar:~$ openssl req -new -key server.key -out server.csr -subj "/C=US/ST=State/L=City/O=Organization/OU=OrgUnit/CN=localhost"
```
Prepare a configuration file (server.conf) for the certificate:
```conf
[ v3_req ]
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[ alt_names ]
DNS.1 = localhost
IP.1 = 127.0.0.1
```
Sign the server certificate with the CA certificate:
```console
foo@bar:~$ openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out server.crt -days 365 -sha256 -extfile server.conf -extensions v3_req
```

_Verify the generated certificates_
```console
foo@bar:~$ openssl verify -CAfile ca.crt server.crt
```

### Run Server Service with Docker
In this example, we will use NGINX as our reversed proxy.

We provide Dockerfiles and docker-compose files for server service deployment using Docker. You can find them in the `docker/server` directory of the project.

**Setting up Nginx Configuration**

We provide configuration templates in `docker/server/nginx_config`.
To configure Nginx as a reverse proxy, follow these steps:

1.  Copy the generated `server.crt` and `server.key` to the `ssl`.
2.  Configure the `nginx.conf` such host, and ports.
3.  Ensure NGINX is configured to listen on port `3377` (or any port you prefer) for incoming connections.

The NGINX will listen to port `3377` (you may change as you wish in the `docker-compose.yml`).

**Run everything**

To run the server with Redis:

1.  Navigate to the `docker/server/with_redis` directory.
2.  Run `docker compose up`.

To run the server without Redis:

1.  Navigate to the `docker/server/without_redis` directory.
2.  Run `docker compose up`.

The public port and client port for both services are `8787` and `8789` respectively. The container name for the trabas server is `trabas_server`, and the nginx container name is `trabas_nginx` for both with and without Redis.

If the server starts successfully, the log will show:

```console
[Public Listerner] Listening on: `0.0.0.0:8787`.
[Client Listerner] Listening on: `0.0.0.0:8789`.
```

## Client Setup
### Setting up Client Service

Trabas has supported TLS connection from client service.

You can follow these steps:
- In your client host machine (local), copy the generated `ca.crt` to `[bin directory]/trabas_config/ssl/`. Otherwise, you may enable `CL_TLS_TOFU_ENABLE` config to enable **Trust On First Use (TOFU)** for TLS connection.

- Ensure the server host and server port are correctly set to our server service (or target NGINX proxy).

- You run as the client service normally with additional `--tls` option:
    ```console
    foo@bar:~$ trabas client serve --host localhost --port 3000 --tls
    ```

Once all steps are complete, the services should function properly.
