use crate::models::Claims;
use uuid::Uuid;

pub struct UserPolicy;

impl UserPolicy {
    pub fn index(current_user: &Claims) -> bool {
        Self::is_admin(current_user)
    }

    pub fn show(current_user: &Claims, target_user_id: Uuid) -> bool {
        Self::is_admin(current_user) || current_user.user_id == target_user_id
    }

    pub fn update(current_user: &Claims, target_user_id: Uuid) -> bool {
        Self::show(current_user, target_user_id)
    }

    pub fn destroy(current_user: &Claims) -> bool {
        Self::index(current_user)
    }

    // In our implementation, admin = 0
    fn is_admin(current_user: &Claims) -> bool {
        current_user.role == 0
    }
}
