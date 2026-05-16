use axum::http::{header, HeaderValue};
use axum::{
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode, Uri},
    response::{IntoResponse, Response as AxumResponse},
};
use leptos::prelude::*;
use leptos_meta::{HashedStylesheet, Link};
use tower::util::ServiceExt;
use tower_http::services::ServeDir;

use sphare_core_common::errors::AppError;
use sphare_core_user::session::ssr::LEPTOS_ENV;

use sphare_cmp_utils::errors::ErrorTemplate;

use sphare_app::app::{AppMeta, I18nProvider};

pub async fn file_and_error_handler(
    uri: Uri,
    State(options): State<LeptosOptions>,
    req: Request<Body>,
) -> AxumResponse {
    let root = options.site_root.clone();
    let res = get_static_file(uri.clone(), &root).await.unwrap();

    if res.status() == StatusCode::OK {
        res.into_response()
    } else {
        let mut errors = Errors::default();
        errors.insert_with_default_key(AppError::NotFound);
        let handler = leptos_axum::render_app_to_stream(
            move || {
                let errors = errors.clone();
                let options = options.clone();
                view! {
                    <!DOCTYPE html>
                    <html>
                        <head>
                            <meta charset="utf-8"/>
                            <meta name="viewport" content="width=device-width, initial-scale=1"/>
                            <AppMeta/>
                            <AutoReload options=options.clone() />
                            // id=leptos means cargo-leptos will hot-reload this stylesheet
                            <HashedStylesheet id="leptos" options/>
                            <Link rel="icon" href="/favicon.ico" />
                        </head>
                        <body>
                            <I18nProvider>
                                <ErrorTemplate outside_errors=errors.clone()/>
                            </I18nProvider>
                        </body>
                    </html>
                }
            },
        );
        handler(req).await.into_response()
    }
}

async fn get_static_file(uri: Uri, root: &str) -> Result<Response<Body>, (StatusCode, String)> {
    let req = Request::builder()
        .uri(uri.clone())
        .body(Body::empty())
        .unwrap();
    // `ServeDir` implements `tower::Service` so we can call it with `tower::ServiceExt::oneshot`
    // This path is relative to the cargo root
    
    let mut response = ServeDir::new(root)
        .oneshot(req)
        .await
        .unwrap_or_else(|err| match err {})
        .into_response();

    if *LEPTOS_ENV == Env::PROD {
        response.headers_mut().append(header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=31536000, immutable"));
    }
    
    Ok(response)
}
