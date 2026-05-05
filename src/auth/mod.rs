use crate::config::{AuthMode, UserRole};
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub username: String,
    pub role: UserRole,
}

#[derive(Debug, Clone, Copy)]
pub enum Permission {
    ViewReports,
    SubmitFeedback,
    ReadAudit,
    Administer,
}

pub fn current_user(config: &crate::config::AppConfig) -> AppResult<AuthContext> {
    if !config.auth.enabled || matches!(config.auth.mode, AuthMode::Disabled) {
        let username = std::env::var("GITWHISPER_USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "local-user".to_string());
        return Ok(AuthContext {
            username,
            role: UserRole::Admin,
        });
    }

    let username = std::env::var("GITWHISPER_USER")
        .or_else(|_| std::env::var("USERNAME"))
        .map_err(|_| {
            AppError::message("Authentication is enabled but no user identity was found.")
        })?;

    if let Some(user) = config
        .auth
        .users
        .iter()
        .find(|user| user.username.eq_ignore_ascii_case(&username))
    {
        return Ok(AuthContext {
            username,
            role: user.role,
        });
    }

    Ok(AuthContext {
        username,
        role: config.auth.default_role,
    })
}

pub fn ensure_permission(
    config: &crate::config::AppConfig,
    permission: Permission,
) -> AppResult<AuthContext> {
    let user = current_user(config)?;
    if has_permission(user.role, permission) {
        Ok(user)
    } else {
        Err(AppError::message(format!(
            "User `{}` does not have permission for this action.",
            user.username
        )))
    }
}

fn has_permission(role: UserRole, permission: Permission) -> bool {
    match role {
        UserRole::Admin => true,
        UserRole::Contributor => matches!(
            permission,
            Permission::ViewReports | Permission::SubmitFeedback
        ),
        UserRole::Viewer => matches!(permission, Permission::ViewReports),
    }
}
