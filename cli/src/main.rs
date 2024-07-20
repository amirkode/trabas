use clap::{error::ErrorKind, CommandFactory, Parser, Subcommand};
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

#[derive(Subcommand)]
enum ClientActions {
    Serve {
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

#[derive(Subcommand)]
enum ServerActions {
    Run {
        #[arg(short, long, default_value_t = 8000)]
        public_port: u16,
        #[arg(short, long, default_value_t = 8000)]
        client_port: u16,
    },
}

#[tokio::main]
async fn main() {
    // init env vars
    init_env_from_config();

    // route commands
    let cli = Cli::parse();
    
    match &cli.command {
        Commands::Client { action } => match action {
            ClientActions::Serve { port } => {
                // client::serve(*port).await;
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
                // TODO: something
            }
        },
    }
}
