use std::fmt;
use std::fmt::Display;
use std::str::FromStr;

use http::status::StatusCode;
use leptos::prelude::*;
use leptos::server_fn::codec::JsonEncoding;
use leptos_fluent::move_tr;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use validator::{ValidationError, ValidationErrors};

#[macro_export] macro_rules! to_app_error {
    ($msg:expr) => {
        |e| AppError::InternalServerError(format!("{}: {}", $msg, e))
    };
}

#[derive(Clone, Debug, Error, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppError {
    AuthenticationError(String),
    NotAuthenticated,
    InsufficientPrivileges,
    SphereBanUntil(chrono::DateTime<chrono::Utc>),
    PermanentSphereBan,
    GlobalBanUntil(chrono::DateTime<chrono::Utc>),
    PermanentGlobalBan,
    CommunicationError(ServerFnErrorErr),
    DatabaseError(String),
    InternalServerError(String),
    NotFound,
    PayloadTooLarge(usize),
    ApubError(String),
}

impl AppError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            AppError::AuthenticationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotAuthenticated | AppError::InsufficientPrivileges | AppError::SphereBanUntil(_) |
            AppError::PermanentSphereBan | AppError::GlobalBanUntil(_) | AppError::PermanentGlobalBan => StatusCode::FORBIDDEN,
            AppError::CommunicationError(error) => match error {
                ServerFnErrorErr::Args(_) | ServerFnErrorErr::MissingArg(_) | ServerFnErrorErr::Serialization(_) | ServerFnErrorErr::Deserialization(_) => StatusCode::BAD_REQUEST,
                ServerFnErrorErr::Registration(_) | ServerFnErrorErr::Request(_) | ServerFnErrorErr::Response(_) => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            AppError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::InternalServerError(_) | AppError::ApubError(_)  => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::PayloadTooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
        }
    }

    pub fn user_message(&self) -> Signal<String> {
        match self {
            AppError::AuthenticationError(_) => move_tr!("authentication-failed-message"),
            AppError::NotAuthenticated => move_tr!("not-authenticated-message"),
            AppError::InsufficientPrivileges => move_tr!("not-authorized-message"),
            AppError::SphereBanUntil(timestamp) => {
                let timestamp_str = timestamp.to_string();
                move_tr!("sphere-ban-until-message", {"timestamp" => timestamp_str.clone()})
            },
            AppError::PermanentSphereBan => move_tr!("permanent-sphere-ban-message"),
            AppError::GlobalBanUntil(timestamp) => {
                let timestamp_str = timestamp.to_string();
                move_tr!("global-ban-until-message", {"timestamp" => timestamp_str.clone()})
            },
            AppError::PermanentGlobalBan => move_tr!("permanent-global-ban-message"),
            AppError::CommunicationError(error) => match error {
                ServerFnErrorErr::Args(_) | ServerFnErrorErr::MissingArg(_) |
                ServerFnErrorErr::Serialization(_) | ServerFnErrorErr::Deserialization(_) => move_tr!("bad-request-message"),
                ServerFnErrorErr::Registration(_) | ServerFnErrorErr::Request(_) | ServerFnErrorErr::Response(_) => move_tr!("unavailable-message"),
                _ => move_tr!("internal-error-message"),
            },
            AppError::DatabaseError(_) => move_tr!("internal-error-message"),
            AppError::InternalServerError(_) | AppError::ApubError(_) => move_tr!("internal-error-message"),
            AppError::NotFound => move_tr!("not-found-message"),
            AppError::PayloadTooLarge(byte_limit) => {
                let byte_limit = *byte_limit as f64 / 1024.0 / 1024.0;
                move_tr!("payload-too-large-message", {"mb_limit" => byte_limit})
            },
        }
    }

    pub fn error_detail(&self) -> Signal<String> {
        match self {
            AppError::AuthenticationError(e) => e.clone().into(),
            AppError::CommunicationError(error) => match error {
                ServerFnErrorErr::Args(e) | ServerFnErrorErr::MissingArg(e) |
                ServerFnErrorErr::Serialization(e) | ServerFnErrorErr::Deserialization(e) => e.clone().into(),
                ServerFnErrorErr::Registration(e) | ServerFnErrorErr::Request(e) | ServerFnErrorErr::Response(e) => e.clone().into(),
                _ => self.user_message(),
            },
            AppError::InternalServerError(e) => e.clone().into(),
            AppError::ApubError(e) => e.clone().into(),
            _ => self.user_message()
        }
    }

    /// Constructs a new [`AppError::InternalServerError`] from some other type.
    pub fn new(msg: impl ToString) -> Self {
        Self::InternalServerError(msg.to_string())
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap_or_default())
    }
}

