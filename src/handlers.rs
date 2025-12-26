use crate::client_registry::SharedClientRegistry;
use crate::google_play_client::Channel;
use crate::openapi_schema::{
    ApiResponse, DownloadInfo, MultiChannelApiResponse, SerializableDetailsResponse,
};
use crate::serializable_types::SerializableDetailsResponse as ActualSerializableDetailsResponse;
use std::collections::HashMap;
use utoipa;
use worker::*;

#[utoipa::path(
    get,
    path = "/v1/details/{package_name}",
    params(
        ("package_name" = String, Path, description = "Android package name (e.g., com.discord)")
    ),
    responses(
        (status = 200, description = "App details retrieved successfully",
         body = MultiChannelApiResponse<SerializableDetailsResponse>,
         headers(
             ("X-Available-Channels" = String, description = "Comma-separated list of available channels")
         )
        ),
        (status = 404, description = "App not found", body = MultiChannelApiResponse<SerializableDetailsResponse>),
        (status = 500, description = "Internal server error", body = MultiChannelApiResponse<SerializableDetailsResponse>)
    ),
    tag = "App Details"
)]
pub async fn get_details_multi(
    package_name: String,
    client_registry: SharedClientRegistry,
) -> Result<Response> {
    match client_registry
        .lock()
        .expect("Failed to lock client registry")
        .get_details_multi(&package_name)
        .await
    {
        Ok(details_map) => {
            let serialized_map: HashMap<String, ActualSerializableDetailsResponse> = details_map
                .into_iter()
                .map(|(channel, details)| {
                    (
                        channel.to_string(),
                        ActualSerializableDetailsResponse(details),
                    )
                })
                .collect();

            let available_channels = serialized_map.keys().cloned().collect::<Vec<_>>().join(",");

            let response = MultiChannelApiResponse {
                success: true,
                data: Some(serialized_map),
                error: None,
            };

            let headers = Headers::new();
            headers.set("Content-Type", "application/json")?;
            headers.set("X-Available-Channels", &available_channels)?;

            Ok(Response::from_json(&response)?.with_headers(headers))
        }
        Err(e) => {
            let response = MultiChannelApiResponse::<ActualSerializableDetailsResponse> {
                success: false,
                data: None,
                error: Some(e),
            };

            Ok(Response::from_json(&response)?.with_status(500))
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/details/{package_name}/{channel}",
    params(
        ("package_name" = String, Path, description = "Android package name (e.g., com.discord)"),
        ("channel" = String, Path, description = "Release channel", example = "stable")
    ),
    responses(
        (status = 200, description = "App details retrieved successfully", body = ApiResponse<SerializableDetailsResponse>),
        (status = 400, description = "Invalid channel", body = ApiResponse<String>),
        (status = 404, description = "App not found", body = ApiResponse<SerializableDetailsResponse>),
        (status = 500, description = "Internal server error", body = ApiResponse<SerializableDetailsResponse>)
    ),
    tag = "App Details"
)]
pub async fn get_details_single(
    package_name: String,
    channel: String,
    client_registry: SharedClientRegistry,
) -> Result<Response> {
    let channel = match Channel::from_str(&channel) {
        Ok(ch) => ch,
        Err(e) => {
            let response = ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(e),
            };
            return Ok(Response::from_json(&response)?.with_status(400));
        }
    };

    let result = client_registry
        .lock()
        .expect("Failed to lock client registry")
        .get_details_with_fallback(&package_name, channel)
        .await;

    match result {
        Ok(Some((_, details))) => {
            let response = ApiResponse {
                success: true,
                data: Some(ActualSerializableDetailsResponse(details)),
                error: None,
            };
            Ok(Response::from_json(&response)?)
        }
        Ok(None) => {
            let response = ApiResponse::<ActualSerializableDetailsResponse> {
                success: false,
                data: None,
                error: Some(format!("App '{}' not found", package_name)),
            };
            Ok(Response::from_json(&response)?.with_status(404))
        }
        Err(e) => {
            let response = ApiResponse::<ActualSerializableDetailsResponse> {
                success: false,
                data: None,
                error: Some(e),
            };
            Ok(Response::from_json(&response)?.with_status(500))
        }
    }
}

