use leptos::either::Either;
use leptos::html;
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::hooks::{use_params_map, use_query_map};
use leptos_use::signal_debounced;
use url::Url;

use sphare_core_common::checks::{check_sphere_name, check_sphere_name_with_options};
use sphare_core_common::constants::{COMMENT_BATCH_SIZE, MAX_CONTENT_LENGTH};
use sphare_core_common::editor::{adjust_textarea_height, TextareaData};
use sphare_core_common::routes::{get_post_id_memo, get_post_link, CREATE_POST_SPHERE_QUERY_PARAM};
use sphare_core_content::comment::CommentWithChildren;
use sphare_core_content::embed::{EmbedType, LinkType};
use sphare_core_content::moderation::Content;
use sphare_core_content::post::{Post, PostWithInfo};

use sphare_iface_content::post::{get_post_inherited_attributes, get_post_with_info_by_id, CreatePost};
use sphare_iface_content::search::get_matching_sphere_header_vec;
use sphare_iface_sphere::sphere_category::get_sphere_category_vec;

use sphare_cmp_base::embed::Embed;
use sphare_cmp_base::moderation::ModeratedBody;
use sphare_cmp_base::post::{PostBadgeList, PostForm};
use sphare_cmp_common::auth_widget::{AuthorWidget, DeleteButton};
use sphare_cmp_common::sphere::SphereHeader;
use sphare_cmp_common::state::{GlobalState, SphereState};
use sphare_cmp_utils::icons::EditIcon;
use sphare_cmp_utils::node_utils::has_reached_scroll_load_threshold;
use sphare_cmp_utils::unpack::{ActionError, SuspenseUnpack, TransitionUnpack};
use sphare_cmp_utils::widget::{ContentBody, DotMenu, ModalDialog, ModalFormButtons, ModeratorWidget, ScoreIndicator, ShareButton, TimeSinceEditWidget, TimeSinceWidget};

use crate::comment::{CommentButtonWithCount, CommentSection};
use crate::moderation::{ModeratePostButton, ModerationInfoButton};
use crate::ranking::VotePanel;

/// Component to display a post
#[component]
pub fn Post() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let params = use_params_map();
    let post_id = get_post_id_memo(params);

    let post_resource = Resource::new(
        move || (
            post_id.get(),
            state.edit_post_action.version().get(),
            state.delete_post_action.version().get(),
            state.moderate_post_action.version().get()
        ),
        move |(post_id, _, _, _)| {
            log::debug!("Load data for post: {post_id}");
            get_post_with_info_by_id(post_id)
        },
    );

    let comment_vec = RwSignal::new(Vec::<CommentWithChildren>::with_capacity(
        COMMENT_BATCH_SIZE as usize,
    ));
    let is_loading = RwSignal::new(false);
    let additional_load_count = RwSignal::new(0);
    let container_ref = NodeRef::<html::Div>::new();

    view! {
        <div
            class="grow flex flex-col content-start gap-1 overflow-x-hidden overflow-y-auto px-0.5"
            on:scroll=move |_| if has_reached_scroll_load_threshold(container_ref) && !is_loading.get_untracked() {
                additional_load_count.update(|value| *value += 1);
            }
            node_ref=container_ref
        >
            <TransitionUnpack resource=post_resource let:post_with_info>
                <div class="card">
                    <div class="card-body">
                        <div class="flex flex-col gap-1 lg:gap-2">
                            <PostTopWidgetBar
                                creator_id=post_with_info.post.creator_id
                                creator_name=post_with_info.post.creator_name.clone()
                                moderator_name=post_with_info.post.moderator_name.clone()
                                is_creator_moderator=post_with_info.post.is_creator_moderator
                                create_timestamp=post_with_info.post.create_timestamp
                                edit_timestamp=post_with_info.post.edit_timestamp
                                is_active=post_with_info.post.is_active()
                            />
                            <h2 class="card-title text-wrap wrap-anywhere">
                            { match post_with_info.post.is_active() {
                                true => post_with_info.post.title.clone().into(),
                                false => move_tr!("deleted")
                            }}
                            </h2>
                            <PostBody
                                body=post_with_info.post.body.clone()
                                markdown_body=post_with_info.post.markdown_body.clone()
                                moderator_message=post_with_info.post.moderator_message.clone()
                                infringed_rule_title=post_with_info.post.infringed_rule_title.clone()
                                is_sphere_rule=post_with_info.post.is_sphere_rule
                                delete_timestamp=post_with_info.post.delete_timestamp
                            />
                            <Embed link=post_with_info.post.link.clone()/>
                            <PostBadgeList
                                sphere_header=None
                                sphere_category=post_with_info.sphere_category.clone()
                                is_spoiler=post_with_info.post.is_spoiler
                                is_nsfw=post_with_info.post.is_nsfw
                                is_pinned=post_with_info.post.is_pinned
                            />
                            <PostBottomWidgetBar post=post_with_info.clone() comment_vec/>
                        </div>
                    </div>
                </div>
            </TransitionUnpack>
            <CommentSection post_id comment_vec is_loading additional_load_count/>
        </div>
    }.into_any()
}

