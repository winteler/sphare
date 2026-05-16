use leptos::prelude::*;
use leptos_router::params::Params;

#[cfg(feature = "ssr")]
use openidconnect as oidc;
#[cfg(feature = "ssr")]
use openidconnect::{reqwest, OAuth2TokenResponse, TokenResponse};

pub const OIDC_ISSUER_URL_ENV: &str = "OIDC_ISSUER_URL";
pub const OIDC_ISSUER_REALM_ENV: &str = "OIDC_ISSUER_REALM";
pub const OIDC_ISSUER_ADMIN_URL_ENV: &str = "OIDC_ISSUER_ADMIN_URL";
pub const AUTH_CLIENT_ID_ENV: &str = "AUTH_CLIENT_ID";
pub const AUTH_CLIENT_SECRET_ENV: &str = "AUTH_CLIENT_SECRET";
pub const PKCE_KEY: &str = "pkce";
pub const NONCE_KEY: &str = "nonce";
pub const OIDC_TOKEN_KEY: &str = "oidc_token";
pub const OIDC_USERNAME_KEY: &str = "oidc_username";
pub const REDIRECT_URL_KEY: &str = "redirect";

#[derive(Params, Debug, PartialEq, Clone)]
pub struct OAuthParams {
    pub code: Option<String>,
    pub state: Option<String>,
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use std::sync::LazyLock;

    use openidconnect::core::CoreTokenResponse;
    use openidconnect::{EndpointMaybeSet, EndpointNotSet, EndpointSet, NonceVerifier, ProviderMetadataWithLogout, RequestTokenError};
    use reqwest::Client;
    use serde_json::Value;
    use url::Url;

    use sphare_core_common::checks::validate_redirect_url;
    use sphare_core_common::db_utils::ssr::get_db_pool;
    use sphare_core_common::errors::AppError;
    use sphare_core_common::routes::{get_app_origin, AUTH_CALLBACK_ROUTE};

    use crate::session::ssr::{get_session, get_user_lock_cache, AuthSession};
    use crate::user::ssr::{create_or_update_user, SqlUser};
    use crate::user::User;

    use super::*;

    static AUTH_REDIRECT: LazyLock<Result<oidc::RedirectUrl, AppError>> = LazyLock::new(|| {
        Ok(
            oidc::RedirectUrl::new(
                Url::parse(&get_app_origin()?)?.join(AUTH_CALLBACK_ROUTE)?.to_string()
            )?
        )
    });

    static OIDC_ISSUER_URL: LazyLock<Result<String, AppError>> = LazyLock::new(||  {
        check_oidc_url(std::env::var(OIDC_ISSUER_URL_ENV)?)
    });

    static OIDC_ISSUER_REALM: LazyLock<Result<String, AppError>> = LazyLock::new(|| {
        Ok(std::env::var(OIDC_ISSUER_REALM_ENV)?)
    });

    static OIDC_ISSUER_ADMIN_URL: LazyLock<Result<String, AppError>> = LazyLock::new(|| {
        check_oidc_url(std::env::var(OIDC_ISSUER_ADMIN_URL_ENV)?)
    });

    type OidcCoreClient = openidconnect::core::CoreClient<
        EndpointSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointMaybeSet,
        EndpointMaybeSet
    >;

    /// A no-op NonceVerifier implementation.
    struct NoNonceVerifier;

    impl NonceVerifier for NoNonceVerifier {
        fn verify(self, _nonce: Option<&openidconnect::Nonce>) -> Result<(), String> {
            Ok(())
        }
    }

    fn trim_trailing_slash(s: &mut String) {
        while s.ends_with('/') && !s.is_empty() {
            s.pop();
        }
    }

    fn check_oidc_url(mut oidc_url: String) -> Result<String, AppError> {
        trim_trailing_slash(&mut oidc_url);
        Url::parse(&oidc_url)?;
        Ok(oidc_url)
    }

