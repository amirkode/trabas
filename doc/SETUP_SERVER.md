## Server Setup
This guide will cover remote server setup especially with **TLS** for client-server connection. For simplicity, we will focus on **Docker** setup.

### Proxy
Trabas utilizes standard TCP Connection for data sharing. It has its own data format (not tied to a particular protocol, i.e: HTTP) for client-server data sharing. If you use a reverse proxy, ensure it forwards the data packet **without protocol specific validation**. You may use NGINX with stream enabled or other tools that offer such feature.

### Run Server Service with Docker
We provide Dockerfile for server service deployment using Docker. You can find it in this directory of the project `docker/server`.

Please copy **Dockerfile** and **entrypoint.sh** somewhere in your server. Then, build the docker image:
```console
foo@bar:~$ docker build -t [image tag] .
```
In the predefined Docker config, the public port and client port respectively will be `8787` and `8789` in the docker container. You can run it with this command:
```console
foo@bar:~$ docker run -d -p [exposed public port]:8787 -p [exposed client port]:8789 [image tag] --name [container name]
```
If it's successful, the log will show:
```console
[Public Listerner] Listening on :0.0.0.0:8787
[Client Listerner] Listening on :0.0.0.0:8789
```
### Setting up TLS
To establish connection via TLS, we need a reverse proxy since the server service has not supported the TLS yet. In this example, we will use NGINX as our reversed proxy.

**Generate CA and Server Certification**

You may generate these certificates using trusted issuers e.g: `Let's Encrypt, DigiCert, etc`. But, you can follow these steps for self-signed certificates using `openssl`.

_Generate CA Certificate_

Create a private key:
```console
foo@bar:~$ openssl genrsa -out ca.key 2048
```
Create a self-signed certificate:
```console
foo@bar:~$ openssl req -x509 -new -nodes -key ca.key -sha256 -days 3650 -out ca.crt -subj "/C=US/ST=State/L=City/O=Organization/OU=OrgUnit/CN=Example CA"

```

_Generate Server Certificate signed by the CA_

Create a private key for CA:
```console
foo@bar:~$ openssl genrsa -out server.key 2048
```
Create a certificate signing request (CSR):
```console
foo@bar:~$ openssl req -new -key server.key -out server.csr -subj "/C=US/ST=State/L=City/O=Organization/OU=OrgUnit/CN=myhost.com"
```
Prepare a configuration file (server.conf) for the certificate:
```conf
[ v3_req ]
keyUsage = keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[ alt_names ]
DNS.1 = myhost.com
```
Sign the server certificate with the CA certificate:
```console
foo@bar:~$ openssl x509 -req -in server.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out server.crt -days 365 -sha256 -extfile server.conf -extensions v3_req
```

_Verify the generated certificates_
```console
foo@bar:~$ openssl verify -CAfile ca.crt server.crt
```

**Setting up NGINX proxy**

We provide `docker compose`configs for the NGINX. You can find it in this directory of the project `docker/server/serve_tls_with_nginx`.

The steps are as follows:

- Copy all files inside the folder to somewhere in your server.
- Copy the generated `server.crt` and `server.key` to the `ssl` folder.
- Configure the `nginx.conf`.
- Lastly run the container:
    ```console
    foo@bar:~$ docker compose up -d
    ```
- The NGINX will listen to port `3377` (you may change as you wish in the `docker-compose.yml`).

### Run everything with Docker Compose
Above operations might take time. We also provide a `Docker Compose` file to run both trabas service and NGINX altogether. You could find in this directory: `docker/server/docker-compose.yml`. CD to the directory and just run:
```console
foo@bar:~$ docker compose up -d
```
Ensure you have everything set (e.g. nginx.conf, ssl, etc.).
## Client Setup
### Setting up Client Service

Trabas has supported TLS connection from client service. You can follow these steps:
- In your client host machine (local), copy the generated `ca.crt` to `[bin directory]/trabas_config/`.
- Ensure the server host and server port are correctly set to the target NGINX proxy.
- You run as the client service normally with additional `--tls` option:
    ```console
    foo@bar:~$ trabas client serve --host localhost --port 3000 --tls
    ```

Once all steps are complete, the services should function properly.