use anyhow::anyhow;
use openidconnect::core::{
    CoreAuthenticationFlow, CoreClient, CoreProviderMetadata, CoreResponseType, CoreUserInfoClaims,
};
use openidconnect::{
    AccessTokenHash, AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    IssuerUrl, Nonce, PkceCodeChallenge, RedirectUrl, Scope,
};

use openidconnect::reqwest::http_client;
use std::io::BufRead;

fn main() -> anyhow::Result<()> {
    // Use OpenID Connect Discovery to fetch the provider metadata.
    use openidconnect::{OAuth2TokenResponse, TokenResponse};
    let provider_metadata = CoreProviderMetadata::discover(
        &IssuerUrl::new("https://huggingface.co".to_string())?,
        http_client,
    )?;

    // Create an OpenID Connect client by specifying the client ID, client secret, authorization URL
    // and token URL.
    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new("64d7dfec-160a-41f0-921f-ab071cf4f16f".to_string()),
        None,
    )
    // Set the URL the user will be redirected to after the authorization process.
    .set_redirect_uri(RedirectUrl::new(
        "http://localhost:5173/login/callback".to_string(),
    )?);

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

    // This is the URL you should redirect the user to, in order to trigger the authorization
    // process.
    println!("Browse to: {}", auth_url);

    let stdin = std::io::stdin();
    let line1: String = stdin.lock().lines().next().unwrap().unwrap();

    println!("Csrf token: {:?}", csrf_token.secret()); // Should match state

    // Once the user has been redirected to the redirect URL, you'll have access to the
    // authorization code. For security reasons, your code should verify that the `state`
    // parameter returned by the server matches `csrf_state`.

    // Now you can exchange it for an access token and ID token.
    let token_response = client
        .exchange_code(AuthorizationCode::new(line1))
        // Set the PKCE code verifier.
        .set_pkce_verifier(pkce_verifier)
        .request(http_client)?;

    // Extract the ID token claims after verifying its authenticity and nonce.
    let id_token = token_response
        .id_token()
        .ok_or_else(|| anyhow!("Server did not return an ID token"))?;
    let claims = id_token.claims(&client.id_token_verifier(), &nonce)?;

    // Verify the access token hash to ensure that the access token hasn't been substituted for
    // another user's.
    if let Some(expected_access_token_hash) = claims.access_token_hash() {
        let actual_access_token_hash =
            AccessTokenHash::from_token(token_response.access_token(), &id_token.signing_alg()?)?;
        if actual_access_token_hash != *expected_access_token_hash {
            return Err(anyhow!("Invalid access token"));
        }
    }

    // The authenticated user's identity is now available. See the IdTokenClaims struct for a
    // complete listing of the available claims.
    println!(
        "User {:?} with e-mail address {} has authenticated successfully",
        claims
            .name()
            .and_then(|name| name.get(None))
            .map(|name| name.as_str())
            .unwrap_or("<not provided>"),
        claims
            .email()
            .map(|email| email.as_str())
            .unwrap_or("<not provided>"),
    );

    // If available, we can use the UserInfo endpoint to request additional information.

    // The user_info request uses the AccessToken returned in the token response. To parse custom
    // claims, use UserInfoClaims directly (with the desired type parameters) rather than using the
    // CoreUserInfoClaims type alias.
    let userinfo: CoreUserInfoClaims = client
        .user_info(token_response.access_token().to_owned(), None)
        .map_err(|err| anyhow!("No user info endpoint: {:?}", err))?
        .request(http_client)
        .map_err(|err| anyhow!("Failed requesting user info: {:?}", err))?;

    println!("Id token {:?}", token_response.access_token().secret());
    // See the OAuth2TokenResponse trait for a listing of other available fields such as
    // access_token() and refresh_token().
    println!("USer info {userinfo:#?}");
    Ok(())
}
