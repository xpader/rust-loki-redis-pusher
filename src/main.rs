use hyper::{Client, Request, Method, Body};
use hyper::client::HttpConnector;
// use hyper_tls::HttpsConnector;
use redis::RedisError;
use redis::aio::Connection;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;

#[derive(Debug, Deserialize)]
struct Config {
    redis: RedisConfig,
    loki: LokiConfig,
}

#[derive(Debug, Deserialize)]
struct RedisConfig {
    host: String,
    db: Option<u8>,
    key: String,
}

#[derive(Debug, Deserialize)]
struct LokiConfig {
    url: String,
}

fn parse_config() -> Config {
    // 打开文件并读取内容
    let mut file = File::open("config.yaml").expect("打开配置文件失败");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("读取配置文件失败");

    // 解析YAML
    let config: Config = serde_yaml::from_str(&contents).expect("配置文件解析失败");

    config
}

async fn get_redis_connection(config: &RedisConfig) -> Result<Connection, RedisError> {
    // connect to redis
    let client = redis::Client::open(format!("redis://{}/", config.host))?;
    let mut con = client.get_async_connection().await?;

    let _: () = redis::cmd("SELECT")
        .arg(config.db.unwrap_or(0))
        .query_async(&mut con)
        .await?;

    Ok(con)
}

fn get_http_client() -> Client<HttpConnector> {
    let client = Client::new();
    // let https = HttpsConnector::new();
    // let client = Client::builder().build::<_, hyper::Body>(https);
    client
}

async fn push_log(http: &Client<HttpConnector>, url: &String, log: String) -> Result<(), hyper::Error> {
    // let uri = "https://m.doustar.cn/".parse().unwrap();
    // let uri = url.clone().parse().unwrap();

    let req = Request::builder()
        .method(Method::POST)
        .uri(url)
        .header("content-type", "application/json")
        .body(Body::from(log)).unwrap();

    // let res = http.get(uri).await.unwrap();
    let res = http.request(req).await?;

    println!("{}", res.status());
    // println!("{:?}", res.headers());
    let body = res.into_body();
    let bytes = hyper::body::to_bytes(body).await.unwrap();
    println!("{}", String::from_utf8_lossy(&bytes));

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = parse_config();

    let mut r = get_redis_connection(&config.redis).await?;
    let http = get_http_client();

    loop {
        let p: Option<(String, String)> = redis::cmd("BRPOP")
            .arg(&config.redis.key)
            .arg(5)
            .query_async(&mut r)
            .await?;

        if let Some((k, v)) = p {
            println!("BRPOP: {} {}", k, v);

            push_log(&http, &config.loki.url, v).await?;
            // let uri = "https://m.doustar.cn/".parse().unwrap();

            // let res = http.get(uri).await.unwrap();
            // println!("{}", res.status());
            // println!("{:?}", res.headers());
            // let body = res.into_body();
            // let bytes = hyper::body::to_bytes(body).await.unwrap();
            // println!("{}", String::from_utf8_lossy(&bytes));
        } else {
            println!("BRPOP None");
        }
    }

    // http_request().await;

    // Ok(())
}