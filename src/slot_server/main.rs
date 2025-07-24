use std::net::SocketAddr;

use axum::{
    extract::{Path, State},
    response::Response,
    routing::get,
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

    let routes = Router::new()
        .route("/{modname}/{*rest}", get(module_redirect))
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
) -> Response {
    // use the first segment of the URL endpoint to look up the module
    let module_info = state.find_module_by_name(&modname).await;

    if let Some(module_info) = module_info {
        log::debug!("Redirecting request to module \"{}\"", module_info.name);
        // perform request forwarding to module
        let rw = reqwest::get(format!(
            "http://{}/{modname}/{modurl}",
            module_info.http_addr
        ))
        .await
        .unwrap();

        // convert reqwest response into axum response
        let resp = Response::builder();

        // // set headers in response
        // for (k, v) in rw.headers().iter() {
        //     resp = resp.header(k, v);
        // }

        // // set extensions in response
        // resp = resp.extension(rw.extensions().clone());

        // // set version
        // resp = resp.version(rw.version());

        resp.body(axum::body::Body::from(rw.bytes().await.unwrap()))
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(format!("Module \"{modname}\" not found").into())
            .unwrap()
    }
}
