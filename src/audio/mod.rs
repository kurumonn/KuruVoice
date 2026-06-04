//! オーディオデバイス層と実行エンジン。設計書 5.1 / 4.1。

pub mod device;
pub mod engine;
pub mod virtual_cable;
pub mod virtual_mic;

pub use device::AudioDeviceInfo;
pub use engine::{Engine, ParamUpdate};
pub use virtual_cable::VirtualCable;
pub use virtual_mic::VirtualMic;