/// Displays the body of a post
#[component]
pub fn PostBody(
    body: String,
    markdown_body: Option<String>,
    moderator_message: Option<String>,
    infringed_rule_title: Option<String>,
    is_sphere_rule: bool,
    delete_timestamp: Option<chrono::DateTime<chrono::Utc>>,
) -> impl IntoView {

    view! {
        <div class="pb-2 lg:w-19/20 xl:w-9/10 2xl:w-17/20 3xl:w-4/5 4xl:w-3/4 5xl:w-7/10">
        {
            match (&delete_timestamp, &moderator_message, &infringed_rule_title) {
                (Some(_), _, _) => view! {
                    <ContentBody
                        body=move_tr!("deleted")
                        is_markdown=false
                    />
                }.into_any(),
                (None, Some(moderator_message), Some(infringed_rule_title)) => view! {
                    <ModeratedBody
                        infringed_rule_title=infringed_rule_title.clone()
                        moderator_message=moderator_message.clone()
                        is_sphere_rule
                    />
                }.into_any(),
                _ => view! {
                    <ContentBody
                        body=body
                        is_markdown=markdown_body.is_some()
                    />
                }.into_any(),
            }
        }
        </div>
    }.into_any()
}

/// Component to encapsulate the widgets displayed at the top of each post
#[component]
fn PostTopWidgetBar(
    creator_id: i64,
    creator_name: String,
    moderator_name: Option<String>,
    is_creator_moderator: bool,
    create_timestamp: chrono::DateTime<chrono::Utc>,
    edit_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    is_active: bool,
) -> impl IntoView {
    view! {
        <div class="flex gap-1">
            {
                is_active.then_some(view! {
                    <AuthorWidget
                        author_id=creator_id
                        author=creator_name
                        is_moderator=is_creator_moderator
                    />
                })
            }
            <ModeratorWidget moderator=moderator_name/>
            <TimeSinceWidget timestamp=create_timestamp/>
            <TimeSinceEditWidget edit_timestamp=edit_timestamp/>
        </div>
    }
}

/// Component to encapsulate the widgets displayed at the bottom of each comment
#[component]
fn PostBottomWidgetBar(
    post: PostWithInfo,
    comment_vec: RwSignal<Vec<CommentWithChildren>>,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let sphere_state = expect_context::<SphereState>();
    let post_id = post.post.post_id;
    let author_id = post.post.creator_id;
    let is_active = post.post.is_active();
    let post_link = get_post_link(&*sphere_state.sphere_name.get_untracked(), post.post.satellite_id, post.post.post_id);
    let stored_post = StoredValue::new(post.post.clone());
    view! {
        <div class="flex items-center gap-1">
            { match is_active {
                true => Either::Left(view! {
                    <VotePanel
                        post_id=post.post.post_id
                        comment_id=None
                        score=post.post.score
                        vote=post.vote.clone()
                    />
                }),
                false => Either::Right(view! {
                    <ScoreIndicator score=post.post.score/>
                }),
            }}
            <CommentButtonWithCount post_id comment_vec count=post.post.num_comments/>
            <DotMenu>
                { is_active.then_some(view! {
                    <EditPostButton author_id post=stored_post/>
                    <SuspenseUnpack resource=state.user let:user>
                    {
                        match user.as_ref().is_some_and(|user| user.user_id == author_id) {
                            true => None,
                            false => Some(view! {
                                <ModeratePostButton post_id/>
                            })
                        }
                    }
                    </SuspenseUnpack>
                    <DeletePostButton post_id author_id/>
                })}
                <ModerationInfoButton content=Content::Post(stored_post.get_value())/>
                {
                    match post_link.clone() {
                        Ok(post_link) => Either::Left(view! { <ShareButton link=post_link/> }),
                        Err(e) => {
                            log::error!("Error while generating post url: {e}");
                            Either::Right(())
                        },
                    }
                }
            </DotMenu>
        </div>
    }
}

