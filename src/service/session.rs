use std::net::IpAddr;

use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    entity::{LoginForm, Session, SessionQuery, SessionTokens, Text, UserQuery},
    error::{EntityError, ServiceError},
    service::Service,
};

impl Service {
    pub async fn login(&self, form: LoginForm) -> Result<SessionTokens, ServiceError> {
        let LoginForm {
            email,
            password,
            ua,
            ip,
            now,
        } = form;

        let user = self.database.get_user_by_email(&email).await?;

        // Off the reactor. The no-such-user branch runs a dummy verify so timing
        // does not reveal whether the address has an account.
        let hasher = self.hasher.clone();
        let stored = user.as_ref().map(|u| u.password_hash.as_ref().to_owned());
        let verified = tokio::task::spawn_blocking(move || match stored {
            Some(hash) => hasher.verify_password(password.expose_secret(), &hash),
            None => {
                hasher.verify_dummy(password.expose_secret());
                Ok(false)
            }
        })
        .await
        .expect("password verification task panicked")?;

        let user = user.ok_or(ServiceError::InvalidCredentials)?;
        if !verified {
            return Err(ServiceError::InvalidCredentials);
        }

        self.mint_session(&user, ua, ip, now).await
    }

    async fn mint_session(
        &self,
        user: &UserQuery,
        ua: Option<Text>,
        ip: Option<IpAddr>,
        now: OffsetDateTime,
    ) -> Result<SessionTokens, ServiceError> {
        let sid = Uuid::now_v7();
        let jti = Uuid::now_v7();

        let session = Session {
            sid,
            jti: self.hasher.keyed_jti_hash(jti)?,
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

        if self.hasher.keyed_jti_hash(claims.jti)? != session.jti {
            // ADR-0003 rejects reuse-detection escalation: warn, no family revoke.
            tracing::warn!(sub = %claims.sub, sid = %claims.sid, "refresh jti mismatch");
            return Err(ServiceError::InvalidCredentials);
        }

        // No compare-and-swap on the jti, so two refreshes racing on one cookie
        // both pass and the loser re-logs-in -- accepted per ADR-0003.
        let user = self.database.get_user(claims.sub).await?;

        let jti = Uuid::now_v7();
        let rotated = Session {
            sid: claims.sid,
            jti: self.hasher.keyed_jti_hash(jti)?,
            ua: session.ua,
            ip: session.ip,
            created_at: session.created_at,
            updated_at: now.into(),
        };
        self.memory.put_session(claims.sub, rotated).await?;

        let access = self
            .jwt
            .issue_access(claims.sub, claims.sid, user.roles.as_slice(), now)?;
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
