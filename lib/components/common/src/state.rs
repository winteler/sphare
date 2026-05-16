use std::collections::HashMap;

use leptos::prelude::*;

use sphare_core_common::common::Rule;
use sphare_core_common::errors::AppError;
use sphare_core_content::filter::SphereCategoryFilter;
use sphare_core_content::ranking::{CommentSortType, PostSortType, SortType};
use sphare_core_sphere::satellite::Satellite;
use sphare_core_sphere::sphere::SphereWithUserInfo;
use sphare_core_sphere::sphere_category::SphereCategory;
use sphare_core_user::notification::Notification;
use sphare_core_user::role::{PermissionLevel, UserSphereRole};
use sphare_core_user::user::User;

use sphare_iface_content::moderation::ModeratePost;
use sphare_iface_content::post::{DeletePost, EditPost};
use sphare_iface_sphere::rule::{get_rule_vec, AddRule, RemoveRule, UpdateRule};
use sphare_iface_sphere::satellite::{get_satellite_vec_by_sphere_name, ActivateSatellite, CreateSatellite, DeactivateSatellite, UpdateSatellite};
use sphare_iface_sphere::sphere::{get_sphere_with_user_info, CreateSphere, Subscribe, Unsubscribe, UpdateSphereDescription};
use sphare_iface_sphere::sphere_category::{get_sphere_category_vec, DeleteSphereCategory, SetSphereCategory};
use sphare_iface_user::auth::{EndSession, Login};
use sphare_iface_user::notification::get_notifications;
use sphare_iface_user::role::{get_sphere_role_vec, SetUserSphereRole};
use sphare_iface_user::user::{DeleteUser, SetUserSettings};

#[derive(Copy, Clone)]
pub struct GlobalState {
    pub login_action: ServerAction<Login>,
    pub logout_action: ServerAction<EndSession>,
    pub delete_user_action: ServerAction<DeleteUser>,
    pub set_settings_action: ServerAction<SetUserSettings>,
    pub subscribe_action: ServerAction<Subscribe>,
    pub unsubscribe_action: ServerAction<Unsubscribe>,
    pub edit_post_action: ServerAction<EditPost>,
    pub delete_post_action: ServerAction<DeletePost>,
    pub create_sphere_action: ServerAction<CreateSphere>,
    pub create_satellite_action: ServerAction<CreateSatellite>,
    pub update_satellite_action: ServerAction<UpdateSatellite>,
    pub activate_satellite_action: ServerAction<ActivateSatellite>,
    pub deactivate_satellite_action: ServerAction<DeactivateSatellite>,
    pub update_sphere_desc_action: ServerAction<UpdateSphereDescription>,
    pub set_sphere_category_action: ServerAction<SetSphereCategory>,
    pub delete_sphere_category_action: ServerAction<DeleteSphereCategory>,
    pub set_sphere_role_action: ServerAction<SetUserSphereRole>,
    pub add_rule_action: ServerAction<AddRule>,
    pub update_rule_action: ServerAction<UpdateRule>,
    pub remove_rule_action: ServerAction<RemoveRule>,
    pub moderate_post_action: ServerAction<ModeratePost>,
    pub sphere_reload_signal: RwSignal<usize>,
    pub post_sort_type: RwSignal<SortType>,
    pub comment_sort_type: RwSignal<SortType>,
    pub show_left_sidebar: RwSignal<bool>,
    pub show_right_sidebar: RwSignal<bool>,
    pub unread_notif_count: RwSignal<usize>,
    pub is_notif_read_map: StoredValue<HashMap<i64, ArcRwSignal<bool>>>,
    pub notif_resource: Resource<Result<Vec<Notification>, AppError>>,
    pub user: Resource<Result<Option<User>, AppError>>,
    pub base_rules: OnceResource<Result<Vec<Rule>, AppError>>,
}

#[derive(Copy, Clone)]
pub struct SphereState {
    pub sphere_name: Memo<String>,
    pub sphere_category_filter: RwSignal<SphereCategoryFilter>,
    pub post_refresh_count: RwSignal<usize>,
    pub permission_level: Signal<PermissionLevel>,
    pub sphere_with_user_info_resource: Resource<Result<SphereWithUserInfo, AppError>>,
    pub satellite_vec_resource: Resource<Result<Vec<Satellite>, AppError>>,
    pub sphere_categories_resource: Resource<Result<Vec<SphereCategory>, AppError>>,
    pub sphere_roles_resource: Resource<Result<Vec<UserSphereRole>, AppError>>,
    pub sphere_rules_resource: Resource<Result<Vec<Rule>, AppError>>,
}

