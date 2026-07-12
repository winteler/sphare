use serde::{Deserialize, Serialize};

use crate::colors::Color;

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct SphereHeader {
    pub sphere_name: String,
    pub icon_url: Option<String>,
    pub is_nsfw: bool,
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct SphereCategoryHeader {
    pub category_name: String,
    pub category_color: Color,
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Rule {
    pub rule_id: i64,
    pub rule_key: i64, // business id to track rule across updates
    pub sphere_id: Option<i64>,
    pub priority: i16,
    pub title: String,
    pub description: String,
    pub markdown_description: Option<String>,
    pub person_id: i64,
    pub create_timestamp: chrono::DateTime<chrono::Utc>,
    pub delete_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

impl SphereHeader {
    pub fn new(sphere_name: String, icon_url: Option<String>, is_nsfw: bool) -> Self {
        Self {
            sphere_name,
            icon_url,
            is_nsfw,
        }
    }
}