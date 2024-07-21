use clap::{error::ErrorKind, CommandFactory, Parser, Subcommand};
use env_logger::{Env, Builder, Target};
use common::config::init_env_from_config;

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
}

// config arg keys for client
const CONFIG_ARG_CL_ID: &str = "client-id";
const CONFIG_ARG_CL_SERVER_HOST: &str = "server-host";
const CONFIG_ARG_CL_SERVER_SIGNING_KEY: &str = "server-signing-key";

// config arg keys for server
const CONFIG_ARG_SV_GEN_KEY: &str = "gen-key";
const CONFIG_ARG_SV_REDIS_HOST: &str = "redis-host";
const CONFIG_ARG_SV_REDIS_PORT: &str = "redis-port";
const CONFIG_ARG_SV_REDIS_PASS: &str = "redis-pass";

// TODO: add monitoring commands
#[derive(Subcommand)]
enum ClientActions {
    Serve {
        #[arg(short, long)]
        host: Option<String>,
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
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
        #[arg(short, long, default_value_t = 8000)]
        public_port: u16,
        #[arg(short, long, default_value_t = 8000)]
        client_port: u16,
    },
    SetConfig {
        #[arg(
            name = CONFIG_ARG_SV_GEN_KEY, 
            long,
            help="Generate server secret"
        )]
        gen_key: Option<Option<String>>,
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
            ClientActions::Serve { host, port } => {
                client::serve((*host).clone(), *port).await;
            },
            ClientActions::SetConfig { client_id, server_host, server_signing_key, force } => {
                if client_id.is_none() && server_host.is_none() && server_signing_key.is_none() {
                    let mut cmd = Cli::command();
                    cmd.error(
                        ErrorKind::MissingRequiredArgument,
                        format!(
                            "At least one of --{}, --{}, or --{} must be provided",
                            CONFIG_ARG_CL_ID,
                            CONFIG_ARG_CL_SERVER_HOST,
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

                if let Some(value) = server_signing_key {
                    client::config::set_server_signing_key((*value).clone(), *force)
                }
            }
        },
        Commands::Server { action } => match action {
            ServerActions::Run { public_port, client_port } => {
                server::run(*public_port, *client_port).await
            },
            ServerActions::SetConfig { gen_key, redis_host, redis_port, redis_pass, force } => {
                if gen_key.is_none() && redis_host.is_none() && redis_port.is_none() && redis_port.is_none() {
                    let mut cmd = Cli::command();
                    cmd.error(
                        ErrorKind::MissingRequiredArgument,
                        format!(
                            "At least one of --{}, --{}, --{}, or --{} must be provided",
                            CONFIG_ARG_SV_GEN_KEY,
                            CONFIG_ARG_SV_REDIS_HOST,
                            CONFIG_ARG_SV_REDIS_PORT,
                            CONFIG_ARG_SV_REDIS_PORT
                        )
                    ).exit();
                }

                // generate secret key and save into config
                if let Some(_) = gen_key {
                    server::config::generate_server_secret(*force);
                }

                // set redis config
                server::config::set_redis_configs((*redis_host).clone(), (*redis_port).clone(), (*redis_pass).clone(), *force)
            }
        },
    }
}
