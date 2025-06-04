use actix_web::{
    get,
    web::{Data, Path, Query},
    App, HttpResponse, HttpServer, Responder,
};
use argh::FromArgs;
use std::{
    collections::BTreeMap,
    net::SocketAddrV4,
    process::exit,
    str::FromStr,
};

mod args;
use args::Args;

mod proxy;
mod interface;

#[get("/rtsp/{tail:.*}")]
async fn rtsp(
    args: Data<Args>,
    mut path: Path<String>,
    mut params: Query<BTreeMap<String, String>>,
) -> impl Responder {
    let path = &mut *path;
    let params = &mut *params;
    let mut params = params.iter().map(|(k, v)| format!("{}={}", k, v));
    let param = params.next().unwrap_or("".to_string());
    let param = params.fold(param, |o, q| format!("{}&{}", o, q));
    HttpResponse::Ok().streaming(proxy::rtsp(
        format!("rtsp://{}?{}", path, param),
        args.interface.clone(),
    ))
}

#[get("/udp/{addr}")]
async fn udp(args: Data<Args>, addr: Path<String>) -> impl Responder {
    let addr = &*addr;
    let addr = match SocketAddrV4::from_str(addr) {
        Ok(addr) => addr,
        Err(e) => return HttpResponse::BadRequest().body(format!("Error: {}", e)),
    };
    HttpResponse::Ok().streaming(proxy::udp(addr, args.interface.clone()))
}

fn usage(cmd: &str) -> std::io::Result<()> {
    let usage = format!(
        r#"Usage: {} [OPTIONS]

Options:
    -b, --bind <BIND>                      Bind address:port [default: 0.0.0.0:7878]
    -I, --interface <INTERFACE>            Interface to request
    -h, --help                             Print help
"#,
        cmd
    );
    eprint!("{}", usage);
    exit(0);
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let args = std::env::args().collect::<Vec<_>>();
    let args = args.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    let args: &[&str] = args.as_ref();
    if args.is_empty() {
        return usage("iptv");
    }
    let args = match Args::from_args(&args[0..1], &args[1..]) {
        Ok(args) => args,
        Err(_) => {
            return usage(args[0]);
        }
    };

    HttpServer::new(move || {
        let args = Data::new(argh::from_env::<Args>());
        App::new()
            .service(rtsp)
            .service(udp)
            .app_data(args)
    })
    .bind(args.bind)?
    .run()
    .await
}
