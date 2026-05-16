use leptos::either::Either;
use leptos::html;
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::hooks::use_query_map;
use leptos_use::BreakpointsTailwind::{Lg, Xxl};
use leptos_use::{breakpoints_tailwind, signal_throttled_with_options, use_breakpoints, ThrottleOptions};

use sphare_core_common::colors::Color;
use sphare_core_common::constants::{MAX_CONTENT_LENGTH, SCROLL_LOAD_THROTTLE_DELAY};
use sphare_core_common::editor::TextareaData;
use sphare_core_common::routes::{get_comment_link, COMMENT_ID_QUERY_PARAM};
use sphare_core_common::unpack::{handle_additional_load, handle_dialog_action_result, handle_initial_load};
use sphare_core_content::comment::{Comment, CommentWithChildren};
use sphare_core_content::moderation::Content;
use sphare_core_content::ranking::Vote;

use sphare_iface_content::comment::{get_comment_tree_by_id, get_post_comment_tree, CreateComment, DeleteComment, EditComment};

use sphare_cmp_base::comment::{CommentBody, COMMENT_MAX_DEPTH, COMMENT_MAX_DEPTH_MOBILE, COMMENT_MAX_DEPTH_SMALL_SCREEN};
use sphare_cmp_base::ranking::CommentSortWidget;
use sphare_cmp_common::auth_widget::{AuthorWidget, DeleteButton, LoginGuardedOpenModalButton};
use sphare_cmp_common::role::IsPinnedCheckbox;
use sphare_cmp_common::state::{GlobalState, SatelliteState, SphereState};
use sphare_cmp_utils::colors::ColorIndicator;
use sphare_cmp_utils::editor::FormMarkdownEditor;
use sphare_cmp_utils::errors::ErrorDisplay;
use sphare_cmp_utils::icons::{AddCommentIcon, EditIcon, LoadingIcon};
use sphare_cmp_utils::unpack::{ActionError, SuspenseUnpack};
use sphare_cmp_utils::widget::{Badge, DotMenu, IsPinnedWidget, LoadIndicators, MinimizeMaximizeWidget, ModalDialog, ModalFormButtons, ModeratorWidget, ScoreIndicator, ShareButton, TimeSinceEditWidget, TimeSinceWidget};

use crate::moderation::{ModerateCommentButton, ModerationInfoButton};
use crate::ranking::VotePanel;

const DEPTH_TO_COLOR_MAPPING_SIZE: usize = 6;
const DEPTH_TO_COLOR_MAPPING: [&str; DEPTH_TO_COLOR_MAPPING_SIZE] = [
    "bg-blue-500",
    "bg-green-500",
    "bg-yellow-500",
    "bg-orange-500",
    "bg-red-500",
    "bg-violet-500",
];

/// Comment section component
#[component]
pub fn CommentSection(
    #[prop(into)]
    post_id: Signal<i64>,
    comment_vec: RwSignal<Vec<CommentWithChildren>>,
    is_loading: RwSignal<bool>,
    additional_load_count: RwSignal<i32>,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let query_comment_id = move || match use_query_map().read().get(COMMENT_ID_QUERY_PARAM) {
        Some(comment_id_string) => comment_id_string.parse::<i64>().ok(),
        None => None,
    };

    view! {
        <CommentSortWidget sort_signal=state.comment_sort_type/>
        { move || {
            match query_comment_id() {
                Some(comment_id) => view! { <CommentTree comment_id comment_vec is_loading/> }.into_any(),
                None => view! { <CommentTreeVec post_id comment_vec is_loading additional_load_count/> }.into_any(),
            }
        }}
    }.into_any()
}

