use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};
// use crate::controllers::api::v1::{registration_controller, sessions_controller, users_controller};
use crate::models;

/// Master OpenAPI structure aggregating all handlers, schemas, and security rules.
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::controllers::api::v1::registration_controller::registration,
        crate::controllers::api::v1::sessions_controller::create,
        crate::controllers::api::v1::users_controller::index,
        crate::controllers::api::v1::users_controller::show,
        crate::controllers::api::v1::users_controller::me,
        crate::controllers::api::v1::users_controller::update_me,
        crate::controllers::api::v1::users_controller::create,
        crate::controllers::api::v1::users_controller::update,
        crate::controllers::api::v1::users_controller::destroy,
    ),
    components(
        schemas(
            crate::serializers::user_serializer::UserSerializer,
            crate::serializers::user_serializer::UserResponseDto,
            crate::serializers::user_serializer::SessionResponseDto,
            crate::serializers::user_serializer::ErrorResponseDto,
            models::UserRegisterRequestDto,
            models::UserLoginRequestDto,
            models::UserLoginResponseDto,
            models::UserCreateRequestDto,
            models::UserUpdateRequestDto,
            models::PaginationParams,
            models::PaginatedResponse<crate::serializers::user_serializer::UserSerializer>,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Auth", description = "Authentication and Registration"),
        (name = "Users", description = "User Management")
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearerAuth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("Input your raw JSON Web Token (JWT)".to_string()))
                        .build(),
                ),
            );
        }
    }
}