/// Component to edit a post
#[component]
pub fn EditPostButton(
    post: StoredValue<Post>,
    author_id: i64
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let show_dialog = RwSignal::new(false);
    let show_button = move || match &(*state.user.read()) {
        Some(Ok(Some(user))) => user.user_id == author_id,
        _ => false,
    };
    let edit_button_class = move || match show_dialog.get() {
        true => "button-rounded-primary",
        false => "button-rounded-neutral",
    };
    view! {
        <Show when=show_button>
            <div>
                <button
                    class=edit_button_class
                    aria-expanded=move || show_dialog.get().to_string()
                    aria-haspopup="dialog"
                    on:click=move |_| show_dialog.update(|show: &mut bool| *show = !*show)
                >
                    <EditIcon/>
                </button>
                <EditPostDialog
                    post=post.get_value()
                    show_dialog
                />
            </div>
        </Show>
    }
}

/// Component to delete a post
#[component]
pub fn DeletePostButton(
    post_id: i64,
    author_id: i64,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    view! {
        <DeleteButton
            title=move_tr!("delete-post")
            id=post_id
            id_name="post_id"
            author_id
            delete_action=state.delete_post_action
        />
    }
}

/// Component to create a new post
#[component]
pub fn CreatePost() -> impl IntoView {
    let create_post_action = ServerAction::<CreatePost>::new();

    let query = use_query_map();
    let sphere_query = move || {
        query.read_untracked().get(CREATE_POST_SPHERE_QUERY_PARAM).unwrap_or_default()
    };

    let is_sphere_selected = RwSignal::new(false);
    let is_sphere_nsfw = RwSignal::new(false);
    let sphere_name_input = RwSignal::new(sphere_query());
    let sphere_name_debounced: Signal<String> = signal_debounced(sphere_name_input, 250.0);

    let title_input = RwSignal::new(String::default());
    let textarea_ref = NodeRef::<html::Textarea>::new();
    let body_data = TextareaData {
        content: RwSignal::new(String::new()),
        textarea_ref,
    };
    let link_input = RwSignal::new(String::default());
    let embed_type_input = RwSignal::new(EmbedType::None);

    let matching_spheres_resource = Resource::new(
        move || sphere_name_debounced.get(),
        move |sphere_prefix| async move {
            match check_sphere_name_with_options(&sphere_prefix, false) {
                Ok(()) => get_matching_sphere_header_vec(sphere_prefix).await,
                Err(_) => Ok(Vec::new()),
            }
        },
    );

    let category_vec_resource = Resource::new(
        move || sphere_name_debounced.get(),
        move |sphere_name| async move {
            match check_sphere_name(&sphere_name) {
                Ok(()) => get_sphere_category_vec(sphere_name).await,
                Err(_) => Ok(Vec::new())
            }
        }
    );

    // TODO: make sphere input into a component with a callback argument when clicking?

    view! {
        <div class="w-full xl:w-3/5 4xl:w-2/5 p-2 mx-auto flex flex-col gap-2 overflow-auto">
            <ActionForm action=create_post_action>
                <div class="flex flex-col gap-2 w-full">
                    <h2 class="py-4 text-4xl text-center">{move_tr!("share-post")}</h2>
                    <div
                        class="dropdown dropdown-end input_outline_primary"
                        class=("input_outline_error", move || !is_sphere_selected.get())
                    >
                        <input
                            tabindex="0"
                            type="text"
                            name="post_location[sphere]"
                            placeholder=move_tr!("sphere")
                            autocomplete="off"
                            class="w-full p-3 text-sm rounded-none"
                            on:input=move |ev| {
                                sphere_name_input.set(event_target_value(&ev).to_lowercase());
                            }
                            maxlength=MAX_CONTENT_LENGTH
                            prop:value=sphere_name_input
                        />
                        <TransitionUnpack resource=matching_spheres_resource let:sphere_header_vec>
                            <ul
                                tabindex="0"
                                class="dropdown-content z-1 menu p-2 mt-1 shadow-sm bg-base-200 rounded-xs w-full"
                                class=("hidden", sphere_header_vec.is_empty())
                            >
                            {
                                match sphere_header_vec.first() {
                                    Some(header) if header.sphere_name == sphere_name_input.get_untracked() => {
                                        is_sphere_nsfw.set(header.is_nsfw);
                                        is_sphere_selected.set(true);
                                    },
                                    _ => {
                                        is_sphere_selected.set(false);
                                        is_sphere_nsfw.set(false);
                                    }
                                };
                                sphere_header_vec.clone().into_iter().map(|sphere_header| {
                                    let sphere_name = sphere_header.sphere_name.clone();
                                    let is_nsfw = sphere_header.is_nsfw;
                                    view! {
                                        <li>
                                            <button
                                                type="button"
                                                on:click=move |_| {
                                                    is_sphere_nsfw.set(is_nsfw);
                                                    is_sphere_selected.set(true);
                                                    sphere_name_input.set(sphere_name.clone())
                                                }
                                            >
                                                <SphereHeader sphere_header/>
                                            </button>
                                        </li>
                                    }
                                }).collect_view()
                            }
                            </ul>
                        </TransitionUnpack>
                    </div>
                    <PostForm
                        title_input
                        body_data
                        embed_type_input
                        link_input
                        sphere_name=sphere_name_input
                        is_parent_spoiler=false
                        is_parent_nsfw=is_sphere_nsfw
                        category_vec_resource
                    />
                    <button type="submit" class="button-secondary" disabled=move || {
                        !is_sphere_selected.get() ||
                        title_input.read().is_empty() ||
                        (
                            body_data.content.read().is_empty() &&
                            *embed_type_input.read() == EmbedType::None
                        ) || (
                            *embed_type_input.read() != EmbedType::None &&
                            link_input.with(|link| link.is_empty() || Url::parse(link).is_err())
                        )
                    }>
                        {move_tr!("publish")}
                    </button>
                </div>
            </ActionForm>
            <ActionError action=create_post_action.into()/>
        </div>
    }
}

