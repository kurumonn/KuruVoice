//! リアルタイム実行エンジン。設計書 4.1 / 5.1 / NF-001 / NF-004。
//!
//! アーキテクチャ:
//! ```text
//!   [入力 cpal callback] --push--> [入力リングバッファ] --pop--> [処理スレッド]
//!                                                                     |
//!                                              DSP Chain (block 単位) |
//!                                                                     v
//!   [出力 cpal callback] <--pop-- [出力リングバッファ] <--push-- [処理スレッド]
//! ```
//! cpal のコールバックはリングバッファの push/pop だけを行い、重い DSP は
//! 専用スレッドで実行する。パラメータ更新は mpsc チャネル、メーターは
//! ロックフリーな `Meters` で受け渡す。これにより RT コールバックでロックを
//! 取らず、アプリ全体が panic で落ちないようにする (NF-004)。
//!
//! T-003: 入出力とも SampleFormat を自動検出し f32↔デバイス形式を変換する。
//! T-004: 入出力サンプルレートが異なる場合は rubato でリアルタイムリサンプリング。
//! T-005: config.audio.buffer_size を BufferSize::Fixed に反映する。

use crate::config::AppConfig;
use crate::dsp::meter::{peak, Meters};
use crate::dsp::DspChain;
use crate::error::{KuruError, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::SampleFormat;
use ringbuf::HeapRb;
use rubato::{FftFixedIn, Resampler};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::Duration;

/// 処理スレッドへのパラメータ更新。
pub enum ParamUpdate {
    /// 設定を差分更新する（T-007: チェーンを再構築せず update_params を呼ぶ）。
    Config(Box<AppConfig>),
}

/// デフォルトのブロックサイズ（config で上書き可）。
const DEFAULT_BLOCK: usize = 256;

/// 起動中のオーディオエンジン。drop / stop で安全に停止する。
pub struct Engine {
    _in_stream: cpal::Stream,
    _out_stream: cpal::Stream,
    stop: Arc<AtomicBool>,
    proc_handle: Option<JoinHandle<()>>,
    tx: Sender<ParamUpdate>,
    pub sample_rate: u32,
}

// ---- T-003: SampleFormat 別ストリームビルダー ----

/// i16 サンプルを f32 に変換する（-1.0 〜 +1.0）。
#[inline]
fn i16_to_f32(v: i16) -> f32 {
    v as f32 / 32768.0
}

/// u16 サンプルを f32 に変換する（-1.0 〜 +1.0）。
#[inline]
fn u16_to_f32(v: u16) -> f32 {
    (v as f32 - 32768.0) / 32768.0
}

/// f32 を i16 に変換する。
#[inline]
fn f32_to_i16(v: f32) -> i16 {
    (v.clamp(-1.0, 1.0) * 32767.0) as i16
}

/// f32 を u16 に変換する。
#[inline]
fn f32_to_u16(v: f32) -> u16 {
    ((v.clamp(-1.0, 1.0) + 1.0) * 32767.5) as u16
}

/// 入力ストリームを SampleFormat に応じて構築する。T-003。
fn build_input_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    format: SampleFormat,
    mut prod: ringbuf::HeapProducer<f32>,
) -> std::result::Result<cpal::Stream, cpal::BuildStreamError> {
    let ch = channels.max(1);
    let err_fn = |e| log::error!("入力ストリームエラー: {e}");

    match format {
        SampleFormat::I16 => device.build_input_stream(
            config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                for frame in data.chunks(ch) {
                    let mono = frame.iter().map(|&s| i16_to_f32(s)).sum::<f32>() / ch as f32;
                    let _ = prod.push(mono);
                }
            },
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            config,
            move |data: &[u16], _: &cpal::InputCallbackInfo| {
                for frame in data.chunks(ch) {
                    let mono = frame.iter().map(|&s| u16_to_f32(s)).sum::<f32>() / ch as f32;
                    let _ = prod.push(mono);
                }
            },
            err_fn,
            None,
        ),
        _ => device.build_input_stream(
            config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                for frame in data.chunks(ch) {
                    let mono = frame.iter().sum::<f32>() / ch as f32;
                    let _ = prod.push(mono);
                }
            },
            err_fn,
            None,
        ),
    }
}

/// 出力ストリームを SampleFormat に応じて構築する。T-003。
fn build_output_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    format: SampleFormat,
    mut cons: ringbuf::HeapConsumer<f32>,
) -> std::result::Result<cpal::Stream, cpal::BuildStreamError> {
    let ch = channels.max(1);
    let err_fn = |e| log::error!("出力ストリームエラー: {e}");

    match format {
        SampleFormat::I16 => device.build_output_stream(
            config,
            move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(ch) {
                    let mono = cons.pop().unwrap_or(0.0);
                    for s in frame.iter_mut() {
                        *s = f32_to_i16(mono);
                    }
                }
            },
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_output_stream(
            config,
            move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(ch) {
                    let mono = cons.pop().unwrap_or(0.0);
                    for s in frame.iter_mut() {
                        *s = f32_to_u16(mono);
                    }
                }
            },
            err_fn,
            None,
        ),
        _ => device.build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(ch) {
                    let mono = cons.pop().unwrap_or(0.0);
                    for s in frame.iter_mut() {
                        *s = mono;
                    }
                }
            },
            err_fn,
            None,
        ),
    }
}

