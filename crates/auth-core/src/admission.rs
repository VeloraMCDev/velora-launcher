//! Admission uses current authority data after signature and expiry validation.
//! A JWT's cached name or role never grants access in place of a live account.

pub struct AccountStatus<'a> {
    pub status: &'a str,
    pub auth_version: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SessionDenied {
    Pending,
    DisabledOrRemoved,
}

impl SessionDenied {
    pub fn message(&self) -> &'static str {
        match self {
            Self::Pending => "your account is waiting for approval",
            Self::DisabledOrRemoved => "account disabled or removed",
        }
    }
}

/// Preserve API v1 admission precedence, including pending accounts with an old token.
/// Authority lookup errors must propagate; callers must not fabricate active status.
pub fn session_admission(token_version: i64, current: Option<AccountStatus<'_>>) -> Result<(), SessionDenied> {
    match current {
        Some(account) if account.status == "active" && account.auth_version == token_version => Ok(()),
        Some(account) if account.status == "pending" => Err(SessionDenied::Pending),
        _ => Err(SessionDenied::DisabledOrRemoved),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revocation_and_status_changes_take_effect_without_reissuing_tokens() {
        let current = |status, version| Some(AccountStatus { status, auth_version: version });
        assert_eq!(session_admission(3, current("active", 3)), Ok(()));
        assert_eq!(session_admission(3, current("active", 4)), Err(SessionDenied::DisabledOrRemoved));
        assert_eq!(session_admission(3, current("disabled", 3)), Err(SessionDenied::DisabledOrRemoved));
        assert_eq!(session_admission(3, current("unexpected", 3)), Err(SessionDenied::DisabledOrRemoved));
        assert_eq!(session_admission(3, None), Err(SessionDenied::DisabledOrRemoved));
        assert_eq!(session_admission(0, current("active", 0)), Ok(()));
    }

    #[test]
    fn pending_message_keeps_legacy_precedence_over_version_mismatch() {
        for version in [0, 3, 99] {
            assert_eq!(session_admission(version, Some(AccountStatus { status: "pending", auth_version: 3 })), Err(SessionDenied::Pending));
        }
        assert_eq!(SessionDenied::Pending.message(), "your account is waiting for approval");
        assert_eq!(SessionDenied::DisabledOrRemoved.message(), "account disabled or removed");
    }
}
