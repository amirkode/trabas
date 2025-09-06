# Setup Tunnel
## **Server Service**

Ensure a redis server is available in your system if the `SV_REDIS_ENABLE` config is set to `true`. Then, initialize the config as mentioned [here](https://github.com/amirkode/trabas/blob/main/doc/CONFIG.md).
Start the service:
```bash
trabas server run --public-port 8001 --client-port 8002
```
This starts the public request and client service listeners. You may want to limit the number of requests in a time for every request to each client service, just pass the `--client-request-limit [your value]` argument.

## **Client Service**

Initialize the config as mentioned [here](https://github.com/amirkode/trabas/blob/main/doc/CONFIG.md). Start the service:
```bash
trabas client serve --host localhost --port 3000
```
this starts the public request and client service listeners.

## **User Access**

Once the server and client are connected, you can access the underlying service in several ways:

- **Prefix Path:**  
  `serverhost:8001/[client_id]`

- **Query Parameter:**  
  `serverhost:8001/?trabas_client_id=[client_id]`

Alternatively, after the first request, you can access the service directly at `serverhost:8001` if the client ID is cached (the client will send a `trabas_client_id` cookie header). To enable this feature, start the server service with the `--cache-client-id` flag.  
You may also use a reverse proxy to hide the actual port if needed.

***Note** that the public endpoint will also be returned by the server once a connection successfully established.