#[utoipa::path(
    get,
    path = "/v1/download/{package_name}/{channel}/{version_code}",
    params(
        ("package_name" = String, Path, description = "Android package name"),
        ("channel" = String, Path, description = "Release channel"),
        ("version_code" = i32, Path, description = "Android version code")
    ),
    responses(
        (status = 200, description = "Download info retrieved successfully", body = ApiResponse<DownloadInfo>),
        (status = 400, description = "Invalid parameters", body = ApiResponse<String>),
        (status = 404, description = "App or version not found", body = ApiResponse<DownloadInfo>),
        (status = 500, description = "Internal server error", body = ApiResponse<DownloadInfo>)
    ),
    tag = "Downloads"
)]
pub async fn get_download_info(
    package_name: String,
    channel: String,
    version_code: Option<i32>,
    client_registry: SharedClientRegistry,
    brand_name: String,
) -> Result<Response> {
    let parsed_channel = match Channel::from_str(&channel) {
        Ok(ch) => ch,
        Err(e) => {
            let response = ApiResponse::<()> {
                success: false,
                data: None,
                error: Some(e),
            };
            return Ok(Response::from_json(&response)?.with_status(400));
        }
    };

    // First, get app details to extract app name and version
    let details_result = client_registry
        .lock()
        .expect("Failed to lock client registry")
        .get_details_with_fallback(&package_name, parsed_channel)
        .await;

    let (app_name, version_string, actual_version_code) = match &details_result {
        Ok(Some((_, details))) => {
            let item = details.item.as_ref();
            let title = item.and_then(|i| i.title.clone());
            let app_details = item
                .and_then(|i| i.details.as_ref())
                .and_then(|d| d.app_details.as_ref());
            let ver_string = app_details.and_then(|a| a.version_string.clone());
            let ver_code = app_details.and_then(|a| a.version_code);
            
            // Extract just the app name (before " - " subtitle if present)
            let clean_name = title.map(|t| {
                t.split(" - ").next().unwrap_or(&t).trim().to_string()
            });
            
            (clean_name, ver_string, ver_code)
        }
        _ => (None, None, None),
    };

    // Now get download info
    let result = client_registry
        .lock()
        .expect("Failed to lock client registry")
        .get_download_info(&package_name, parsed_channel, version_code)
        .await;

    match result {
        Ok(Some((_, download_info))) => {
            let (main_apk_url, splits_data, additional_files_data) = download_info;

            let splits: Vec<_> = splits_data
                .into_iter()
                .map(|(name, url)| crate::openapi_schema::SplitFile {
                    name,
                    download_url: url,
                })
                .collect();

            let additional_files: Vec<_> = additional_files_data
                .into_iter()
                .map(|(filename, url)| crate::openapi_schema::AdditionalFile {
                    filename,
                    download_url: url,
                })
                .collect();

            // Build suggested filename: {brand}_{appname}_{channel}_{version}.apk
            let channel_display = match parsed_channel {
                Channel::Stable => "Stable",
                Channel::Beta => "Beta",
                Channel::Alpha => "Alpha",
            };
            
            let suggested_filename = build_suggested_filename(
                &brand_name,
                app_name.as_deref(),
                channel_display,
                version_string.as_deref(),
            );

            let openapi_download_info = DownloadInfo {
                suggested_filename: Some(suggested_filename),
                app_name,
                version_string,
                version_code: actual_version_code,
                channel: Some(channel_display.to_lowercase()),
                main_apk_url,
                splits,
                additional_files,
            };

            let response = ApiResponse {
                success: true,
                data: Some(openapi_download_info),
                error: None,
            };
            Ok(Response::from_json(&response)?)
        }
        Ok(None) => {
            let response = ApiResponse::<DownloadInfo> {
                success: false,
                data: None,
                error: Some(format!("App '{}' not found", package_name)),
            };
            Ok(Response::from_json(&response)?.with_status(404))
        }
        Err(e) => {
            let response = ApiResponse::<DownloadInfo> {
                success: false,
                data: None,
                error: Some(e),
            };
            Ok(Response::from_json(&response)?.with_status(500))
        }
    }
}

/// Build a suggested filename for the APK download
/// Format: {brand}_{appname}_{channel}_{version}.apk
fn build_suggested_filename(
    brand: &str,
    app_name: Option<&str>,
    channel: &str,
    version: Option<&str>,
) -> String {
    let sanitize = |s: &str| -> String {
        s.chars()
            .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
            .collect::<String>()
            .replace("__", "_")
            .trim_matches('_')
            .to_string()
    };

    let brand_part = sanitize(brand);
    let app_part = app_name.map(|n| sanitize(n)).unwrap_or_else(|| "App".to_string());
    let channel_part = sanitize(channel);
    
    match version {
        Some(v) => {
            // Clean version string (remove channel suffix if present, e.g., "289.20 - Stable" -> "289.20")
            let clean_version = v.split(" - ").next().unwrap_or(v).trim();
            let version_part = sanitize(clean_version);
            format!("{}_{}_{}_{}.apk", brand_part, app_part, channel_part, version_part)
        }
        None => format!("{}_{}_{}.apk", brand_part, app_part, channel_part),
    }
}

