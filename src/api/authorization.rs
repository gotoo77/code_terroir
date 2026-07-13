use crate::models::user::UserRole;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    ManageCatalog,
    ManageBatches,
    RecallBatches,
    CreateQualityCheck,
    UpdateQualityCheck,
    ManageQrTags,
    ManageProducer,
}

pub fn is_allowed(role: &UserRole, action: Action) -> bool {
    match action {
        Action::ManageCatalog | Action::ManageBatches | Action::ManageQrTags => {
            matches!(role, UserRole::Admin | UserRole::Atelier)
        }
        Action::RecallBatches | Action::UpdateQualityCheck => {
            matches!(role, UserRole::Admin | UserRole::Quality)
        }
        Action::CreateQualityCheck => matches!(
            role,
            UserRole::Admin | UserRole::Quality | UserRole::Atelier
        ),
        Action::ManageProducer => matches!(role, UserRole::Admin),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissions_follow_the_declared_role_matrix() {
        let cases = [
            (UserRole::Admin, [true, true, true, true, true, true, true]),
            (
                UserRole::Quality,
                [false, false, true, true, true, false, false],
            ),
            (
                UserRole::Atelier,
                [true, true, false, true, false, true, false],
            ),
            (
                UserRole::Logistics,
                [false, false, false, false, false, false, false],
            ),
            (
                UserRole::ReadOnly,
                [false, false, false, false, false, false, false],
            ),
        ];
        let actions = [
            Action::ManageCatalog,
            Action::ManageBatches,
            Action::RecallBatches,
            Action::CreateQualityCheck,
            Action::UpdateQualityCheck,
            Action::ManageQrTags,
            Action::ManageProducer,
        ];

        for (role, expected) in cases {
            for (index, action) in actions.iter().copied().enumerate() {
                assert_eq!(
                    is_allowed(&role, action),
                    expected[index],
                    "unexpected permission for role {role} and action {action:?}"
                );
            }
        }
    }
}
