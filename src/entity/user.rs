use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::EntityError;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum Role {
    Reader,
    Uploader,
    Moderator,
    Admin,
}

impl FromStr for Role {
    type Err = EntityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Reader" => Ok(Self::Reader),
            "Uploader" => Ok(Self::Uploader),
            "Moderator" => Ok(Self::Moderator),
            "Admin" => Ok(Self::Admin),
            other => Err(EntityError::InvalidRole(other.to_owned())),
        }
    }
}

impl AsRef<str> for Role {
    fn as_ref(&self) -> &str {
        match self {
            Self::Reader => "Reader",
            Self::Uploader => "Uploader",
            Self::Moderator => "Moderator",
            Self::Admin => "Admin",
        }
    }
}

#[derive(Debug, Default)]
pub struct UserClaims {
    pub id: Uuid,
    pub roles: Vec<Role>,
}

impl UserClaims {
    pub fn holds(&self, role: Role) -> bool {
        self.roles.contains(&role)
    }

    pub fn can_moderate(&self) -> bool {
        self.holds(Role::Moderator) || self.holds(Role::Admin)
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::entity::{Role, UserClaims};

    fn claims(roles: &[Role]) -> UserClaims {
        UserClaims {
            id: Uuid::now_v7(),
            roles: roles.to_vec(),
        }
    }

    #[test]
    fn every_role_round_trips() {
        for s in ["Reader", "Uploader", "Moderator", "Admin"] {
            let role: Role = s.parse().unwrap();
            assert_eq!(role.as_ref(), s);
        }
    }

    #[test]
    fn unknown_role_is_rejected() {
        let res = "Owner".parse::<Role>();
        assert!(res.is_err());
    }

    #[test]
    fn a_guest_holds_nothing() {
        let guest = UserClaims::default();

        assert!(guest.id.is_nil());
        assert!(!guest.can_moderate());
    }

    #[test]
    fn only_moderators_and_admins_moderate() {
        assert!(claims(&[Role::Moderator]).can_moderate());
        assert!(claims(&[Role::Admin]).can_moderate());
        assert!(!claims(&[Role::Reader, Role::Uploader]).can_moderate());
    }
}
