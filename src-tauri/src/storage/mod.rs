//! 嵌入式 LSM 键值存储（[`sled`]，与 Google LevelDB 同属 LSM 家族；纯 Rust，无本机 C++ 依赖）。
//! 若必须使用 Google LevelDB C++，需在构建环境安装 `cmake`、`libleveldb-dev` 并改用 `leveldb` crate。
//!
//! 数据模型：按 **树（namespace）** 分表，键值为 **JSON**（`serde_json`），便于演进与调试。

pub mod migrate;

/// 数据库目录名（位于 `app_data_dir` 下）
pub const SLED_DB_DIR: &str = "wordwing.sled";

pub fn open_database(app_data_dir: &std::path::Path) -> Result<sled::Db, String> {
    let path = app_data_dir.join(SLED_DB_DIR);
    sled::open(&path).map_err(|e| format!("打开 sled 数据库 {:?}: {}", path, e))
}
