use serde::Serialize;

#[derive(Serialize)]
pub struct SmartFolder {
    pub id: i64,
    pub name: String,
    pub query_text: String,
    pub use_vector: bool,
    pub use_ocr: bool,
    pub use_note: bool,
    pub use_filename: bool,
    pub match_count: usize,
    /// 第七轮 M011：完整条件行的序列化（`ConditionRow[]` 的 JSON）。
    /// NULL = 旧文件夹，仍按上面的 7 个字段解释。
    /// 非空时，条件的求值在前端引擎完成（集合运算在前端），后端不猜。
    pub criteria_json: Option<String>,
    /// 供前端判断「这个文件夹必须由前端条件引擎求值」——
    /// 列表里的匹配数无法实时计算，UI 应显示 `?` 而不是假数字。
    pub needs_frontend_eval: bool,
}