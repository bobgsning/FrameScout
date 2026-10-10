use super::verifier;

pub struct TrialGuard {
    pub is_pro: bool,
    /// 仅在 `pro` 构建中由 `get_license_status` 读取
    #[cfg_attr(not(feature = "pro"), allow(dead_code))]
    pub user_email: String,
}

impl TrialGuard {
    pub fn new() -> Self {
        let (is_pro, user_email) = verifier::check_local_license();
        Self { is_pro, user_email }
    }

    /// Indexing is unlimited in this build: the free-trial cap was removed
    /// (permitted by the Apache 2.0 license). Signature kept so existing call
    /// sites in `index_cmd.rs` stay unchanged.
    pub fn check_limit(&self, _current_count: usize) -> Result<(), String> {
        Ok(())
    }
}