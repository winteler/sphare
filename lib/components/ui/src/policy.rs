use std::str::FromStr;

use leptos::either::Either;
use leptos::prelude::*;
use leptos_fluent::move_tr;

use sphare_core_common::routes::{CONTENT_POLICY_ROUTE, PRIVACY_POLICY_ROUTE, RULES_ROUTE};
use sphare_core_sphere::rule::BaseRule;

use sphare_cmp_common::state::GlobalState;
use sphare_cmp_utils::errors::ErrorDisplay;
use sphare_cmp_utils::icons::{LoadingIcon, NsfwIcon, SpoilerIcon};
use sphare_cmp_utils::widget::{ContentBody, TitleCollapse};

use crate::sidebar::HomeSidebar;

#[component]
pub fn AboutSphare() -> impl IntoView {
    view! {
        <div class="w-full overflow-y-auto">
            <div class="flex flex-col gap-4 w-4/5 lg:w-1/2 4xl:w-2/5 mx-auto py-4">
                <h1 class="text-3xl font-bold text-center">{move_tr!("about-sphare")}</h1>
                <p class="text-justify">
                    {move_tr!("about-sphare-content")}
                </p>
                <h2 class="text-xl font-semibold">{move_tr!("rules-and-moderation")}</h2>
                <p class="text-justify">
                    {move_tr!("about-sphare-rules-1")}
                    <a href=RULES_ROUTE>{move_tr!("about-sphare-rules-link")}</a>
                    {move_tr!("about-sphare-rules-2")}
                </p>
                <FutureImprovements/>
                <NameExplanation/>
                <OriginsAndGoals/>
            </div>
        </div>
        <HomeSidebar/>
    }
}

#[component]
pub fn TermsAndConditions() -> impl IntoView {
    view! {
        <div class="w-full overflow-y-auto">
            <div class="flex flex-col gap-4 items-center w-4/5 lg:w-1/2 4xl:w-2/5 mx-auto py-4">
                <h1 class="text-3xl font-bold text-center">{move_tr!("terms-and-conditions")}</h1>
                <SphareInfo/>
                <AcceptanceOfTerms/>
                <DescriptionOfService/>
                <UserResponsibilities/>
                <Moderation/>
                <IntellectualProperty/>
                <LimitationOfLiability/>
                <DataProtection/>
                <Amendments/>
                <GoverningLaw/>
            </div>
        </div>
        <HomeSidebar/>
    }
}

#[component]
pub fn PrivacyPolicy() -> impl IntoView {
    view! {
        <div class="w-full overflow-y-auto">
            <div class="flex flex-col gap-4 items-center w-4/5 lg:w-1/2 4xl:w-2/5 mx-auto py-4">
                <h1 class="text-3xl font-bold text-center">{move_tr!("privacy-policy")}</h1>
                <SphareInfo/>
                <AboutPrivacyPolicy/>
                <DataCollection/>
                <DataCollectionPurpose/>
                <LegalBasis/>
                <Cookies/>
                <DataSharing/>
                <DataStorage/>
                <UserRights/>
                <PrivacyPolicyChanges/>
            </div>
        </div>
        <HomeSidebar/>
    }
}

