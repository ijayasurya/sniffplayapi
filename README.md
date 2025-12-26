<h1 align="center">
  <img src=".github/logo.gif" alt="sniffer from minecraft" width="320">
</h1>

**Sniff** is a specialized API service designed to retrieve Google Play Store app
details across different release channels (Stable, Beta, Alpha). It provides a clean
interface to access app metadata including version information, changelog, download
sizes, and other details for Android applications. **Now with direct APK streaming downloads!**

## Features

- **Multi-Channel Support**: Access app details from Stable, Beta, and Alpha channels (where available)
- **Intelligent Track Detection**: Automatically identifies which channels are available for specific apps
- **Unified API**: Simple REST API endpoints for accessing app information
- **Direct APK Downloads**: Stream APKs directly with custom branded filenames
- **Streaming Proxy**: Downloads start immediately without server-side buffering

## API Endpoints

### Get App Details (All Available Channels)

```
GET /v1/details/:package_name
```

Returns details for all available channels for the specified package.

**Response Headers:**

- `X-Available-Channels`: Comma-separated list of available channels for the app

### Get App Details (Specific Channel)

```
GET /v1/details/:package_name/:channel
```

Returns details for a specific channel (stable, beta, or alpha) if available.

**Possible channels:**

- `stable` - Production release (always available)
- `beta` - Beta program release (only available for certain apps)
- `alpha` - Alpha program release (only available for certain apps)

**Response Format:**

Successful responses follow this structure:

```jsonc
{
  "success": true,
  "data": {
    // For multi-channel: track name -> details
    "stable": {
      /* app details */
    },
    "beta": {
      /* app details */
    },
    // Alpha if available
  },
  "error": null,
}
```

Error responses:

```json
{
  "success": false,
  "data": null,
  "error": "Error message describing the issue"
}
```

### Get Download Info

```
GET /v1/download/:package_name/:channel
GET /v1/download/:package_name/:channel/:version_code
```

Retrieves download URLs and metadata for an app. Optionally specify a version code for a specific version.

**Parameters:**

- `package_name`: The package identifier of the app (e.g., `com.discord`)
- `channel`: Release channel (`stable`, `beta`, or `alpha`)
- `version_code`: (Optional) The specific Android version code to download

**Response Format:**

```json
{
  "success": true,
  "data": {
    "suggested_filename": "Sniff_Discord_Stable_289.20.apk",
    "app_name": "Discord",
    "version_string": "289.20",
    "version_code": 289020,
    "channel": "stable",
    "main_apk_url": "https://play.googleapis.com/download/by-token/download?token=...",
    "splits": [
      {
        "name": "config.xxhdpi",
        "download_url": "https://play.googleapis.com/download/by-token/download?token=..."
      },
      {
        "name": "config.arm64_v8a",
        "download_url": "https://play.googleapis.com/download/by-token/download?token=..."
      }
    ],
    "additional_files": []
  },
  "error": null
}
```

### Direct APK Download (Streaming Proxy)

```
GET /v1/apk/:package_name/:channel
GET /v1/apk/:package_name/:channel/:version_code
```

**Streams the APK file directly** with a custom filename. The download starts immediately without buffering the entire file server-side.

**Parameters:**

- `package_name`: The package identifier of the app (e.g., `com.discord`)
- `channel`: Release channel (`stable`, `beta`, or `alpha`)
- `version_code`: (Optional) The specific Android version code to download

**Response:**

- Returns the APK binary with `Content-Disposition: attachment; filename="..."` header
- Filename format: `{BRAND_NAME}_{AppName}_{Channel}_{Version}.apk`
- Example: `Sniff_Discord_Stable_289.20.apk`

**Example Usage:**

```bash
# Download latest stable version
curl -OJ https://your-api.com/v1/apk/com.discord/stable

# Download specific version
curl -OJ https://your-api.com/v1/apk/com.discord/stable/289020
```

## Deployment

Sniff is designed to be deployed as a Cloudflare Worker, providing global distribution and low-latency access to the API.

## Environment Variables

The following environment variables are required:

| Variable | Description |
|----------|-------------|
| `DEVICE_NAME` | Device identifier for Google Play API (e.g., `px_7a`) |
| `BRAND_NAME` | Brand name prefix for APK filenames (e.g., `Sniff`) |
| `STABLE_EMAIL` | Email for stable track access |
| `STABLE_AAS_TOKEN` | Authentication token for stable track |
| `BETA_EMAIL` | Email enrolled in beta programs |
| `BETA_AAS_TOKEN` | Authentication token for beta access |
| `ALPHA_EMAIL` | Email enrolled in alpha programs |
| `ALPHA_AAS_TOKEN` | Authentication token for alpha access |

### Customizing APK Filenames

Set the `BRAND_NAME` environment variable in `wrangler.toml`:

```toml
[vars]
DEVICE_NAME = "px_7a"
BRAND_NAME = "YourBrand"
```

Downloaded APKs will be named: `YourBrand_AppName_Channel_Version.apk`

## API Documentation

Interactive OpenAPI documentation is available at:

```
GET /openapi.json
```
