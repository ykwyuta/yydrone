//! ドローン戦線 — シミュレーション本体
//!
//! 描画にも OS にも依存しないので、Windows 版にはそのまま、ブラウザ版には
//! WebAssembly にコンパイルして組み込む。乱数の種が同じなら同じ展開になる。

pub mod ai;
pub mod defs;
pub mod geo;
pub mod history;
pub mod path;
pub mod rng;
pub mod world;

#[cfg(test)]
mod tests;