/// Component displaying a vector of comment trees
#[component]
pub fn CommentTreeVec(
    #[prop(into)]
    post_id: Signal<i64>,
    comment_vec: RwSignal<Vec<CommentWithChildren>>,
    is_loading: RwSignal<bool>,
    additional_load_count: RwSignal<i32>,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let load_error = RwSignal::new(None);
    let is_small_screen = use_breakpoints(breakpoints_tailwind()).lt(Xxl);
    let is_mobile = use_breakpoints(breakpoints_tailwind()).lt(Lg);

    let _initial_comments_resource = LocalResource::new(
        move || async move {
            is_loading.set(true);
            let initial_load = get_post_comment_tree(
                post_id.get(),
                state.comment_sort_type.get(),
                Some(get_max_comment_depth(is_mobile.get_untracked(), is_small_screen.get_untracked())),
                0
            ).await;
            handle_initial_load(initial_load, comment_vec, load_error, None);
            is_loading.set(false);
        }
    );

    let additional_load_count_throttled: Signal<i32> = signal_throttled_with_options(
        additional_load_count,
        SCROLL_LOAD_THROTTLE_DELAY,
        ThrottleOptions::default().leading(true).trailing(false)
    );

    let _additional_comments_resource = LocalResource::new(
        move || async move {
            if additional_load_count_throttled.get() > 0 {
                is_loading.set(true);
                let num_post = comment_vec.read_untracked().len();
                let additional_load = get_post_comment_tree(
                    post_id.get(),
                    state.comment_sort_type.get_untracked(),
                    Some(get_max_comment_depth(is_mobile.get_untracked(), is_small_screen.get_untracked())),
                    num_post
                ).await;
                handle_additional_load(additional_load, comment_vec, load_error);
                is_loading.set(false);
            }
        }
    );

    view! {
        <div class="flex flex-col h-fit">
            <For
                // a function that returns the items we're iterating over; a signal is fine
                each= move || comment_vec.get().into_iter().enumerate()
                // a unique key for each item as a reference
                key=|(_index, comment)| comment.comment.comment_id
                // renders each item to a view
                children=move |(index, comment_with_children)| {
                    view! {
                        <CommentBox
                            comment_with_children
                            depth=0
                            ranking=index
                        />
                    }.into_any()
                }
            />
        </div>
        <Show when=move || load_error.read().is_some()>
        {
            let error = load_error.get().unwrap();
            view! {
                <div class="flex justify-start py-4"><ErrorDisplay error/></div>
            }
        }
        </Show>
        <LoadIndicators is_loading load_error/>
    }.into_any()
}

/// Component displaying a comment's tree
#[component]
pub fn CommentTree(
    #[prop(into)]
    comment_id: i64,
    comment_vec: RwSignal<Vec<CommentWithChildren>>,
    is_loading: RwSignal<bool>,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let load_error = RwSignal::new(None);
    let is_small_screen = use_breakpoints(breakpoints_tailwind()).lt(Xxl);
    let is_mobile = use_breakpoints(breakpoints_tailwind()).lt(Lg);

    // we set a signal with a local resource instead of using the resource directly to reuse the components from the ordinary comment tree
    let _comment_resource = LocalResource::new(
        move || async move {
            is_loading.set(true);
            let comment_tree = get_comment_tree_by_id(
                comment_id,
                state.comment_sort_type.get(),
                Some(get_max_comment_depth(is_mobile.get_untracked(), is_small_screen.get_untracked())),
            ).await;
            handle_initial_load(comment_tree.map(|comment| vec![comment]), comment_vec, load_error, None);
            is_loading.set(false);
        }
    );

    view! {
        <a href="?" class="button-secondary mt-1 w-fit">
            {move_tr!("single-comment-tree")}
        </a>
        { move || comment_vec.read().first().map(|comment| view! {
                <div class="flex flex-col h-fit">
                    <CommentBox
                        comment_with_children=comment.clone()
                        depth=0
                        ranking=0
                    />
                </div>
            })
        }
        <Show when=is_loading>
            <LoadingIcon/>
        </Show>
    }.into_any()
}

