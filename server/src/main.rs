use axum::Router;
use std::net::SocketAddr;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use clap::Parser;

#[derive(Parser)]
#[command(name = "WasmServer")]
#[command(version)]
#[command(about = "Serves static files from assets directory", long_about = None)]
struct Args {
    /// The port to serve on
    #[arg(short, long)]
    port: u16,
    #[arg(long)]
    public: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "server=warn,tower_http=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    serve(serve_assets(), args.port, args.public).await;
}

fn serve_assets() -> Router {
    // `ServeDir` allows setting a fallback if an asset is not found
    // so with this `GET /assets/doesnt-exist.jpg` will return `index.html`
    // rather than a 404
    let serve_dir = ServeDir::new("assets").not_found_service(ServeFile::new("assets/index.html"));

    Router::new()
        .nest_service("/assets", serve_dir.clone())
        .fallback_service(serve_dir)
}

async fn serve(app: Router, port: u16, public: bool) {
    let ip = if public { [0, 0, 0, 0] } else { [127, 0, 0, 1] };
    let addr = SocketAddr::from((ip, port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();
}
