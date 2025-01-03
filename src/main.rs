use hyper::{Client, Request, Method, Body};
use hyper::client::HttpConnector;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use redis::RedisError;
use redis::aio::Connection;
use serde::Deserialize;
use tokio::time;
use std::fs::File;
use std::io::Read;
use std::time::Duration;
use std::env;
use base64::{Engine as _, engine::general_purpose};

fn empty_string() -> String {
    "".to_string()
}

#[derive(Debug, Deserialize)]
struct Config {
    redis: RedisConfig,
    loki: LokiConfig,
}

#[derive(Debug, Deserialize)]
struct RedisConfig {
    host: String,
    #[serde(default = "empty_string")]
    username: String,
    #[serde(default = "empty_string")]
    password: String,
    db: Option<u8>,
    key: String,
}

#[derive(Debug, Deserialize)]
struct LokiConfig {
    url: String,
    #[serde(default = "empty_string")]
    username: String,
    #[serde(default = "empty_string")]
    password: String
}

impl RedisConfig {

    pub fn to_dsn(&self) -> String {
        let dsn = if self.username != "" || self.password != "" {
            format!("redis://{}:{}@{}/", self.username, self.password, self.host)
        } else {
            format!("redis://{}/", self.host)
        };
        dsn
    }

}

type HttpClient = Client<HttpsConnector<HttpConnector>>;

fn parse_config() -> Config {
    let config_path = env::current_dir().expect("无法获取当前工作目录").join("config.yaml");

    let config = if config_path.exists() {
        let path_str = config_path.to_str().unwrap();
        let mut file = File::open(path_str).expect("打开配置文件失败");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("读取配置文件失败");
        serde_yaml::from_str(&contents).expect("配置文件解析失败")
    } else {
        println!("{:?}", env::vars());

        Config {
            redis: RedisConfig {
                host: env::var("REDIS_HOST").expect("No found env REDIS_HOST."),
                username: env::var("REDIS_USERNAME").unwrap_or_else(|_| "".to_string()),
                password: env::var("REDIS_PASSWORD").unwrap_or_else(|_| "".to_string()),
                db: env::var("REDIS_DB").unwrap_or("0".to_string()).parse().ok(),
                key: env::var("REDIS_KEY").unwrap_or_else(|_| "loki_push_queue".to_string())
            },
            loki: LokiConfig {
                url: env::var("LOKI_URL").unwrap_or_else(|_| "http://127.0.0.1:3100/loki/api/v1/push".to_string()),
                username: env::var("LOKI_USERNAME").unwrap_or_else(|_| "".to_string()),
                password: env::var("LOKI_PASSWORD").unwrap_or_else(|_| "".to_string()),
            }
        }
    };

    config
}

async fn get_redis_connection(config: &RedisConfig) -> Result<Connection, RedisError> {
    // connect to redis
    let client = redis::Client::open(config.to_dsn())?;
    let mut con = client.get_async_connection().await?;

    if let Some(db) = config.db {
        println!("Use redis db {}", db);
        redis::cmd("SELECT")
            .arg(db)
            .query_async::<_, ()>(&mut con)
            .await?;
    }

    Ok(con)
}

fn get_http_client() -> HttpClient {
    // let client = Client::new();
    // let https = HttpsConnector::new();

    let https = HttpsConnectorBuilder::new()
        .with_native_roots()
        .unwrap()
        .https_or_http()
        .enable_http1()
        .build();

    let client = Client::builder().build::<_, hyper::Body>(https);
    client
}

async fn push_log(http: &HttpClient, loki: &LokiConfig, log: String, auth: &Option<String>) -> Result<(), hyper::Error> {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(&loki.url)
        .header("content-type", "application/json");

    if let Some(auth_str) = auth {
        builder = builder.header("Authorization", auth_str);
    }

    let req = builder.body(Body::from(log)).unwrap();
    let res = http.request(req).await?;

    println!("{}", res.status());
    let body = res.into_body();
    let bytes = hyper::body::to_bytes(body).await.unwrap();
    println!("{}", String::from_utf8_lossy(&bytes));

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = parse_config();

    let mut redis_conn_result = get_redis_connection(&config.redis).await;

    let mut con = match redis_conn_result {
        Ok(r) => r,
        Err(e) => panic!("Connection to redis error: {}", e)
    };

    let http = get_http_client();

    // authorization for loki api
    let auth = if config.loki.username != "" || config.loki.password != "" {
        let mut b64 = general_purpose::STANDARD.encode(format!("{}:{}", config.loki.username, config.loki.password));
        b64.insert_str(0, "Basic ");
        Some(b64)
    } else {
        None
    };

    'pusher: loop {
        let pop_result: Result<Option<(String, String)>, RedisError> = redis::cmd("BRPOP")
            .arg(&config.redis.key)
            .arg(5)
            .query_async(&mut con)
            .await;

        match pop_result {
            Ok(p) => {
                if let Some((k, v)) = p {
                    println!("BRPOP: {} {}", k, v);

                    let pushed = push_log(&http, &config.loki, v, &auth).await;

                    if let Err(err) = pushed {
                        println!("Push to loki error: {:}", err);
                    }

                    // let uri = "https://example.com/".parse().unwrap();
                    // let res = http.get(uri).await.unwrap();
                    // println!("{}", res.status());
                    // println!("{:?}", res.headers());
                    // let body = res.into_body();
                    // let bytes = hyper::body::to_bytes(body).await.unwrap();
                    // println!("{}", String::from_utf8_lossy(&bytes));
                } else {
                    println!("BRPOP None");
                }
            },
            Err(e) => {
                println!("Redis pop error: {}, try to reconnect", e);

                // Reconnect to redis
                loop {
                    redis_conn_result = get_redis_connection(&config.redis).await;

                    match redis_conn_result {
                        Ok(r) => {
                            con = r;
                            println!("Connect to redis success.");
                            continue 'pusher;
                        },
                        Err(e) => {
                            println!("Connect to redis failed: {}, try to reconnect..", e);
                            time::sleep(Duration::from_secs(2)).await;
                            continue;
                        }
                    };
                }
            }
        };
    }

    // Ok(())
}
