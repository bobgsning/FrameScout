use crate::constants::FREE_TRIAL_LIMIT;
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

    pub fn check_limit(&self, current_count: usize) -> Result<(), String> {
        if self.is_pro {
            return Ok(());
        }
        if current_count >= FREE_TRIAL_LIMIT {
            return Err(format!(
                "You've hit the free trial cap of {} images. 🚀 Upgrade to Pro and index without limits!",
                FREE_TRIAL_LIMIT
            ));
        }
        Ok(())
    }
}