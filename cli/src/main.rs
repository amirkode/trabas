use std::collections::HashMap;
use std::panic;

use log::{self, info, LevelFilter};
use once_cell::sync::Lazy;
use clap::{error::ErrorKind, CommandFactory, Parser, Subcommand};
use env_logger::{Env, Builder, Target};
use common::{config::{init_env_from_config, set_configs}, log::StickyLogger};
use trabas::{PROJECT_NAME, PROJECT_VERSION};
use ctrlc;

const CONFIG_KEY_GLOBAL_DEBUG: &str = "DEBUG";

// TODO: complete help info
#[derive(Parser)]
#[command(name = "trabas")]
#[command(version = PROJECT_VERSION, about = "A lightweight http tunneling tool")]
#[command(long_about = "A lightweight HTTP tunneling tool for securely exposing local services to the internet.")]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
    GlobalConfig {
        #[arg(long)]
        set_debug: bool,
        #[arg(long)]
        unset_debug: bool,
    },
    Client {
        #[command(subcommand)]
        action: ClientActions,
    },
    Server {
        #[command(subcommand)]
        action: ServerActions,
    },
    Version { }
}

// config arg keys for client
const CONFIG_ARG_CL_ID: &str = "client-id";
const CONFIG_ARG_CL_SERVER_HOST: &str = "server-host";
const CONFIG_ARG_CL_SERVER_PORT: &str = "server-port";
const CONFIG_ARG_CL_SERVER_SIGNING_KEY: &str = "server-signing-key";

// config arg keys for server
const CONFIG_ARG_SV_GEN_KEY: &str = "gen-key";
const CONFIG_ARG_SV_KEY: &str = "key";
const CONFIG_ARG_SV_REDIS_ENABLE: &str = "redis-enable";
const CONFIG_ARG_SV_REDIS_HOST: &str = "redis-host";
const CONFIG_ARG_SV_REDIS_PORT: &str = "redis-port";
const CONFIG_ARG_SV_REDIS_PASS: &str = "redis-pass";
const CONFIG_ARG_SV_PUBLIC_ENDPOINT: &str = "public-endpoint";

// config arg keys for server cache
const CONFIG_ARG_SV_CACHE_CLIENT_ID: &str = "client-id";
const CONFIG_ARG_SV_CACHE_METHOD: &str = "method";
const CONFIG_ARG_SV_CACHE_PATH: &str = "path";
const CONFIG_ARG_SV_CACHE_EXP_DURATION: &str = "exp-duration";

// TODO: add monitoring commands
#[derive(Subcommand)]
enum ClientActions {
    Serve {
        #[arg(long)]
        host: Option<String>,
        #[arg(long, default_value_t = 3000)]
        port: u16,
        #[arg(long)]
        tls: bool,
    },
    SetConfig {
        #[arg(
            name = CONFIG_ARG_CL_ID, 
            long, 
            help="Client ID used for server handshake"
        )]
        client_id: Option<Option<String>>,
        #[arg(
            name = CONFIG_ARG_CL_SERVER_HOST, 
            long,
            help="Server Host for tunneling"
        )]
        server_host: Option<String>,
        #[arg(
            name = CONFIG_ARG_CL_SERVER_PORT, 
            long,
            help="Server Port for tunneling"
        )]
        server_port: Option<u16>,
        #[arg(
            name = CONFIG_ARG_CL_SERVER_SIGNING_KEY, 
            long,
            help="Server Signing Key for signing client signature"
        )]
        server_signing_key: Option<String>,
        #[arg(
            long, 
            help = "Force apply the config",
        )]
        force: bool,
    }
}

#[derive(Subcommand)]
enum ServerActions {
    Run {
        // TODO: add option to flush all requests from redis
        #[arg(long)]
        host: Option<String>,
        #[arg(long, default_value_t = 8000)]
        public_port: u16,
        #[arg(long, default_value_t = 8000)]
        client_port: u16,
        // max number of requests in a time per client across service-wide
        #[arg(long)]
        client_request_limit: Option<u16>,
        // cache client id in the cookie, so the next hit from browser
        // does not have to define in the path
        #[arg(long)]
        cache_client_id: bool,
    },
    CacheConfig {
        #[command(subcommand)]
        action: ServerCacheActions,
    },
    SetConfig {
        #[arg(
            name = CONFIG_ARG_SV_GEN_KEY, 
            long,
            help="Generate server secret"
        )]
        gen_key: Option<Option<String>>,
        #[arg(
            name = CONFIG_ARG_SV_KEY, 
            long,
            help="Set server secret"
        )]
        key: Option<String>,
        #[arg(
            name = CONFIG_ARG_SV_REDIS_ENABLE, 
            long,
            help="Whether the Redis is enabled"
        )]
        redis_enable: Option<String>,
        #[arg(
            name = CONFIG_ARG_SV_REDIS_HOST, 
            long,
            help="Redis Host for queueing"
        )]
        redis_host: Option<String>,
        #[arg(
            name = CONFIG_ARG_SV_REDIS_PORT, 
            long,
            help="Redis Port for queueing"
        )]
        redis_port: Option<String>,
        #[arg(
            name = CONFIG_ARG_SV_REDIS_PASS, 
            long,
            help="Redis Pass for queueing"
        )]
        redis_pass: Option<String>,
        #[arg(
            name = CONFIG_ARG_SV_PUBLIC_ENDPOINT, 
            long,
            help="Public accessible endpoint"
        )]
        public_endpoint: Option<String>,
        #[arg(
            long, 
            help = "Force apply the config",
        )]
        force: bool,
    }
}