    pub fn get_auth_redirect() -> Result<&'static oidc::RedirectUrl, AppError> {
        AUTH_REDIRECT.as_ref().map_err(AppError::clone)
    }

    pub fn get_oidc_issuer_url() -> Result<&'static String, AppError> {
        OIDC_ISSUER_URL.as_ref().map_err(AppError::clone)
    }

    pub fn get_oidc_token_endpoint() -> Result<String, AppError> {
        match (OIDC_ISSUER_ADMIN_URL.as_ref(), OIDC_ISSUER_REALM.as_ref()) {
            (Ok(admin_url), Ok(realm)) => Ok(format!("{admin_url}/realms/{realm}/protocol/openid-connect/token")),
            (Err(e), _) => Err(e.clone()),
            (_, Err(e)) => Err(e.clone()),
        }
    }

    pub fn get_oidc_delete_user_endpoint(user_oidc_id: &str) -> Result<String, AppError> {
        match (OIDC_ISSUER_ADMIN_URL.as_ref(), OIDC_ISSUER_REALM.as_ref()) {
            (Ok(admin_url), Ok(realm)) => Ok(format!("{admin_url}/admin/realms/{realm}/users/{user_oidc_id}")),
            (Err(e), _) => Err(e.clone()),
            (_, Err(e)) => Err(e.clone()),
        }
    }

    pub fn get_oidc_client_id() -> Result<oidc::ClientId, AppError> {
        Ok(oidc::ClientId::new(std::env::var(AUTH_CLIENT_ID_ENV)?))
    }

    fn get_oidc_client_secret() -> Result<oidc::ClientSecret, AppError> {
        Ok(oidc::ClientSecret::new(std::env::var(AUTH_CLIENT_SECRET_ENV)?))
    }

    pub fn get_logout_redirect() -> Result<oidc::PostLogoutRedirectUrl, AppError> {
        Ok(oidc::PostLogoutRedirectUrl::new(get_app_origin()?)?)
    }

    pub fn get_oidc_http_client() -> Result<Client, AppError> {
        let http_client = reqwest::ClientBuilder::new()
            // Following redirects opens the client up to SSRF vulnerabilities.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(AppError::from)?;

        Ok(http_client)
    }

    pub async fn get_provider_metadata(http_client: &Client) -> Result<ProviderMetadataWithLogout, AppError> {
        let issuer_url = oidc::IssuerUrl::new(get_oidc_issuer_url()?.clone())?;
        let provider_metadata = ProviderMetadataWithLogout::discover_async(issuer_url.clone(), http_client).await?;

        Ok(provider_metadata)
    }

    pub async fn get_oidc_client(http_client: &Client) -> Result<OidcCoreClient, AppError> {
        let auth_redirect = get_auth_redirect()?;
        let provider_metadata = get_provider_metadata(http_client).await?;
        // TODO cache client with a periodic refresh?
        // Create an OpenID Connect client by specifying the client ID, client secret, authorization URL and token URL.
        let client = oidc::core::CoreClient::from_provider_metadata(
            provider_metadata.clone(),
            get_oidc_client_id()?,
            Some(get_oidc_client_secret()?),
        )
            // Set the URL the user will be redirected to after the authorization process.
            .set_redirect_uri(auth_redirect.clone());

        Ok(client)
    }

    pub async fn check_user() -> Result<User, AppError> {
        let user = get_user().await?;
        user.ok_or(AppError::NotAuthenticated)
    }

    pub fn reload_user(user_id: i64) -> Result<(), AppError> {
        let auth_session = get_session()?;
        auth_session.cache_clear_user(user_id);
        Ok(())
    }
    
    fn get_nonce(auth_session: &AuthSession) -> Option<oidc::Nonce> {
        match auth_session.session.get::<String>(NONCE_KEY) {
            Some(nonce) if !nonce.is_empty() => Some(oidc::Nonce::new(nonce)),
            _ => None,
        }
    }

    async fn get_oidc_provider_token(http_client: &Client) -> Result<String, AppError> {

        let token_url = get_oidc_token_endpoint()?;
        let client_id = get_oidc_client_id()?;
        let client_secret = get_oidc_client_secret()?;
        let params = [
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.secret()),
            ("grant_type", "client_credentials"),
        ];
        let token_response = http_client.post(token_url)
            .form(&params)
            .send().await?
            .json::<Value>().await?;

        let access_token = token_response["access_token"]
            .as_str()
            .ok_or(AppError::new("Failed to retrieve access token"))?;

        Ok(access_token.to_string())
    }

    pub async fn get_user() -> Result<Option<User>, AppError> {
        let auth_session = get_session()?;
        if let Some(user) = &auth_session.current_user {
            let user_lock = get_user_lock_cache()?.get_user_lock(user.user_id).await;

            // Lock the mutex for this user
            let _lock = user_lock.lock().await;

            let token_response: CoreTokenResponse = auth_session
                .session
                .get(OIDC_TOKEN_KEY)
                .ok_or(AppError::new("Token missing, cannot check validity."))?;

            let id_token = token_response.id_token().ok_or(AppError::new("Id token missing."))?;

            let http_client = get_oidc_http_client()?;
            let client = get_oidc_client(&http_client).await?;

            let claims = match get_nonce(&auth_session) {
                Some(nonce) => id_token.claims(&client.id_token_verifier(), &nonce),
                None => id_token.claims(&client.id_token_verifier(), NoNonceVerifier),
            };
            match claims {
                Err(openidconnect::ClaimsVerificationError::Expired(_)) => {
                    log::debug!("Id token expired, refresh tokens.");
                    auth_session.session.remove(NONCE_KEY);
                    let refresh_token = token_response.refresh_token().ok_or(AppError::new("Error getting refresh token."))?;
                    let token_response = client
                        .exchange_refresh_token(refresh_token)?
                        .request_async(&http_client)
                        .await;

                    match token_response {
                        Ok(token_response) => {
                            let sql_user = process_oidc_token_response(token_response, auth_session.clone(), client).await?;
                            let db_pool = get_db_pool()?;
                            let user = User::get(sql_user.user_id, &db_pool).await;
                            log::debug!("Logged in as {:?}", sql_user);
                            auth_session.cache_clear_user(sql_user.user_id);
                            Ok(user)
                        }
                        Err(e) => {
                            match e {
                                RequestTokenError::ServerResponse(response) => {
                                    log::error!("Failed to refresh token: server returned an error: {:?}", response);
                                }
                                RequestTokenError::Request(http_err) => {
                                    log::error!("Failed to refresh token: HTTP request failed: {:?}", http_err);
                                }
                                RequestTokenError::Parse(err, body) => {
                                    log::error!("Failed to refresh token: failed to parse response: {:?}. Response body: {:?}", err, body);
                                }
                                RequestTokenError::Other(msg) => {
                                    log::error!("Failed to refresh token: other error: {:?}", msg);
                                }
                            }
                            auth_session.logout_user();
                            Ok(None)
                        }
                    }
                },
                Err(e) => {
                    log::error!("Unexpected error while getting claims: {e}");
                    auth_session.session.remove(NONCE_KEY);
                    auth_session.session.remove(OIDC_TOKEN_KEY);
                    auth_session.logout_user();
                    Ok(None)
                },
                Ok(claims) => {
                    log::debug!("Id token valid until {}", claims.expiration());
                    Ok(auth_session.current_user)
                },
            }
        } else {
            log::debug!("Not logged in.");
            Ok(None)
        }
    }

    /// process the input token response, upsert the corresponding user and returns it
    pub async fn process_oidc_token_response(
        token_response: CoreTokenResponse,
        auth_session: AuthSession,
        client: OidcCoreClient,
    ) -> Result<SqlUser, AppError> {
        // Extract the ID token claims after verifying its authenticity and nonce.
        let id_token = token_response
            .id_token()
            .ok_or(AppError::new("Id token missing."))?;

        let claims = match get_nonce(&auth_session) {
            Some(nonce) => id_token.claims(&client.id_token_verifier(), &nonce),
            None => id_token.claims(&client.id_token_verifier(), NoNonceVerifier),
        };

        let id_token_verifier = client.id_token_verifier();
        let claims = match claims {
            Ok(claims) => claims,
            Err(e) => {
                log::error!("Failed to get claims: {e}.");
                return Err(e.into());
            }
        };

        // Verify the access token hash to ensure that the access token hasn't been substituted for another user's.
        if let Some(expected_access_token_hash) = claims.access_token_hash() {
            let actual_access_token_hash = oidc::AccessTokenHash::from_token(
                token_response.access_token(),
                id_token.signing_alg()?,
                id_token.signing_key(&id_token_verifier)?,
            )?;
            if actual_access_token_hash != *expected_access_token_hash {
                return Err(AppError::new("Invalid access token"));
            }
        }

        // The authenticated user's identity is now available. See the IdTokenClaims struct for a
        // complete listing of the available claims.
        log::debug!(
            "User {} with e-mail address {} has authenticated successfully",
            claims.subject().as_str(),
            claims
                .email()
                .map(|email| email.as_str())
                .unwrap_or("<not provided>"),
        );

        auth_session.session.remove(OIDC_TOKEN_KEY);
        auth_session.session.set(OIDC_TOKEN_KEY, token_response.clone());

        let oidc_id = claims.subject().to_string();
        let db_pool = get_db_pool()?;

        let username: String = claims.preferred_username().ok_or(AppError::new("Username missing from token"))?.to_string();
        let email: String = claims.email().ok_or(AppError::new("Email missing from token"))?.to_string();
        let user = create_or_update_user(&oidc_id, &username, &email, &db_pool).await?;

        Ok(user)
    }

    pub async fn redirect_to_oidc_provider(redirect_url: String) -> Result<(), AppError> {
        validate_redirect_url(&redirect_url)?;
        let client = get_oidc_client(&get_oidc_http_client()?).await?;
        // Generate the full authorization URL.
        let (auth_url, _csrf_token, nonce) = client
            .authorize_url(
                oidc::core::CoreAuthenticationFlow::AuthorizationCode,
                oidc::CsrfToken::new_random,
                oidc::Nonce::new_random,
            ).url();

        let auth_session = get_session()?;

        auth_session.session.set(NONCE_KEY, nonce);
        auth_session.session.set(REDIRECT_URL_KEY, redirect_url);

        // Redirect to the auth page
        leptos_axum::redirect(auth_url.as_ref());
        Ok(())
    }

    pub async fn navigate_to_user_account() -> Result<(), AppError> {
        let issuer_url = get_oidc_issuer_url()?;
        let client = Client::new();
        // Fetch the discovery endpoint data
        let response = client.get(issuer_url.clone()).send().await.map_err(AppError::new)?.json::<Value>().await.map_err(AppError::new)?;

        // Obtain the account service url from it
        let account_service_url = response.get("account-service").and_then(|v| v.as_str()).ok_or(AppError::new("Account-service missing from provider"))?;

        // Redirect to the user account page
        leptos_axum::redirect(account_service_url);
        Ok(())
    }

    pub async fn delete_user_in_oidc_provider(user: &User) -> Result<(), AppError> {
        // clear auth session
        let auth_session = get_session()?;
        auth_session.session.remove(OIDC_TOKEN_KEY);
        auth_session.logout_user();
        // delete user in provider if admin endpoint defined
        let http_client = Client::builder().build()?;
        let access_token = get_oidc_provider_token(&http_client).await?;
        let delete_user_endpoint = get_oidc_delete_user_endpoint(&user.oidc_id)?;

        let response = http_client.delete(delete_user_endpoint.as_str())
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await?;

        match response.status().is_success() {
            true => Ok(()),
            false => Err(AppError::new(format!("Failed to delete user: {:?}", response.text().await?))),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_trim_trailing_slash() {
            let mut test_string = String::from("https://test.com/a/");
            trim_trailing_slash(&mut test_string);
            assert_eq!(test_string, "https://test.com/a");
            trim_trailing_slash(&mut test_string);
            assert_eq!(test_string, "https://test.com/a");
        }

        #[test]
        fn test_get_oidc_url_from_env() {
            let valid_url = String::from("https://login.sphare.space/realms/sphare");
            let trimmed_url = String::from("https://login.sphare.space/realms/sphare/");
            let invalid_url = String::from("invalid");
            assert_eq!(check_oidc_url(valid_url.clone()), Ok(valid_url));
            assert_eq!(check_oidc_url(trimmed_url.clone()).as_deref(), Ok(trimmed_url.trim_end_matches('/')));
            assert!(check_oidc_url(invalid_url).is_err());
        }
    }
}
