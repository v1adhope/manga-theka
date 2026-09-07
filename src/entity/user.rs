use std::str::FromStr;
use std::sync::LazyLock;

use regex::Regex;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    entity::{Email, Entity},
    error::EntityError,
};

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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
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

#[derive(Debug)]
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

impl From<PasswordHash> for String {
    fn from(hash: PasswordHash) -> Self {
        hash.0
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Uuid,
    pub email: Email,
    pub username: Username,
    #[serde(skip_serializing)]
    pub password_hash: PasswordHash,
    pub roles: Vec<Role>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub verified_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl Entity for User {
    const NAME: &'static str = "User";
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
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::entity::{Password, Role, UserClaims, Username};

    fn claims(roles: &[Role]) -> UserClaims {
        UserClaims {
            id: Uuid::now_v7(),
            sid: Uuid::now_v7(),
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
    fn only_moderators_and_admins_moderate() {
        assert!(claims(&[Role::Moderator]).can_moderate());
        assert!(claims(&[Role::Admin]).can_moderate());
        assert!(!claims(&[Role::Reader, Role::Uploader]).can_moderate());
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
