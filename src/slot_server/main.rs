use std::net::SocketAddr;

use axum::{
    extract::{Path, Request, State},
    response::{Redirect, Response},
    routing::{any, get},
    Router,
};
use init::initialize;
use reqwest::StatusCode;

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

    let modules = store::ModuleStore::new();

    let tls_conf = match axum_server::tls_openssl::OpenSSLConfig::from_pem_file(
        &args.cert_file,
        &args.key_file,
    ) {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to open PEM files: \"{e}\"");
            std::process::exit(1);
        }
    };

    tokio::spawn(upgrade::redirect_http_to_https(args.clone()));

    module_handler::module_listener(modules.clone(), &args).await;

    let default_redirect = args.default_redirect;
    let routes = Router::new()
        .route("/{modname}/{*rest}", any(module_redirect))
        .route(
            "/",
            get(async move || Redirect::temporary(&default_redirect)),
        )
        .with_state(modules);

    let https_addr = SocketAddr::new(args.web_addr, args.https_port);

    log::info!("Webserver listening on {https_addr}");
    axum_server::bind_openssl(https_addr, tls_conf)
        .serve(routes.into_make_service())
        .await
        .unwrap();
}

#[axum::debug_handler]
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

        // perform request forwarding to module
        let url =
            format!("http://{}/{modname}/{modurl}", module_info.http_addr);

        let mod_resp = req_client
            .request(req.method().clone(), url)
            .headers(req.headers().clone())
            // .version(req.version()) only forwarding request as HTTP/1
            .send()
            .await
            .unwrap();

        // convert reqwest Response into axum Response
        let mut resp = Response::builder();

        // set headers in response
        for (k, v) in mod_resp.headers().iter() {
            resp = resp.header(k, v);
        }

        // set extensions in response
        resp = resp.extension(mod_resp.extensions().clone());

        // set version
        resp = resp.version(mod_resp.version());

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
