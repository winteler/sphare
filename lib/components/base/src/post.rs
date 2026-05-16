use leptos::html;
use leptos::prelude::*;
use leptos::server_fn::const_format::concatcp;
use leptos_fluent::move_tr;
use strum::IntoEnumIterator;

use sphare_core_common::common::{SphereCategoryHeader, SphereHeader};
use sphare_core_common::constants::{MAX_CONTENT_LENGTH, MAX_LINK_LENGTH, MAX_TITLE_LENGTH};
use sphare_core_common::editor::TextareaData;
use sphare_core_common::errors::AppError;
use sphare_core_common::routes::get_post_path;
use sphare_core_content::embed::EmbedType;
use sphare_core_content::post::{Post, PostWithSphereInfo};
use sphare_core_sphere::sphere_category::SphereCategory;

use sphare_cmp_common::auth_widget::AuthorWidget;
use sphare_cmp_common::role::IsPinnedCheckbox;
use sphare_cmp_common::sphere::SphereHeaderLink;
use sphare_cmp_utils::editor::{FormMarkdownEditor, LengthLimitedInput};
use sphare_cmp_utils::form::LabeledFormCheckbox;
use sphare_cmp_utils::icons::NsfwIcon;
use sphare_cmp_utils::node_utils::has_reached_scroll_load_threshold;
use sphare_cmp_utils::unpack::SuspenseUnpack;
use sphare_cmp_utils::widget::{CommentCountWidget, HelpButton, LoadIndicators, ScoreIndicator, SpoilerBadge, TagsWidget, TimeSinceWidget};

use crate::embed::EmbedPreview;
use crate::sphere_category::{SphereCategoryBadge, SphereCategoryDropdown};

/// Component to initially load on the server a vector of post and load additional post on the client upon scrolling
#[component]
pub fn PostListWithInitLoad(
    /// resource to load initial posts
    post_vec_resource: Resource<Result<Vec<PostWithSphereInfo>, AppError>>,
    /// signal containing additionally loaded posts when scrolling
    #[prop(into)]
    additional_post_vec: Signal<Vec<PostWithSphereInfo>>,
    /// signal indicating new posts are being loaded
    #[prop(into)]
    is_loading: Signal<bool>,
    /// signal containing an eventual loading error in order to display it
    #[prop(into)]
    load_error: Signal<Option<AppError>>,
    /// signal to request loading additional posts
    #[prop(optional)]
    additional_load_count: RwSignal<i32>,
    /// reference to the container of the posts in order to reset scroll position when context changes
    #[prop(optional)]
    list_ref: NodeRef<html::Ul>,
    #[prop(default = true)]
    show_sphere_header: bool,
    #[prop(default = true)]
    add_y_overflow_auto: bool,
) -> impl IntoView {
    const BASE_LIST_CLASS: &str = "flex flex-col w-full pr-2 divide-y divide-base-content/20 ";
    let list_class = match add_y_overflow_auto {
        true => concatcp!(BASE_LIST_CLASS, "overflow-y-auto"),
        false => BASE_LIST_CLASS,
    };
    view! {
        <ul class=list_class
            on:scroll=move |_| if has_reached_scroll_load_threshold(list_ref) && !is_loading.get_untracked() {
                additional_load_count.update(|value| *value += 1);
            }
            node_ref=list_ref
        >
            <SuspenseUnpack resource=post_vec_resource fallback= move || ().into_any() let:post_vec>
                <PostMiniatureList post_vec=post_vec.clone() show_sphere_header/>
            </SuspenseUnpack>
            <PostMiniatureList post_vec=additional_post_vec show_sphere_header/>
        </ul>
        <LoadIndicators load_error is_loading/>
    }
}

/// Component to display a vector of sphere posts and indicate when more need to be loaded
#[component]
pub fn PostListWithIndicators(
    /// signal containing the posts to display
    #[prop(into)]
    post_vec: Signal<Vec<PostWithSphereInfo>>,
    /// signal indicating new posts are being loaded
    #[prop(into)]
    is_loading: Signal<bool>,
    /// signal containing an eventual loading error in order to display it
    #[prop(into)]
    load_error: Signal<Option<AppError>>,
    /// signal to request loading additional posts
    additional_load_count: RwSignal<i32>,
    /// reference to the container of the posts in order to reset scroll position when context changes
    list_ref: NodeRef<html::Ul>,
    #[prop(default = true)]
    show_sphere_header: bool,
) -> impl IntoView {
    view! {
        <ul class="flex flex-col overflow-y-auto w-full pr-2 divide-y divide-base-content/20"
            on:scroll=move |_| if has_reached_scroll_load_threshold(list_ref) && !is_loading.get_untracked() {
                additional_load_count.update(|value| *value += 1);
            }
            node_ref=list_ref
        >
            <PostMiniatureList post_vec show_sphere_header/>
        </ul>
        <LoadIndicators load_error is_loading/>
    }
}