/// Comment box component
#[component]
pub fn CommentBox(
    comment_with_children: CommentWithChildren,
    depth: usize,
    ranking: usize,
) -> impl IntoView {
    let comment = RwSignal::new(comment_with_children.comment);
    let child_comments = RwSignal::new(comment_with_children.child_comments);
    let maximize = RwSignal::new(true);
    let sidebar_css = move || {
        if *maximize.read() {
            "p-0.5 rounded-sm hover:bg-base-200 flex flex-col justify-start items-center gap-1"
        } else {
            "p-0.5 rounded-sm hover:bg-base-200 flex flex-col justify-center items-center"
        }
    };
    let color_bar_css = format!(
        "{} rounded-full h-full w-1 ",
        DEPTH_TO_COLOR_MAPPING[(depth + ranking) % DEPTH_TO_COLOR_MAPPING.len()]
    );

    let is_small_screen = use_breakpoints(breakpoints_tailwind()).lt(Xxl);
    let is_mobile = use_breakpoints(breakpoints_tailwind()).lt(Lg);
    let collapse_children = Memo::new(move |_| depth >= get_max_comment_depth(is_mobile.get(), is_small_screen.get()) && !child_comments.read().is_empty());

    view! {
        <div class="w-full flex lg:gap-1 pt-4">
            <div
                class=sidebar_css
                on:click=move |_| maximize.update(|value: &mut bool| *value = !*value)
            >
                <MinimizeMaximizeWidget is_maximized=maximize/>
                <Show when=maximize>
                    <div class=color_bar_css.clone()/>
                </Show>
            </div>
            <div class="grow flex flex-col gap-1 pl-1">
                <Show when=maximize>
                    <CommentTopWidgetBar comment/>
                    <CommentBody comment depth/>
                </Show>
                <CommentBottomWidgetBar
                    comment=comment
                    vote=comment_with_children.vote
                    child_comments
                />
                <div
                    class="w-full flex flex-col"
                    class:hidden=move || !*maximize.read()
                >
                { move || match collapse_children.get() {
                    true => {
                        Either::Left(view! {
                            <a
                                href=format!("?{COMMENT_ID_QUERY_PARAM}={}", comment.read_untracked().comment_id)
                                class="w-fit mx-auto button-neutral p-2"
                            >
                                {move_tr!("load-replies")}
                            </a>
                        })
                    },
                    false => {
                        Either::Right(view! {
                            <For
                                each= move || child_comments.get().into_iter().enumerate()
                                key=|(_index, comment)| comment.comment.comment_id
                                children=move |(index, comment_with_children)| {
                                    view! {
                                        <CommentBox
                                            comment_with_children
                                            depth=depth+1
                                            ranking=ranking+index
                                        />
                                    }.into_any()
                                }
                            />
                        })
                    },
                }}
                </div>
            </div>
        </div>
    }.into_any()
}

/// Component to encapsulate the widgets displayed at the top of each comment
#[component]
pub fn CommentTopWidgetBar(
    comment: RwSignal<Comment>,
) -> impl IntoView {
    let author_id = comment.read_untracked().creator_id;
    let author = comment.read_untracked().creator_name.clone();
    let timestamp = Signal::derive(move || comment.read().create_timestamp);
    let edit_timestamp = Signal::derive(move || comment.read().edit_timestamp);
    let moderator = Signal::derive(move || comment.read().moderator_name.clone());
    let is_active = Signal::derive(move || comment.read().is_active());
    let is_moderator_comment = comment.read_untracked().is_creator_moderator;
    let is_pinned = Signal::derive(move || comment.read().is_pinned);
    let is_query_comment = move || match use_query_map().read().get(COMMENT_ID_QUERY_PARAM) {
        Some(query_comment_id) => query_comment_id.parse::<i64>().is_ok_and(|query_comment_id| query_comment_id == comment.read().comment_id),
        None => false,
    };
    view! {
        <div class="flex gap-1 items-center">
            {
                move || is_active.get().then_some(view! {
                    <AuthorWidget
                        author_id
                        author=author.clone()
                        is_moderator=is_moderator_comment
                    />
                })
            }
            <ModeratorWidget moderator/>
            <IsPinnedWidget is_pinned/>
            <TimeSinceWidget timestamp/>
            <TimeSinceEditWidget edit_timestamp/>
            <Show when=is_query_comment>
                <ColorIndicator color=Color::Red class="w-3 h-3 rounded-full"/>
            </Show>
        </div>
    }.into_any()
}

