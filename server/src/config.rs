use std::{collections::HashMap, sync::Arc};
use std::{
    fs::{self, create_dir_all, File},
    io::Write,
    path::Path,
};

use common::_info;
use common::{
    config::*, 
    data::dto::cache_config::CacheConfig, 
    security::generate_hmac_key
};

use crate::{
    data::repository::cache_repo::{CacheRepo, CacheRepoProcMemImpl}, 
    service::cache_service::CacheService
};

use openssl::{
    asn1::Asn1Time,
    bn::{BigNum, MsbOption},
    hash::MessageDigest,
    nid::Nid,
    pkey::PKey,
    pkcs12::Pkcs12,
    rsa::Rsa,
    x509::{
        X509, X509NameBuilder, X509Req, X509ReqBuilder,
        extension::{BasicConstraints, ExtendedKeyUsage, KeyUsage, SubjectAlternativeName},
    },
};
use native_tls::Identity;
use std::path::PathBuf;

// additional keys for server configuration
pub mod ext_keys {
    pub const CLIENT_ID_COOKIE_KEY: &str = "trabas_client_id";
    pub const TUNNEL_ID_HEADER_KEY: &str = "trabas_tunnel_id";
}

#[derive(Debug, Clone)]
pub struct ServerRequestConfig {
    pub host: String, 
    pub public_port: u16, 
    pub client_port: u16,
    pub client_request_limit: u16,
    pub cache_client_id: bool,
    pub return_tunnel_id: bool,
    pub tls: bool,
}

impl ServerRequestConfig {
    pub fn new(
        host: String, 
        public_port: u16, 
        client_port: u16, 
        client_request_limit: u16, 
        cache_client_id: bool,
        return_tunnel_id: bool,
        tls: bool
    ) -> Self {
        ServerRequestConfig {
            host,
            public_port,
            client_port,
            client_request_limit,
            cache_client_id,
            return_tunnel_id,
            tls
        }
    }

    pub fn public_svc_address(&self) -> String {
        format!("{}:{}", self.host, self.public_port)
    }

    pub fn client_svc_address(&self) -> String {
        format!("{}:{}", self.host, self.client_port)
    }
}


// simple validation for config keys
pub fn validate_configs() -> HashMap<String, String> {
    let config = get_configs_from_proc_env();
    let use_redis_default = "false".to_string();
    let use_redis = *config.get(keys::CONFIG_KEY_SERVER_REDIS_ENABLE).unwrap_or(&use_redis_default) == "true".to_string();
    let mut required_keys = [
        keys::CONFIG_KEY_SERVER_SECRET,
    ].to_vec();
    if use_redis {
        // here we must define define redis configuration
        required_keys.extend([
            keys::CONFIG_KEY_SERVER_REDIS_HOST,
            keys::CONFIG_KEY_SERVER_REDIS_PORT,
            keys::CONFIG_KEY_SERVER_REDIS_PASS
        ])
    }
    for key in required_keys {
        if !config.contains_key(key) {
            panic!("{} config has not been set.", key)
        }
    }

    // returning required config values to the caller
    let mut res = HashMap::new();
    res.insert(keys::CONFIG_KEY_SERVER_REDIS_ENABLE.to_string(), if use_redis { "true".to_string() } else { "false".to_string() });

    res
}

pub fn generate_server_secret(force: bool) -> () {
    // check whether the secret is already set
    let config = get_configs_from_proc_env();
    if config.contains_key(keys::CONFIG_KEY_SERVER_SECRET) && !force {
        println!("Server Secret is already generated, please check it in the config file. Consider using --force option to force regenerating");
        return;
    }

    let key = generate_hmac_key(32);
    set_configs(HashMap::from([
        (String::from(keys::CONFIG_KEY_SERVER_SECRET), key.clone())
    ]));

    println!("Server Secret generated!");
    println!("Value: {}", key);
    println!("You may find the value later in the config file")
}

