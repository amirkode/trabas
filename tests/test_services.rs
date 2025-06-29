#[cfg(test)]
mod tests {
    use std::{collections::{HashMap, HashSet}, env, sync::Arc, time::Duration};

    use common::{_error, _info};
    use common::config::keys as config_keys;
    use common::data::dto::cache_config::CacheConfig;
    use env_logger::{Env, Builder, Target};
    use reqwest::{header::{HeaderMap, HeaderValue, COOKIE, SET_COOKIE}, Client, Response};
    use server::service::cache_service::CacheService;
    use std::sync::Once;
    use tokio::{sync::Mutex, task::{self, JoinHandle}, time::sleep};
    use std::sync::Mutex as StdMutex;
    use trabas::mocks::{
        client::mock_underlying_repo::MockUnderlyingRepo, 
        config::MockConfigHandlerImpl,
        server::{
            mock_cache_repo::MockCacheRepo, 
            mock_client_repo::MockClientRepo, 
            mock_request_repo::MockRequestRepo, 
            mock_response_repo::MockResponseRepo
        }
    };

    async fn send_http_request(url: String, cookies: Option<HashMap<String, String>>) -> Result<Response, String> {
        let client = Client::new();
        let mut headers = HeaderMap::new();
        if let Some(cookies) = cookies {
            let cookie_string = cookies
                .iter()
                .map(|(key, value)| format!("{}={}", key, value))
                .collect::<Vec<_>>()
                .join("; ");
            headers.insert(COOKIE, HeaderValue::from_str(&cookie_string).unwrap());
        }

        let response = client
            .get(url)
            .headers(headers)
            .send().await
            .map_err(|e| format!("{}",  e))?;
        let response_status = response.status().as_u16();

        if response_status >= 200 && response_status < 300 {
            return Ok(response);    
        }

        Err("Invalid status code".to_string())
    }

    static INIT: Once = Once::new();

