use const_format::formatcp;
use url::Url;
use validator::ValidationError;

use crate::constants::{MAX_SATELLITE_NAME_LENGTH, MAX_SPHERE_NAME_LENGTH, MAX_TITLE_LENGTH, MAX_USERNAME_LENGTH};
use crate::errors::AppError;
use crate::routes::get_app_origin;

/// # Returns whether the given string `input` is shorter or equal than the given max length and, if not `is_empty_ok` than it's not empty
///
/// ```
/// use sphare_core_common::checks::{check_string_length};
/// use sphare_core_common::errors::AppError;
///
/// assert!(check_string_length("hello", "input", 5, false).is_ok());
/// assert_eq!(check_string_length("hello", "input", 4, false), Err(AppError::new("input exceeds the maximum length: 4.")));
/// assert_eq!(check_string_length("", "input", 4, false), Err(AppError::new("input cannot be empty.")));
/// ```
pub fn check_string_length(
    input: &str,
    input_name: &str,
    max_length: usize,
    is_empty_ok: bool,
) -> Result<(), AppError> {
    match (input.len() > max_length, !is_empty_ok && input.is_empty()) {
        (true, _) => Err(AppError::new(format!("{input_name} exceeds the maximum length: {max_length}."))),
        (_, true) => Err(AppError::new(format!("{input_name} cannot be empty."))),
        (false, false) => Ok(()),
    }
}

/// # Returns whether a sphere name is valid, accepting empty string optionally
///
/// # Valid sphere names contain only ascii alphanumeric characters, '-', '_' and have a maximum length of `MAX_SPHERE_NAME_LENGTH`
///
/// ```
/// use sphare_core_common::checks::{check_sphere_name_with_options};
/// use sphare_core_common::constants::MAX_SPHERE_NAME_LENGTH;
/// use sphare_core_common::errors::AppError;
///
/// assert!(check_sphere_name_with_options("-Abc123_", true).is_ok());
/// assert!(check_sphere_name_with_options("", true).is_err());
/// assert!(check_sphere_name_with_options("", false).is_ok());
/// assert!(check_sphere_name_with_options(" name", true).is_err());
/// assert!(check_sphere_name_with_options("name%", true).is_err());
/// assert!(check_sphere_name_with_options(&"a".repeat(MAX_SPHERE_NAME_LENGTH), true).is_ok());
/// assert!(check_sphere_name_with_options(&"a".repeat(MAX_SPHERE_NAME_LENGTH + 1), true).is_err());
/// ```
pub fn check_sphere_name_with_options(name: &str, check_empty: bool) -> Result<(), ValidationError> {
    if name.is_empty() && check_empty {
        Err(ValidationError::new("Sphere name cannot be empty."))
    } else if !name.chars().all(move |c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        Err(ValidationError::new("Sphere name can only contain alphanumeric characters, dashes and underscores."))
    } else if name.len() > MAX_SPHERE_NAME_LENGTH {
        Err(ValidationError::new(formatcp!("Sphere name cannot exceed {MAX_SPHERE_NAME_LENGTH} characters.")))
    } else {
        Ok(())
    }
}

/// # Returns whether a sphere name is valid.
///
/// # Valid sphere names contain only ascii alphanumeric characters, '-', '_' and have a maximum length of `MAX_SPHERE_NAME_LENGTH`
///
/// ```
/// use sphare_core_common::checks::{check_sphere_name};
/// use sphare_core_common::constants::MAX_SPHERE_NAME_LENGTH;
/// use sphare_core_common::errors::AppError;
///
/// assert!(check_sphere_name("-Abc123_").is_ok());
/// assert!(check_sphere_name("").is_err());
/// assert!(check_sphere_name(" name").is_err());
/// assert!(check_sphere_name("name%").is_err());
/// assert!(check_sphere_name(&"a".repeat(MAX_SPHERE_NAME_LENGTH)).is_ok());
/// assert!(check_sphere_name(&"a".repeat(MAX_SPHERE_NAME_LENGTH + 1)).is_err());
/// ```
pub fn check_sphere_name(name: &str) -> Result<(), ValidationError> {
    check_sphere_name_with_options(name, true)
}

/// # Returns whether a satellite name is valid.
///
/// # Valid satellite names contain only ascii alphanumeric characters, '-', '_' and have a maximum length of `MAX_SPHERE_NAME_LENGTH`
///
/// ```
/// use sphare_core_common::checks::{check_satellite_name};
/// use sphare_core_common::constants::{MAX_SATELLITE_NAME_LENGTH};
/// use sphare_core_common::errors::AppError;
///
/// assert!(check_satellite_name("-Abc123_").is_ok());
/// assert!(check_satellite_name("").is_err());
/// assert!(check_satellite_name(" name").is_err());
/// assert!(check_satellite_name("name%").is_err());
/// assert!(check_satellite_name(&"a".repeat(MAX_SATELLITE_NAME_LENGTH)).is_ok());
/// assert!(check_satellite_name(&"a".repeat(MAX_SATELLITE_NAME_LENGTH + 1)).is_err());
/// ```
pub fn check_satellite_name(name: &str) -> Result<(), ValidationError> {
    if name.is_empty() {
        Err(ValidationError::new("Satellite name cannot be empty."))
    } else if !name.chars().all(move |c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        Err(ValidationError::new("Satellite name can only contain alphanumeric characters, dashes and underscores."))
    } else if name.len() > MAX_SATELLITE_NAME_LENGTH {
        Err(ValidationError::new(formatcp!("Satellite name cannot exceed {MAX_SPHERE_NAME_LENGTH} characters.")))
    } else {
        Ok(())
    }
}

