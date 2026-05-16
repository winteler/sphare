pub const SITE_NAME: &str = "Sphare";

pub const SECONDS_IN_MINUTE: i64 = 60;
pub const MINUTES_IN_HOUR: i64 = 60;
pub const HOURS_IN_DAY: i64 = 24;
pub const DAYS_IN_MONTH: i64 = 31;
pub const DAYS_IN_YEAR: i64 = 365;


pub const SECONDS_IN_HOUR: i64 = MINUTES_IN_HOUR*SECONDS_IN_MINUTE;
pub const SECONDS_IN_DAY: i64 = HOURS_IN_DAY*SECONDS_IN_HOUR;
pub const SECONDS_IN_MONTH: i64 = DAYS_IN_MONTH*SECONDS_IN_DAY;
pub const SECONDS_IN_YEAR: i64 = DAYS_IN_YEAR*SECONDS_IN_DAY;


pub const SPOILER_TAG: &str = "||";
pub const MULTI_LINE_SPOILER_TAG: &str = "|||";


pub const HOT_ORDER_BY_COLUMN: &str = "recommended_score";
pub const TRENDING_ORDER_BY_COLUMN: &str = "trending_score";
pub const BEST_ORDER_BY_COLUMN: &str = "score";
pub const RECENT_ORDER_BY_COLUMN: &str = "create_timestamp";


pub const SITE_ROOT: &str = "/";
pub const IMAGE_TYPE: &str = "image/";
pub const SCROLL_LOAD_THROTTLE_DELAY: f64 = 3000.0;


pub const LOGO_ICON_PATH: &str = "/svg/planet.svg";
pub const POPULAR_ICON_PATH: &str = "/svg/shooting_star.svg";


pub const MAX_SPHERE_NAME_LENGTH: usize = 20;
pub const MAX_SPHERE_DESCRIPTION_LENGTH: usize = 1000;
pub const MAX_SATELLITE_NAME_LENGTH: usize = 50;
pub const MAX_USERNAME_LENGTH: usize = 30;
pub const MAX_TITLE_LENGTH: u64 = 250;
pub const MAX_CONTENT_LENGTH: u64 = 20000;
pub const MAX_LINK_LENGTH: u64 = 500;
pub const MAX_MOD_MESSAGE_LENGTH: usize = 500;
pub const MAX_SEARCH_QUERY_LENGTH: usize = 200;
pub const MAX_CATEGORY_NAME_LENGTH: usize = 50;
pub const MAX_CATEGORY_DESCRIPTION_LENGTH: usize = 500;


pub const SPHERE_NAME_PARAM: &str = "sphere_name";
pub const IMAGE_FILE_PARAM: &str = "image";


pub const USER_FETCH_LIMIT: i64 = 100;
pub const SPHERE_FETCH_LIMIT: usize = 100;
pub const SPHERE_HEADER_FETCH_LIMIT: usize = 10;
pub const POST_BATCH_SIZE: i64 = 50;
pub const COMMENT_BATCH_SIZE: i64 = 50;