impl FromStr for AppError {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl FromServerFnError for AppError {
    type Encoder = JsonEncoding;

    fn from_server_fn_error(error: ServerFnErrorErr) -> Self {
        match error {
            ServerFnErrorErr::ServerError(message) => serde_json::from_str(message.as_str()).unwrap_or(AppError::InternalServerError(message.clone())),
            _ => AppError::CommunicationError(error),
        }
    }
}

impl From<ValidationError> for AppError {
    fn from(error: ValidationError) -> Self {
        AppError::new(error)
    }
}

impl From<ValidationErrors> for AppError {
    fn from(errors: ValidationErrors) -> Self {
        AppError::new(errors)
    }
}

impl From<quick_xml::Error> for AppError {
    fn from(error: quick_xml::Error) -> Self {
        AppError::InternalServerError(error.to_string())
    }
}

impl From<std::string::FromUtf8Error> for AppError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        AppError::InternalServerError(error.to_string())
    }
}

impl From<url::ParseError> for AppError {
    fn from(error: url::ParseError) -> Self {
        AppError::AuthenticationError(error.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        AppError::InternalServerError(value.to_string())
    }
}

#[cfg(feature = "ssr")]
mod ssr {
    use activitypub_federation::error::Error;
    use axum::response::{IntoResponse, Response};
    use openidconnect::SignatureVerificationError;
    use sqlx;

    use crate::errors::AppError;

    impl From<sqlx::Error> for AppError {
        fn from(error: sqlx::Error) -> Self {
            match error {
                sqlx::Error::RowNotFound => AppError::NotFound,
                _ => AppError::DatabaseError(error.to_string()),
            }
        }
    }

    impl From<std::env::VarError> for AppError {
        fn from(error: std::env::VarError) -> Self {
            AppError::InternalServerError(error.to_string())
        }
    }

    impl From<openidconnect::ClaimsVerificationError> for AppError {
        fn from(error: openidconnect::ClaimsVerificationError) -> Self {
            AppError::AuthenticationError(error.to_string())
        }
    }

    impl From<openidconnect::ConfigurationError> for AppError {
        fn from(error: openidconnect::ConfigurationError) -> Self {
            AppError::AuthenticationError(error.to_string())
        }
    }

    impl From<openidconnect::SigningError> for AppError {
        fn from(error: openidconnect::SigningError) -> Self {
            AppError::AuthenticationError(error.to_string())
        }
    }

    impl From<SignatureVerificationError> for AppError {
        fn from(value: SignatureVerificationError) -> Self {
            AppError::AuthenticationError(value.to_string())
        }
    }

    impl<T: std::error::Error> From<openidconnect::DiscoveryError<T>> for AppError {
        fn from(error: openidconnect::DiscoveryError<T>) -> Self {
            AppError::AuthenticationError(error.to_string())
        }
    }

    impl<A: std::error::Error, B: openidconnect::ErrorResponse> From<openidconnect::RequestTokenError<A, B>> for AppError {
        fn from(error: openidconnect::RequestTokenError<A, B>) -> Self {
            AppError::AuthenticationError(error.to_string())
        }
    }

    impl From<reqwest::Error> for AppError {
        fn from(value: reqwest::Error) -> Self {
            AppError::InternalServerError(value.to_string())
        }
    }

