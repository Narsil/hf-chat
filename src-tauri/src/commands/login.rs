use crate::entities::user;
use crate::State;
use log::{info, warn};
use openidconnect::{
    core::{CoreAuthenticationFlow, CoreClient, CoreErrorResponseType, CoreProviderMetadata},
    reqwest::async_http_client,
    AccessTokenHash, AuthorizationCode, ClientId, CsrfToken, IssuerUrl, Nonce, OAuth2TokenResponse,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RequestTokenError, Scope,
    StandardErrorResponse, TokenResponse,
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use std::io::Write;

static OPENID_SECRET: Option<&'static str> = option_env!("OPENID_SECRET");

async fn core_client() -> Result<CoreClient, OpenidError> {
    let provider_metadata = CoreProviderMetadata::discover_async(
        IssuerUrl::new("https://huggingface.co".to_string())?,
        async_http_client,
    )
    .await?;
    let client_id = if let Some(secret) = OPENID_SECRET {
        ClientId::new(secret.to_string())
    } else {
        warn!("Open id secret wasn't set");
        ClientId::new("dummy-secret".to_string())
    };
    Ok(CoreClient::from_provider_metadata(
        provider_metadata,
        client_id,
        None,
    ))
}

pub struct Openid {
    csrf_token: CsrfToken,
    nonce: Nonce,
    pkce_verifier: PkceCodeVerifier,
}

#[derive(Debug, thiserror::Error)]
pub enum OpenidError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Url error {0}")]
    Url(#[from] openidconnect::url::ParseError),

    #[error("Discover error {0}")]
    Discovery(#[from] openidconnect::DiscoveryError<openidconnect::reqwest::Error<reqwest::Error>>),

    #[error("Signing error {0}")]
    Signing(#[from] openidconnect::SigningError),

    #[error("Locking error")]
    Lock,

    #[error("Request token error {0}")]
    RequestTokenError(
        #[from]
        RequestTokenError<
            openidconnect::reqwest::Error<reqwest::Error>,
            StandardErrorResponse<CoreErrorResponseType>,
        >,
    ),

    #[error("Claims verification error {0}")]
    Claims(#[from] openidconnect::ClaimsVerificationError),

    #[error("Invalid token")]
    InvalidToken,

    #[error("Invalid csrf token")]
    InvalidCsrf,

    #[error("Missing token")]
    MissingToken,

    #[error("Unset validators")]
    UnsetValidators,
}

// we must manually implement serde::Serialize
impl serde::Serialize for OpenidError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[tauri::command]
pub async fn login(state: tauri::State<'_, State>, url: String) -> Result<String, OpenidError> {
    // Create an OpenID Connect client by specifying the client ID, client secret, authorization URL
    // and token URL.
    let redirect_url = format!("{url}");
    info!("Redirect to : {redirect_url}");
    let client = core_client()
        .await?
        // Set the URL the user will be redirected to after the authorization process.
        .set_redirect_uri(RedirectUrl::new(redirect_url)?);

    // Generate a PKCE challenge.
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate the full authorization URL.
    let (auth_url, csrf_token, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        // Set the desired scopes.
        .add_scope(Scope::new("profile".to_string()))
        .add_scope(Scope::new("inference-api".to_string()))
        // Set the PKCE code challenge.
        .set_pkce_challenge(pkce_challenge)
        .url();
    let mut guard = state.openid.try_lock().map_err(|_| OpenidError::Lock)?;
    *guard = Some(Openid {
        csrf_token,
        nonce,
        pkce_verifier,
    });
    info!("Authentication url {auth_url}");
    Ok(auth_url.to_string())
}

#[tauri::command]
pub async fn login_callback(
    app_state: tauri::State<'_, State>,
    code: String,
    state: String,
) -> Result<(), OpenidError> {
    info!("Login callback {code} {state}");

    let Openid {
        csrf_token,
        nonce,
        pkce_verifier,
    } = {
        let mut openid = app_state.openid.try_lock().map_err(|_| OpenidError::Lock)?;
        openid.take().ok_or(OpenidError::UnsetValidators)?
    };

    // Create an OpenID Connect client by specifying the client ID, client secret, authorization URL
    // and token URL.
    let client = core_client().await?.set_redirect_uri(RedirectUrl::new(
        "http://localhost:1420/login/callback".to_string(),
    )?);

    // Once the user has been redirected to the redirect URL, you'll have access to the
    // authorization code. For security reasons, your code should verify that the `state`
    // parameter returned by the server matches `csrf_state`.
    if csrf_token.secret() != &state {
        return Err(OpenidError::InvalidCsrf);
    }

    // Now you can exchange it for an access token and ID token.
    let token_response = client
        .exchange_code(AuthorizationCode::new(code))
        // Set the PKCE code verifier.
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await?;

    // Extract the ID token claims after verifying its authenticity and nonce.
    let id_token = token_response
        .id_token()
        .ok_or_else(|| OpenidError::MissingToken)?;
    let claims = id_token.claims(&client.id_token_verifier(), &nonce)?;

    // Verify the access token hash to ensure that the access token hasn't been substituted for
    // another user's.
    if let Some(expected_access_token_hash) = claims.access_token_hash() {
        let actual_access_token_hash =
            AccessTokenHash::from_token(token_response.access_token(), &id_token.signing_alg()?)?;
        if actual_access_token_hash != *expected_access_token_hash {
            return Err(OpenidError::InvalidToken);
        }
    }

    // The authenticated user's identity is now available. See the IdTokenClaims struct for a
    // complete listing of the available claims.
    let token_path = app_state.cache.token_path();
    let token = token_response.access_token().secret();
    if !token_path.exists() {
        if let Ok(mut file) = std::fs::File::create(token_path) {
            file.write_all(token.as_bytes())?;
        }
    }
    let name = claims
        .name()
        .and_then(|name| name.get(None))
        .map(|name| name.as_str())
        .unwrap_or("<not provided>");
    let profile = claims
        .picture()
        .and_then(|name| name.get(None))
        .map(|name| name.as_str())
        .unwrap_or("<not provided>");
    let db = &app_state.db;
    let new_user = user::ActiveModel {
        name: Set(name.to_string()),
        profile: Set(profile.to_string()),
        ..Default::default()
    };

    let user = new_user.insert(db).await.expect("Insert user");
    info!("Found user {user:?}");

    Ok(())
}
