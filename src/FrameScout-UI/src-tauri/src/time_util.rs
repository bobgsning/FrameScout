// 轻量时间工具：不引入 chrono，只提供 UTC 时间戳字符串用于备份文件名与批次号。

use std::time::{SystemTime, UNIX_EPOCH};

/// 当前 UTC 时间，格式 `YYYYMMDD_HHMMSS`。
/// 用于迁移备份文件名（`pre_migrate_v1_20261002_103747.db`）与扫描批次号
/// （`scan_20261002_103747`）。统一用 UTC 避免时区设置带来的排序歧义。
pub fn utc_stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, m, d) = civil_from_days((secs / 86400) as i64);
    let hms = secs % 86400;
    format!(
        "{:04}{:02}{:02}_{:02}{:02}{:02}",
        y,
        m,
        d,
        hms / 3600,
        (hms % 3600) / 60,
        hms % 60
    )
}

/// 当前 UTC 时间的 ISO-8601 形式 `YYYY-MM-DDTHH:MM:SSZ`。
/// 用于 `file_events.occurred_at` / `batches.started_at` / `finished_at` 等
/// 「给人看、也要能排序」的时间列（字典序即时间序）。
pub fn utc_now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, m, d) = civil_from_days((secs / 86400) as i64);
    let hms = secs % 86400;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        hms / 3600,
        (hms % 3600) / 60,
        hms % 60
    )
}

/// 天数（自 1970-01-01）→ 年月日（Howard Hinnant 的 civil_from_days 算法，公历）。
pub fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as i64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::civil_from_days;

    #[test]
    fn known_dates_are_correct() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(18_628), (2021, 1, 1));
        assert_eq!(civil_from_days(18_762), (2021, 5, 15));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }
}
