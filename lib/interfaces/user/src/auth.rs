use leptos::prelude::*;

#[cfg(feature = "ssr")]
use openidconnect as oidc;

#[cfg(feature = "ssr")]
use {
    openidconnect::TokenResponse,
    sphare_core_common::{
        checks::validate_redirect_url,
        constants::SITE_ROOT,
    },
    sphare_core_user::{
        auth::ssr::{get_oidc_http_client, get_provider_metadata},
        auth::*,
        session::ssr::get_session,
    }
};

use sphare_core_common::errors::AppError;
use sphare_core_user::user::User;

#[server]
pub async fn login(redirect_url: String) -> Result<Option<String>, AppError> {
    let current_user = get_user().await;

    if let Ok(Some(_)) = current_user
    {
        return Ok(None);
    }

    let oidc_redirect_url = ssr::get_oidc_login_redirect_url(redirect_url).await?;
    leptos_axum::redirect(oidc_redirect_url.as_str());
    Ok(Some(oidc_redirect_url.to_string()))
}

#[server]
pub async fn navigate_to_user_account() -> Result<String, AppError> {
    let user_account_url = ssr::get_user_account_url().await?;
    leptos_axum::redirect(&user_account_url);
    Ok(user_account_url)
}

#[server]
pub async fn authenticate_user(auth_code: String) -> Result<(), AppError> {
    // Once the user has been redirected to the redirect URL, you'll have access to the
    // authorization code. For security reasons, your code should verify that the `state`
    // parameter returned by the server matches `csrf_state`.

    let auth_session = get_session()?;

    let redirect_url = auth_session
        .session
        .get(REDIRECT_URL_KEY)
        .unwrap_or(String::from(SITE_ROOT));
    validate_redirect_url(&redirect_url)?;

    if let Some(user) = auth_session.current_user {
        log::debug!("User {} was already authenticated", user.username);
    } else {
        let http_client = get_oidc_http_client()?;
        let client = ssr::get_oidc_client(&http_client).await?;

        // Now you can exchange it for an access token and ID token.
        let token_response = client
            .exchange_code(oidc::AuthorizationCode::new(auth_code))
            .map_err(AppError::from)?
            .request_async(&http_client)
            .await
            .map_err(AppError::from)?;

        let db_user = ssr::process_oidc_token_response(token_response, auth_session.clone(), client).await?;
        auth_session.login_user(db_user.person_id);
        auth_session.remember_user(true);
    }

    leptos_axum::redirect(redirect_url.as_ref());
    Ok(())
}

#[server]
pub async fn get_user() -> Result<Option<User>, AppError> {
    ssr::get_user().await
}

#[server]
pub async fn end_session(redirect_url: String) -> Result<String, AppError> {
    log::debug!("Logout, redirect_url: {redirect_url}");
    validate_redirect_url(&redirect_url)?;
    let http_client = get_oidc_http_client()?;
    let auth_session = get_session()?;
    let user = &auth_session.current_user;
    let token_response: oidc::core::CoreTokenResponse =
        auth_session
            .session
            .get(OIDC_TOKEN_KEY)
            .ok_or(AppError::InternalServerError(String::from("Not authenticated.")))?;

    let id_token = token_response.id_token().ok_or(AppError::AuthenticationError(String::from("Id token missing.")))?;

    let logout_endpoint = get_provider_metadata(&http_client).await?
        .additional_metadata()
        .end_session_endpoint
        .clone()
        .ok_or(AppError::new("Missing end session endpoint from provider metadata."))?;

    let logout_request = oidc::LogoutRequest::from(logout_endpoint)
        .set_client_id(ssr::get_oidc_client_id()?)
        .set_id_token_hint(id_token)
        .set_post_logout_redirect_uri(oidc::PostLogoutRedirectUrl::new(redirect_url).map_err(AppError::from)?);

    let oidc_redirect_url = logout_request.http_get_url().to_string();
    leptos_axum::redirect(&oidc_redirect_url);

    auth_session.session.remove(OIDC_TOKEN_KEY);
    if let Some(user) = user {
        auth_session.cache_clear_user(user.person_id);
    }
    auth_session.logout_user();

    Ok(oidc_redirect_url)
}