#[component]
pub fn ContentPolicy() -> impl IntoView {
    view! {
        <div class="w-full overflow-y-auto">
            <div class="flex flex-col gap-4 w-4/5 lg:w-1/2 4xl:w-2/5 mx-auto py-4">
                <h1 class="text-3xl font-bold text-center">{move_tr!("content-policy")}</h1>
                <p class="text-justify">
                    {move_tr!("content-policy-intro")}
                </p>
                <div class="flex flex-col gap-2">
                    <h2 class="text-xl font-semibold">{move_tr!("banned-content-title")}</h2>
                    <p>{move_tr!("banned-content-intro")}</p>
                    <ul class="list-disc list-inside">
                        <li>{move_tr!("banned-content-1")}</li>
                        <li>{move_tr!("banned-content-2")}</li>
                        <li>{move_tr!("banned-content-3")}</li>
                        <li>{move_tr!("banned-content-4")}</li>
                        <li>{move_tr!("banned-content-5")}</li>
                        <li>{move_tr!("banned-content-6")}</li>
                        <li>{move_tr!("banned-content-7")}</li>
                        <li>{move_tr!("banned-content-8")}</li>
                        <li>{move_tr!("banned-content-9")}</li>
                        <li>{move_tr!("banned-content-10")}</li>
                        <li>{move_tr!("banned-content-11")}</li>
                        <li>{move_tr!("banned-content-12")}</li>
                        <li>{move_tr!("banned-content-13")}</li>
                        <li>{move_tr!("banned-content-14")}</li>
                    </ul>
                </div>
                <div class="flex flex-col gap-2">
                    <h2 class="text-xl font-semibold">{move_tr!("sensitive-content-title")}</h2>
                    <h3 class="text-lg font-semibold">{move_tr!("mature-content-title")}</h3>
                    <div class="text-justify">
                        {move_tr!("mature-content-description")}
                        <NsfwIcon class="inline-flex"/>
                    </div>
                    <h3 class="text-lg font-semibold">{move_tr!("spoiler-content-title")}</h3>
                    <div class="text-justify">
                        {move_tr!("spoiler-content-description")}
                        <div class="h-fit w-fit px-1 py-0.5 bg-black rounded-full inline-flex relative top-1"><SpoilerIcon/></div>
                    </div>
                    <p>{move_tr!("spoiler-content-label-1")}</p>
                    <ul class="list-disc list-inside text-justify">
                        <li>{move_tr!("spoiler-content-label-2")}</li>
                        <li>{move_tr!("spoiler-content-label-3")}</li>
                    </ul>
                    <p class="text-justify">{move_tr!("spoiler-content-label-4")}</p>
                </div>
            </div>
        </div>
        <HomeSidebar/>
    }
}

#[component]
pub fn FutureImprovements() -> impl IntoView {
    view! {
        <h2 class="text-xl font-semibold">{move_tr!("future-improvements-title")}</h2>
        <p>{move_tr!("future-improvements-intro")}</p>
        <ul class="list-disc list-inside text-justify">
            <li>{move_tr!("future-improvements-1")}</li>
            <li>{move_tr!("future-improvements-2")}</li>
            <li>{move_tr!("future-improvements-3")}</li>
            <li>{move_tr!("future-improvements-4")}</li>
            <li>{move_tr!("future-improvements-5")}</li>
            <li>{move_tr!("future-improvements-6")}</li>
            <li>{move_tr!("future-improvements-7")}</li>
        </ul>
    }
}

#[component]
pub fn NameExplanation() -> impl IntoView {
    view! {
        <h2 class="text-xl font-semibold">{move_tr!("name-explanation-title")}</h2>
        <p class="text-justify">{move_tr!("name-explanation-content-1")}</p>
        <p class="text-justify">{move_tr!("name-explanation-content-2")}</p>
    }
}

#[component]
pub fn OriginsAndGoals() -> impl IntoView {
    view! {
        <h2 class="text-xl font-semibold">{move_tr!("origin-goals-title")}</h2>
        <p class="text-justify">{move_tr!("origin-goals-1")}</p>
        <p class="text-justify">{move_tr!("origin-goals-2")}</p>
        <p class="text-justify">{move_tr!("origin-goals-3")}</p>
        <p class="text-justify">{move_tr!("origin-goals-4")}</p>
        <p class="text-justify">{move_tr!("origin-goals-5")}</p>
    }
}

#[component]
pub fn Rules() -> impl IntoView {
    let state = expect_context::<GlobalState>();
    view! {
        <div class="w-full overflow-y-auto">
            <div class="flex flex-col gap-4 items-center w-4/5 lg:w-1/2 4xl:w-2/5 mx-auto py-4">
                <h1 class="text-3xl font-bold text-center">{move_tr!("rules")}</h1>
                <p class="text-justify">{move_tr!("rules-intro")}</p>
                <Suspense fallback=move || view! { <LoadingIcon/> }.into_any()>
                {
                    move || Suspend::new(async move {
                        match &state.base_rules.await {
                            Ok(rule_vec) => {
                                Either::Left(rule_vec.iter().enumerate().map(|(index, rule)| {
                                let rule_enum = BaseRule::from_str(&rule.title).expect("Should get base rule.");
                                let title = rule_enum.get_localized_title();
                                view! {
                                    <div class="flex flex-col gap-2">
                                        <h2 class="text-xl font-semibold">{move || format!("{}. {}", index + 1, title.read())}</h2>
                                        <ContentBody
                                            body=rule_enum.get_localized_description()
                                            is_markdown=rule.markdown_description.is_some()
                                            attr:class="text-justify"
                                        />
                                    </div>
                                }
                                }).collect_view())
                            },
                            Err(e) => Either::Right(view! { <ErrorDisplay error=e.clone()/> } ),
                        }
                    })
                }
                </Suspense>
            </div>
        </div>
        <HomeSidebar/>
    }
}