// Actions for managing server/request cache
#[derive(Subcommand)]
enum ServerCacheActions {
    List { },
    Set {
        #[arg(
            name = CONFIG_ARG_SV_CACHE_CLIENT_ID, 
            long,
            help="Client ID"
        )]
        client_id: String,
        #[arg(
            name = CONFIG_ARG_SV_CACHE_METHOD, 
            long,
            help="HTTP Method"
        )]
        method: String,
        #[arg(
            name = CONFIG_ARG_SV_CACHE_PATH, 
            long,
            help="Request Path"
        )]
        path: String,
        #[arg(
            name = CONFIG_ARG_SV_CACHE_EXP_DURATION, 
            long,
            help="Cache expiry duration in seconds"
        )]
        exp_duration: u32,
    },
    Remove {
        #[arg(
            name = CONFIG_ARG_SV_CACHE_CLIENT_ID, 
            long,
            help="Client ID"
        )]
        client_id: String,
        #[arg(
            name = CONFIG_ARG_SV_CACHE_METHOD, 
            long,
            help="HTTP Method"
        )]
        method: String,
        #[arg(
            name = CONFIG_ARG_SV_CACHE_PATH, 
            long,
            help="Request Path"
        )]
        path: String,
    }
}

fn show_version() {
    println!("{} v{}", PROJECT_NAME, PROJECT_VERSION);
}

// lazy init using once_cell
static LOGGER: Lazy<StickyLogger> = Lazy::new(|| StickyLogger::new(8, 12, false));

fn cleanup_logger_state() {
    if let Ok(mut state) = LOGGER.state.lock() {
        LOGGER.cleanup(&mut state);
    }
}

fn setup_exit_handler() {
    ctrlc::set_handler(move || {
        // on Ctrl+C, lock the logger and perform cleanup
        cleanup_logger_state();
        std::process::exit(0);
    }).expect("Error setting Ctrl+C handler");

    panic::set_hook(Box::new(|info| {
        cleanup_logger_state();
        if let Some(s) = info.payload().downcast_ref::<&str>() {
            println!("{}", s);
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            println!("{}", s);
        } else {
            println!("Panic occurred without a message.");
        }

        std::process::exit(0);
    }));
}

fn create_logo() -> Vec<String> {
    // Define each letter as an array of 5 strings (7 characters wide).
    let t = [
        "███████",
        "   █   ",
        "   █   ",
        "   █   ",
        "   █   ",
    ];
    let r = [
        "███████",
        "█     █",
        "███████",
        "█   █  ",
        "█   ██ ", // Updated bottom row for R
    ];
    let a = [
        "   █   ",
        "  █ █  ",
        " █████ ",
        "█     █",
        "█     █",
    ];
    let b = [
        "███████",
        "█     █",
        "███████",
        "█     █",
        "███████",
    ];
    let s = [
        " █████ ",
        "█      ",
        " █████ ",
        "      █",
        " █████ ",
    ];
    
    let line1 = format!("{} {} {} {} {} {}", t[0], r[0], a[0], b[0], a[0], s[0]);
    let line2 = format!("{} {} {} {} {} {}", t[1], r[1], a[1], b[1], a[1], s[1]);
    let line3 = format!("{} {} {} {} {} {}", t[2], r[2], a[2], b[2], a[2], s[2]);
    let line4 = format!("{} {} {} {} {} {}", t[3], r[3], a[3], b[3], a[3], s[3]);
    let line5 = format!("{} {} {} {} {} {} v{} by Liter8.sh", t[4], r[4], a[4], b[4], a[4], s[4], PROJECT_VERSION);
    
    vec!["Welcome to:".to_string(), "".to_string(), line1, line2, line3, line4, line5, "".to_string()]
}

fn init_logger() {
    let debug = std::env::var(CONFIG_KEY_GLOBAL_DEBUG).unwrap_or_default() == "true";
    if !debug {
        log::set_logger(&*LOGGER).unwrap();
        log::set_max_level(LevelFilter::Info);
    } else {
        // init logger
        Builder::from_env(Env::default().default_filter_or("info"))
        .target(Target::Stdout)
        .format_timestamp_millis()
        .init();
    }

    // print logo
    let logo = create_logo();
    for l in logo {
        info!("{}", l);
    }
}

