// =========================================================================
//  Windows 路径规范化（v3.2.0 工程纪律）
// =========================================================================
//  为什么必须有这一层：
//   SQLite 的 TEXT 默认按 BINARY 排序（区分大小写），而 Windows 文件系统对路径
//   大小写不敏感。同一文件若以不同大小写形态（或 `/` 与 `\` 混用、尾部多一个
//   分隔符）进入系统，会被视为「一条新增 + 一条消失」，实为同一文件。
//   因此**所有写入 files.path / frame_vectors.path 的路径，都必须先过这里**。
//
//  规范化规则（顺序固定）：
//   1. 去掉首尾空白；
//   2. `/` 统一为 `\`；
//   3. 去掉 extended-length 前缀 `\\?\` / `\\.\`（它会绕开后续所有规范化）；
//   4. 折叠重复分隔符；
//   5. 盘符字母统一大写（`c:\` → `C:\`）；
//   6. 折叠 `.` 与 `..` 路径段；
//   7. 去掉尾部多余分隔符（保留根目录形态 `C:\`）。
//
//  ⚠️ 已知边界：本函数**不做全路径大小写折叠**。
//     折叠整条路径能彻底消灭大小写重复，但会改变 UI 上呈现给用户的文件名形态。
//     是否接受「库内一律存小写化路径、显示层另存原始形态」，待定。
//     在此之前，`C:\Users\a.jpg` 与 `C:\users\A.jpg` 仍会被视为两条记录。

/// 把任意来源的路径字符串规范化为统一形态。空输入返回空串。
pub fn normalize_path(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut s = trimmed.replace('/', "\\");
    s = strip_extended_prefix(&s);
    s = collapse_separators(&s);
    s = uppercase_drive_letter(&s);
    s = resolve_dot_segments(&s);
    strip_trailing_separator(&s)
}

/// P1-15 / D2 修复：全路径大小写折叠键。
///
/// `normalize_path` 保留原始大小写用于显示，但 NTFS 大小写不敏感、SQLite TEXT 用 BINARY 比较，
/// 同一文件可能以不同大小写形态存两条记录。此函数返回 `normalize_path(path).to_lowercase()`，
/// 供 `contains_path` / 去重 / 以图搜图排除自身等比较场景使用，不动存储形态。
pub fn path_key(raw: &str) -> String {
    normalize_path(raw).to_lowercase()
}

/// 去掉 `\\?\` / `\\.\` 前缀，使后续规则能在普通路径语义下工作。
fn strip_extended_prefix(s: &str) -> String {
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        rest.to_string()
    } else if let Some(rest) = s.strip_prefix(r"\\.\") {
        rest.to_string()
    } else {
        s.to_string()
    }
}

/// 折叠连续分隔符（`a\\\\b` → `a\b`），但保留 UNC 开头的双分隔符。
fn collapse_separators(s: &str) -> String {
    let is_unc = s.starts_with("\\\\");
    let mut out = String::with_capacity(s.len());
    let mut prev_sep = false;
    for (i, ch) in s.char_indices() {
        if ch == '\\' {
            // UNC 的前两个分隔符必须保留
            if prev_sep && !(is_unc && i == 1) {
                continue;
            }
            prev_sep = true;
        } else {
            prev_sep = false;
        }
        out.push(ch);
    }
    out
}

/// 盘符字母大写化（`c:\x` → `C:\x`）。UNC 与相对路径原样返回。
fn uppercase_drive_letter(s: &str) -> String {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        let mut out = s.to_string();
        out.replace_range(0..1, &(bytes[0] as char).to_ascii_uppercase().to_string());
        out
    } else {
        s.to_string()
    }
}

/// 折叠 `.` 与 `..` 段。分别处理三种前缀：UNC(`\\server\share`)、盘符(`C:\`)、相对路径。
fn resolve_dot_segments(s: &str) -> String {
    let (prefix, rest): (String, Vec<&str>) = if s.starts_with("\\\\") {
        // UNC：前两段是 server 与 share，不可被 `..` 吃掉
        let mut parts: Vec<&str> = s.split('\\').filter(|p| !p.is_empty()).collect();
        if parts.len() >= 2 {
            let server = parts.remove(0);
            let share = parts.remove(0);
            (format!("\\\\{}\\{}", server, share), parts)
        } else {
            (s.to_string(), Vec::new())
        }
    } else if let Some(idx) = s.find(":\\") {
        let drive = s[..idx + 2].to_string();
        let rest: Vec<&str> = s[idx + 2..].split('\\').filter(|p| !p.is_empty()).collect();
        (drive, rest)
    } else {
        (String::new(), s.split('\\').filter(|p| !p.is_empty()).collect())
    };

    let mut out: Vec<&str> = Vec::with_capacity(rest.len());
    for seg in rest {
        match seg {
            "." => {}
            ".." => {
                out.pop();
            }
            other => out.push(other),
        }
    }

    let joined = out.join("\\");
    if prefix.is_empty() {
        joined
    } else if prefix.ends_with('\\') {
        format!("{}{}", prefix, joined)
    } else {
        format!("{}\\{}", prefix, joined)
    }
}

/// 去掉尾部分隔符，但保留根目录形态（`C:\` 长度为 3）。
fn strip_trailing_separator(s: &str) -> String {
    if s.len() > 3 && s.ends_with('\\') {
        s[..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_path;

    #[test]
    fn normalizes_separators_and_drive_letter() {
        assert_eq!(normalize_path("c:/Users/bob/a.jpg"), "C:\\Users\\bob\\a.jpg");
        assert_eq!(normalize_path("  C:\\Users\\bob\\a.jpg  "), "C:\\Users\\bob\\a.jpg");
    }

    #[test]
    fn strips_extended_prefix_and_trailing_separator() {
        assert_eq!(normalize_path("\\\\?\\C:\\Users\\a.jpg"), "C:\\Users\\a.jpg");
        assert_eq!(normalize_path("C:\\Users\\bob\\"), "C:\\Users\\bob");
        assert_eq!(normalize_path("C:\\"), "C:\\");
    }

    #[test]
    fn collapses_duplicated_separators() {
        assert_eq!(normalize_path("C:\\\\Users\\\\\\bob\\\\a.jpg"), "C:\\Users\\bob\\a.jpg");
    }

    #[test]
    fn resolves_dot_segments() {
        assert_eq!(normalize_path("C:\\Users\\bob\\.\\a.jpg"), "C:\\Users\\bob\\a.jpg");
        assert_eq!(normalize_path("C:\\Users\\bob\\..\\ann\\a.jpg"), "C:\\Users\\ann\\a.jpg");
    }

    #[test]
    fn keeps_unc_prefix_intact() {
        assert_eq!(
            normalize_path("\\\\server\\share\\..\\x\\a.jpg"),
            "\\\\server\\share\\x\\a.jpg"
        );
    }

    #[test]
    fn empty_input_stays_empty() {
        assert_eq!(normalize_path(""), "");
        assert_eq!(normalize_path("   "), "");
    }
}