/// Component to display a vector of sphere posts and indicate when more need to be loaded
#[component]
pub fn PostMiniatureList(
    /// signal containing initial load of posts
    #[prop(into)]
    post_vec: Signal<Vec<PostWithSphereInfo>>,
    #[prop(default = true)]
    show_sphere_header: bool,
) -> impl IntoView {
    view! {
        <For
            each= move || post_vec.get().into_iter()
            key=|post| post.post.post_id
            children=move |post_info| {
                let post = post_info.post;
                let sphere_header = match show_sphere_header {
                    true => Some(SphereHeader::new(post_info.sphere_name.clone(), post_info.sphere_icon_url, false)),
                    false => None,
                };
                let post_path = get_post_path(&post_info.sphere_name, post.satellite_id, post.post_id);
                view! {
                    <li>
                        <a href=post_path>
                            <div class="flex flex-col gap-1 pl-1 pt-1 pb-2 my-1 rounded-sm hover:bg-base-200">
                                <h2 class="card-title pl-1 w-full whitespace-pre-wrap text-wrap wrap-anywhere">{post.title.clone()}</h2>
                                <PostBadgeList
                                    sphere_header
                                    sphere_category=post_info.sphere_category
                                    is_spoiler=post.is_spoiler
                                    is_nsfw=post.is_nsfw
                                    is_pinned=post.is_pinned
                                />
                                <div class="flex gap-1">
                                    <ScoreIndicator score=post.score/>
                                    <CommentCountWidget count=post.num_comments/>
                                    <AuthorWidget
                                        author_id=post.creator_id
                                        author=post.creator_name.clone()
                                        is_moderator=post.is_creator_moderator
                                    />
                                    <TimeSinceWidget timestamp=post.create_timestamp/>
                                </div>
                            </div>
                        </a>
                    </li>
                }
            }
        />
    }
}

/// Component to display a post's sphere, its category and whether it's a spoiler/NSFW
#[component]
pub fn PostBadgeList(
    sphere_header: Option<SphereHeader>,
    sphere_category: Option<SphereCategoryHeader>,
    is_spoiler: bool,
    is_nsfw: bool,
    is_pinned: bool,
) -> impl IntoView {
    match (sphere_header, sphere_category, is_spoiler, is_nsfw, is_pinned) {
        (None, None, false, false, false) => None,
        (sphere_header, sphere_category, is_spoiler, is_nsfw, is_pinned) => Some(view! {
            <div class="flex gap-2 items-center">
            {
                sphere_header.map(|sphere_header| view! { <SphereHeaderLink sphere_header/> })
            }
            {
                sphere_category.map(|category_header| view! { <SphereCategoryBadge category_header/> })
            }
            <TagsWidget is_spoiler is_nsfw is_pinned/>
            </div>
        })
    }
}