#[component]
fn SphareInfo() -> impl IntoView {
    view! {
        <div class="flex flex-col items-center gap-1">
            <p>{move_tr!("info-validity")}</p>
            <p>{move_tr!("info-operator")}</p>
        </div>
    }
}

#[component]
fn AcceptanceOfTerms() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <p class="text-justify">{move_tr!("acceptance-of-terms-content")}</p>
        </div>
    }
}

#[component]
fn DescriptionOfService() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <h2 class="text-2xl font-semibold">{move_tr!("description-of-service-title")}</h2>
            <p class="text-justify">{move_tr!("description-of-service-content")}</p>
        </div>
    }
}

#[component]
fn UserResponsibilities() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <h2 class="text-2xl font-semibold">{move_tr!("user-responsibilities-title")}</h2>
            <p>{move_tr!("user-responsibilities-1")}</p>
            <ul class="list-disc list-inside text-justify">
                <li>{move_tr!("user-responsibilities-bullet-1")}</li>
                <li>
                    {move_tr!("user-responsibilities-bullet-2-1")}
                    <a href=RULES_ROUTE class="link text-primary">
                        {move_tr!("rules")}
                    </a>
                    {move_tr!("user-responsibilities-bullet-2-2")}
                    <a href=CONTENT_POLICY_ROUTE class="link text-primary">
                        {move_tr!("content-policy")}
                    </a>
                    {move_tr!("user-responsibilities-bullet-2-3")}
                </li>
                <li>{move_tr!("user-responsibilities-bullet-3")}</li>
                <li>{move_tr!("user-responsibilities-bullet-4")}</li>
                <li>{move_tr!("user-responsibilities-bullet-5")}</li>
            </ul>
            <p>{move_tr!("user-responsibilities-2")}</p>
        </div>
    }
}

#[component]
fn Moderation() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <h2 class="text-2xl font-semibold">{move_tr!("moderation-title")}</h2>
            <p class="text-justify">{move_tr!("moderation-content")}</p>
        </div>
    }
}

#[component]
fn IntellectualProperty() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <h2 class="text-2xl font-semibold">{move_tr!("intellectual-property-title")}</h2>
            <p class="text-justify">{move_tr!("intellectual-property-content-1")}</p>
            <p class="text-justify">{move_tr!("intellectual-property-content-2")}</p>
        </div>
    }
}

#[component]
fn LimitationOfLiability() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <h2 class="text-2xl font-semibold">{move_tr!("liability-limitation-title")}</h2>
            <p class="text-justify">{move_tr!("liability-limitation-content")}</p>
            <ul class="list-disc list-inside text-justify">
                <li>{move_tr!("liability-limitation-bullet-1")}</li>
                <li>{move_tr!("liability-limitation-bullet-2")}</li>
                <li>{move_tr!("liability-limitation-bullet-3")}</li>
            </ul>
        </div>
    }
}

#[component]
fn DataProtection() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <h2 class="text-2xl font-semibold">{move_tr!("data-protection-title")}</h2>
            <p>
                {move_tr!("data-protection-content-1")}
                <a href=PRIVACY_POLICY_ROUTE class="link text-primary">
                    {move_tr!("privacy-policy")}
                </a>
                {move_tr!("data-protection-content-2")}
            </p>
        </div>
    }
}

#[component]
fn Amendments() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <h2 class="text-2xl font-semibold">{move_tr!("amendments-title")}</h2>
            <p class="text-justify">{move_tr!("amendments-content")}</p>
        </div>
    }
}

#[component]
fn GoverningLaw() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <h2 class="text-2xl font-semibold">{move_tr!("governing-law-title")}</h2>
            <p class="text-justify">{move_tr!("governing-law-content")}</p>
        </div>
    }
}

#[component]
fn AboutPrivacyPolicy() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <p class="text-justify">{move_tr!("about-privacy-policy-content")}</p>
        </div>
    }
}

#[component]
fn DataCollection() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <h2 class="text-2xl font-semibold">{move_tr!("data-collection-title")}</h2>
            <p class="text-justify">{move_tr!("data-collection-content")}</p>
            <ul class="list-disc list-inside">
                <li>{move_tr!("data-collection-bullet-1")}</li>
                <li>{move_tr!("data-collection-bullet-2")}</li>
                <li>{move_tr!("data-collection-bullet-3")}</li>
                <li>{move_tr!("data-collection-bullet-4")}</li>
            </ul>
        </div>
    }
}

