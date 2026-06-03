//! オーディオデバイス層。設計書 5.1 / F-003。
//!
//! OS ごとの差を cpal が吸収する。ここではデバイス列挙とデフォルト取得、
//! 名前からのデバイス検索を提供する。

use cpal::traits::{DeviceTrait, HostTrait};

/// デバイス情報。設計書 5.1.3。
#[derive(Debug, Clone)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_default: bool,
    pub channels: u16,
    pub sample_rates: Vec<u32>,
}

/// 入力デバイス名の一覧。
pub fn input_device_names() -> Vec<String> {
    let host = cpal::default_host();
    let default = default_input_name();
    let mut names = Vec::new();
    if let Ok(devices) = host.input_devices() {
        for d in devices {
            if let Ok(name) = d.name() {
                names.push(name);
            }
        }
    }
    // デフォルトを先頭に。
    if let Some(def) = default {
        names.retain(|n| n != &def);
        names.insert(0, def);
    }
    names
}

/// 出力デバイス名の一覧。
pub fn output_device_names() -> Vec<String> {
    let host = cpal::default_host();
    let default = default_output_name();
    let mut names = Vec::new();
    if let Ok(devices) = host.output_devices() {
        for d in devices {
            if let Ok(name) = d.name() {
                names.push(name);
            }
        }
    }
    if let Some(def) = default {
        names.retain(|n| n != &def);
        names.insert(0, def);
    }
    names
}

pub fn default_input_name() -> Option<String> {
    cpal::default_host()
        .default_input_device()
        .and_then(|d| d.name().ok())
}

pub fn default_output_name() -> Option<String> {
    cpal::default_host()
        .default_output_device()
        .and_then(|d| d.name().ok())
}

/// 名前から入力デバイスを探す。"default"/空文字はデフォルト入力。
pub fn find_input(name: &str) -> Option<cpal::Device> {
    let host = cpal::default_host();
    if name.is_empty() || name.eq_ignore_ascii_case("default") {
        return host.default_input_device();
    }
    if let Ok(mut devices) = host.input_devices() {
        if let Some(d) = devices.find(|d| d.name().map(|n| n == name).unwrap_or(false)) {
            return Some(d);
        }
    }
    // 見つからなければデフォルトにフォールバック（6.1 InputDeviceNotFound）。
    host.default_input_device()
}

/// 名前から出力デバイスを探す。"default"/空文字はデフォルト出力。
pub fn find_output(name: &str) -> Option<cpal::Device> {
    let host = cpal::default_host();
    if name.is_empty() || name.eq_ignore_ascii_case("default") {
        return host.default_output_device();
    }
    if let Ok(mut devices) = host.output_devices() {
        if let Some(d) = devices.find(|d| d.name().map(|n| n == name).unwrap_or(false)) {
            return Some(d);
        }
    }
    host.default_output_device()
}

/// 全デバイス情報を収集する（CLI の --list-devices 用）。
pub fn collect_info() -> (Vec<AudioDeviceInfo>, Vec<AudioDeviceInfo>) {
    let host = cpal::default_host();
    let def_in = default_input_name();
    let def_out = default_output_name();

    let mut inputs = Vec::new();
    if let Ok(devices) = host.input_devices() {
        for d in devices {
            if let Some(info) = device_info(&d, def_in.as_deref()) {
                inputs.push(info);
            }
        }
    }
    let mut outputs = Vec::new();
    if let Ok(devices) = host.output_devices() {
        for d in devices {
            if let Some(info) = device_info(&d, def_out.as_deref()) {
                outputs.push(info);
            }
        }
    }
    (inputs, outputs)
}

fn device_info(d: &cpal::Device, default_name: Option<&str>) -> Option<AudioDeviceInfo> {
    let name = d.name().ok()?;
    let is_default = default_name == Some(name.as_str());
    let (channels, sample_rates) = match d
        .default_output_config()
        .or_else(|_| d.default_input_config())
    {
        Ok(cfg) => (cfg.channels(), vec![cfg.sample_rate().0]),
        Err(_) => (0, vec![]),
    };
    Some(AudioDeviceInfo {
        name,
        is_default,
        channels,
        sample_rates,
    })
}
