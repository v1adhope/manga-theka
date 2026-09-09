use std::net::IpAddr;

use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    entity::{
        Email, LoginForm, Password, Session, SessionQuery, SessionTokens, ShortText, UserClaims,
        UserQuery,
    },
    error::{EntityError, ServiceError},
    service::Service,
};

impl Service {
    pub fn authenticate_access(&self, access_token: &str) -> Option<UserClaims> {
        let claims = self.jwt.verify_access(access_token).ok()?;

        Some(UserClaims {
            id: claims.sub,
            sid: claims.sid,
            roles: claims.roles,
        })
    }

    pub async fn login(&self, form: LoginForm) -> Result<SessionTokens, ServiceError> {
        let LoginForm {
            email,
            password,
            ua,
            ip,
            now,
        } = form;

        let user = self.authenticate(&email, password).await?;

        self.mint_session(&user, ua, ip, now).await
    }

    /// Runs verification even for an unknown email, so timing does not leak account existence.
    async fn authenticate(
        &self,
        email: &Email,
        password: Password,
    ) -> Result<UserQuery, ServiceError> {
        let user = self.database.get_user_by_email(email).await?;

        const PLACEHOLDER_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$\
            nE6GFRm4pmXbgWhIZf0QNg$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

        let hash = match user.as_ref() {
            Some(u) => u.password_hash.as_ref().to_owned(),
            None => PLACEHOLDER_HASH.to_owned(),
        };

        let hasher = self.hasher.clone();
        tokio::task::spawn_blocking(move || hasher.verify_password(password, &hash))
            .await
            .expect("password verification task panicked")?;

        user.ok_or(ServiceError::InvalidCredentials)
    }

    async fn mint_session(
        &self,
        user: &UserQuery,
        ua: Option<ShortText>,
        ip: Option<IpAddr>,
        now: OffsetDateTime,
    ) -> Result<SessionTokens, ServiceError> {
        let sid = Uuid::now_v7();
        let jti = Uuid::now_v7();

        let session = Session {
            sid,
            jti: self.hasher.compute_keyed_hex_hash(jti)?,
            ua,
            ip,
            created_at: now.into(),
            updated_at: now.into(),
        };
        self.memory.put_session(user.id, session).await?;

        let access = self
            .jwt
            .issue_access(user.id, sid, user.roles.as_slice(), now)?;
        let refresh = self.jwt.issue_refresh(user.id, sid, jti, now)?;

        Ok(SessionTokens { access, refresh })
    }

    pub async fn refresh_session(
        &self,
        refresh_token: &str,
        now: OffsetDateTime,
    ) -> Result<SessionTokens, ServiceError> {
        let claims = self.jwt.verify_refresh(refresh_token)?;

        let session = self
            .memory
            .get_session(claims.sub, claims.sid)
            .await?
            .ok_or(ServiceError::InvalidCredentials)?;

        if self.hasher.compute_keyed_hex_hash(claims.jti)? != session.jti {
            tracing::warn!(sub = %claims.sub, sid = %claims.sid, "refresh jti mismatch");
            return Err(ServiceError::InvalidCredentials);
        }

        let roles = self.database.get_user_roles(claims.sub).await?;

        let jti = Uuid::now_v7();
        let rotated = Session {
            sid: claims.sid,
            jti: self.hasher.compute_keyed_hex_hash(jti)?,
            ua: session.ua,
            ip: session.ip,
            created_at: session.created_at,
            updated_at: now.into(),
        };
        self.memory.put_session(claims.sub, rotated).await?;

        let access = self
            .jwt
            .issue_access(claims.sub, claims.sid, roles.as_slice(), now)?;
        let refresh = self.jwt.issue_refresh(claims.sub, claims.sid, jti, now)?;

        Ok(SessionTokens { access, refresh })
    }

    pub async fn list_sessions(&self, sub: Uuid) -> Result<Vec<SessionQuery>, ServiceError> {
        let sessions = self.memory.list_sessions(sub).await?;

        Ok(sessions.into_iter().map(SessionQuery::from).collect())
    }

    /// Exempt from the 24h rule -- a freshly logged-in user can always self-logout.
    pub async fn revoke_current_session(&self, sub: Uuid, sid: Uuid) -> Result<(), ServiceError> {
        self.memory
            .revoke_session(sub, sid)
            .await
            .map_err(Into::into)
    }

    pub async fn revoke_session(
        &self,
        sub: Uuid,
        current_sid: Uuid,
        target_sid: Uuid,
        now: OffsetDateTime,
    ) -> Result<(), ServiceError> {
        self.ensure_revoker(sub, current_sid, now).await?;

        if !self.memory.owns_session(sub, target_sid).await? {
            return Err(EntityError::not_readable::<Session>().into());
        }

        self.memory
            .revoke_session(sub, target_sid)
            .await
            .map_err(Into::into)
    }

    pub async fn revoke_all_sessions(
        &self,
        sub: Uuid,
        current_sid: Uuid,
        now: OffsetDateTime,
    ) -> Result<(), ServiceError> {
        self.ensure_revoker(sub, current_sid, now).await?;

        self.memory
            .revoke_all_sessions(sub)
            .await
            .map_err(Into::into)
    }

    async fn ensure_revoker(
        &self,
        sub: Uuid,
        current_sid: Uuid,
        now: OffsetDateTime,
    ) -> Result<(), ServiceError> {
        let current = self
            .memory
            .get_session(sub, current_sid)
            .await?
            .ok_or(ServiceError::InvalidCredentials)?;

        current.ensure_revoker(now)?;

        Ok(())
    }
}