#[component]
fn DataCollectionPurpose() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <h2 class="text-2xl font-semibold">{move_tr!("data-collect-purpose-title")}</h2>
            <p class="text-justify">{move_tr!("data-collect-purpose-content")}</p>
            <ul class="list-disc list-inside text-justify">
                <li>{move_tr!("data-collect-purpose-bullet-1")}</li>
                <li>{move_tr!("data-collect-purpose-bullet-2")}</li>
                <li>{move_tr!("data-collect-purpose-bullet-3")}</li>
                <li>{move_tr!("data-collect-purpose-bullet-4")}</li>
            </ul>
        </div>
    }
}

#[component]
fn LegalBasis() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <h2 class="text-2xl font-semibold">{move_tr!("legal-basis-title")}</h2>
            <p >{move_tr!("legal-basis-content")}</p>
            <ul class="list-disc list-inside text-justify">
                <li>{move_tr!("legal-basis-bullet-1")}</li>
                <li>{move_tr!("legal-basis-bullet-2")}</li>
                <li>{move_tr!("legal-basis-bullet-3")}</li>
                <li>{move_tr!("legal-basis-bullet-4")}</li>
            </ul>
        </div>
    }
}

#[component]
fn Cookies() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <h2 class="text-2xl font-semibold">{move_tr!("cookies-title")}</h2>
            <p >{move_tr!("cookies-content")}</p>
            <ul class="list-disc list-inside">
                <li>{move_tr!("cookies-bullet-1")}</li>
                <li>{move_tr!("cookies-bullet-2")}</li>
            </ul>
        </div>
    }
}

#[component]
fn DataSharing() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <h2 class="text-2xl font-semibold">{move_tr!("data-sharing-title")}</h2>
            <p class="text-justify">{move_tr!("data-sharing-content")}</p>
            <ul class="list-disc list-inside">
                <li>{move_tr!("data-sharing-bullet-1")}</li>
                <li>{move_tr!("data-sharing-bullet-2")}</li>
            </ul>
        </div>
    }
}

#[component]
fn DataStorage() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <h2 class="text-2xl font-semibold">{move_tr!("data-storage-title")}</h2>
            <p class="text-justify">{move_tr!("data-storage-content")}</p>
        </div>
    }
}

#[component]
fn UserRights() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <h2 class="text-2xl font-semibold">{move_tr!("user-rights-title")}</h2>
            <p class="text-justify">{move_tr!("user-rights-content")}</p>
            <ul class="list-disc list-inside text-justify">
                <li>{move_tr!("user-rights-bullet-1")}</li>
                <li>{move_tr!("user-rights-bullet-2")}</li>
                <li>{move_tr!("user-rights-bullet-3")}</li>
                <li>{move_tr!("user-rights-bullet-4")}</li>
            </ul>
            <p class="text-justify">{move_tr!("user-rights-contact")}</p>
        </div>
    }
}

#[component]
fn PrivacyPolicyChanges() -> impl IntoView {
    view! {
        <div class="w-full flex flex-col gap-1">
            <h2 class="text-2xl font-semibold">{move_tr!("policy-change-title")}</h2>
            <p class="text-justify">{move_tr!("policy-change-content")}</p>
        </div>
    }
}

#[component]
pub fn Faq() -> impl IntoView {
    view! {
        <div class="w-full overflow-y-auto">
            <div class="flex flex-col gap-4 w-4/5 xl:w-2/3 2xl:w-2/5 mx-auto py-4">
                <h1 class="text-3xl font-bold text-center">{move_tr!("faq")}</h1>
                <FaqItem
                    title=move_tr!("faq-registration-missing-email-question")
                    content=move_tr!("faq-registration-missing-email-answer")
                />
                <FaqItem
                    title=move_tr!("faq-registration-error-question")
                    content=move_tr!("faq-registration-error-answer")
                />
                <FaqItem
                    title=move_tr!("faq-post-not-visible-question")
                    content=move_tr!("faq-post-not-visible-answer")
                />
                <FaqItem
                    title=move_tr!("faq-image-question")
                    content=move_tr!("faq-image-answer")
                />
            </div>
        </div>
        <HomeSidebar/>
    }
}

#[component]
pub fn FaqItem(
    title: Signal<String>,
    content: Signal<String>,
) -> impl IntoView {
    view! {
        <TitleCollapse
            title
            is_open=false
        >
            <div class="p-1">{content}</div>
        </TitleCollapse>
    }
}