use const_format::formatcp;
use leptos::html;
use leptos::prelude::*;
use leptos_fluent::move_tr;

use sphare_core_common::errors::AppError;
use sphare_core_common::routes::{get_post_path, COMMENT_ID_QUERY_PARAM};
use sphare_core_content::comment::{Comment, CommentWithContext};

use sphare_cmp_common::auth_widget::AuthorWidget;
use sphare_cmp_common::sphere::SphereHeader;
use sphare_cmp_utils::node_utils::has_reached_scroll_load_threshold;
use sphare_cmp_utils::widget::{ContentBody, IsPinnedWidget, LoadIndicators, ScoreIndicator, TimeSinceWidget};

use crate::moderation::ModeratedBody;

pub const COMMENT_MAX_DEPTH_MOBILE: usize = 5;
pub const COMMENT_MAX_DEPTH_SMALL_SCREEN: usize = 10;
pub const COMMENT_MAX_DEPTH: usize = 15;

/// Displays the body of a comment
#[component]
pub fn CommentBody(
    #[prop(into)]
    comment: Signal<Comment>,
    #[prop(optional)]
    depth: usize,
) -> impl IntoView {
    const BASE_CLASS: &str = "pl-2 text-left text-sm";
    let class = match depth {
        d if d <= COMMENT_MAX_DEPTH_MOBILE => formatcp!("{BASE_CLASS} lg:w-19/20 xl:w-9/10 2xl:w-17/20 3xl:w-4/5 4xl:w-3/4 5xl:w-7/10"),
        d if d <= (COMMENT_MAX_DEPTH_MOBILE + COMMENT_MAX_DEPTH_SMALL_SCREEN)/2 => formatcp!("{BASE_CLASS} xl:w-19/20 2xl:w-9/10 3xl:w-17/20 4xl:w-4/5 5xl:w-3/4"),
        d if d <= COMMENT_MAX_DEPTH_SMALL_SCREEN => formatcp!("{BASE_CLASS} 2xl:w-19/20 3xl:w-9/10 4xl:w-17/20 5xl:w-4/5"),
        d if d <= (COMMENT_MAX_DEPTH_SMALL_SCREEN + COMMENT_MAX_DEPTH)/2 => formatcp!("{BASE_CLASS} 3xl:w-19/20 4xl:w-9/10 5xl:w-17/20"),
        _ => formatcp!("{BASE_CLASS} 4xl:w-19/20 5xl:w-9/10"),
    };

    view! {
        {
            move || comment.with(|comment| match (
                &comment.delete_timestamp,
                &comment.moderator_message,
                &comment.infringed_rule_title
            ) {
                (Some(_), _, _) => view! {
                    <div>
                        <ContentBody
                            body=move_tr!("deleted")
                            is_markdown=false
                        />
                    </div>
                }.into_any(),
                (None, Some(moderator_message), Some(infringed_rule_title)) => view! {
                    <ModeratedBody
                        infringed_rule_title=infringed_rule_title.clone()
                        moderator_message=moderator_message.clone()
                        is_sphere_rule=comment.is_sphere_rule
                    />
                }.into_any(),
                _ => view! {
                    <div class=class>
                        <ContentBody
                            body=comment.body.clone()
                            is_markdown=comment.markdown_body.is_some()
                        />
                    </div>
                }.into_any(),
            })
        }
    }.into_any()
}

/// Displays a comment with context (post title, sphere, score, etc.)
#[component]
pub fn CommentWithContext(
    comment: CommentWithContext
) -> impl IntoView {
    let comment_id = comment.comment.comment_id;
    let score = comment.comment.score;
    let author_id = comment.comment.creator_id;
    let author = comment.comment.creator_name.clone();
    let is_moderator = comment.comment.is_creator_moderator;
    let timestamp = comment.comment.create_timestamp;
    let is_pinned = comment.comment.is_pinned;

    let post_path = get_post_path(&comment.sphere_header.sphere_name, comment.satellite_id, comment.comment.post_id);
    view! {
        <a
            href=format!("{post_path}?{COMMENT_ID_QUERY_PARAM}={}", comment_id)
            class="w-full flex flex-col gap-1 pl-1 pt-1 pb-2 my-1 rounded-sm hover:bg-base-200"
        >
            <CommentBody comment=comment.comment/>
            <div class="flex gap-1 items-center">
                <div class="text-sm">{comment.post_title}</div>
                <IsPinnedWidget is_pinned/>
            </div>
            <div class="flex gap-1">
                <SphereHeader sphere_header=comment.sphere_header/>
                <ScoreIndicator score/>
                <AuthorWidget author_id author is_moderator/>
                <TimeSinceWidget timestamp/>
            </div>
        </a>
    }
}

/// Component to display a vector of comments and indicate when more need to be loaded
#[component]
pub fn CommentMiniatureList(
    /// signal containing the comments to display
    #[prop(into)]
    comment_vec: Signal<Vec<CommentWithContext>>,
    /// signal indicating new comments are being loaded
    #[prop(into)]
    is_loading: Signal<bool>,
    /// signal containing an eventual loading error in order to display it
    #[prop(into)]
    load_error: Signal<Option<AppError>>,
    /// signal to request loading additional comments
    additional_load_count: RwSignal<i32>,
    /// reference to the container of the comments in order to reset scroll position when context changes
    list_ref: NodeRef<html::Ul>,
) -> impl IntoView {
    view! {
        <ul class="flex flex-col overflow-y-auto w-full pr-2 divide-y divide-base-content/20"
            on:scroll=move |_| if has_reached_scroll_load_threshold(list_ref) && !is_loading.get_untracked() {
                additional_load_count.update(|value| *value += 1);
            }
            node_ref=list_ref
        >
            <For
                each= move || comment_vec.get().into_iter()
                key=|comment| comment.comment.comment_id
                let(comment)
            >
                <li>
                    <CommentWithContext comment/>
                </li>
            </For>
        </ul>
        <LoadIndicators load_error is_loading/>
    }
}