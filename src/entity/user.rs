use std::str::FromStr;
use std::sync::LazyLock;

use regex::Regex;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entity::{Bounded, BoundedVec, Email, Entity, Timestamp},
    error::EntityError,
};

// Closed vocabulary: one value per `Role` variant. Adding or removing a variant
// means updating this constant.
pub const MAX_USER_ROLES: usize = 4;

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

pub struct UserRolesBound;

impl Bounded for UserRolesBound {
    const MAX: usize = MAX_USER_ROLES;
    const NAME: &'static str = "user roles";
}

pub type Roles = BoundedVec<Role, UserRolesBound>;

impl TryFrom<Vec<String>> for Roles {
    type Error = EntityError;

    fn try_from(raw: Vec<String>) -> Result<Self, Self::Error> {
        raw.into_iter()
            .map(|role| role.parse())
            .collect::<Result<Vec<Role>, _>>()?
            .try_into()
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Username(String);

impl TryFrom<String> for Username {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        static PATTERN: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9_]{3,32}$").unwrap());

        if !PATTERN.is_match(&s) {
            return Err(EntityError::UsernameIsMalformed);
        }
        Ok(Self(s))
    }
}

impl AsRef<str> for Username {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct Password(SecretString);

impl TryFrom<String> for Password {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        const MIN: usize = 16;
        const MAX: usize = 128;

        let len = s.chars().count();
        if !(MIN..=MAX).contains(&len) {
            return Err(EntityError::PasswordLengthOutOfRange(MIN, MAX));
        }
        Ok(Self(SecretString::from(s)))
    }
}

impl Password {
    pub fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }
}

#[derive(Debug, PartialEq)]
pub struct PasswordHash(String);

impl TryFrom<String> for PasswordHash {
    type Error = EntityError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        const MAX: usize = 255;

        if s.chars().count() > MAX {
            return Err(EntityError::PasswordHashExceedsCharLimit(MAX));
        }
        Ok(Self(s))
    }
}

impl AsRef<str> for PasswordHash {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug)]
pub struct User {
    pub id: Uuid,
    pub email: Email,
    pub username: Username,
    pub password: Password,
    pub created_at: Timestamp,
}

impl Entity for User {
    const NAME: &'static str = "User";
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserQuery {
    pub id: Uuid,
    pub email: Email,
    pub username: Username,
    pub roles: Roles,
    pub verified_at: Option<Timestamp>,
    pub created_at: Timestamp,
}

#[derive(Debug)]
pub struct UserCredentials {
    pub id: Uuid,
    pub password_hash: PasswordHash,
    pub roles: Roles,
}

#[derive(Debug, Clone)]
pub struct UserClaims {
    pub id: Uuid,
    pub sid: Uuid,
    pub roles: Vec<Role>,
}

impl UserClaims {
    pub fn holds(&self, role: Role) -> bool {
        self.roles.contains(&role)
    }

    pub fn can_moderate(&self) -> bool {
        self.holds(Role::Moderator) || self.holds(Role::Admin)
    }

    pub fn is_content_writer(&self) -> bool {
        self.holds(Role::Uploader) || self.can_moderate()
    }

    pub fn has_standing_over(&self, owner: Uuid) -> bool {
        self.id == owner || self.can_moderate()
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::{
        entity::fixtures::claims,
        entity::{MAX_USER_ROLES, Password, Role, Roles, Username},
    };

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
    fn roles_parse_from_their_wire_names() {
        let roles = Roles::try_from(vec!["Reader".to_owned(), "Admin".to_owned()]).unwrap();

        assert_eq!(roles.as_slice(), [Role::Reader, Role::Admin]);
    }

    #[test]
    fn an_unknown_name_rejects_the_whole_set() {
        let res = Roles::try_from(vec!["Reader".to_owned(), "Owner".to_owned()]);

        assert!(res.is_err());
    }

    #[test]
    fn more_values_than_the_vocabulary_are_rejected() {
        let res = Roles::try_from(vec!["Reader".to_owned(); MAX_USER_ROLES + 1]);

        assert!(res.is_err());
    }

    #[test]
    fn only_moderators_and_admins_moderate() {
        assert!(claims(Uuid::now_v7(), &[Role::Moderator]).can_moderate());
        assert!(claims(Uuid::now_v7(), &[Role::Admin]).can_moderate());
        assert!(!claims(Uuid::now_v7(), &[Role::Reader, Role::Uploader]).can_moderate());
    }

    #[test]
    fn uploaders_moderators_and_admins_are_content_writers() {
        assert!(claims(Uuid::now_v7(), &[Role::Uploader]).is_content_writer());
        assert!(claims(Uuid::now_v7(), &[Role::Moderator]).is_content_writer());
        assert!(claims(Uuid::now_v7(), &[Role::Admin]).is_content_writer());
        assert!(!claims(Uuid::now_v7(), &[Role::Reader]).is_content_writer());
    }

    #[test]
    fn a_username_within_the_alphabet_and_length_is_valid() {
        for s in ["abc", "a_b_9", &"a".repeat(32)] {
            assert!(
                Username::try_from(s.to_owned()).is_ok(),
                "{s} must be valid"
            );
        }
    }

    #[test]
    fn a_username_outside_the_length_bounds_is_rejected() {
        for s in ["ab", &"a".repeat(33)] {
            assert!(
                Username::try_from(s.to_owned()).is_err(),
                "{s} must be rejected"
            );
        }
    }

    #[test]
    fn a_username_with_disallowed_characters_is_rejected() {
        for s in ["has space", "dot.dot", "dash-dash", "unïcode"] {
            assert!(
                Username::try_from(s.to_owned()).is_err(),
                "{s} must be rejected"
            );
        }
    }

    #[test]
    fn a_password_within_16_to_128_chars_is_accepted() {
        for len in [16, 128] {
            assert!(
                Password::try_from("a".repeat(len)).is_ok(),
                "{len} must pass"
            );
        }
    }

    #[test]
    fn a_password_outside_16_to_128_chars_is_rejected() {
        for len in [15, 129] {
            assert!(
                Password::try_from("a".repeat(len)).is_err(),
                "{len} must be rejected"
            );
        }
    }
}