pub fn generate_ssl_keys(server_config_path: Option<String>, host: Option<String>, ip: Option<String>, force: bool) -> () {
    // TODO: add TOFU option, without CA
    // generate self-signed SSL keys and saved in trabas_config/ssl
    let base_config_dir = get_config_path();
    let ssl_dir = Path::new(&base_config_dir).join("ssl");

    if let Err(e) = create_dir_all(&ssl_dir) {
        println!("Failed to create ssl directory: {}", e);
        return;
    }

    let ca_key_path = ssl_dir.join("ca.key");
    let ca_crt_path = ssl_dir.join("ca.crt");
    let server_key_path = ssl_dir.join("server.key");
    let server_csr_path = ssl_dir.join("server.csr");
    let server_crt_path = ssl_dir.join("server.crt");

    let mut alt_dns: Vec<String> = vec!["localhost".to_string()];
    let mut alt_ips: Vec<String> = vec!["127.0.0.1".to_string()];

    match server_config_path.as_ref() {
        Some(p) => {
            if let Ok(content) = fs::read_to_string(p) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("DNS.") {
                        if let Some((_, v)) = trimmed.split_once('=') {
                            let val = v.trim();
                            if !val.is_empty() && !alt_dns.contains(&val.to_string()) {
                                alt_dns.push(val.to_string());
                            }
                        }
                    } else if trimmed.starts_with("IP.") {
                        if let Some((_, v)) = trimmed.split_once('=') {
                            let val = v.trim();
                            if !val.is_empty() && !alt_ips.contains(&val.to_string()) {
                                alt_ips.push(val.to_string());
                            }
                        }
                    }
                }
            } else {
                println!("Warning: Cannot read server conf at '{}', using defaults", p);
            }
        }
        None => {
            // host/ip if available
            if let Some(h) = &host { alt_dns = vec![h.clone()]; }
            if let Some(i) = &ip { alt_ips = vec![i.clone()]; }

            // write server.conf with default SANs
            let conf_path = ssl_dir.join("server.conf");
            let chosen_host = alt_dns.first().cloned().unwrap_or_else(|| "localhost".to_string());
            let chosen_ip = alt_ips.first().cloned().unwrap_or_else(|| "127.0.0.1".to_string());
            let default_conf = format!("[ v3_req ]\nkeyUsage = keyEncipherment, dataEncipherment\nextendedKeyUsage = serverAuth\nsubjectAltName = @alt_names\n\n[ alt_names ]\nDNS.1 = {}\nIP.1 = {}\n", chosen_host, chosen_ip);
            if force || !conf_path.exists() {
                if let Err(e) = fs::write(&conf_path, default_conf) {
                    println!("Failed to write default server.conf: {}", e);
                    return;
                } else {
                    println!("Default server.conf created at {}", conf_path.display());
                }
            }
        }
    }

    if !force && ca_key_path.exists() && ca_crt_path.exists() && server_key_path.exists() && server_csr_path.exists() && server_crt_path.exists() {
        println!("SSL keys already exist. Use --force to regenerate.");
        println!("Paths:\n- {}\n- {}\n- {}\n- {}\n- {}", ca_key_path.display(), ca_crt_path.display(), server_key_path.display(), server_csr_path.display(), server_crt_path.display());
        return;
    }

    // ================ Generate CA key ================
    let ca_rsa = match Rsa::generate(2048) { Ok(k) => k, Err(e) => { println!("Failed to generate CA key: {}", e); return; } };
    let mut ca_pkey = match PKey::from_rsa(ca_rsa) { Ok(k) => k, Err(e) => { println!("Failed to create CA PKey: {}", e); return; } };

    // ===== Generate CA certificate (self-signed) =====
    let mut ca_name = match X509NameBuilder::new() { Ok(b) => b, Err(e) => { println!("Failed to init CA subject: {}", e); return; } };
    ca_name.append_entry_by_nid(Nid::COUNTRYNAME, "US").ok();
    ca_name.append_entry_by_nid(Nid::STATEORPROVINCENAME, "State").ok();
    ca_name.append_entry_by_nid(Nid::LOCALITYNAME, "City").ok();
    ca_name.append_entry_by_nid(Nid::ORGANIZATIONNAME, "Organization").ok();
    ca_name.append_entry_by_nid(Nid::ORGANIZATIONALUNITNAME, "OrgUnit").ok();
    ca_name.append_entry_by_nid(Nid::COMMONNAME, "Example CA").ok();
    let ca_name = ca_name.build();

    let mut ca_builder = match X509::builder() { Ok(b) => b, Err(e) => { println!("Failed to init CA cert builder: {}", e); return; } };
    ca_builder.set_version(2).ok();
    
    let mut serial = BigNum::new().unwrap();
    serial.rand(128, MsbOption::MAYBE_ZERO, false).ok();
    
    let serial = serial.to_asn1_integer().unwrap();
    ca_builder.set_serial_number(&serial).ok();
    ca_builder.set_subject_name(&ca_name).ok();
    ca_builder.set_issuer_name(&ca_name).ok();
    ca_builder.set_pubkey(&ca_pkey).ok();
    ca_builder.set_not_before(&Asn1Time::days_from_now(0).unwrap()).ok();
    ca_builder.set_not_after(&Asn1Time::days_from_now(3650).unwrap()).ok();

    // append extensions
    if let Ok(ext) = BasicConstraints::new().critical().ca().build() {
        ca_builder.append_extension(ext).ok();
    }
    if let Ok(ext) = KeyUsage::new().key_cert_sign().crl_sign().build() {
        ca_builder.append_extension(ext).ok();
    }

    ca_builder.sign(&ca_pkey, MessageDigest::sha256()).ok();
    let mut ca_cert = ca_builder.build();

    // if CA already exists on disk and not forced, reuse it to avoid mismatch with server cert
    if !force && ca_key_path.exists() && ca_crt_path.exists() {
        if let (Ok(key_bytes), Ok(crt_bytes)) = (fs::read(&ca_key_path), fs::read(&ca_crt_path)) {
            if let (Ok(parsed_key), Ok(parsed_cert)) = (
                PKey::private_key_from_pem(&key_bytes),
                X509::from_pem(&crt_bytes)
            ) {
                ca_pkey = parsed_key;
                ca_cert = parsed_cert;
            }
        }
    }

    // ============== Generate Server key ==============
    let sv_rsa = match Rsa::generate(2048) { Ok(k) => k, Err(e) => { println!("Failed to generate Server key: {}", e); return; } };
    let sv_pkey = match PKey::from_rsa(sv_rsa) { Ok(k) => k, Err(e) => { println!("Failed to create Server PKey: {}", e); return; } };

    // ================= Generate CSR ==================
    let mut req_builder = match X509ReqBuilder::new() { Ok(b) => b, Err(e) => { println!("Failed to init CSR builder: {}", e); return; } };
    let mut sv_name = match X509NameBuilder::new() { Ok(b) => b, Err(e) => { println!("Failed to init Server subject: {}", e); return; } };
    sv_name.append_entry_by_nid(Nid::COUNTRYNAME, "US").ok();
    sv_name.append_entry_by_nid(Nid::STATEORPROVINCENAME, "State").ok();
    sv_name.append_entry_by_nid(Nid::LOCALITYNAME, "City").ok();
    sv_name.append_entry_by_nid(Nid::ORGANIZATIONNAME, "Organization").ok();
    sv_name.append_entry_by_nid(Nid::ORGANIZATIONALUNITNAME, "OrgUnit").ok();
    // set CN from chosen DNS, fallback to localhost
    let chosen_cn = alt_dns.first().cloned().unwrap_or_else(|| "localhost".to_string());
    sv_name.append_entry_by_nid(Nid::COMMONNAME, &chosen_cn).ok();
    let sv_name = sv_name.build();

    req_builder.set_subject_name(&sv_name).ok();
    req_builder.set_pubkey(&sv_pkey).ok();
    req_builder.sign(&sv_pkey, MessageDigest::sha256()).ok();
    let csr: X509Req = req_builder.build();

    // ======== Sign Server certificate with CA ========
    let mut sv_builder = match X509::builder() { Ok(b) => b, Err(e) => { println!("Failed to init Server cert builder: {}", e); return; } };
    sv_builder.set_version(2).ok();

    let mut serial = BigNum::new().unwrap();
    serial.rand(128, MsbOption::MAYBE_ZERO, false).ok();
    
    let serial = serial.to_asn1_integer().unwrap();
    sv_builder.set_serial_number(&serial).ok();
    sv_builder.set_subject_name(csr.subject_name()).ok();
    sv_builder.set_issuer_name(ca_cert.subject_name()).ok();
    sv_builder.set_pubkey(&sv_pkey).ok();
    sv_builder.set_not_before(&Asn1Time::days_from_now(0).unwrap()).ok();
    sv_builder.set_not_after(&Asn1Time::days_from_now(365).unwrap()).ok();

    // append extensions
    {
        if let Ok(ext) = BasicConstraints::new().critical().build() {
            if sv_builder.append_extension(ext).is_err() { println!("Warning: failed to append BasicConstraints"); }
        }
    }
    {
        if let Ok(ext) = KeyUsage::new().digital_signature().key_encipherment().build() {
            if sv_builder.append_extension(ext).is_err() { println!("Warning: failed to append KeyUsage"); }
        }
    }
    {
        if let Ok(ext) = ExtendedKeyUsage::new().server_auth().build() {
            if sv_builder.append_extension(ext).is_err() { println!("Warning: failed to append ExtendedKeyUsage"); }
        }
    }
    {
        let ctx = sv_builder.x509v3_context(Some(&ca_cert), None);
        let mut san_b = SubjectAlternativeName::new();
        for d in &alt_dns { san_b.dns(d); }
        for ip in &alt_ips { san_b.ip(ip); }
        if let Ok(ext) = san_b.build(&ctx) {
            if sv_builder.append_extension(ext).is_err() { println!("Warning: failed to append SubjectAlternativeName"); }
        }
    }

    sv_builder.sign(&ca_pkey, MessageDigest::sha256()).ok();
    let sv_cert = sv_builder.build();

    // write outputs
    if let Err(e) = write_bytes(&ca_key_path, &ca_pkey.private_key_to_pem_pkcs8().unwrap(), force) { println!("{}", e); return; }
    if let Err(e) = write_bytes(&ca_crt_path, &ca_cert.to_pem().unwrap(), force) { println!("{}", e); return; }
    if let Err(e) = write_bytes(&server_key_path, &sv_pkey.private_key_to_pem_pkcs8().unwrap(), force) { println!("{}", e); return; }
    if let Err(e) = write_bytes(&server_csr_path, &csr.to_pem().unwrap(), force) { println!("{}", e); return; }
    if let Err(e) = write_bytes(&server_crt_path, &sv_cert.to_pem().unwrap(), force) { println!("{}", e); return; }

    println!("SSL Keys generated!\nPaths:\n- {}\n- {}\n- {}\n- {}\n- {}",
        ca_key_path.display(), ca_crt_path.display(), server_key_path.display(), server_csr_path.display(), server_crt_path.display()
    );
}

