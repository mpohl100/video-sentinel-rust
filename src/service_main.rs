use std::error::Error;
use std::net::SocketAddr;

use async_graphql::http::{GraphQLPlaygroundConfig, playground_source};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::Router;
use axum::extract::State;
use axum::response::Html;
use axum::routing::post;
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;

use video_sentinel::graphql::{VideoSentinelSchema, create_schema};

#[derive(Parser)]
#[command(name = "video-sentinel-service")]
struct CliArgs {
    #[arg(long, default_value_t = 8080)]
    port: u16,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let args = CliArgs::parse();
    let schema = create_schema();

    let tls_config = build_tls_config().await?;

    let app = Router::new()
        .route("/graphql", post(graphql_handler).get(graphql_playground))
        .with_state(schema);

    let addr = SocketAddr::from(([127, 0, 0, 1], args.port));
    println!(
        "GraphQL endpoint available at https://localhost:{}/graphql",
        args.port
    );

    axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn build_tls_config() -> Result<RustlsConfig, Box<dyn Error>> {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
    let cert_pem = cert.cert.pem().into_bytes();
    let key_pem = cert.signing_key.serialize_pem().into_bytes();
    let tls_config = RustlsConfig::from_pem(cert_pem, key_pem).await?;
    Ok(tls_config)
}

async fn graphql_handler(
    State(schema): State<VideoSentinelSchema>,
    request: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(request.into_inner()).await.into()
}

async fn graphql_playground() -> Html<String> {
    Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}
