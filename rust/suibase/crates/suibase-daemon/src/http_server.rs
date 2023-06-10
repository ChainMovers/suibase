use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use anyhow::{anyhow, Result};
use axum::{
    body::Body,
    extract::State,
    http::{uri::Uri, Request, Response},
    //response::Html,
    //http::Request,
    routing::get,
    Router,
};
//use hyper::service::{make_service_fn, service_fn};
//use hyper::{Body, Request, Response, Server};
//use hyper::Request;
// FutureExt, SubsystemHandle};
use tokio_graceful_shutdown::SubsystemHandle;

use hyper::client::HttpConnector;
//use std::convert::Infallible;

type Client = hyper::client::Client<HttpConnector, Body>;

pub struct HttpServer {
    // Configuration.
    pub enabled: bool,
}

impl HttpServer {
    /*
    async fn process_request(_: Request<Body>) -> Result<Response<Body>, Infallible> {
        Ok(Response::new(Body::from("Hello World!")))
    }*/

    async fn handler(State(client): State<Client>, mut req: Request<Body>) -> Response<Body> {
        let path = req.uri().path();
        let path_query = req
            .uri()
            .path_and_query()
            .map(|v| v.as_str())
            .unwrap_or(path);

        let uri = format!("http://127.0.0.1:9123{}", path_query);

        *req.uri_mut() = Uri::try_from(uri).unwrap();

        client.request(req).await.unwrap()
    }

    pub async fn run(self, subsys: SubsystemHandle) -> Result<()> {
        let client: Client = hyper::Client::builder().build(HttpConnector::new());
        let app = Router::new()
            .route("/", get(Self::handler))
            .with_state(client);

        let bind_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9124);
        log::info!("HttpServer listening on {}", bind_address);

        axum::Server::bind(&bind_address)
            .serve(app.into_make_service())
            .with_graceful_shutdown(subsys.on_shutdown_requested())
            .await
            .map_err(|err| anyhow! {err})

        /* Hyper way (without Axum):
        // For every connection, we must make a `Service` to handle all
        // incoming HTTP requests on said connection.


        let make_svc = make_service_fn(|_conn| {
            // This is the `Service` that will handle the connection.
            // `service_fn` is a helper to convert a function that
            // returns a Response into a `Service`.
            async { Ok::<_, Infallible>(service_fn(HttpServer::process_request)) }
        });

        let bind_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 9124);
        log::info!("HttpServer listening on {}", bind_address);

        let server = Server::bind(&bind_address).serve(make_svc);

        server
            .with_graceful_shutdown(subsys.on_shutdown_requested())
            .await
            .map_err(|err| anyhow! {err})
        */
    }
}