fn write_bytes(path: &Path, content: &[u8], force: bool) -> Result<(), String> {
    if path.exists() && !force { return Ok(()); }
    let mut f = File::create(path).map_err(|e| format!("Failed to create {}: {}", path.display(), e))?;
    f.write_all(content).map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
    Ok(())
}

pub fn get_server_identity_from_pem() -> Result<Identity, String> {
    let base = common::config::get_config_path();
    let ssl_dir = PathBuf::from(base).join("ssl");
    let cert_path = ssl_dir.join("server.crt");
    let key_path = ssl_dir.join("server.key");

    _info!("cert_path: {}", cert_path.display());
    _info!("key_path: {}", key_path.display());

    let cert_bytes = std::fs::read(cert_path).map_err(|e| format!("read server.crt: {}", e))?;
    let key_bytes = std::fs::read(key_path).map_err(|e: std::io::Error| format!("read server.key: {}", e))?;

    let cert = X509::from_pem(&cert_bytes).map_err(|e| format!("parse cert pem: {}", e))?;
    let pkey = PKey::private_key_from_pem(&key_bytes).map_err(|e| format!("parse key pem: {}", e))?;

    // no CA chain provided here; self-signed or signed by local CA
    let mut builder: openssl::pkcs12::Pkcs12Builder = Pkcs12::builder();
    builder.name("trabas");
    builder.pkey(&pkey);
    builder.cert(&cert);
    let pkcs12 = builder.build2(" ").map_err(|e| format!("build pkcs12: {}", e))?; // empty password with a space to avoid empty pass issue on some platforms
    let der = pkcs12.to_der().map_err(|e| format!("pkcs12 to der: {}", e))?;
    
    Identity::from_pkcs12(&der, " ").map_err(|e| format!("identity from pkcs12: {}", e))
}


