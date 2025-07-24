use std::net::SocketAddr;

use axum::{
    handler::HandlerWithoutStateExt,
    http::{uri::Authority, StatusCode, Uri},
    response::Redirect,
    BoxError,
};
use axum_extra::extract::Host;

use crate::cli::Args;

/// An independent webserver that only serves to redirect clients to the main
/// webserver using HTTPS
pub async fn redirect_http_to_https(args: Args) {
    fn make_https(
        host: &str,
        uri: Uri,
        https_port: u16,
    ) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let authority: Authority = host.parse()?;
        let bare_host = match authority.port() {
            Some(port_struct) => authority
                .as_str()
                .strip_suffix(port_struct.as_str())
                .unwrap()
                .strip_suffix(':')
                .unwrap(), // if authority.port() is Some(port) then we can be sure authority ends with :{port}
            None => authority.as_str(),
        };

        parts.authority = Some(format!("{bare_host}:{https_port}").parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(&host, uri, args.https_port) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(e) => {
                log::warn!("Failed to convert URI to HTTPS: \"{e}\"");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::new(args.web_addr, args.http_port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    log::info!(
        "HTTP redirect server listening on {}",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, redirect.into_make_service())
        .await
        .unwrap();
}
