#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, HashSet},
        env,
        sync::{Arc, Mutex as StdMutex, Arc as StdArc, Once},
        time::Duration,
    };
    use env_logger::{Builder, Env, Target};
    use log::{Level, Metadata, Record, SetLoggerError};
    use reqwest::{
        header::{HeaderMap, HeaderValue, COOKIE, SET_COOKIE},
        Client, Response,
    };
    use tokio::{
        sync::Mutex,
        task::{self, JoinHandle},
        time::sleep,
    };
    use common::{
        _error, _info,
        config::keys as config_keys,
        data::dto::cache_config::CacheConfig,
        version::set_root_version,
    };
    use trabas::mocks::{
        client::mock_underlying_repo::MockUnderlyingRepo,
        config::MockConfigHandlerImpl,
        server::{
            mock_cache_repo::MockCacheRepo,
            mock_client_repo::MockClientRepo,
            mock_request_repo::MockRequestRepo,
            mock_response_repo::MockResponseRepo,
        },
    };
    use trabas::PROJECT_VERSION;
    use server::service::cache_service::CacheService;

    async fn send_http_request(url: String, cookies: Option<HashMap<String, String>>) -> Result<Response, String> {
        send_http_request_with_timeout(url, cookies, Duration::from_secs(60)).await
    }

    async fn send_http_request_with_timeout(url: String, cookies: Option<HashMap<String, String>>, timeout: Duration) -> Result<Response, String> {
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| format!("Failed to build client: {}", e))?;
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
            set_root_version(PROJECT_VERSION);
            // mock env/config
            let server_secret = "74657374696e676b6579313233343536";
            env::set_var(String::from(config_keys::CONFIG_KEY_SERVER_SECRET), server_secret);
            env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_SERVER_HOST), "127.0.0.1");
            env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_SERVER_PORT), "3334");
            env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY), server_secret);
            env::set_var(String::from(config_keys::CONFIG_KEY_GLOBAL_DEBUG), "true");

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
        let underlying_repo4 = Arc::new(MockUnderlyingRepo::new(mock_response.clone(), Arc::new(StdMutex::new(|| {}))));
        let use_tls = false;

        // set client id
        env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_ID), "shared_client_id");

        // run 3 client tunnels
        let client_tunnel1_exec = tokio::spawn(async move {
            client::serve(String::from("The target underlying address, This has no effect"), underlying_repo1, use_tls).await;
        });
        let client_tunnel2_exec = tokio::spawn(async move {
            client::serve(String::from("The target underlying address, This has no effect"), underlying_repo2, use_tls).await;
        });
        let client_tunnel3_exec  = tokio::spawn(async move {
            client::serve(String::from("The target underlying address, This has no effect"), underlying_repo3, use_tls).await;
        });
        let client_tunnel4_exec = tokio::spawn(async move {
            client::serve(String::from("The target underlying address, This has no effect"), underlying_repo4, use_tls).await;
        });

        // delay for 5 seconds for all tunnels to start up
        sleep(Duration::from_secs(5)).await;

        // test hit to server public service, 
        // it should return the mock reponse client service
        let url = String::from("http://127.0.0.1:3333/shared_client_id/ping");
        // disptach 20 requests
        let mut tunnel_ids: HashSet<String> = HashSet::new();
        let request_cnt = 20;
        for _ in 0..request_cnt {
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

        assert_eq!(tunnel_ids.len(), 4);

        // abort all services
        server_exec.abort();
        client_tunnel1_exec.abort();
        client_tunnel2_exec.abort();
        client_tunnel3_exec.abort();
        client_tunnel4_exec.abort();

        // TODO: add more relevant assertions
    }

    #[tokio::test]
    async fn test_e2e_request_flow_with_incompatible_client_version() {
        // setup test logger capture
        let logs = StdArc::new(StdMutex::new(Vec::new()));
        let _ = set_test_logger(logs.clone());

        // common envs
        let server_secret = "74657374696e676b6579313233343536";
        env::set_var(String::from(config_keys::CONFIG_KEY_SERVER_SECRET), server_secret);
        env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_SERVER_HOST), "127.0.0.1");
        env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_SERVER_PORT), "3334");
        env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_SERVER_SIGNING_KEY), server_secret);

        // set server verison config
        std::env::set_var("TEST_MIN_CLIENT_VERSION", "1.0.0");
        std::env::set_var("TEST_SERVER_VERSION", "1.0.0");

        // server service
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
                    false
                ),
                cache_repo,
                client_repo,
                request_repo,
                response_repo,
                config_handler
            ).await;
        });

        // delay for 2 seconds to wait the server to start up
        sleep(Duration::from_secs(2)).await;

        // patch the client version to be lower than required
        env::set_var("TEST_CLIENT_VERSION", "0.9.9");
        env::set_var("TEST_MIN_SERVER_VERSION", "1.0.0");
        env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_ID), "client_incompatible1");

        // start the client services
        let mock_response = String::from("pong");
        let underlying_repo1 = Arc::new(MockUnderlyingRepo::new(mock_response.clone(), Arc::new(StdMutex::new(|| {}))));
        let underlying_repo2 = underlying_repo1.clone();
        let underlying_repo3 = underlying_repo1.clone();
        let use_tls = false;
        let client1_exec = tokio::spawn(async move {
            client::serve(String::from("This has no effect"), underlying_repo1, use_tls).await;
        });

        // attempt to hit public endpoint
        let url = String::from("http://127.0.0.1:3333/client_incompatible2/ping");
        let response = send_http_request(url.clone(), None).await;
        assert!(response.is_err(), "Expected error response, got: {:?}", response);
        assert_eq!(response.unwrap_err(), "Invalid status code");

        // delay for 1 seconds
        sleep(Duration::from_secs(1)).await;

        // now, we simulate if the min service version is larger than the server version
        env::set_var("TEST_CLIENT_VERSION", "1.0.0");
        env::set_var("TEST_MIN_SERVER_VERSION", "1.0.1");
        env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_ID), "client_incompatible2");

        let client2_exec = tokio::spawn(async move {
            client::serve(String::from("This has no effect"), underlying_repo2, use_tls).await;
        });

        // delay for 1 seconds
        sleep(Duration::from_secs(1)).await;

        // second attempt still fails
        let url = String::from("http://127.0.0.1:3333/client_incompatible2/ping");
        let response = send_http_request(url.clone(), None).await;
        assert!(response.is_err(), "Expected error response, got: {:?}", response);
        assert_eq!(response.unwrap_err(), "Invalid status code");

        // reset the version codes, this will fallback to the constants in each module
        env::set_var("TEST_MIN_CLIENT_VERSION", "");
        env::set_var("TEST_SERVER_VERSION", "");
        env::set_var("TEST_CLIENT_VERSION", "");
        env::set_var("TEST_MIN_SERVER_VERSION", "");
        env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_ID), "client_compatible");

        let client3_exec = tokio::spawn(async move {
            client::serve(String::from("This has no effect"), underlying_repo3, use_tls).await;
        });

        // final delay
        sleep(Duration::from_secs(2)).await;

        // this one must be successful, as the client version is now compatible
        let url = String::from("http://127.0.0.1:3333/client_compatible/ping");
        let response = send_http_request(url.clone(), None).await;
        assert!(response.is_ok(), "Expected successful response, got: {:?}", response);

        let logs = logs.lock().unwrap();
        if !logs.is_empty() {
            // this should be triggered when the test is run individually
            // since the log capture is set up
            let find = vec![
                "Version mismatch: Server version code = 1.0.0 (required ≥ 1.0.0) | Client version code = 0.9.9 (required ≥ 1.0.0).",
                "Version mismatch: Server version code = 1.0.0 (required ≥ 1.0.1) | Client version code = 1.0.0 (required ≥ 1.0.0).",
                "Successfully authenticated and registered with the server service."
            ];
            for log in find {
                assert!(logs.iter().any(|l| l.contains(log)));
            }
        }

        // abort all services
        server_exec.abort();
        client1_exec.abort();
        client2_exec.abort();
        client3_exec.abort();
    }
    
    struct TestLogger {
        logs: StdArc<StdMutex<Vec<String>>>,
    }
    
    impl log::Log for TestLogger {
        fn enabled(&self, metadata: &Metadata) -> bool {
            metadata.level() <= Level::Info
        }
        fn log(&self, record: &Record) {
            use chrono::Utc;
            let now = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
            let formatted = format!(
                "{} {} {}: {}",
                now,
                record.level(),
                record.target(),
                record.args()
            );
            println!("{}", formatted);
            if self.enabled(record.metadata()) {
                let mut logs = self.logs.lock().unwrap();
                logs.push(formatted);
            }
        }
        fn flush(&self) {}
    }
    
    fn set_test_logger(logs: StdArc<StdMutex<Vec<String>>>) -> Result<(), SetLoggerError> {
        let logger = Box::leak(Box::new(TestLogger { logs }));
        log::set_logger(logger).map(|()| log::set_max_level(log::LevelFilter::Info))
    }

    #[tokio::test]
    async fn test_e2e_request_flow_with_client_disconnect_stop_signal() {
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

        let slow_mock_response = String::from("slow_pong");
        
        // simulate a slow underlying repository
        struct SlowMockUnderlyingRepo {
            mock_response: String,
        }
        
        impl SlowMockUnderlyingRepo {
            fn new(mock_response: String) -> Self {
                Self { mock_response }
            }
        }
        
        #[async_trait::async_trait]
        impl client::data::repository::underlying_repo::UnderlyingRepo for SlowMockUnderlyingRepo {
            async fn forward(&self, _: Vec<u8>, _: String) -> Result<Vec<u8>, String> {
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                
                if let Ok(res) = common::net::http_string_response_as_bytes(self.mock_response.clone(), http::StatusCode::from_u16(200).unwrap()) {
                    return Ok(res);
                }
        
                Err(String::from("An error occurred"))
            }

            async fn test_connection(&self, _: String) -> Result<(), String> {
                // always return ok for mock
                Ok(())
            }
        }

        let slow_underlying_repo = Arc::new(SlowMockUnderlyingRepo::new(slow_mock_response.clone()));

        // start client service with slow response
        env::set_var(String::from(config_keys::CONFIG_KEY_CLIENT_ID), "slow_client_test");
        let client_exec = tokio::spawn(async move {
            client::serve(String::from("The target underlying address, This has no effect"), slow_underlying_repo, false).await;
        });

        // wait for client to start
        sleep(Duration::from_secs(2)).await;

        let start_time = std::time::Instant::now();
        
        // simulate immediate client disconnect
        let request_task = tokio::spawn(async move {
            let stream = tokio::net::TcpStream::connect("127.0.0.1:3333").await.unwrap();
            let request = format!(
                "GET /slow_client_test/ping HTTP/1.1\r\n\
                 Host: 127.0.0.1:3333\r\n\
                 Connection: keep-alive\r\n\r\n"
            );
    
            use tokio::io::AsyncWriteExt;
            let (read_half, mut write_half) = tokio::io::split(stream);
            
            write_half.write_all(request.as_bytes()).await.unwrap();
            
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            drop(write_half);
            drop(read_half);
        });


        let _ = request_task.await;
        let elapsed = start_time.elapsed();
        assert!(elapsed.as_secs() <= 3);
        
        // note that there's no asserttion for stop signal, but we should see these logs:
        // - "Client connection has been closed for request: [request id]"
        // -  "Signal to stop received."

        sleep(Duration::from_secs(1)).await;

        // abort services
        server_exec.abort();
        client_exec.abort();
    }
}
