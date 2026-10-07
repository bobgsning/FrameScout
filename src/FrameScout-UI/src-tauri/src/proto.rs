// =============================================================
//  Protobuf Code Generation Inclusion
// =============================================================
//  生成代码里包含大量为将来准备的消息与字段（如 BGE-M3 文本通道的
//  TextEmbedding / TextEntry / SearchResponse），当前版本尚未使用它们，
//  因此整个模块放开 dead_code —— 生成代码不该由「是否被引用」来评判。

pub mod framescout {
    #![allow(dead_code)]

    include!(concat!(env!("OUT_DIR"), "/framescout.rs"));
}
