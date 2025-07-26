use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{Path, Request, State},
    response::{Redirect, Response},
    routing::{any, get},
    Router,
};
use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo};
use init::initialize;
use reqwest::StatusCode;
use tokio_rustls::{
    rustls::{
        crypto::{ring, CryptoProvider},
        pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer},
        ServerConfig,
    },
    TlsAcceptor,
};
use tower::Service;

use crate::store::ModuleStore;

mod cli;
mod init;
mod module_handler;
mod store;
mod upgrade;

#[tokio::main]
async fn main() {
    let (args, _logger_handle) = initialize();
    log::debug!("Completed initialization");

    CryptoProvider::install_default(ring::default_provider())
        .expect("Valid crypto implementation");

    let modules = store::ModuleStore::new();

    let key = PrivateKeyDer::from_pem_file(&args.key_file).unwrap();

    let certs = CertificateDer::pem_file_iter(&args.cert_file)
        .unwrap()
        .map(Result::unwrap)
        .collect();

    let mut rustls_config = match ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
    {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to open PEM files: \"{e}\"");
            std::process::exit(1);
        }
    };

    rustls_config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let tls_acceptor = TlsAcceptor::from(Arc::new(rustls_config));

    tokio::spawn(upgrade::redirect_http_to_https(args.clone()));

    module_handler::module_listener(modules.clone(), &args).await;

    let default_redirect = args.default_redirect;
    let routes = Router::new()
        .route(
            "/favicon.ico",
            get(async || -> Response {
                match tokio::fs::read("favicon.ico").await {
                    Ok(ico) => Response::new(ico.into()),
                    Err(_) => Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body("favicon.ico not set".into())
                        .unwrap(),
                }
            }),
        )
        .route("/{modname}/{*rest}", any(module_redirect))
        .route(
            "/",
            get(async move || Redirect::temporary(&default_redirect)),
        )
        .with_state(modules);

    let https_addr = SocketAddr::new(args.web_addr, args.https_port);

    log::info!("Webserver listening on {https_addr}");
    // let listener = tokio::net::TcpListener::bind(https_addr).await.unwrap();
    // axum_server::Server::from_tcp(listener.into_std().unwrap())
    //     .acceptor(axum_server::tls_openssl::OpenSSLAcceptor::new(tls_conf))
    //     .http_builder()
    //     .http1_only()
    //     .serve_connection(TokioIo, routes.into_make_service());

    let tcp_listener = tokio::net::TcpListener::bind(https_addr).await.unwrap();

    loop {
        let tower_service = routes.clone();
        let tls_acceptor = tls_acceptor.clone();

        // Wait for new tcp connection
        let (cnx, addr) = tcp_listener.accept().await.unwrap();

        tokio::spawn(async move {
            // Wait for tls handshake to happen
            let Ok(stream) = tls_acceptor.accept(cnx).await else {
                log::error!(
                    "Error during TLS handshake connection from {addr}"
                );
                return;
            };

            // Hyper has its own `AsyncRead` and `AsyncWrite` traits and doesn't
            // use tokio. `TokioIo` converts between them.
            let stream = TokioIo::new(stream);

            // Hyper also has its own `Service` trait and doesn't use tower. We
            // can use `hyper::service::service_fn` to create a hyper `Service`
            // that calls our app through `tower::Service::call`.
            let hyper_service = hyper::service::service_fn(
                move |request: Request<Incoming>| {
                    // We have to clone `tower_service` because hyper's
                    // `Service` uses `&self` whereas tower's `Service` requires
                    // `&mut self`.
                    // We don't need to call `poll_ready` since `Router` is
                    // always ready.
                    tower_service.clone().call(request)
                },
            );

            let ret = hyper_util::server::conn::auto::Builder::new(
                TokioExecutor::new(),
            )
            .http1_only()
            .serve_connection_with_upgrades(stream, hyper_service)
            .await;

            if let Err(err) = ret {
                log::warn!("Error serving connection from {addr}: {err}");
            }
        });
    }
}

async fn module_redirect(
    State(state): State<ModuleStore>,
    Path((modname, modurl)): Path<(String, String)>,
    req: Request,
) -> Response {
    // use the first segment of the URL endpoint to look up the module
    let module_info = state.find_module_by_name(&modname).await;

    if let Some(module_info) = module_info {
        log::debug!("Redirecting request to module \"{}\"", module_info.name);

        // set up reqwest client with request headers
        let req_client = reqwest::Client::new();
        // let req_client = reqwest::ClientBuilder::new().http2_prior_knowledge().build().unwrap();

        // perform request forwarding to module
        let url =
            format!("http://{}/{modname}/{modurl}", module_info.http_addr);

        // TODO: filter necessary headers (e.g., auth)

        // - host: localhost:8001
        // - user-agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:143.0) Gecko/20100101 Firefox/143.0
        // - accept: audio/webm,audio/ogg,audio/wav,audio/*;q=0.9,application/ogg;q=0.7,video/*;q=0.6,*/*;q=0.5
        // - accept-language: en-US,en;q=0.5
        // - range: bytes=2654208-
        // - sec-gpc: 1
        // - connection: keep-alive
        // - referer: https://localhost:8001/meta/index
        // - sec-fetch-dest: audio
        // - sec-fetch-mode: no-cors
        // - sec-fetch-site: same-origin
        // - accept-encoding: identity
        // - priority: u=4
        // - pragma: no-cache
        // - cache-control: no-cache

        let Ok(mod_resp) = req_client
            .request(req.method().clone(), url)
            // .headers(req.headers().clone())
            // .version(req.version())
            .send()
            .await
        else {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(
                    format!(
                        "Module \"{modname}\" did not respond or unable to \
                         make request"
                    )
                    .into(),
                )
                .unwrap();
        };

        // convert reqwest Response into axum Response
        let mut resp = Response::builder();

        // set headers in response
        for (k, v) in mod_resp.headers().iter() {
            if ["content-type", "cache-control"]
                .contains(&k.as_str().to_lowercase().as_str())
            {
                resp = resp.header(k, v);
            }
        }

        // // set extensions in response
        // resp = resp.extension(mod_resp.extensions().clone());

        // // set version
        // resp = resp.version(mod_resp.version());

        resp.body(axum::body::Body::from(mod_resp.bytes().await.unwrap()))
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(
                format!("Module \"{modname}\" is offline or does not exist")
                    .into(),
            )
            .unwrap()
    }
}