/// Component to encapsulate the widgets displayed at the bottom of each comment
#[component]
pub fn CommentBottomWidgetBar(
    comment: RwSignal<Comment>,
    vote: Option<Vote>,
    child_comments: RwSignal<Vec<CommentWithChildren>>,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let sphere_state = expect_context::<SphereState>();
    let satellite_state = use_context::<SatelliteState>();
    let vote = vote;
    let (comment_id, post_id, score, author_id) =
        comment.with_untracked(|comment| {
            (
                comment.comment_id,
                comment.post_id,
                comment.score,
                comment.creator_id,
            )
        });
    let comment_link = get_comment_link(
        &*sphere_state.sphere_name.read_untracked(),
        satellite_state.map(|state| state.satellite_id.get_untracked()),
        post_id,
        comment_id,
    );
    let content = Signal::derive(move || Content::Comment(comment.get()));
    let is_active = Signal::derive(move || comment.read().is_active());
    view! {
        <div class="flex gap-1 items-center">
            { move || match is_active.get() {
                true => Either::Left(view! {
                    <VotePanel
                        post_id
                        comment_id=Some(comment_id)
                        score
                        vote=vote.clone()
                    />
                }),
                false => Either::Right(view! {
                    <ScoreIndicator score/>
                }),
            }}
            <CommentButton
                post_id
                comment_vec=child_comments
                parent_comment_id=Some(comment_id)
            />
            <DotMenu>
                { move || is_active.get().then_some(view!{
                    <EditCommentButton
                        comment_id
                        author_id
                        comment
                    />
                    <SuspenseUnpack resource=state.user let:user>
                    {
                        match user.as_ref().is_some_and(|user| user.user_id == author_id) {
                            true => None,
                            false => Some(view! {
                                <ModerateCommentButton
                                    comment_id
                                    comment
                                />
                            })
                        }
                    }
                    </SuspenseUnpack>
                    <DeleteCommentButton comment_id author_id comment/>
                })}
                <ModerationInfoButton content/>
                {
                    match comment_link.clone() {
                        Ok(comment_link) => Either::Left(view! { <ShareButton link=comment_link/> }),
                        Err(e) => {
                            log::error!("Error while generating comment url: {e}");
                            Either::Right(())
                        },
                    }
                }
            </DotMenu>
        </div>
    }.into_any()
}

/// Component to open the comment form
#[component]
pub fn CommentButton(
    post_id: i64,
    comment_vec: RwSignal<Vec<CommentWithChildren>>,
    #[prop(default = None)]
    parent_comment_id: Option<i64>,
) -> impl IntoView {
    let show_dialog = RwSignal::new(false);
    let comment_button_class = Signal::derive(move || match show_dialog.get() {
        true => "button-rounded-primary py-1",
        false => "button-rounded-neutral py-1",
    });

    view! {
        <div class="flex items-center">
            <LoginGuardedOpenModalButton
                show_dialog
                button_class=comment_button_class
            >
                <AddCommentIcon/>
            </LoginGuardedOpenModalButton>
            <CommentDialog
                post_id
                parent_comment_id
                comment_vec
                show_dialog
            />
        </div>
    }.into_any()
}

/// Component to open the comment form and indicate comment count
#[component]
pub fn CommentButtonWithCount(
    post_id: i64,
    comment_vec: RwSignal<Vec<CommentWithChildren>>,
    count: i32,
    #[prop(default = None)]
    parent_comment_id: Option<i64>,
) -> impl IntoView {
    let show_dialog = RwSignal::new(false);
    let comment_button_class = Signal::derive(move || match show_dialog.get() {
        true => "button-rounded-primary p-1 px-2",
        false => "button-rounded-neutral p-1 px-2",
    });

    view! {
        <LoginGuardedOpenModalButton
            show_dialog
            button_class=comment_button_class
        >
            <Badge text=count.to_string()>
                <AddCommentIcon/>
            </Badge>
        </LoginGuardedOpenModalButton>
        <CommentDialog
            post_id
            parent_comment_id
            comment_vec
            show_dialog
        />
    }.into_any()
}