#[utoipa::path(
    get,
    path = "/v1/apk/{package_name}/{channel}",
    params(
        ("package_name" = String, Path, description = "Android package name (e.g., com.discord)"),
        ("channel" = String, Path, description = "Release channel: stable, beta, or alpha")
    ),
    responses(
        (status = 200, description = "APK file streamed successfully", content_type = "application/vnd.android.package-archive"),
        (status = 400, description = "Invalid channel"),
        (status = 404, description = "App not found or no download URL available"),
        (status = 502, description = "Failed to fetch APK from upstream")
    ),
    tag = "Direct APK Download"
)]
/// Stream APK file directly with custom filename
/// 
/// Downloads the APK from Google Play and streams it to the client with a
/// custom filename in the format: `{BRAND_NAME}_{AppName}_{Channel}_{Version}.apk`
/// 
/// The download starts immediately without buffering the entire file server-side.
/// 
/// Also supports version-specific downloads: `/v1/apk/{package_name}/{channel}/{version_code}`
pub async fn proxy_download(
    package_name: String,
    channel: String,
    version_code: Option<i32>,
    client_registry: SharedClientRegistry,
    brand_name: String,
) -> Result<Response> {
    let parsed_channel = match Channel::from_str(&channel) {
        Ok(ch) => ch,
        Err(e) => {
            return Ok(Response::error(format!("Invalid channel: {}", e), 400)?);
        }
    };

    // Get app details for filename
    let details_result = client_registry
        .lock()
        .expect("Failed to lock client registry")
        .get_details_with_fallback(&package_name, parsed_channel)
        .await;

    let (app_name, version_string) = match &details_result {
        Ok(Some((_, details))) => {
            let item = details.item.as_ref();
            let title = item.and_then(|i| i.title.clone());
            let app_details = item
                .and_then(|i| i.details.as_ref())
                .and_then(|d| d.app_details.as_ref());
            let ver_string = app_details.and_then(|a| a.version_string.clone());
            
            let clean_name = title.map(|t| {
                t.split(" - ").next().unwrap_or(&t).trim().to_string()
            });
            
            (clean_name, ver_string)
        }
        _ => (None, None),
    };

    // Get download info
    let result = client_registry
        .lock()
        .expect("Failed to lock client registry")
        .get_download_info(&package_name, parsed_channel, version_code)
        .await;

    match result {
        Ok(Some((_, download_info))) => {
            let (main_apk_url, _, _) = download_info;
            
            let download_url = match main_apk_url {
                Some(url) => url,
                None => {
                    return Ok(Response::error("No download URL available", 404)?);
                }
            };

            // Build the filename
            let channel_display = match parsed_channel {
                Channel::Stable => "Stable",
                Channel::Beta => "Beta",
                Channel::Alpha => "Alpha",
            };
            
            let filename = build_suggested_filename(
                &brand_name,
                app_name.as_deref(),
                channel_display,
                version_string.as_deref(),
            );

            // Fetch the APK from Google - streaming response
            let fetch_request = Request::new(&download_url, Method::Get)?;
            let apk_response = Fetch::Request(fetch_request).send().await?;
            
            if apk_response.status_code() != 200 {
                return Ok(Response::error(
                    format!("Failed to fetch APK: HTTP {}", apk_response.status_code()),
                    502,
                )?);
            }

            // Get the response body as a stream and pass through headers
            let headers = Headers::new();
            headers.set("Content-Type", "application/vnd.android.package-archive")?;
            headers.set(
                "Content-Disposition",
                &format!("attachment; filename=\"{}\"", filename),
            )?;
            
            // Copy Content-Length from upstream if available
            if let Ok(Some(content_length)) = apk_response.headers().get("Content-Length") {
                headers.set("Content-Length", &content_length)?;
            }
            headers.set("Cache-Control", "no-cache")?;

            // Stream the response body directly without buffering
            Ok(apk_response.with_headers(headers))
        }
        Ok(None) => {
            Ok(Response::error(format!("App '{}' not found", package_name), 404)?)
        }
        Err(e) => {
            Ok(Response::error(format!("Error: {}", e), 500)?)
        }
    }
}

