use leptos::either::EitherOf3;
use leptos::prelude::*;

use sphare_core_common::errors::AppError;
use sphare_core_common::unpack::action_has_error;

use crate::errors::{ErrorDetail, ErrorDisplay};
use crate::icons::LoadingIcon;

/// Component to render a server action's error
#[component]
pub fn ActionError<
    T: Send + Sync + 'static,
    A: Send + Sync + 'static,
>(
    action: Action<A, Result<T, AppError>>
) -> impl IntoView {
    view! {
        <Show when=action_has_error(action)>
        {
            match &*action.value().read() {
                Some(Err(e)) => view! { <ErrorDisplay error=e.clone()/> }.into_any(),
                _ => ().into_any(),
            }
        }
        </Show>
    }
}

#[component]
pub fn UnpackAction<
    T: Clone + Send + Sync + 'static,
    A: Send + Sync + 'static,
    V: IntoView + 'static,
    F: Fn(T) -> V + Send + Sync +  'static,
    FB: Fn() -> FV + Send + Sync +  'static,
    FV: IntoView + 'static,
>(
    action: Action<A, Result<T, AppError>>,
    children: F,
    fallback: FB,
) -> impl IntoView {
    let fallback = StoredValue::new(fallback);
    let children = StoredValue::new(children);

    view! {
        <Suspense fallback=move || {
            if action.pending().get() {
                Some(fallback.with_value(|fallback| fallback()))
            } else {
                None
            }
        }>
            {move || match action.value().get() {
                Some(Ok(value)) => Some(Ok(children.with_value(|children| children(value)))),
                Some(Err(e)) => Some(Err(e)),
                None => None,
            }}
        </Suspense>
    }.into_any()
}

async fn unpack_resource<
    T: Clone + Send + Sync + 'static,
    V: IntoView + 'static,
    F: Fn(&T) -> V + Clone + Send + Sync + 'static,
>(
    resource: Resource<Result<T, AppError>>,
    show_error_detail: bool,
    children: StoredValue<F>,
) -> impl IntoView {
    match (&resource.await, show_error_detail) {
        (Ok(value), _) => EitherOf3::A(children.with_value(|children| children(value))),
        (Err(e), false) => EitherOf3::B(view! { <ErrorDisplay error=e.clone()/> } ),
        (Err(e), true) => EitherOf3::C(view! { <ErrorDetail error=e.clone()/> } ),
    }
}

#[component]
pub fn SuspenseUnpack<
    T: Clone + Send + Sync + 'static,
    V: IntoView + 'static,
    F: Fn(&T) -> V + Clone + Send + Sync + 'static,
>(
    resource: Resource<Result<T, AppError>>,
    #[prop(into, default = Box::new(|| view! { <LoadingIcon/> }.into_any()).into())]
    fallback: ViewFnOnce,
    #[prop(default = false)]
    show_error_detail: bool,
    children: F,
) -> impl IntoView {
    let children = StoredValue::new(children);

    view! {
        <Suspense fallback>
        {
            move || Suspend::new(async move {
                unpack_resource(resource, show_error_detail, children).await
            })
        }
        </Suspense>
    }.into_any()
}

#[component]
pub fn TransitionUnpack<
    T: Clone + Send + Sync + 'static,
    V: IntoView + 'static,
    F: Fn(&T) -> V + Clone + Send + Sync + 'static,
>(
    resource: Resource<Result<T, AppError>>,
    #[prop(into, default = Box::new(|| view! { <LoadingIcon/> }.into_any()).into())]
    fallback: ViewFnOnce,
    #[prop(default = false)]
    show_error_detail: bool,
    children: F,
) -> impl IntoView {
    let children = StoredValue::new(children);

    view! {
        <Transition fallback>
        {
            move || Suspend::new(async move {
                unpack_resource(resource, show_error_detail, children).await
            })
        }
        </Transition>
    }.into_any()
}