#[tokio::main]
async fn main() {
    // exit handler
    setup_exit_handler();

    // init env vars
    init_env_from_config();

    // init logger
    init_logger();

    // route commands
    let cli = Cli::parse();
    
    match &cli.command {
        Commands::GlobalConfig { set_debug, unset_debug } => {
            if *set_debug && *unset_debug {
                let mut cmd = Cli::command();
                cmd.error(
                    ErrorKind::MissingRequiredArgument,
                    "Cannot set both --set-debug and --unset-debug at once."
                ).exit();
            }

            if *set_debug {
                set_configs(HashMap::from([
                    (String::from(CONFIG_KEY_GLOBAL_DEBUG), "true".to_string())
                ]));
                println!("Global config {} is set to true", CONFIG_KEY_GLOBAL_DEBUG)
            } else {
                set_configs(HashMap::from([
                    (String::from(CONFIG_KEY_GLOBAL_DEBUG), "false".to_string())
                ]));
                println!("Global config {} is set to false", CONFIG_KEY_GLOBAL_DEBUG)
            }
        },
        Commands::Client { action } => match action {
            ClientActions::Serve { host, port , tls } => {
                client::entry_point((*host).clone(), *port, *tls).await;
            },
            ClientActions::SetConfig { client_id, server_host, server_port, server_signing_key, force } => {
                cleanup_logger_state();
                
                if client_id.is_none() && server_host.is_none() && server_port.is_none() && server_signing_key.is_none() {
                    let mut cmd = Cli::command();
                    let error_message = format!(
                        "At least one of the following arguments must be provided: --{}, --{}, --{}, or --{}",
                        CONFIG_ARG_CL_ID,
                        CONFIG_ARG_CL_SERVER_HOST,
                        CONFIG_ARG_CL_SERVER_PORT,
                        CONFIG_ARG_CL_SERVER_SIGNING_KEY
                    );
                    
                    cmd.error(
                        ErrorKind::MissingRequiredArgument,
                        error_message
                    ).exit();
                }

                if let Some(value) = client_id {
                    client::config::generate_client_id((*value).clone(), *force)
                }

                if let Some(value) = server_host {
                    client::config::set_server_host((*value).clone(), *force)
                }

                if let Some(value) = server_port {
                    client::config::set_server_port(*value, *force)
                }

                if let Some(value) = server_signing_key {
                    client::config::set_server_signing_key((*value).clone(), *force)
                }
            }
        },
        Commands::Server { action } => match action {
            ServerActions::Run { host, public_port, client_port,  client_request_limit, cache_client_id} => {
                let root_host = match host {
                    Some(value) => (*value).clone(),
                    None => String::from("127.0.0.1")
                };
                let client_request_limit = match client_request_limit {
                    Some(value) => *value,
                    None => 0
                };
                server::entry_point(root_host, *public_port, *client_port, client_request_limit, *cache_client_id).await
            },
            ServerActions::CacheConfig { action } => match action {
                ServerCacheActions::List { } => {
                    cleanup_logger_state();
                    server::config::show_cache_config().await;
                },
                ServerCacheActions::Set { client_id, method, path, exp_duration } => {
                    cleanup_logger_state();
                    server::config::set_cache_config((*client_id).clone(), (*method).clone(), (*path).clone(), *exp_duration).await;
                },
                ServerCacheActions::Remove { client_id, method, path } => {
                    cleanup_logger_state();
                    server::config::remove_cache_config((*client_id).clone(), (*method).clone(), (*path).clone()).await;
                },
            },
            ServerActions::SetConfig { gen_key, key, redis_enable, redis_host, redis_port, redis_pass, public_endpoint, force } => {
                cleanup_logger_state();

                if gen_key.is_none() && 
                    key.is_none() && 
                    redis_enable.is_none() &&
                    redis_host.is_none() &&
                    redis_port.is_none() && 
                    redis_pass.is_none() &&
                    public_endpoint.is_none() {
                    let mut cmd = Cli::command();
                    let error_message = format!(
                        "At least one of the following arguments must be provided: --{}, --{}, --{}, --{}, --{}, --{} or --{}",
                        CONFIG_ARG_SV_GEN_KEY,
                        CONFIG_ARG_SV_KEY,
                        CONFIG_ARG_SV_REDIS_ENABLE,
                        CONFIG_ARG_SV_REDIS_HOST,
                        CONFIG_ARG_SV_REDIS_PORT,
                        CONFIG_ARG_SV_REDIS_PASS,
                        CONFIG_ARG_SV_PUBLIC_ENDPOINT
                    );

                    cmd.error(
                        ErrorKind::MissingRequiredArgument,
                        error_message
                    ).exit();
                }

                if gen_key.is_some() && key.is_some() {
                    let mut cmd = Cli::command();
                    cmd.error(
                        ErrorKind::MissingRequiredArgument,
                        format!(
                            "Cannot set both --{} and --{} at once.",
                            CONFIG_ARG_SV_GEN_KEY,
                            CONFIG_ARG_SV_KEY
                        )
                    ).exit();
                }

                // generate secret key and save into config
                if let Some(_) = gen_key {
                    server::config::generate_server_secret(*force);
                }

                // set redis config
                server::config::set_server_configs(
                    (*key).clone(),
                    (*redis_enable).clone(),
                    (*redis_host).clone(),
                    (*redis_port).clone(),
                    (*redis_pass).clone(),
                    (*public_endpoint).clone(),
                    *force);
            }
        },
        Commands::Version { } => {
            show_version();
        }
    }
}
