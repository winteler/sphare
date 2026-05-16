use leptos::prelude::*;
use leptos_fluent::{move_tr};

use sphare_core_content::moderation::Content;
use sphare_core_sphere::rule::{get_rule_description, get_rule_title};

use sphare_cmp_utils::icons::HammerIcon;
use sphare_cmp_utils::widget::ContentBody;

/// Displays the body of a moderated post or comment
#[component]
pub fn ModeratedBody(
    infringed_rule_title: String,
    moderator_message: String,
    is_sphere_rule: bool,
) -> impl IntoView {
    let infringed_rule_title = get_rule_title(&infringed_rule_title, is_sphere_rule);
    view! {
        <div class="flex">
            <div class="shrink-0 flex justify-center items-center p-2 rounded-l bg-base-content/20">
                <HammerIcon/>
            </div>
            <div class="p-2 rounded-r bg-base-300">
                <div class="w-full flex flex-col">
                    <div class="max-w-full whitespace-normal break-words">{moderator_message}</div>
                    <div class="font-semibold pt-1">{move_tr!("infringed-rule")}</div>
                    <div class="max-w-full whitespace-normal break-words">{infringed_rule_title}</div>
                </div>
            </div>
        </div>
    }.into_any()
}

/// Component to display the details of a moderation instance
#[component]
pub fn ModerationInfoDialog(
    moderated_content: Content,
    rule_title: String,
    rule_description: String,
    is_sphere_rule: bool,
) -> impl IntoView {
    let title = get_rule_title(&rule_title, is_sphere_rule);
    let description = get_rule_description(&rule_title, &rule_description, is_sphere_rule);
    view! {
        <div class="flex flex-col gap-3">
            <h1 class="text-center font-bold text-2xl">"Ban details"</h1>
            {
                match &moderated_content {
                    Content::Post(post) => view! {
                        <div class="flex flex-col gap-1 p-2 border-b">
                            <h1 class="font-bold text-xl">{move_tr!("content")}</h1>
                            <div>{post.title.clone()}</div>
                            <ContentBody
                                body=post.body.clone()
                                is_markdown=post.markdown_body.is_some()
                            />
                        </div>
                        <div class="flex flex-col gap-1 p-2 border-b">
                            <h1 class="font-bold text-xl">{move_tr!("moderator-message")}</h1>
                            <div>{post.moderator_message.clone()}</div>
                        </div>
                    }.into_any(),
                    Content::Comment(comment) => {
                        view! {
                            <div class="flex flex-col gap-1 p-2 border-b border-base-content/20">
                                <div class="font-bold text-xl">{move_tr!("content")}</div>
                                <ContentBody
                                    body=comment.body.clone()
                                    is_markdown=comment.markdown_body.is_some()
                                />
                            </div>
                            <div class="flex flex-col gap-1 p-2 border-b">
                                <div class="font-bold text-xl">{move_tr!("moderator-message")}</div>
                                <div>{comment.moderator_message.clone()}</div>
                            </div>
                        }.into_any()
                    }
                }
            }
            <div class="flex flex-col gap-1 p-2">
                <h1 class="font-bold text-xl">{move_tr!("infringed-rule")}</h1>
                <div class="text-lg font-semibold">{title}</div>
                <ContentBody
                    body=description
                    is_markdown=!is_sphere_rule
                />
            </div>
        </div>
    }
}