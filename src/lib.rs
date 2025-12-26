mod client_registry;
mod google_play_client;
mod handlers;
mod openapi_schema;
mod serializable_types;

use client_registry::create_registry;
use openapi_schema::ApiDoc;
use utoipa::OpenApi;
use worker::*;

struct AppState {
    client_registry: client_registry::SharedClientRegistry,
    env: Env,
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    let client_registry = create_registry(env.clone()).await;
    let state = AppState { client_registry, env: env.clone() };

    let router = Router::with_data(state);

    router
        .get("/", |_req, _ctx| {
            let url = Url::parse("https://oksurya.com")?;
            Response::redirect(url)
        })
        .get("/openapi.json", |_req, _ctx| {
            let spec = ApiDoc::openapi().to_pretty_json().unwrap();

            let headers = Headers::new();
            headers.set("Content-Type", "application/json")?;

            Ok(Response::ok(&spec)?.with_headers(headers))
        })
        .get_async("/v1/details/:package_name", |_req, ctx| async move {
            let package_name = ctx.param("package_name").unwrap().to_string();
            handlers::get_details_multi(package_name, ctx.data.client_registry.clone()).await
        })
        .get_async(
            "/v1/details/:package_name/:channel",
            |_req, ctx| async move {
                let package_name = ctx.param("package_name").unwrap().to_string();
                let channel = ctx.param("channel").unwrap().to_string();
                handlers::get_details_single(
                    package_name,
                    channel,
                    ctx.data.client_registry.clone(),
                )
                .await
            },
        )
        .get_async(
            "/v1/download/:package_name/:channel/:version_code",
            |_req, ctx| async move {
                let package_name = ctx.param("package_name").unwrap().to_string();
                let channel = ctx.param("channel").unwrap().to_string();
                let version_code: i32 = ctx.param("version_code").unwrap().parse().unwrap_or(0);
                let brand_name = ctx.data.env.var("BRAND_NAME").map(|v| v.to_string()).unwrap_or_else(|_| "Sniff".to_string());
                handlers::get_download_info(
                    package_name,
                    channel,
                    Some(version_code),
                    ctx.data.client_registry.clone(),
                    brand_name,
                )
                .await
            },
        )
        .get_async(
            "/v1/download/:package_name/:channel",
            |_req, ctx| async move {
                let package_name = ctx.param("package_name").unwrap().to_string();
                let channel = ctx.param("channel").unwrap().to_string();
                let brand_name = ctx.data.env.var("BRAND_NAME").map(|v| v.to_string()).unwrap_or_else(|_| "Sniff".to_string());
                handlers::get_download_info(
                    package_name,
                    channel,
                    None,
                    ctx.data.client_registry.clone(),
                    brand_name,
                )
                .await
            },
        )
        // Proxy download endpoints - directly downloads APK with correct filename
        .get_async(
            "/v1/apk/:package_name/:channel/:version_code",
            |_req, ctx| async move {
                let package_name = ctx.param("package_name").unwrap().to_string();
                let channel = ctx.param("channel").unwrap().to_string();
                let version_code: i32 = ctx.param("version_code").unwrap().parse().unwrap_or(0);
                let brand_name = ctx.data.env.var("BRAND_NAME").map(|v| v.to_string()).unwrap_or_else(|_| "Sniff".to_string());
                handlers::proxy_download(
                    package_name,
                    channel,
                    Some(version_code),
                    ctx.data.client_registry.clone(),
                    brand_name,
                )
                .await
            },
        )
        .get_async(
            "/v1/apk/:package_name/:channel",
            |_req, ctx| async move {
                let package_name = ctx.param("package_name").unwrap().to_string();
                let channel = ctx.param("channel").unwrap().to_string();
                let brand_name = ctx.data.env.var("BRAND_NAME").map(|v| v.to_string()).unwrap_or_else(|_| "Sniff".to_string());
                handlers::proxy_download(
                    package_name,
                    channel,
                    None,
                    ctx.data.client_registry.clone(),
                    brand_name,
                )
                .await
            },
        )
        .run(req, env)
        .await
}
