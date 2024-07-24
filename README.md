# Trabas - an HTTP Tunneling Tool [UNRELEASED - COMING SOON]
> This project is still under initial development. A breaking change might happen anytime. Please, use it while watching for the latest update.

Trabas is an (ngrok-like inspired) HTTP tunneling written in Rust utilizing standard TCP connection for data exchange. It offers simplicity for basic tunneling usage through an _easy-to-use_ All-in-One CLI. Find out more below!

## Install
Soon
## Usage
This tool consists of two services:
- Server Service
  - Listens for public requests and client service.
  - Forwards public request to clent service.
  - Utilizes Redis for request queue.
- Client Service
    - Forwards request received from server service to the underlying service.

Here's how to start tunneling:

**Service Service**
```console
foo@bar:~$ trabas server run --public-port 8001 --client-port
 8002
```
this starts the public request and client service listeners.

**Client Service**
```console
foo@bar:~$ trabas client serve --host localhost --port 300
```
this starts the public request and client service listeners.

## Demo
Soon

## Features
- Basic HTTP Tunneling.
- All utilities in one binary file.
- A **Service Serrvice** could handle multiple **Client Services**.
- Many more soon.
### Logs
Soon

## Development Build
Soon

## Contribution
Coming soon

## Disclaimer
This project was intended for personal use

## License
Copyright (c) 2024-present [Authors](https://github.com/amirkode/trabas/blob/main/AUTHORS) and Contributors.

Trabas is distributed under the [MIT License](https://opensource.org/license/mit/).