#[derive(Copy, Clone)]
pub struct SatelliteState {
    pub satellite_id: Memo<i64>,
    pub sort_type: RwSignal<SortType>,
    pub category_id_filter: RwSignal<Option<i64>>,
    pub satellite_resource: Resource<Result<Satellite, AppError>>,
}

impl GlobalState {
    pub fn new(
        user: Resource<Result<Option<User>, AppError>>,
        logout_action: ServerAction<EndSession>,
        delete_user_action: ServerAction<DeleteUser>,
        create_sphere_action: ServerAction<CreateSphere>,
        set_settings_action: ServerAction<SetUserSettings>,
    ) -> Self {
        let is_notif_read_map = StoredValue::new(HashMap::new());
        Self {
            login_action: ServerAction::<Login>::new(),
            logout_action,
            delete_user_action,
            set_settings_action,
            subscribe_action: ServerAction::<Subscribe>::new(),
            unsubscribe_action: ServerAction::<Unsubscribe>::new(),
            edit_post_action: ServerAction::<EditPost>::new(),
            delete_post_action: ServerAction::<DeletePost>::new(),
            create_sphere_action,
            create_satellite_action: ServerAction::<CreateSatellite>::new(),
            update_satellite_action: ServerAction::<UpdateSatellite>::new(),
            activate_satellite_action: ServerAction::<ActivateSatellite>::new(),
            deactivate_satellite_action: ServerAction::<DeactivateSatellite>::new(),
            update_sphere_desc_action: ServerAction::<UpdateSphereDescription>::new(),
            set_sphere_category_action: ServerAction::<SetSphereCategory>::new(),
            delete_sphere_category_action: ServerAction::<DeleteSphereCategory>::new(),
            set_sphere_role_action: ServerAction::<SetUserSphereRole>::new(),
            add_rule_action: ServerAction::<AddRule>::new(),
            update_rule_action: ServerAction::<UpdateRule>::new(),
            remove_rule_action: ServerAction::<RemoveRule>::new(),
            moderate_post_action: ServerAction::<ModeratePost>::new(),
            sphere_reload_signal: RwSignal::new(0),
            post_sort_type: RwSignal::new(SortType::Post(PostSortType::Hot)),
            comment_sort_type: RwSignal::new(SortType::Comment(CommentSortType::Best)),
            show_left_sidebar: RwSignal::new(false),
            show_right_sidebar: RwSignal::new(false),
            unread_notif_count: RwSignal::new(0),
            is_notif_read_map,
            notif_resource: Resource::new(
                move || (),
                move |_| {
                    is_notif_read_map.write_value().clear();
                    async move {
                        get_notifications().await
                    }
                },
            ),
            user,
            base_rules: OnceResource::new(get_rule_vec(None))
        }
    }
}

impl SphereState {
    pub fn new(
        sphere_name: Memo<String>,
        state: GlobalState,
    ) -> Self  {
        Self {
            sphere_name,
            sphere_category_filter: RwSignal::new(SphereCategoryFilter::All),
            post_refresh_count: RwSignal::new(0),
            permission_level: Signal::derive(
                move || match &(*state.user.read()) {
                    Some(Ok(Some(user))) => user.get_sphere_permission_level(&*sphere_name.read()),
                    _ => PermissionLevel::None,
                }
            ),
            sphere_with_user_info_resource: Resource::new(
                move || (
                    sphere_name.get(),
                    state.update_sphere_desc_action.version().get(),
                    state.sphere_reload_signal.get(),
                ),
                move |(sphere_name, _, _)| get_sphere_with_user_info(sphere_name)
            ),
            satellite_vec_resource: Resource::new(
                move || (
                    sphere_name.get(),
                    state.create_satellite_action.version().get(),
                    state.update_satellite_action.version().get(),
                    state.deactivate_satellite_action.version().get(),
                ),
                move |(sphere_name, _, _, _)| get_satellite_vec_by_sphere_name(sphere_name, false)
            ),
            sphere_categories_resource: Resource::new(
                move || (
                    sphere_name.get(),
                    state.set_sphere_category_action.version().get(),
                    state.delete_sphere_category_action.version().get()
                ),
                move |(sphere_name, _, _)| get_sphere_category_vec(sphere_name)
            ),
            sphere_roles_resource: Resource::new(
                move || (sphere_name.get(), state.set_sphere_role_action.version().get()),
                move |(sphere_name, _)| get_sphere_role_vec(sphere_name),
            ),
            sphere_rules_resource: Resource::new(
                move || (
                    sphere_name.get(),
                    state.add_rule_action.version().get(),
                    state.update_rule_action.version().get(),
                    state.remove_rule_action.version().get()
                ),
                move |(sphere_name, _, _, _)| get_rule_vec(Some(sphere_name)),
            ),
        }
    }
}