impl Engine {
    /// 入出力デバイスを開き、処理スレッドを起動する。
    pub fn start(
        in_device: &cpal::Device,
        out_device: &cpal::Device,
        config: &AppConfig,
        meters: Arc<Meters>,
    ) -> Result<Engine> {
        // ---- デバイス設定取得 ----
        let in_default = in_device
            .default_input_config()
            .map_err(|e| KuruError::DeviceError(format!("入力デフォルト設定取得失敗: {e}")))?;
        let in_channels = in_default.channels();
        let in_sample_rate = in_default.sample_rate();
        let in_format = in_default.sample_format();

        let out_default = out_device
            .default_output_config()
            .map_err(|e| KuruError::DeviceError(format!("出力デフォルト設定取得失敗: {e}")))?;
        let out_channels = out_default.channels();
        let out_sample_rate = out_default.sample_rate();
        let out_format = out_default.sample_format();

        if out_sample_rate.0 != in_sample_rate.0 {
            log::info!(
                "入力 {}Hz / 出力 {}Hz: rubato でリアルタイムリサンプリングします。",
                in_sample_rate.0,
                out_sample_rate.0
            );
        }

        // T-005: config の buffer_size を BufferSize::Fixed に反映。0 → Default。
        let buf_size = if config.audio.buffer_size > 0 {
            cpal::BufferSize::Fixed(config.audio.buffer_size as u32)
        } else {
            cpal::BufferSize::Default
        };

        let in_cfg = cpal::StreamConfig {
            channels: in_channels,
            sample_rate: in_sample_rate,
            buffer_size: buf_size,
        };
        let out_cfg = cpal::StreamConfig {
            channels: out_channels,
            sample_rate: out_sample_rate, // T-004: 出力デバイスのネイティブレートを使う
            buffer_size: buf_size,
        };

        let block_size = if config.audio.buffer_size > 0 {
            config.audio.buffer_size
        } else {
            DEFAULT_BLOCK
        };

        // リングバッファ（約 0.5 秒ぶん）。
        let cap = (in_sample_rate.0 as usize / 2).max(block_size * 8);
        let in_rb = HeapRb::<f32>::new(cap);
        let (in_prod, mut in_cons) = in_rb.split();
        let out_cap = (out_sample_rate.0 as usize / 2).max(block_size * 8);
        let out_rb = HeapRb::<f32>::new(out_cap);
        let (mut out_prod, out_cons) = out_rb.split();

        // T-003: SampleFormat 対応ストリーム構築
        let in_stream =
            build_input_stream(in_device, &in_cfg, in_channels as usize, in_format, in_prod)
                .map_err(|e| KuruError::StreamBuildError(format!("入力: {e}")))?;

        let out_stream = build_output_stream(
            out_device,
            &out_cfg,
            out_channels as usize,
            out_format,
            out_cons,
        )
        .map_err(|e| KuruError::StreamBuildError(format!("出力: {e}")))?;

        // ---- 処理スレッド ----
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx): (Sender<ParamUpdate>, Receiver<ParamUpdate>) = mpsc::channel();
        let mut chain = DspChain::from_config(config, in_sample_rate.0 as f32, block_size);
        let stop_thread = stop.clone();
        let sr_in = in_sample_rate.0;
        let sr_out = out_sample_rate.0;

        // T-004: サンプルレートが異なる場合に rubato リサンプラーを生成
        let mut resampler: Option<FftFixedIn<f32>> = if sr_in != sr_out {
            match FftFixedIn::<f32>::new(sr_in as usize, sr_out as usize, block_size, 2, 1) {
                Ok(r) => Some(r),
                Err(e) => {
                    log::error!("リサンプラー構築失敗: {e}. サンプルレート変換なしで続行します。");
                    None
                }
            }
        } else {
            None
        };

        let proc_handle = std::thread::Builder::new()
            .name("kuruvoice-dsp".to_string())
            .spawn(move || {
                let mut buf = vec![0.0f32; block_size];
                while !stop_thread.load(Ordering::Relaxed) {
                    // T-007: チェーン再構築ではなく差分更新
                    while let Ok(update) = rx.try_recv() {
                        match update {
                            ParamUpdate::Config(cfg) => {
                                chain.update_params(&cfg);
                            }
                        }
                    }

                    if in_cons.len() >= block_size {
                        for slot in buf.iter_mut() {
                            *slot = in_cons.pop().unwrap_or(0.0);
                        }
                        meters.set_input(peak(&buf));
                        chain.process(&mut buf);
                        meters.set_output(peak(&buf));

                        // T-004: 出力レートへリサンプリング（必要な場合のみ）
                        if let Some(ref mut rs) = resampler {
                            let waves_in = vec![buf.clone()];
                            match rs.process(&waves_in, None) {
                                Ok(waves_out) => {
                                    for &s in &waves_out[0] {
                                        let _ = out_prod.push(s);
                                    }
                                }
                                Err(e) => {
                                    log::warn!("リサンプリングエラー: {e}");
                                    for &s in buf.iter() {
                                        let _ = out_prod.push(s);
                                    }
                                }
                            }
                        } else {
                            for &s in buf.iter() {
                                let _ = out_prod.push(s);
                            }
                        }
                    } else {
                        std::thread::sleep(Duration::from_micros(500));
                    }
                }
            })
            .map_err(|e| KuruError::DeviceError(format!("処理スレッド起動失敗: {e}")))?;

        in_stream
            .play()
            .map_err(|e| KuruError::StreamBuildError(format!("入力 play: {e}")))?;
        out_stream
            .play()
            .map_err(|e| KuruError::StreamBuildError(format!("出力 play: {e}")))?;

        Ok(Engine {
            _in_stream: in_stream,
            _out_stream: out_stream,
            stop,
            proc_handle: Some(proc_handle),
            tx,
            sample_rate: sr_in,
        })
    }

    /// パラメータ（設定）を更新する。T-007: チェーン再構築せず差分更新。
    pub fn update_config(&self, cfg: AppConfig) {
        let _ = self.tx.send(ParamUpdate::Config(Box::new(cfg)));
    }

    /// 明示的に停止する。
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.proc_handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.shutdown();
    }
}