    impl From<activitypub_federation::error::Error> for AppError {
        fn from(value: Error) -> Self {
            AppError::InternalServerError(format!("ActivityPub error: {value}"))
        }
    }

    impl IntoResponse for AppError {
        fn into_response(self) -> Response {
            (self.status_code(), self.to_string()).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::LazyLock;

    use fluent_templates::{static_loader, StaticLoader};
    use http::StatusCode;
    use leptos::prelude::*;
    use leptos_fluent::{tr, I18n, Language};
    use quick_xml::errors::SyntaxError;

    use crate::errors::AppError;

    const EN_LANG: Language = Language {
        id: "en",
        name: "English",
        dir: &leptos_fluent::WritingDirection::Ltr,
        flag: None,
        script: None,
    };
    const FR_LANG: Language = Language {
        id: "fr",
        name: "Français",
        dir: &leptos_fluent::WritingDirection::Ltr,
        flag: None,
        script: None,
    };
    const LANGUAGES: &[&Language] = &[
        &EN_LANG,
        &FR_LANG,
    ];

    #[test]
    fn test_app_error_status_code() {
        let test_string = String::from("test");
        let test_timestamp = chrono::DateTime::from_timestamp_nanos(0);
        let server_fn_error = ServerFnErrorErr::ServerError(String::from("test"));
        let args_error = ServerFnErrorErr::Args(String::from("test"));
        let missing_arg_error = ServerFnErrorErr::MissingArg(String::from("test"));
        let request_error = ServerFnErrorErr::Request(String::from("test"));
        let response_error = ServerFnErrorErr::Response(String::from("test"));
        let registration_error = ServerFnErrorErr::Registration(String::from("test"));
        let serialization_error = ServerFnErrorErr::Serialization(String::from("test"));
        let deserialization_error = ServerFnErrorErr::Deserialization(String::from("test"));
        assert_eq!(AppError::AuthenticationError(test_string.clone()).status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(AppError::NotAuthenticated.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(AppError::InsufficientPrivileges.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(AppError::SphereBanUntil(test_timestamp).status_code(), StatusCode::FORBIDDEN);
        assert_eq!(AppError::PermanentSphereBan.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(AppError::GlobalBanUntil(test_timestamp).status_code(), StatusCode::FORBIDDEN);
        assert_eq!(AppError::PermanentGlobalBan.status_code(), StatusCode::FORBIDDEN);
        assert_eq!(AppError::CommunicationError(server_fn_error).status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(AppError::CommunicationError(args_error).status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(AppError::CommunicationError(missing_arg_error).status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(AppError::CommunicationError(serialization_error).status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(AppError::CommunicationError(deserialization_error).status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(AppError::CommunicationError(request_error).status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(AppError::CommunicationError(response_error).status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(AppError::CommunicationError(registration_error).status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(AppError::DatabaseError(test_string.clone()).status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(AppError::InternalServerError(test_string.clone()).status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(AppError::NotFound.status_code(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_app_error_user_message() {
        let owner = Owner::new();
        owner.set();
        static_loader! {
            static TRANSLATIONS = {
                locales: "../../../locales",
                fallback_language: "en",
            };
        }
        let compound: Vec<&LazyLock<StaticLoader>> = vec![&TRANSLATIONS];
        let i18n = I18n::new(
            RwSignal::new(&LANGUAGES[0]),
            LANGUAGES,
            Signal::derive(move || compound.clone())
        );

        provide_context(i18n);

        let test_string = String::from("test");
        let test_timestamp = chrono::DateTime::from_timestamp_nanos(0);
        let server_fn_error = ServerFnErrorErr::ServerError(String::from("test"));
        let args_error = ServerFnErrorErr::Args(String::from("test"));
        let missing_arg_error = ServerFnErrorErr::MissingArg(String::from("test"));
        let request_error = ServerFnErrorErr::Request(String::from("test"));
        let response_error = ServerFnErrorErr::Response(String::from("test"));
        let registration_error = ServerFnErrorErr::Registration(String::from("test"));
        let serialization_error = ServerFnErrorErr::Serialization(String::from("test"));
        let deserialization_error = ServerFnErrorErr::Deserialization(String::from("test"));
        assert_eq!(AppError::AuthenticationError(test_string.clone()).user_message().get_untracked(), tr!("authentication-failed-message"));
        assert_eq!(AppError::NotAuthenticated.user_message().get_untracked(), tr!("not-authenticated-message"));
        assert_eq!(AppError::InsufficientPrivileges.user_message().get_untracked(), tr!("not-authorized-message"));
        assert_eq!(AppError::SphereBanUntil(test_timestamp).user_message().get_untracked(), tr!("sphere-ban-until-message", {"timestamp" => test_timestamp.to_string()}));
        assert_eq!(AppError::PermanentSphereBan.user_message().get_untracked(), tr!("permanent-sphere-ban-message"));
        assert_eq!(AppError::GlobalBanUntil(test_timestamp).user_message().get_untracked(), tr!("global-ban-until-message", {"timestamp" => test_timestamp.to_string()}));
        assert_eq!(AppError::PermanentGlobalBan.user_message().get_untracked(), tr!("permanent-global-ban-message"));
        assert_eq!(AppError::CommunicationError(server_fn_error).user_message().get_untracked(), tr!("internal-error-message"));
        assert_eq!(AppError::CommunicationError(args_error).user_message().get_untracked(), tr!("bad-request-message"));
        assert_eq!(AppError::CommunicationError(missing_arg_error).user_message().get_untracked(), tr!("bad-request-message"));
        assert_eq!(AppError::CommunicationError(serialization_error).user_message().get_untracked(), tr!("bad-request-message"));
        assert_eq!(AppError::CommunicationError(deserialization_error).user_message().get_untracked(), tr!("bad-request-message"));
        assert_eq!(AppError::CommunicationError(request_error).user_message().get_untracked(), tr!("unavailable-message"));
        assert_eq!(AppError::CommunicationError(response_error).user_message().get_untracked(), tr!("unavailable-message"));
        assert_eq!(AppError::CommunicationError(registration_error).user_message().get_untracked(), tr!("unavailable-message"));
        assert_eq!(AppError::DatabaseError(test_string.clone()).user_message().get_untracked(), tr!("internal-error-message"));
        assert_eq!(AppError::InternalServerError(test_string.clone()).user_message().get_untracked(), tr!("internal-error-message"));
        assert_eq!(AppError::NotFound.user_message().get_untracked(), tr!("not-found-message"));
    }

    #[test]
    fn test_app_error_new() {
        let test_str = "test";
        assert_eq!(AppError::new(test_str), AppError::InternalServerError(String::from(test_str)));
    }

    #[test]
    fn test_app_error_display_and_from_string() {
        let test_string = String::from("test");
        let test_timestamp = chrono::DateTime::from_timestamp_nanos(0);
        let server_fn_error = ServerFnErrorErr::ServerError(String::from("test"));
        let server_fn_error_2 = ServerFnErrorErr::MissingArg(test_string.clone());
        assert_eq!(
            AppError::from_str(AppError::AuthenticationError(test_string.clone()).to_string().as_str()).expect("AppError should be convert to string and back"),
            AppError::AuthenticationError(test_string.clone())
        );
        assert_eq!(
            AppError::from_str(AppError::NotAuthenticated.to_string().as_str()).expect("AppError should be convert to string and back"),
            AppError::NotAuthenticated
        );
        assert_eq!(
            AppError::from_str(AppError::InsufficientPrivileges.to_string().as_str()).expect("AppError should be convert to string and back"),
            AppError::InsufficientPrivileges
        );
        assert_eq!(
            AppError::from_str(AppError::SphereBanUntil(test_timestamp).to_string().as_str()).expect("AppError should be convert to string and back"),
            AppError::SphereBanUntil(test_timestamp)
        );
        assert_eq!(
            AppError::from_str(AppError::PermanentSphereBan.to_string().as_str()).expect("AppError should be convert to string and back"),
            AppError::PermanentSphereBan
        );
        assert_eq!(
            AppError::from_str(AppError::GlobalBanUntil(test_timestamp).to_string().as_str()).expect("AppError should be convert to string and back"),
            AppError::GlobalBanUntil(test_timestamp)
        );
        assert_eq!(
            AppError::from_str(AppError::PermanentGlobalBan.to_string().as_str()).expect("AppError should be convert to string and back"),
            AppError::PermanentGlobalBan
        );
        assert_eq!(
            AppError::from_str(AppError::CommunicationError(server_fn_error.clone()).to_string().as_str()).expect("AppError should be convert to string and back"),
            AppError::CommunicationError(server_fn_error)
        );
        assert_eq!(
            AppError::from_str(AppError::CommunicationError(server_fn_error_2.clone()).to_string().as_str()).expect("AppError should be convert to string and back"),
            AppError::CommunicationError(server_fn_error_2)
        );
        assert_eq!(
            AppError::from_str(AppError::DatabaseError(test_string.clone()).to_string().as_str()).expect("AppError should be convert to string and back"),
            AppError::DatabaseError(test_string.clone())
        );
        assert_eq!(
            AppError::from_str(AppError::InternalServerError(test_string.clone()).to_string().as_str()).expect("AppError should be convert to string and back"),
            AppError::InternalServerError(test_string.clone())
        );
        assert_eq!(
            AppError::from_str(AppError::NotFound.to_string().as_str()).expect("AppError should be convert to string and back"),
            AppError::NotFound
        );
        assert!(AppError::from_str("invalid").is_err());
    }

    #[test]
    fn test_app_error_from_string_utf8_error() {
        // some invalid bytes, in a vector
        let invalid_bytes = vec![0, 159, 146, 150];
        let error = String::from_utf8(invalid_bytes);
        assert!(error.is_err());
        let error =  error.unwrap_err();
        assert_eq!(AppError::from(error.clone()), AppError::InternalServerError(error.to_string()));
    }

    #[test]
    fn test_app_error_from_openidconnect_url_parse_error() {
        let error = url::ParseError::InvalidDomainCharacter;
        assert_eq!(AppError::from(error), AppError::AuthenticationError(error.to_string()));
    }

    #[test]
    fn test_app_error_from_quick_xml_error() {
        let error = quick_xml::Error::Syntax(SyntaxError::UnclosedComment);
        assert_eq!(AppError::from(error.clone()), AppError::InternalServerError(error.to_string()));
    }

    #[test]
    #[cfg(feature = "ssr")]
    fn test_app_error_from_sqlx_error() {
        let error_string = String::from("test");
        assert_eq!(AppError::from(sqlx::Error::RowNotFound), AppError::NotFound);
        assert_eq!(AppError::from(sqlx::Error::PoolTimedOut), AppError::DatabaseError(sqlx::Error::PoolTimedOut.to_string()));
        assert_eq!(AppError::from(sqlx::Error::ColumnNotFound(error_string.clone())), AppError::DatabaseError(sqlx::Error::ColumnNotFound(error_string).to_string()));
    }

    #[test]
    #[cfg(feature = "ssr")]
    fn test_app_error_from_env_var_error() {
        let env_var_error = std::env::var("not_existing");
        assert!(env_var_error.is_err());
        let env_var_error =  env_var_error.unwrap_err();
        assert_eq!(AppError::from(env_var_error.clone()), AppError::InternalServerError(env_var_error.to_string()));
    }

    #[test]
    #[cfg(feature = "ssr")]
    fn test_app_error_from_openidconnect_discovery_error() {
        // as this is a generic error type, we need to provide it with a type implementing error
        assert_eq!(
            AppError::from(openidconnect::DiscoveryError::<openidconnect::ConfigurationError>::Validation(String::from("test"))),
            AppError::AuthenticationError(openidconnect::DiscoveryError::<openidconnect::ConfigurationError>::Validation(String::from("test")).to_string())
        );
    }
}