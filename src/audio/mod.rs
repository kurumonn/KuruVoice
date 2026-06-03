//! オーディオデバイス層と実行エンジン。設計書 5.1 / 4.1。

pub mod device;
pub mod engine;

pub use device::AudioDeviceInfo;
pub use engine::{Engine, ParamUpdate};