    fn init_test_env() {
        // to prevent reinitialization
        INIT.call_once(|| {
            // mock env/config
            let server_secret = "74657374696e676b6579313233343536";
            env::set_var(String::from(config_keys::CONFIG_KEY_SERVER_SECRET), server_secret);
            env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_SERVER_HOST), "127.0.0.1");
            env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_SERVER_PORT), "3334");
            env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY), server_secret);

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
        let cache_repo = Arc::new(MockCacheRepo::new());
        let client_repo = Arc::new(MockClientRepo::new());
        let request_repo = Arc::new(MockRequestRepo::new());
        let response_repo = Arc::new(MockResponseRepo::new());
        let config_handler = Arc::new(MockConfigHandlerImpl::new());
        let server_exec = tokio::spawn(async move {
            server::run(
                server::config::ServerRequestConfig::new(
                    "127.0.0.1".to_string(),
                    3333, 
                    3334, 
                    0, // no request limit
                    false, // no cache client id
                    false
                ),
                cache_repo, 
                client_repo, 
                request_repo, 
                response_repo,
                config_handler).await;
        });

        // delay for 2 seconds to wait the server to start up
        sleep(Duration::from_secs(2)).await;

        // start client service
        let mock_response = String::from("pong");
        let underlying_repo1 = Arc::new(MockUnderlyingRepo::new(mock_response.clone(), Arc::new(StdMutex::new(|| {}))));
        let underlying_repo2 = Arc::new(MockUnderlyingRepo::new(mock_response.clone(), Arc::new(StdMutex::new(|| {}))));
        let underlying_repo3 = Arc::new(MockUnderlyingRepo::new(mock_response.clone(), Arc::new(StdMutex::new(|| {}))));
        let use_tls = false;

        // set client1 id
        env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_ID), "client1");

        // run 3 clients
        let client1_exec = tokio::spawn(async move {
            client::serve(String::from("The target underlying address, This has no effect"), underlying_repo1, use_tls).await;
        });

        // delay for 2 seconds to wait the client 1 to start up
        sleep(Duration::from_secs(2)).await;
        // set client2 id
        env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_ID), "client2");

        let client2_exec = tokio::spawn(async move {
            client::serve(String::from("The target underlying address, This has no effect"), underlying_repo2, use_tls).await;
        });

        // delay for 2 seconds to wait the client 2 to start up
        sleep(Duration::from_secs(2)).await;
        // set client3 id
        env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_ID), "client3");

        let client3_exec = tokio::spawn(async move {
            client::serve(String::from("The target underlying address, This has no effect"), underlying_repo3, use_tls).await;
        });

        // delay for 2 seconds to wait the client 3 to start up
        sleep(Duration::from_secs(2)).await;

        // test hit to server public service, 
        // it should return the mock reponse client service
        for i in 1..=3 {
            // each client processes 10 requests
            for _ in 0..10 {
                let url = if i % 2 == 0 { 
                    format!("http://127.0.0.1:3333/client{}/ping?ha=hu&hu=ha", i)
                } else {
                    format!("http://127.0.0.1:3333/ping?trabas_client_id=client{}&ha=hu&hu=ha", i)
                };
                let response = match send_http_request(url.clone(), None).await {
                    Ok(res) => res.text().await.unwrap(),
                    Err(err) => {
                        println!("Error getting response: {}", err);
                        err
                    }
                };

                assert_eq!(response, mock_response);
            }
        }

        // abort all services
        server_exec.abort();
        client1_exec.abort();
        client2_exec.abort();
        client3_exec.abort();
    }

    #[tokio::test]
    async fn test_e2e_request_flow_with_request_limit() {
        // init mock env
        init_test_env();

        // start server service
        let request_limit: u16 = 5;
        let cache_repo = Arc::new(MockCacheRepo::new());
        let client_repo = Arc::new(MockClientRepo::new());
        let request_repo = Arc::new(MockRequestRepo::new());
        let response_repo = Arc::new(MockResponseRepo::new());
        let config_handler = Arc::new(MockConfigHandlerImpl::new());
        let server_exec = tokio::spawn(async move {
            server::run(
                server::config::ServerRequestConfig::new(
                    "127.0.0.1".to_string(),
                    3333, 
                    3334, 
                    request_limit,
                    false,
                    false
                ), 
                cache_repo, 
                client_repo, 
                request_repo, 
                response_repo,
                config_handler).await;
        });

        // delay for 2 seconds to wait the server to start up
        sleep(Duration::from_secs(2)).await;

        // start client service
        let mock_response = String::from("pong");
        let mock_callback = Arc::new(StdMutex::new(|| {}));
        let underlying_repo = Arc::new(MockUnderlyingRepo::new(mock_response.clone(), mock_callback));
        let underlying_svc_address = String::from("This has no effect");
        let use_tls = false;

        // set client1 id
        env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_ID), "client1");
        
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
        for i in 0..request_cnt {
            let responses_clone = Arc::clone(&responses);
            let handle = task::spawn(async move {
                let url = if i % 2 == 0 { 
                    String::from("http://127.0.0.1:3333/client1/ping")
                } else {
                    String::from("http://127.0.0.1:3333/ping?trabas_client_id=client1")
                };
                match send_http_request(url, None).await {
                    Ok(res) => {
                        responses_clone.lock().await.push(res.text().await.unwrap());
                    },
                    Err(err) => {
                        _error!("Error getting response: {}", err);
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
        _info!("Request success count: {}", success_cnt);
        assert!(success_cnt < request_cnt);

        // abort all services
        server_exec.abort();
        client_exec.abort();
    }

    #[tokio::test]
    async fn test_e2e_request_flow_with_cache() {
        // init mock env
        init_test_env();

        // start server service
        let cache_repo = Arc::new(MockCacheRepo::new());
        let client_repo = Arc::new(MockClientRepo::new());
        let request_repo = Arc::new(MockRequestRepo::new());
        let response_repo = Arc::new(MockResponseRepo::new());
        let config_handler = Arc::new(MockConfigHandlerImpl::new());
        let cache_service = CacheService::new(
            cache_repo.clone(), config_handler.clone(), String::from(config_keys::CONFIG_KEY_SERVER_CACHE_CONFIGS));
        let server_exec = tokio::spawn(async move {
            server::run(
                server::config::ServerRequestConfig::new(
                    "127.0.0.1".to_string(),
                    3333, 
                    3334, 
                    0,
                    false,
                    false
                ), 
                cache_repo, 
                client_repo, 
                request_repo, 
                response_repo, 
                config_handler.clone()).await;
        });

        // delay for 2 seconds to wait the server to start up
        sleep(Duration::from_secs(2)).await;

        // the number of request processed by underlying service
        let success_real_cnt = Arc::new(StdMutex::new(0));
        // start client service
        let mock_response = String::from("pong");
        let mock_callback = Arc::new(StdMutex::new({
            let cnt = success_real_cnt.clone();
            move || {
                let mut counter = cnt.lock().unwrap();
                *counter += 1;
            }
        }));
        let underlying_repo = Arc::new(MockUnderlyingRepo::new(mock_response.clone(), mock_callback));
        let underlying_svc_address = String::from("This has no effect");
        let use_tls = false;

        // set client1 id
        env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_ID), "client1");

        let client_exec = tokio::spawn(async move {
            client::serve(underlying_svc_address, underlying_repo, use_tls).await;
        });

        // delay for 2 seconds to wait the client to start up
        sleep(Duration::from_secs(2)).await;

        // set cache config for "client1" on "/ping" with 5 seconds expiry duration
        cache_service.set_cache_config(CacheConfig::new(String::from("client1"), String::from("GET"), String::from("/ping"), 3)).await.unwrap();

        // test bulk hit to server public service,
        let request_cnt = 5;
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        let responses = Arc::new(Mutex::new(Vec::<String>::new()));
        // it should return the mock reponse client service
        for i in 0..request_cnt {
            let responses_clone = Arc::clone(&responses);
            let handle = task::spawn(async move {
                let url = if i % 2 == 0 { 
                    String::from("http://127.0.0.1:3333/client1/ping")
                } else {
                    String::from("http://127.0.0.1:3333/ping?trabas_client_id=client1")
                };
                match send_http_request(url, None).await {
                    Ok(res) => {
                        responses_clone.lock().await.push(res.text().await.unwrap());
                    },
                    Err(err) => {
                        _error!("Error getting response: {}", err);
                    }
                };
            });

            handles.push(handle);

            if i == 1 {
                // on second request delay for 2 seconds to pass the cache expiration
                sleep(Duration::from_secs(2)).await;
            } else {
                // delay for 20 ms, to wait first request to be cached
                sleep(Duration::from_millis(50)).await;
            }
        }

        for handle in handles {
            let _ = handle.await;
        }

        for response in responses.lock().await.iter() {
            assert_eq!(*response, mock_response);
        }

        let success_cnt = responses.lock().await.len();
        _info!("Request total (with cached) success count : {}", success_cnt);
        _info!("Request real success count                : {}", *(success_real_cnt.lock().unwrap()));
        assert!(success_cnt == request_cnt);
        assert!(*(success_real_cnt.lock().unwrap()) < success_cnt);

        // abort all services
        server_exec.abort();
        client_exec.abort();
    }

    #[tokio::test]
    async fn test_e2e_request_flow_with_client_id_cache_enabled() {
        // init mock env
        init_test_env();

        // start server service
        let cache_repo = Arc::new(MockCacheRepo::new());
        let client_repo = Arc::new(MockClientRepo::new());
        let request_repo = Arc::new(MockRequestRepo::new());
        let response_repo = Arc::new(MockResponseRepo::new());
        let config_handler = Arc::new(MockConfigHandlerImpl::new());
        let server_exec = tokio::spawn(async move {
            // enable the client id cache
            let cache_client_id = true;
            server::run(
                server::config::ServerRequestConfig::new(
                    "127.0.0.1".to_string(),
                    3333, 
                    3334, 
                    0,
                    cache_client_id,
                    false
                ), 
                cache_repo, 
                client_repo, 
                request_repo, 
                response_repo,
                config_handler).await;
        });

        // delay for 2 seconds to wait the server to start up
        sleep(Duration::from_secs(2)).await;

        // start client service
        let mock_response = String::from("pong");
        let mock_callback = Arc::new(StdMutex::new(|| {}));
        let underlying_repo = Arc::new(MockUnderlyingRepo::new(mock_response.clone(), mock_callback));
        let underlying_svc_address = String::from("This has no effect");
        let use_tls = false;

        // set client1 id
        env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_ID), "client1");

        let client_exec = tokio::spawn(async move {
            client::serve(underlying_svc_address, underlying_repo, use_tls).await;
        });

        // delay for 2 seconds to wait the client to start up
        sleep(Duration::from_secs(2)).await;

        // test bulk hit to server public service,
        let request_cnt = 5;
        let mut handles: Vec<JoinHandle<()>> = Vec::new();
        let responses = Arc::new(Mutex::new(Vec::<String>::new()));
        // it should return the mock reponse client service
        for i in 0..request_cnt {
            let responses_clone = Arc::clone(&responses);
            let handle = task::spawn(async move {
                // without specifying /client1 path
                let url = String::from("http://127.0.0.1:3333/ping");
                let cookies = HashMap::from([
                    ("trabas_client_id".to_string(), "client1".to_string())
                ]);
                match send_http_request(url, Some(cookies)).await {
                    Ok(res) => {
                        // check set cookie header
                        if let Some(set_cookie_headers) = res.headers().get(SET_COOKIE) {
                            assert!(set_cookie_headers.to_str().unwrap().contains("trabas_client_id=client1;"));
                        } else {
                            println!("No Set-Cookie headers found.");
                            assert!(false);
                        }
                        
                        responses_clone.lock().await.push(res.text().await.unwrap());
                    },
                    Err(err) => {
                        _error!("Error getting response: {}", err);
                    }
                };
            });

            handles.push(handle);

            if i == 1 {
                // on second request delay for 2 seconds to pass the cache expiration
                sleep(Duration::from_secs(2)).await;
            } else {
                // delay for 20 ms, to wait first request to be cached
                sleep(Duration::from_millis(50)).await;
            }
        }

        for handle in handles {
            let _ = handle.await;
        }

        for response in responses.lock().await.iter() {
            assert_eq!(*response, mock_response);
        }

        let success_cnt = responses.lock().await.len();
        _info!("Request total success count : {}", success_cnt);
        assert!(success_cnt == request_cnt);

        // abort all services
        server_exec.abort();
        client_exec.abort();
    }

    #[tokio::test]
    async fn test_e2e_request_flow_with_multiple_tunnels_with_single_client_id() {
        // init mock env
        init_test_env();

        // start server service
        let cache_repo = Arc::new(MockCacheRepo::new());
        let client_repo = Arc::new(MockClientRepo::new());
        let request_repo = Arc::new(MockRequestRepo::new());
        let response_repo = Arc::new(MockResponseRepo::new());
        let config_handler = Arc::new(MockConfigHandlerImpl::new());
        let server_exec = tokio::spawn(async move {
            server::run(
                server::config::ServerRequestConfig::new(
                    "127.0.0.1".to_string(),
                    3333, 
                    3334, 
                    0,
                    false,
                    true
                ), 
                cache_repo, 
                client_repo, 
                request_repo, 
                response_repo,
                config_handler).await;
        });

        // delay for 2 seconds to wait the server to start up
        sleep(Duration::from_secs(2)).await;

        // start client service
        let mock_response = String::from("pong");
        let underlying_repo1 = Arc::new(MockUnderlyingRepo::new(mock_response.clone(), Arc::new(StdMutex::new(|| {}))));
        let underlying_repo2 = Arc::new(MockUnderlyingRepo::new(mock_response.clone(), Arc::new(StdMutex::new(|| {}))));
        let underlying_repo3 = Arc::new(MockUnderlyingRepo::new(mock_response.clone(), Arc::new(StdMutex::new(|| {}))));
        let use_tls = false;

        // set client id
        env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_ID), "shared_client_id");

        // run 3 client tunnels
        let client_tunnel1_exec = tokio::spawn(async move {
            client::serve(String::from("The target underlying address, This has no effect"), underlying_repo1, use_tls).await;
        });

        // delay for 2 seconds to wait the client tunnel 1 to start up
        sleep(Duration::from_secs(2)).await;

        let client_tunnel2_exec = tokio::spawn(async move {
            client::serve(String::from("The target underlying address, This has no effect"), underlying_repo2, use_tls).await;
        });

        // delay for 2 seconds to wait the client tunnel 2 to start up
        sleep(Duration::from_secs(2)).await;

        let client_tunnel3_exec = tokio::spawn(async move {
            client::serve(String::from("The target underlying address, This has no effect"), underlying_repo3, use_tls).await;
        });

        // delay for 2 seconds to wait the client tunnel 3 to start up
        sleep(Duration::from_secs(2)).await;

        // test hit to server public service, 
        // it should return the mock reponse client service
        let url = String::from("http://127.0.0.1:3333/shared_client_id/ping");
        // disptach 20 requests
        let mut tunnel_ids: HashSet<String> = HashSet::new();
        for _ in 0..20 {
            let response = match send_http_request(url.clone(), None).await {
                Ok(res) => {
                    let tunnel_id = res.headers().get("trabas_tunnel_id")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or_default();
                    if !tunnel_id.is_empty() {
                        tunnel_ids.insert(tunnel_id.to_string());
                    }
                    res.text().await.unwrap()
                },
                Err(err) => {
                    println!("Error getting response: {}", err);
                    err
                }
            };

            assert_eq!(response, mock_response);
        }

        _info!("Tunnel IDs: {:?}", tunnel_ids);

        assert_eq!(tunnel_ids.len(), 3);

        // abort all services
        server_exec.abort();
        client_tunnel1_exec.abort();
        client_tunnel2_exec.abort();
        client_tunnel3_exec.abort();

        // TODO: add more relevant assertions
    }
}