/// # Returns whether a post's title is valid.
///
/// ```
/// use sphare_core_common::checks::{check_post_title};
/// use sphare_core_common::constants::MAX_TITLE_LENGTH;
/// use sphare_core_common::errors::AppError;
///
/// assert!(check_post_title("title").is_ok());
/// assert!(check_post_title("").is_err());
/// assert!(check_post_title("invalid\ntitle").is_err());
/// assert!(check_post_title("also invalid\rtitle").is_err());
/// assert!(check_post_title(&"a".repeat(MAX_TITLE_LENGTH as usize)).is_ok());
/// assert!(check_post_title(&"a".repeat(MAX_TITLE_LENGTH as usize + 1)).is_err());
/// ```
pub fn check_post_title(title: &str) -> Result<(), ValidationError> {
    if title.is_empty() {
        Err(ValidationError::new("Post title cannot be empty."))
    } else if title.len() > MAX_TITLE_LENGTH as usize {
        Err(ValidationError::new(formatcp!("Post title cannot exceed {MAX_TITLE_LENGTH} characters.")))
    } else if title.contains(&['\r', '\n'][..]) {
        Err(ValidationError::new(formatcp!("Post title cannot contain newlines.")))
    } else {
        Ok(())
    }
}

/// # Returns whether a username is valid.
///
/// # Valid usernames contain only ascii alphanumeric characters, '-', '_' and have a maximum length of `MAX_USERNAME_LENGTH`
///
/// ```
/// use sphare_core_common::checks::{check_username};
/// use sphare_core_common::constants::MAX_USERNAME_LENGTH;
/// use sphare_core_common::errors::AppError;
///
/// assert!(check_username("-Abc123_", false).is_ok());
/// assert!(check_username(" name", false).is_err());
/// assert!(check_username("name%", false).is_err());
/// assert!(check_username("", false).is_err());
/// assert!(check_username("", true).is_ok());
/// assert!(check_username(&"a".repeat(MAX_USERNAME_LENGTH), false).is_ok());
/// assert!(check_username(&"a".repeat(MAX_USERNAME_LENGTH + 1), false).is_err());
/// ```
pub fn check_username(name: &str, is_empty_ok: bool) -> Result<(), AppError> {
    if !name.chars().all(move |c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        Err(AppError::new("Username can only contain alphanumeric characters, dashes and underscores."))
    } else {
        check_string_length(name, "Username", MAX_USERNAME_LENGTH, is_empty_ok)
    }
}

pub fn validate_redirect_url(redirect_url: &str) -> Result<(), AppError> {
    let app_origin_str = get_app_origin()?;
    let app_origin = Url::parse(&app_origin_str).map_err(AppError::new)?;
    if let Ok(url) = Url::parse(redirect_url) {
        // absolute URL: check that scheme and domain correspond to app origin
        match url.origin() == app_origin.origin() {
            true => Ok(()),
            false => Err(AppError::new(format!("The redirect url {redirect_url} must have Sphare's origin {app_origin}."))),
        }
    } else if is_valid_pathname(redirect_url) {
        Ok(())
    } else {
        Err(AppError::new(format!("Invalid redirect url {redirect_url}: neither a valid url or pathname.")))
    }
}

fn is_valid_pathname(path: &str) -> bool {
    // Check if the path starts with '/' and is not empty
    if !path.starts_with('/') {
        return false
    }

    // Use Path to normalize and check for traversal
    let path_obj = std::path::Path::new(path);
    if path_obj.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return false
    }

    // Ensure no URL-encoded or invalid sequences
    !path.contains("//") && !path.contains("%2f") && !path.contains("%2e%2e")
}

#[cfg(test)]
mod tests {
    use sealed_test::prelude::*;

    use crate::checks::{is_valid_pathname, validate_redirect_url};
    use crate::routes::APP_ORIGIN_ENV;

    #[sealed_test]
    fn test_validate_redirect_url() {
        unsafe {
            std::env::set_var(APP_ORIGIN_ENV, "https://sphare.space");
        }
        assert!(validate_redirect_url("https://sphare.space/valid/url").is_ok());
        assert!(validate_redirect_url("http://sphare.space/valid/url").is_err());
        assert!(validate_redirect_url("https://invalid.redirect/").is_err());
        assert!(validate_redirect_url("/a/path/is/ok/too").is_ok());
        assert!(validate_redirect_url("a/path/is/ok/too").is_err());
        assert!(validate_redirect_url("/a/path/is/ok/too").is_ok());
    }

    #[test]
    fn test_is_valid_pathname() {
        assert_eq!(is_valid_pathname("/valid/pathname"), true);
        assert_eq!(is_valid_pathname("invalid/pathname"), false);
        assert_eq!(is_valid_pathname("//invalid/pathname"), false);
        assert_eq!(is_valid_pathname("/invalid/../pathname"), false);
        assert_eq!(is_valid_pathname("/invalid/%2f/pathname"), false);
        assert_eq!(is_valid_pathname("/invalid/%2e%2e/pathname"), false);
    }
}