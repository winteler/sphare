use serde::{Deserialize, Serialize};

use sphare_core_common::constants::{BEST_ORDER_BY_COLUMN, HOT_ORDER_BY_COLUMN, RECENT_ORDER_BY_COLUMN, TRENDING_ORDER_BY_COLUMN};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum PostSortType {
    Hot,
    Trending,
    Best,
    Recent,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum CommentSortType {
    Best,
    Recent,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum SortType {
    Post(PostSortType),
    Comment(CommentSortType),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[cfg_attr(feature = "ssr", derive(sqlx::Type))]
#[repr(i16)]
pub enum VoteValue {
    Up = 1,
    None = 0,
    Down = -1,
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Vote {
    pub vote_id: i64,
    pub user_id: i64,
    pub comment_id: Option<i64>,
    pub post_id: i64,
    pub value: VoteValue,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl PostSortType {
    pub fn to_order_by_code(self) -> &'static str {
        match self {
            PostSortType::Hot => HOT_ORDER_BY_COLUMN,
            PostSortType::Trending => TRENDING_ORDER_BY_COLUMN,
            PostSortType::Best => BEST_ORDER_BY_COLUMN,
            PostSortType::Recent => RECENT_ORDER_BY_COLUMN,
        }
    }
}

impl CommentSortType {
    pub fn to_order_by_code(self) -> &'static str {
        match self {
            CommentSortType::Best => BEST_ORDER_BY_COLUMN,
            CommentSortType::Recent => RECENT_ORDER_BY_COLUMN,
        }
    }
}

impl SortType {
    pub fn to_order_by_code(self) -> &'static str {
        match self {
            SortType::Post(post_sort_type) => post_sort_type.to_order_by_code(),
            SortType::Comment(comment_sort_type) => comment_sort_type.to_order_by_code(),
        }
    }
}

impl From<i16> for VoteValue {
    fn from(value: i16) -> VoteValue {
        match value {
            1..=i16::MAX => VoteValue::Up,
            0 => VoteValue::None,
            i16::MIN..=-1_i16 => VoteValue::Down,
        }
    }
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use crate::ranking::{Vote, VoteValue};
    use sphare_core_common::errors::AppError;
    use sphare_core_user::user::User;
    use sqlx::PgPool;

    pub async fn vote_on_content(
        vote_value: VoteValue,
        post_id: i64,
        comment_id: Option<i64>,
        vote_id: Option<i64>,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Option<Vote>, AppError> {
        user.check_can_publish()?;
        let (prev_vote_value, vote) = if let Some(vote_id) = vote_id {
            let current_vote = get_user_vote_on_content(post_id,  comment_id, vote_id, user, db_pool).await?;
            if current_vote.value == vote_value {
                log::debug!("Vote already has the right value, don't update it.");
                (current_vote.value, Some(current_vote))
            } else if vote_value != VoteValue::None {
                log::debug!("Update vote {vote_id} with value {vote_value:?}");
                let vote = sqlx::query_as!(
                    Vote,
                    "UPDATE votes SET value = $1
                    WHERE
                        vote_id = $2 AND
                        post_id = $3 AND
                        comment_id IS NOT DISTINCT FROM $4 AND
                        user_id = $5 AND
                        NOT EXISTS (
                            SELECT * FROM user_bans b
                            JOIN posts p ON p.sphere_id = b.sphere_id
                            WHERE
                                p.post_id = $3 AND
                                b.user_id = $5 AND
                                b.delete_timestamp IS NULL AND
                                (b.until_timestamp <= NOW() OR b.until_timestamp IS NULL)
                        )
                    RETURNING *",
                    vote_value as i16,
                    vote_id,
                    post_id,
                    comment_id,
                    user.user_id,
                )
                    .fetch_one(db_pool)
                    .await?;
                (current_vote.value, Some(vote))
            } else {
                log::debug!("Delete vote {vote_id}");
                sqlx::query!(
                    "DELETE from votes
                    WHERE vote_id = $1 AND
                          post_id = $2 AND
                          user_id = $3",
                    vote_id,
                    post_id,
                    user.user_id,
                )
                    .execute(db_pool)
                    .await?;
                (current_vote.value, None)
            }
        } else {
            log::debug!("Create vote for content {post_id}, comment {comment_id:?}, user {} with value {vote_value:?}", user.user_id);
            let vote = sqlx::query_as!(
                Vote,
                "INSERT INTO votes (post_id, comment_id, user_id, value)
                SELECT $1, $2, $3, $4
                WHERE NOT EXISTS (
                    SELECT * FROM user_bans b
                    JOIN posts p ON p.sphere_id = b.sphere_id
                    WHERE
                        p.post_id = $1 AND
                        b.user_id = $3 AND
                        b.delete_timestamp IS NULL AND
                        (b.until_timestamp <= NOW() OR b.until_timestamp IS NULL)
                ) RETURNING *",
                post_id,
                comment_id,
                user.user_id,
                vote_value as i16,
            )
                .fetch_one(db_pool)
                .await?;
            (VoteValue::None, Some(vote))
        };

        update_content_score(
            vote_value,
            post_id,
            comment_id,
            prev_vote_value,
            db_pool,
        ).await?;

        Ok(vote)
    }

    async fn update_content_score(
        vote: VoteValue,
        post_id: i64,
        comment_id: Option<i64>,
        previous_vote: VoteValue,
        db_pool: &PgPool,
    ) -> Result<(), AppError> {
        if vote != previous_vote {
            let (score_delta, minus_delta) = get_vote_deltas(vote, previous_vote);

            if comment_id.is_some() {
                sqlx::query!(
                    "UPDATE comments set score = score + $1, score_minus = score_minus + $2 where comment_id = $3",
                    score_delta,
                    minus_delta,
                    comment_id,
                )
                    .execute(db_pool)
                    .await?;
            } else {
                sqlx::query!(
                    "UPDATE posts set score = score + $1, score_minus = score_minus + $2, scoring_timestamp = NOW() where post_id = $3",
                    score_delta,
                    minus_delta,
                    post_id,
                )
                    .execute(db_pool)
                    .await?;
            }
        }

        Ok(())
    }

    fn get_vote_deltas(vote: VoteValue, previous_vote: VoteValue) -> (i32, i32) {
        let score_delta = (vote as i32) - (previous_vote as i32);
        let minus_delta = if vote == VoteValue::Down && previous_vote != VoteValue::Down {
            1
        } else if vote != VoteValue::Down && previous_vote == VoteValue::Down {
            -1
        } else {
            0
        };

        (score_delta, minus_delta)
    }

    async fn get_user_vote_on_content(
        post_id: i64,
        comment_id: Option<i64>,
        vote_id: i64,
        user: &User,
        db_pool: &PgPool,
    ) -> Result<Vote, AppError> {
        let vote = sqlx::query_as!(
            Vote,
            "SELECT *
            FROM votes
            WHERE
                post_id = $1 AND
                comment_id IS NOT DISTINCT FROM $2 AND
                vote_id = $3 AND
                user_id = $4",
            post_id,
            comment_id,
            vote_id,
            user.user_id,
        )
            .fetch_one(db_pool)
            .await?;

        Ok(vote)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_get_vote_deltas() {
            let mut vote = VoteValue::Up;
            let mut previous_vote = VoteValue::None;
            let (mut score_delta, mut minus_delta) = get_vote_deltas(vote, previous_vote);
            assert_eq!((score_delta, minus_delta), (1, 0));

            vote = VoteValue::None;
            previous_vote = VoteValue::Up;
            (score_delta, minus_delta) = get_vote_deltas(vote, previous_vote);
            assert_eq!((score_delta, minus_delta), (-1, 0));

            vote = VoteValue::Down;
            previous_vote = VoteValue::None;
            (score_delta, minus_delta) = get_vote_deltas(vote, previous_vote);
            assert_eq!((score_delta, minus_delta), (-1, 1));

            vote = VoteValue::None;
            previous_vote = VoteValue::Down;
            (score_delta, minus_delta) = get_vote_deltas(vote, previous_vote);
            assert_eq!((score_delta, minus_delta), (1, -1));

            vote = VoteValue::Up;
            previous_vote = VoteValue::Down;
            (score_delta, minus_delta) = get_vote_deltas(vote, previous_vote);
            assert_eq!((score_delta, minus_delta), (2, -1));

            vote = VoteValue::Down;
            previous_vote = VoteValue::Up;
            (score_delta, minus_delta) = get_vote_deltas(vote, previous_vote);
            assert_eq!((score_delta, minus_delta), (-2, 1));
        }
    }
}

pub fn update_vote_value(vote: &mut VoteValue, is_upvote: bool) {
    *vote = match *vote {
        VoteValue::Up => {
            if is_upvote {
                VoteValue::None
            } else {
                VoteValue::Down
            }
        }
        VoteValue::None => {
            if is_upvote {
                VoteValue::Up
            } else {
                VoteValue::Down
            }
        }
        VoteValue::Down => {
            if is_upvote {
                VoteValue::Up
            } else {
                VoteValue::None
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::ranking::{update_vote_value, CommentSortType, PostSortType, SortType, VoteValue};
    use sphare_core_common::constants::{BEST_ORDER_BY_COLUMN, HOT_ORDER_BY_COLUMN, RECENT_ORDER_BY_COLUMN, TRENDING_ORDER_BY_COLUMN};

    #[test]
    fn test_post_sort_type_to_order_by_code() {
        assert_eq!(PostSortType::Hot.to_order_by_code(), HOT_ORDER_BY_COLUMN);
        assert_eq!(PostSortType::Trending.to_order_by_code(), TRENDING_ORDER_BY_COLUMN);
        assert_eq!(PostSortType::Best.to_order_by_code(), BEST_ORDER_BY_COLUMN);
        assert_eq!(PostSortType::Recent.to_order_by_code(), RECENT_ORDER_BY_COLUMN);
    }

    #[test]
    fn test_comment_sort_type_to_order_by_code() {
        assert_eq!(CommentSortType::Best.to_order_by_code(), BEST_ORDER_BY_COLUMN);
        assert_eq!(CommentSortType::Recent.to_order_by_code(), RECENT_ORDER_BY_COLUMN);
    }

    #[test]
    fn test_sort_type_to_order_by_code() {
        assert_eq!(SortType::Post(PostSortType::Hot).to_order_by_code(), HOT_ORDER_BY_COLUMN);
        assert_eq!(SortType::Post(PostSortType::Trending).to_order_by_code(), TRENDING_ORDER_BY_COLUMN);
        assert_eq!(SortType::Post(PostSortType::Best).to_order_by_code(), BEST_ORDER_BY_COLUMN);
        assert_eq!(SortType::Post(PostSortType::Recent).to_order_by_code(), RECENT_ORDER_BY_COLUMN);
        assert_eq!(SortType::Comment(CommentSortType::Best).to_order_by_code(), BEST_ORDER_BY_COLUMN);
        assert_eq!(SortType::Comment(CommentSortType::Recent).to_order_by_code(), RECENT_ORDER_BY_COLUMN);
    }

    #[test]
    fn test_vote_value_from_i64() {
        assert_eq!(VoteValue::from(1), VoteValue::Up);
        assert_eq!(VoteValue::from(123), VoteValue::Up);
        assert_eq!(VoteValue::from(0), VoteValue::None);
        assert_eq!(VoteValue::from(-1), VoteValue::Down);
        assert_eq!(VoteValue::from(-312), VoteValue::Down);
    }

    #[test]
    fn test_update_vote_value() {
        let mut vote = VoteValue::None;
        update_vote_value(&mut vote, true);
        assert_eq!(vote, VoteValue::Up);
        update_vote_value(&mut vote, true);
        assert_eq!(vote, VoteValue::None);
        update_vote_value(&mut vote, false);
        assert_eq!(vote, VoteValue::Down);
        update_vote_value(&mut vote, true);
        assert_eq!(vote, VoteValue::Up);
        update_vote_value(&mut vote, false);
        assert_eq!(vote, VoteValue::Down);
        update_vote_value(&mut vote, false);
        assert_eq!(vote, VoteValue::None);
    }
}