/// Component to create a new post
#[component]
pub fn PostForm(
    title_input: RwSignal<String>,
    body_data: TextareaData,
    embed_type_input: RwSignal<EmbedType>,
    link_input: RwSignal<String>,
    #[prop(into)]
    sphere_name: Signal<String>,
    #[prop(into)]
    is_parent_spoiler: Signal<bool>,
    #[prop(into)]
    is_parent_nsfw: Signal<bool>,
    category_vec_resource: Resource<Result<Vec<SphereCategory>, AppError>>,
    #[prop(default = None)]
    current_post: Option<StoredValue<Post>>,
    /// reference to the title textarea node
    #[prop(optional)]
    title_textarea_ref: NodeRef<html::Textarea>,
    /// reference to the link textarea node
    #[prop(optional)]
    link_textarea_ref: NodeRef<html::Textarea>,
) -> impl IntoView {
    let (is_markdown, is_spoiler, is_nsfw, is_pinned, category_id) = match current_post {
        Some(post) => post.with_value(|post| {
            (post.markdown_body.is_some(), post.is_spoiler, post.is_nsfw, post.is_pinned, post.category_id)
        }),
        None => (false, false, false, false, None),
    };

    view! {
        <LengthLimitedInput
            name="post_inputs[title]"
            placeholder=move_tr!("title")
            content=title_input
            autofocus=true
            minlength=Some(1)
            maxlength=Some(MAX_TITLE_LENGTH as usize)
            textarea_ref=title_textarea_ref
        />
        <FormMarkdownEditor
            name="post_inputs[body]"
            is_markdown_name="post_inputs[is_markdown]"
            placeholder=move_tr!("content")
            data=body_data
            is_markdown
            maxlength=Some(MAX_CONTENT_LENGTH as usize)
            is_empty_ok=Signal::derive(move || embed_type_input.read() != EmbedType::None)
        />
        <LinkForm link_input embed_type_input title_input textarea_ref=link_textarea_ref/>
        { move || {
            match is_parent_spoiler.get() {
                true => view! {
                    <LabeledFormCheckbox
                        name="post_inputs[post_tags][is_spoiler]"
                        label=move_tr!("spoiler")
                        label_icon_view=move || view! { <SpoilerBadge/> }
                        value=true
                        disabled=true
                    />
                },
                false => view! {
                    <LabeledFormCheckbox
                        name="post_inputs[post_tags][is_spoiler]"
                        label_icon_view=move || view! { <SpoilerBadge/> }
                        label=move_tr!("spoiler")
                        value=is_spoiler
                    />
                },
            }
        }}
        { move || {
            match is_parent_nsfw.get() {
                true => view! {
                    <LabeledFormCheckbox
                        name="post_inputs[post_tags][is_nsfw]"
                        label=move_tr!("nsfw-content")
                        label_icon_view=move || view! { <NsfwIcon/> }
                        value=true
                        disabled=true
                    />
                },
                false => view! {
                    <LabeledFormCheckbox
                        name="post_inputs[post_tags][is_nsfw]"
                        label=move_tr!("nsfw-content")
                        label_icon_view=move || view! { <NsfwIcon/> }
                        value=is_nsfw
                    />
                },
            }
        }}
        <IsPinnedCheckbox sphere_name name="post_inputs[post_tags][is_pinned]" value=is_pinned/>
        <SphereCategoryDropdown category_vec_resource init_category_id=category_id name="post_inputs[post_tags][category_id]" show_inactive=false/>
    }
}

/// Component to give a link to external content
#[component]
pub fn LinkForm(
    embed_type_input: RwSignal<EmbedType>,
    link_input: RwSignal<String>,
    title_input: RwSignal<String>,
    /// reference to the textarea node
    #[prop(optional)]
    textarea_ref: NodeRef<html::Textarea>,
) -> impl IntoView {
    let select_trigger = RwSignal::new(0);
    let select_ref = NodeRef::<html::Select>::new();
    view! {
        <div class="flex flex-col gap-2">
            <div class="flex gap-2 items-center">
                <span class="label-text w-fit">{move_tr!("link")}</span>
                <LinkHelpButton/>
                <select
                    name="post_inputs[embed_type]"
                    class="select_input"
                    node_ref=select_ref
                >
                {
                    EmbedType::iter().map(|embed_type| view! {
                        <option
                            selected=move || embed_type_input.get_untracked() == embed_type
                            on:click=move |_| embed_type.on_select(
                                embed_type_input,
                                link_input,
                                select_trigger,
                                textarea_ref,
                            )
                            value={<&'static str>::from(embed_type)}
                        >
                            {embed_type.get_localized_name()}
                        </option>
                    }.into_any()).collect_view()
                }
                </select>
                <LengthLimitedInput
                    name="post_inputs[link]"
                    placeholder=move_tr!("link-url")
                    content=link_input
                    maxlength=Some(MAX_LINK_LENGTH as usize)
                    class="flex-1"
                    textarea_ref
                />
            </div>
            <EmbedPreview embed_type_input link_input select_trigger title_input select_ref/>
        </div>
    }
}

/// Help button explaining how the link form functions
#[component]
pub fn LinkHelpButton() -> impl IntoView {
    view! {
        <HelpButton
            modal_class="absolute bottom-full left-0 z-10 mb-1 -mr-1 p-2 w-86 lg:w-128 bg-base-200 rounded-sm"
            icon_class="h-3 w-3"
        >
            <div class="relative flex flex-col gap-2 leading-snug text-justify text-xs lg:text-sm">
                {move_tr!("link-help")}
            </div>
        </HelpButton>
    }
}