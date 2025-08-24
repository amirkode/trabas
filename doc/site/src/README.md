# Introduction to

```
     ███████ ███████    █    ███████    █     █████ 
        █    █     █   █ █   █     █   █ █   █      
        █    ███████  █████  ███████  █████   █████ 
        █    █   █   █     █ █     █ █     █       █
        █    █   ██  █     █ ███████ █     █  █████  
```

Trabas is an (ngrok-like inspired) HTTP tunneling written in Rust utilizing standard TCP connection for data exchange. It offers simplicity for basic tunneling usage through an _easy-to-use_ All-in-One CLI. Find out more below!


## Features
- Basic HTTP Tunneling.
- All utilities in one binary file.
- A **Server Service** could handle multiple **Client Services**.
- Rate Limiter.
- Request Cache.
- Many more soon.

## Usage
This tool consists of two services:
- Server Service
  - Listens for public requests and client service.
  - Forwards public request to clent service.
  - Optionally uses Redis for request queueing (recommended for multiple server instances).
- Client Service
  - Forwards request received from server service to the underlying service.

## Diagram
Here's an example of how users access our local service through the internet:

![basic architecture](images/arch.png)

## Demo
Here's the demo showing how the tunneling works using `Trabas`.

[![Watch the video](images/demo.png)](https://jotling.liter8.sh/trabas-demo-v1?media=video)