/// Dialog to publish a comment
#[component]
pub fn CommentDialog(
    post_id: i64,
    parent_comment_id: Option<i64>,
    comment_vec: RwSignal<Vec<CommentWithChildren>>,
    show_dialog: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <ModalDialog
            class="w-full flex justify-center"
            show_dialog
        >
            <CommentForm
                post_id
                parent_comment_id
                comment_vec
                show_form=show_dialog
            />
        </ModalDialog>
    }
}

/// Form to publish a comment
#[component]
pub fn CommentForm(
    post_id: i64,
    parent_comment_id: Option<i64>,
    comment_vec: RwSignal<Vec<CommentWithChildren>>,
    show_form: RwSignal<bool>,
) -> impl IntoView {
    let sphere_name = expect_context::<SphereState>().sphere_name;
    let textarea_ref = NodeRef::<html::Textarea>::new();
    let comment_data = TextareaData {
        content: RwSignal::new(String::new()),
        textarea_ref,
    };

    let is_comment_empty = Signal::derive(move || comment_data.content.read().is_empty());

    let create_comment_action = ServerAction::<CreateComment>::new();

    Effect::new(move |_| {
        if let Some(Ok(comment)) = create_comment_action.value().get() {
            comment_vec.update(|comment_vec| comment_vec.insert(0, comment));
            show_form.set(false);
        }
    });

    view! {
        <div class="bg-base-100 shadow-xl p-3 rounded-xs flex flex-col gap-3 w-full lg:w-3/5 2xl:w-2/5">
            <div class="text-center font-bold text-2xl">{move_tr!("share-comment")}</div>
            <ActionForm action=create_comment_action>
                <div class="flex flex-col gap-3 w-full">
                    <input
                        type="text"
                        name="post_id"
                        class="hidden"
                        value=post_id
                    />
                    <input
                        type="text"
                        name="parent_comment_id"
                        class="hidden"
                        value=parent_comment_id
                    />
                    <FormMarkdownEditor
                        name="comment"
                        is_markdown_name="is_markdown"
                        placeholder=move_tr!("your-comment")
                        data=comment_data
                        maxlength=Some(MAX_CONTENT_LENGTH as usize)
                    />
                    <IsPinnedCheckbox sphere_name=sphere_name/>
                    <ModalFormButtons
                        disable_publish=is_comment_empty
                        show_form
                    />
                </div>
            </ActionForm>
            <ActionError action=create_comment_action.into()/>
        </div>
    }
}

/// Component to open the edit comment form
#[component]
pub fn EditCommentButton(
    comment_id: i64,
    author_id: i64,
    comment: RwSignal<Comment>,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let show_dialog = RwSignal::new(false);
    let show_button = move || match &(*state.user.read()) {
        Some(Ok(Some(user))) => user.user_id == author_id,
        _ => false,
    };
    let comment_button_class = move || match show_dialog.get() {
        true => "button-rounded-primary",
        false => "button-rounded-neutral",
    };

    view! {
        <Suspense>
            <Show when=show_button>
                <div>
                    <button
                        class=comment_button_class
                        aria-expanded=move || show_dialog.get().to_string()
                        aria-haspopup="dialog"
                        on:click=move |_| show_dialog.update(|show: &mut bool| *show = !*show)
                    >
                        <EditIcon/>
                    </button>
                    <EditCommentDialog
                        comment_id
                        comment
                        show_dialog
                    />
                </div>
            </Show>
        </Suspense>
    }
}

