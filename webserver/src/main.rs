use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use clap::Parser;
use warp::Filter;


#[derive(Parser,Debug)]
#[clap(name="webserver")]
struct Args {
    /// webserver port
    #[clap(short = 'p', long, value_parser)]
    port: u16,
}


#[tokio::main]
async fn main() {
    let args = Args::parse();

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127,0,0,1)), args.port);

    let index = warp::get()
                    .and(warp::path::end())
                    .and(warp::fs::file("./web/index.html"));
                
    let resources = warp::path("static")
                        .and(warp::fs::dir("./web/static"));

    let scripts = warp::path("scripts")
                        .and(warp::fs::dir("./web/scripts"));

    let routes = index.or(resources).or(scripts);

    println!("starting webserver at localhost:{:?}", args.port);

    warp::serve(routes)
            .run(addr)
            .await;
}
