use clap::{error::ErrorKind, CommandFactory, Parser, Subcommand};
use env_logger::{Env, Builder, Target};
use common::config::init_env_from_config;
use trabas::{PROJECT_NAME, PROJECT_VERSION};

// TODO: complete help info
#[derive(Parser)]
#[command(name = "trabas")]
#[command(about = "A light-weight http tunneling tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
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
const CONFIG_ARG_SV_REDIS_HOST: &str = "redis-host";
const CONFIG_ARG_SV_REDIS_PORT: &str = "redis-port";
const CONFIG_ARG_SV_REDIS_PASS: &str = "redis-pass";

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

// TODO: add monitoring commands
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

#[tokio::main]
async fn main() {
    // init env vars
    init_env_from_config();

    // init logger
    Builder::from_env(Env::default().default_filter_or("info"))
    .target(Target::Stdout)
    .format_timestamp_millis()
    .init();

    // route commands
    let cli = Cli::parse();
    
    match &cli.command {
        Commands::Client { action } => match action {
            ClientActions::Serve { host, port , tls } => {
                client::entry_point((*host).clone(), *port, *tls).await;
            },
            ClientActions::SetConfig { client_id, server_host, server_port, server_signing_key, force } => {
                if client_id.is_none() && server_host.is_none() && server_port.is_none() && server_signing_key.is_none() {
                    let mut cmd = Cli::command();
                    cmd.error(
                        ErrorKind::MissingRequiredArgument,
                        format!(
                            "At least one of --{}, --{}, --{}, or --{} must be provided",
                            CONFIG_ARG_CL_ID,
                            CONFIG_ARG_CL_SERVER_HOST,
                            CONFIG_ARG_CL_SERVER_PORT,
                            CONFIG_ARG_CL_SERVER_SIGNING_KEY
                        )
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
            ServerActions::Run { host, public_port, client_port,  client_request_limit} => {
                let root_host = match host {
                    Some(value) => (*value).clone(),
                    None => String::from("127.0.0.1")
                };
                let client_request_limit = match client_request_limit {
                    Some(value) => *value,
                    None => 0
                };
                server::entry_point(root_host, *public_port, *client_port, client_request_limit).await
            },
            ServerActions::CacheConfig { action } => match action {
                ServerCacheActions::List { } => {
                    server::config::show_cache_config().await;
                },
                ServerCacheActions::Set { client_id, method, path, exp_duration } => {
                    server::config::set_cache_config((*client_id).clone(), (*method).clone(), (*path).clone(), *exp_duration).await;
                },
                ServerCacheActions::Remove { client_id, method, path } => {
                    server::config::remove_cache_config((*client_id).clone(), (*method).clone(), (*path).clone()).await;
                },
            },
            ServerActions::SetConfig { gen_key, key, redis_host, redis_port, redis_pass, force } => {
                if gen_key.is_none() && key.is_none() && redis_host.is_none() && redis_port.is_none() && redis_pass.is_none() {
                    let mut cmd = Cli::command();
                    cmd.error(
                        ErrorKind::MissingRequiredArgument,
                        format!(
                            "At least one of --{}, --{}, --{}, --{}, or --{} must be provided.",
                            CONFIG_ARG_SV_GEN_KEY,
                            CONFIG_ARG_SV_KEY,
                            CONFIG_ARG_SV_REDIS_HOST,
                            CONFIG_ARG_SV_REDIS_PORT,
                            CONFIG_ARG_SV_REDIS_PASS
                        )
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
                server::config::set_redis_configs((*key).clone(), (*redis_host).clone(), (*redis_port).clone(), (*redis_pass).clone(), *force)
            }
        },
        Commands::Version { } => {
            show_version();
        }
    }
}
