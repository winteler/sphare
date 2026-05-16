use leptos::either::Either;
use leptos::prelude::*;
use leptos_fluent::move_tr;

use sphare_core_common::common::Rule;
use sphare_core_sphere::rule::{get_rule_description, get_rule_title};

use sphare_cmp_common::state::GlobalState;
use sphare_cmp_utils::errors::ErrorDisplay;
use sphare_cmp_utils::icons::LoadingIcon;
use sphare_cmp_utils::widget::{Collapse, ContentBody, TitleCollapse};

/// List of collapsable rules
#[component]
pub fn BaseRuleList() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    view! {
        <TitleCollapse title=move_tr!("rules")>
            <Suspense fallback=move || view! { <LoadingIcon/> }.into_any()>
            {
                move || Suspend::new(async move {
                    match &state.base_rules.await {
                        Ok(rule_vec) => Either::Left(view!{
                            <RuleList rule_vec=rule_vec.clone()/>
                        }),
                        Err(e) => Either::Right(view! { <ErrorDisplay error=e.clone()/> } ),
                    }
                })
            }
            </Suspense>
        </TitleCollapse>
    }
}

/// List of collapsable rules
#[component]
pub fn RuleList(
    rule_vec: Vec<Rule>,
) -> impl IntoView {
    let rule_elems = rule_vec.into_iter().enumerate().map(|(index, rule)| {
        let is_markdown = rule.markdown_description.is_some();
        let is_sphere_rule = rule.sphere_id.is_some();

        let title = get_rule_title(&rule.title, is_sphere_rule);
        let description = get_rule_description(&rule.title, &rule.description, is_sphere_rule);
        let title_view = move || view! {
            <div class="flex gap-2">
                <div class="font-semibold">{format!("{}.", index+1)}</div>
                <div class="text-left font-semibold">{title}</div>
            </div>
        };
        view! {
            <Collapse
                title_view
                is_open=false
            >
                <div class="pl-1 pb-3">
                    <ContentBody body=description is_markdown/>
                </div>
            </Collapse>
        }
    }).collect_view();

    view! {
        <div class="flex flex-col pl-1 pt-1 gap-1">
        {rule_elems}
        </div>
    }
}