/// Component to delete a comment
#[component]
pub fn DeleteCommentButton(
    comment_id: i64,
    author_id: i64,
    comment: RwSignal<Comment>,
) -> impl IntoView {
    let delete_comment_action = ServerAction::<DeleteComment>::new();

    Effect::new(move |_| {
        if let Some(Ok(_)) = delete_comment_action.value().get() {
            comment.update(|comment| {
                comment.delete_timestamp = Some(chrono::Utc::now());
            });
        }
    });

    view! {
        <DeleteButton
            title=move_tr!("delete-comment")
            id=comment_id
            id_name="comment_id"
            author_id
            delete_action=delete_comment_action
        />
    }
}

/// Dialog to edit a comment
#[component]
pub fn EditCommentDialog(
    comment_id: i64,
    comment: RwSignal<Comment>,
    show_dialog: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <ModalDialog
            class="w-full flex justify-center"
            show_dialog
        >
            <EditCommentForm
                comment_id
                comment
                show_form=show_dialog
            />
        </ModalDialog>
    }
}

/// Form to edit a comment
#[component]
pub fn EditCommentForm(
    comment_id: i64,
    comment: RwSignal<Comment>,
    show_form: RwSignal<bool>,
) -> impl IntoView {
    let sphere_name = expect_context::<SphereState>().sphere_name;
    let (current_body, is_markdown) =
        comment.with_untracked(|comment| match &comment.markdown_body {
            Some(body) => (body.clone(), true),
            None => (comment.body.clone(), false),
        });
    let textarea_ref = NodeRef::<html::Textarea>::new();
    let comment_data = TextareaData {
        content: RwSignal::new(current_body),
        textarea_ref,
    };
    let is_comment_empty = Signal::derive(
        move || comment_data.content.read().is_empty()
    );
    let edit_comment_action = ServerAction::<EditComment>::new();

    let edit_comment_result = edit_comment_action.value();

    Effect::new(move |_| handle_dialog_action_result(edit_comment_result.get(), comment, show_form));

    view! {
        <div class="bg-base-100 shadow-xl p-3 rounded-xs flex flex-col gap-3 w-full lg:w-3/5 2xl:w-2/5">
            <div class="text-center font-bold text-2xl">{move_tr!("edit-comment")}</div>
            <ActionForm action=edit_comment_action>
                <div class="flex flex-col gap-3 w-full">
                    <input
                        type="text"
                        name="comment_id"
                        class="hidden"
                        value=comment_id
                    />
                    <FormMarkdownEditor
                        name="comment"
                        is_markdown_name="is_markdown"
                        placeholder=move_tr!("your-comment")
                        data=comment_data
                        is_markdown
                        maxlength=Some(MAX_CONTENT_LENGTH as usize)
                    />
                    <IsPinnedCheckbox sphere_name value=comment.read_untracked().is_pinned/>
                    <ModalFormButtons
                        disable_publish=is_comment_empty
                        show_form
                    />
                </div>
            </ActionForm>
            <ActionError action=edit_comment_action.into()/>
        </div>
    }
}

/// Returns the depth for nested comments depending on the platform
fn get_max_comment_depth(
    is_mobile: bool,
    is_small_screen: bool,
) -> usize {
    match (is_mobile, is_small_screen) {
        (true, _) => COMMENT_MAX_DEPTH_MOBILE,
        (false, true) => COMMENT_MAX_DEPTH_SMALL_SCREEN,
        (false, false) => COMMENT_MAX_DEPTH,
    }
}

#[cfg(test)]
mod tests {
    use sphare_cmp_base::comment::{COMMENT_MAX_DEPTH, COMMENT_MAX_DEPTH_MOBILE, COMMENT_MAX_DEPTH_SMALL_SCREEN};
    use crate::comment::{get_max_comment_depth};
    #[test]
    fn test_permission_level_from_string() {
        assert_eq!(get_max_comment_depth(false, false), COMMENT_MAX_DEPTH);
        assert_eq!(get_max_comment_depth(false, true), COMMENT_MAX_DEPTH_SMALL_SCREEN);
        assert_eq!(get_max_comment_depth(true, true), COMMENT_MAX_DEPTH_MOBILE);
        assert_eq!(get_max_comment_depth(true, false), COMMENT_MAX_DEPTH_MOBILE);
    }
}