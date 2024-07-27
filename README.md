# Trabas - an HTTP Tunneling Tool [UNRELEASED/PRE-ALPHA - COMING SOON]
> This project is still under initial development. A breaking change might happen anytime. Please, use it while watching for the latest update.

Trabas is an (ngrok-like inspired) HTTP tunneling written in Rust utilizing standard TCP connection for data exchange. It offers simplicity for basic tunneling usage through an _easy-to-use_ All-in-One CLI. Find out more below!

## Features
- Basic HTTP Tunneling.
- All utilities in one binary file.
- A **Service Service** could handle multiple **Client Services**.
- Many more soon.

## Usage
This tool consists of two services:
- Server Service
  - Listens for public requests and client service.
  - Forwards public request to clent service.
  - Utilizes Redis for request queue.
- Client Service
    - Forwards request received from server service to the underlying service.

### Install
To use the tool, you can use the all-in-one binary available from the latest release (coming soon).
Alternatively, you may to build binary file using cargo:
```bash
git clone https://github.com/amirkode/trabas.git && cd trabas
cargo build --release --manifest-path cli/Cargo.toml
```
you can find the built binary in `target/release/trabas`.

Here's how to start tunneling:

**Service Service**

Ensure a redis server is available in your system. Then, initialize the config as mentioned [here](https://github.com/amirkode/trabas/blob/main/doc/CONFIG.md).
Start the service:
```console
foo@bar:~$ trabas server run --public-port 8001 --client-port 8002
```
this starts the public request and client service listeners.

**Client Service**

Initialize the config as mentioned [here](https://github.com/amirkode/trabas/blob/main/doc/CONFIG.md). Start the service:
```console
foo@bar:~$ trabas client serve --host localhost --port 3000
```
this starts the public request and client service listeners.

**Deployment**

For remote server deployment, minimal config as mentioned earlier is adequate for testing purpose. But, you may secure the data exchange between client service and server service though TLS connection. The well-rounded example can be found [here](https://github.com/amirkode/trabas/blob/main/doc/CONFIG.md).

## Demo
Soon

### Logs
Soon

## Contribution
Coming soon


## License
Copyright (c) 2024-present [Authors](https://github.com/amirkode/trabas/blob/main/AUTHORS) and Contributors.

Trabas is distributed under the [MIT License](https://opensource.org/license/mit/).