pub fn set_server_configs(
    key: Option<String>,
    redis_enable: Option<String>,
    redis_host: Option<String>,
    redis_port: Option<String>,
    redis_pass: Option<String>,
    public_endpoint: Option<String>,
    public_request_timeout: Option<String>,
    force: bool,
) -> () {
    let config = get_configs_from_proc_env();
    let mut config_to_set = HashMap::new();

    #[derive(PartialEq, Copy, Clone)]
    enum ValueType { Int }
    let key_types: HashMap<&str, ValueType> = [
        (keys::CONFIG_KEY_SERVER_REDIS_PORT, ValueType::Int),
        (keys::CONFIG_KEY_SERVER_PUBLIC_REQUEST_TIMEOUT, ValueType::Int),
        // TODO: add more types as needed
    ].iter().map(|(k, v)| (*k, *v)).collect();

    let config_options = [
        (redis_enable, keys::CONFIG_KEY_SERVER_REDIS_ENABLE, "Redis enable flag"),
        (key, keys::CONFIG_KEY_SERVER_SECRET, "Server secret"),
        (redis_host, keys::CONFIG_KEY_SERVER_REDIS_HOST, "Redis Host"),
        (redis_port, keys::CONFIG_KEY_SERVER_REDIS_PORT, "Redis Port"),
        (redis_pass, keys::CONFIG_KEY_SERVER_REDIS_PASS, "Redis Pass"),
        (public_endpoint, keys::CONFIG_KEY_SERVER_PUBLIC_ENDPOINT, "Public Endpoint"),
        (public_request_timeout, keys::CONFIG_KEY_SERVER_PUBLIC_REQUEST_TIMEOUT, "Public Request Timeout"),
    ];

    for (opt, key_str, msg) in config_options.iter() {
        if let Some(val) = opt {
            // validate the type
            match key_types.get(key_str) {
                Some(ValueType::Int) => {
                    if val.parse::<u32>().is_err() {
                        println!("{msg} must be an integer value.");
                        return;
                    }
                }
                // TODO: might add boolean
                _ => {}
            }
            if config.contains_key(*key_str) && !force {
                println!("{msg} is already set, please check it in the config file. Consider using --force option to force resetting");
                return;
            }
            config_to_set.insert(String::from(*key_str), val.clone());
        }
    }

    set_configs(config_to_set);

    println!("Server Configurations have been set!");
    println!("You may find the value later again in the config file");
}