/// Dialog to edit a post
#[component]
pub fn EditPostDialog(
    post: Post,
    show_dialog: RwSignal<bool>,
) -> impl IntoView {
    let post = StoredValue::new(post);
    view! {
        <ModalDialog
            class="w-full flex justify-center"
            show_dialog
        >
            <EditPostForm
                post
                show_form=show_dialog
            />
        </ModalDialog>
    }
}

/// Form to edit a post
#[component]
pub fn EditPostForm(
    post: StoredValue<Post>,
    show_form: RwSignal<bool>,
) -> impl IntoView {
    let state = expect_context::<GlobalState>();
    let sphere_state = expect_context::<SphereState>();

    let (post_id, title, body, link_type, link_url) = post.with_value(|post| (
        post.post_id,
        post.title.clone(),
        match &post.markdown_body {
            Some(body) => body.clone(),
            None => post.body.clone(),
        },
        post.link.link_type,
        post.link.link_url.clone(),
    ));
    let title_input = RwSignal::new(title);
    let title_textarea_ref = NodeRef::<html::Textarea>::new();
    let body_textarea_ref = NodeRef::<html::Textarea>::new();
    let link_textarea_ref = NodeRef::<html::Textarea>::new();
    let body_data = TextareaData {
        content: RwSignal::new(body),
        textarea_ref: body_textarea_ref,
    };
    let embed_type_input = RwSignal::new(match link_type {
        LinkType::None => EmbedType::None,
        LinkType::Link => EmbedType::Link,
        _ => EmbedType::Embed,
    });
    let link_input = RwSignal::new(link_url.unwrap_or_default());
    let disable_publish = Signal::derive(move || {
        title_input.read().is_empty() ||
        (
            body_data.content.read().is_empty() &&
            embed_type_input.read() == EmbedType::None
        ) || (
            embed_type_input.read() != EmbedType::None &&
            link_input.read().is_empty()
        )
    });

    let inherited_attributes_resource = Resource::new(
        move || (),
        move |_| get_post_inherited_attributes(post_id)
    );

    // effect also needed here as the one in editor.rs somehow doesn't work inside a suspense
    Effect::new(move || adjust_textarea_height(title_textarea_ref));
    Effect::new(move || adjust_textarea_height(body_data.textarea_ref));
    Effect::new(move || adjust_textarea_height(link_textarea_ref));

    view! {
        <div class="bg-base-100 shadow-xl p-3 rounded-xs flex flex-col gap-3 w-full xl:w-3/5 4xl:w-2/5">
            <div class="text-center font-bold text-2xl">{move_tr!("edit-post")}</div>
            <ActionForm action=state.edit_post_action>
                <div class="flex flex-col gap-3 w-full">
                    <input
                        type="text"
                        name="post_id"
                        class="hidden"
                        value=post_id
                    />
                    <SuspenseUnpack resource=inherited_attributes_resource let:inherited_post_attr>
                        <PostForm
                            title_input
                            body_data
                            embed_type_input
                            link_input
                            sphere_name=sphere_state.sphere_name
                            is_parent_spoiler=inherited_post_attr.is_spoiler
                            is_parent_nsfw=inherited_post_attr.is_nsfw
                            category_vec_resource=sphere_state.sphere_categories_resource
                            current_post=Some(post)
                            title_textarea_ref
                            link_textarea_ref
                        />
                    </SuspenseUnpack>
                    <ModalFormButtons
                        disable_publish
                        show_form
                    />
                </div>
            </ActionForm>
            <ActionError action=state.edit_post_action.into()/>
        </div>
    }
}
