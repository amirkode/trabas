use std::{env, sync::Arc, time::Duration};

use env_logger::{Env, Builder, Target};
use log::{error, info};
use reqwest::Client;
use std::sync::Once;
use tokio::{sync::Mutex, task::{self, JoinHandle}, time::sleep};
use trabas::mocks::{
    client::mock_underlying_repo::MockUnderlyingRepo, 
    server::{
        mock_client_repo::MockClientRepo, 
        mock_request_repo::MockRequestRepo, 
        mock_response_repo::MockResponseRepo
    }
};

async fn send_http_request(url: String) -> Result<String, String> {
    let client = Client::new();
    let response = client.get(url).send().await
        .map_err(|e| format!("{}",  e))?;
    let status = response.status();
    let text = response.text().await
        .map_err(|e| format!("{}",  e))?;
    if status != 200 {
        return Err(text);
    }

    Ok(text)
}

static INIT: Once = Once::new();

fn init_test_env() {
    // to prevent reinitialization
    INIT.call_once(|| {
        // mock env/config
        let server_secret = "74657374696e676b6579313233343536";
        env::set_var(String::from(server::config::CONFIG_KEY_SERVER_SECRET), server_secret);
        env::set_var(String::from(client::config::CONFIG_KEY_CLIENT_ID), "client1");
        env::set_var(String::from(client::config::CONFIG_KEY_CLIENT_SERVER_HOST), "127.0.0.1");
        env::set_var(String::from(client::config::CONFIG_KEY_CLIENT_SERVER_PORT), "3334");
        env::set_var(String::from(client::config::CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY), server_secret);

        // init logger
        Builder::from_env(Env::default().default_filter_or("info"))
        .target(Target::Stdout)
        .format_timestamp_millis()
        .init();
    });
}

#[tokio::test]
async fn test_e2e_request_flow() {
    // init mock env
    init_test_env();

    // start server service
    let client_repo = Arc::new(MockClientRepo::new());
    let request_repo = Arc::new(MockRequestRepo::new());
    let response_repo = Arc::new(MockResponseRepo::new());
    let public_svc_address = String::from("127.0.0.1:3333");
    let client_svc_address = String::from("127.0.0.1:3334");
    let server_exec = tokio::spawn(async move {
        server::run(public_svc_address, client_svc_address, 0, client_repo, request_repo, response_repo).await;
    });

    // delay for 2 seconds to wait the server to start up
    sleep(Duration::from_secs(2)).await;

    // start client service
    let mock_response = String::from("pong");
    let underlying_repo = Arc::new(MockUnderlyingRepo::new(mock_response.clone()));
    let underlying_svc_address = String::from("This has no effect");
    let use_tls = false;
    let client_exec = tokio::spawn(async move {
        client::serve(underlying_svc_address, underlying_repo, use_tls).await;
    });

    // delay for 2 seconds to wait the client to start up
    sleep(Duration::from_secs(2)).await;

    // test hit to server public service, 
    // it should return the mock reponse client service
    let url = String::from("http://127.0.0.1:3333/client1/ping");
    let response = match send_http_request(url).await {
        Ok(res) => res,
        Err(err) => {
            println!("Error getting response: {}", err);
            err
        }
    };

    assert_eq!(response, mock_response);

    // abort all services
    server_exec.abort();
    client_exec.abort();
}

#[tokio::test]
async fn test_e2e_request_flow_with_request_limit() {
    // init mock env
    init_test_env();

    // start server service
    let request_limit: u16 = 5;
    let client_repo = Arc::new(MockClientRepo::new());
    let request_repo = Arc::new(MockRequestRepo::new());
    let response_repo = Arc::new(MockResponseRepo::new());
    let public_svc_address = String::from("127.0.0.1:3333");
    let client_svc_address = String::from("127.0.0.1:3334");
    let server_exec = tokio::spawn(async move {
        server::run(public_svc_address, client_svc_address, request_limit, client_repo, request_repo, response_repo).await;
    });

    // delay for 2 seconds to wait the server to start up
    sleep(Duration::from_secs(2)).await;

    // start client service
    let mock_response = String::from("pong");
    let underlying_repo = Arc::new(MockUnderlyingRepo::new(mock_response.clone()));
    let underlying_svc_address = String::from("This has no effect");
    let use_tls = false;
    let client_exec = tokio::spawn(async move {
        client::serve(underlying_svc_address, underlying_repo, use_tls).await;
    });

    // delay for 2 seconds to wait the client to start up
    sleep(Duration::from_secs(2)).await;

    // test bulk hit to server public service,
    let request_cnt = 15;
    let mut handles: Vec<JoinHandle<()>> = Vec::new();
    let responses = Arc::new(Mutex::new(Vec::<String>::new()));
    // it should return the mock reponse client service
    for _ in 0..request_cnt {
        let responses_clone = Arc::clone(&responses);
        let handle = task::spawn(async move {
            let url = String::from("http://127.0.0.1:3333/client1/ping");
            match send_http_request(url).await {
                Ok(res) => {
                    responses_clone.lock().await.push(res);
                },
                Err(err) => {
                    error!("Error getting response: {}", err);
                }
            };
        });

        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }

    for response in responses.lock().await.iter() {
        assert_eq!(*response, mock_response);
    }

    let success_cnt = responses.lock().await.len();
    info!("Request success count: {}", success_cnt);
    assert!(success_cnt < request_cnt);

    // abort all services
    server_exec.abort();
    client_exec.abort();
}