// Cache Configs
pub fn get_cache_service(
        cache_repo: Arc<dyn CacheRepo + Send + Sync>, 
        config_handler: Arc<dyn ConfigHandler + Send + Sync>) -> CacheService {
    let cache_service = CacheService::new(
        cache_repo, 
        config_handler,
        String::from(keys::CONFIG_KEY_SERVER_CACHE_CONFIGS)
    );

    cache_service
}

fn get_cache_service_for_settings() -> CacheService {
    validate_configs();
    // use in process memo repo since we don't really need this
    // TODO: might consider separate cache & cache config in different services
    let cache_repo = Arc::new(CacheRepoProcMemImpl::new());
    let config_handler = Arc::new(ConfigHandlerImpl{});
    
    get_cache_service(cache_repo, config_handler)
}

pub async fn set_cache_config(client_id: String, method: String, path: String, exp_duration: u32) {
    let cache_service = get_cache_service_for_settings();
    cache_service.set_cache_config(CacheConfig::new(client_id.clone(), method.clone(), path.clone(), exp_duration)).await.unwrap();

    println!("Cache config has been set (Client ID: {}, Method: {}, Path: {}, Duration: {} seconds)", client_id, method, path, exp_duration);
}

pub async fn remove_cache_config(client_id: String, method: String, path: String) {
    let cache_service = get_cache_service_for_settings();

    cache_service.remove_cache_config(client_id.clone(), method.clone(), path.clone()).await.unwrap();

    println!("Cache config has been unset (Client ID: {}, Method: {}, Path: {})", client_id, method, path);
}

pub async fn show_cache_config() {
    let cache_service = get_cache_service_for_settings();

    cache_service.show_cache_config().await.unwrap();
}
