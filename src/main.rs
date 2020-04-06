mod common;
mod config;
mod controller;
mod middleware;
mod model;
mod router;
mod util;
use actix_cors::Cors;
use actix_web::{get, http::header, middleware::Logger, App, HttpRequest, HttpServer, Responder};
use config::AppConf;
use lazy_static::lazy_static;
use log::info;
use middleware::access_cnt;
use middleware::login_auth;
use mongodb::{options::ClientOptions, Client as MongoClient};
use redis::Client;

pub const ACCESS_CNT: &str = "blog_access_cnt";

lazy_static! {
    pub static ref GLOBAL_CONF: AppConf = AppConf::new("conf/app.toml");
    pub static ref REDIS: Client = {
        let redis_address = format!(
            "redis://{}:{}",
            GLOBAL_CONF.redis.ip, GLOBAL_CONF.redis.port
        );
        redis::Client::open(redis_address.as_str()).unwrap()
    };
    pub static ref MONGO: MongoClient = {
        let client_options = ClientOptions::parse(&format!(
            "mongodb://{}:{}",
            GLOBAL_CONF.mongo.ip, GLOBAL_CONF.mongo.port
        ))
        .expect("Failed new mongo options");
        MongoClient::with_options(client_options).expect("Failed to initialize standalone client.")
    };
}

// 共享数据
pub struct GlobalData {
    redis_client: Client,
}

fn init_logger() {
    use chrono::Local;
    use std::io::Write;

    let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info");
    // 设置日志打印格式
    env_logger::Builder::from_env(env)
        .format(|buf, record| {
            writeln!(
                buf,
                "{} {} [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.module_path().unwrap_or("<unnamed>"),
                &record.args()
            )
        })
        .init();
    info!("env_logger initialized.");
}

#[get("/greet")]
async fn greet(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}", &name)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    init_logger();
    info!("hello world");
    let app_conf = AppConf::new("conf/app.toml");
    let binding_address = format!("{}:{}", "0.0.0.0", app_conf.server.port.unwrap_or(80u16));
    let server = HttpServer::new(move || {
        let redis_address = format!("redis://{}:{}", app_conf.redis.ip, app_conf.redis.port);
        let redis_client = redis::Client::open(redis_address.as_str()).unwrap();
        let global_data = GlobalData {
            redis_client: redis_client.clone(),
        };
        App::new()
            .data(global_data)
            .wrap(login_auth::LoginAuthMid::new(
                vec!["/admin/*".to_string()],
                vec!["/admin/*".to_string()],
            ))
            .wrap(access_cnt::AccessCnt::new(redis_client.clone()))
            // 设置response header ，解决跨域问题
            // 注意这个wrap一定要放在最后边，因为wrap的middleware执行顺序是从下往上的
            .wrap(
                Cors::new()
                    .allowed_origin("http://localhost:8080")
                    //.allowed_origin("chrome-extension://aicmkgpgakddgnaphhhpliifpcfhicfo")
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT, header::ORIGIN])
                    .allowed_header(header::CONTENT_TYPE)
                    .supports_credentials()
                    .max_age(86400)
                    .finish(),
            )
            .wrap(Logger::default())
            .configure(router::route)
            .service(greet)
    })
    .bind(binding_address)
    .expect("can't bind to port:80");

    server.run().await
}