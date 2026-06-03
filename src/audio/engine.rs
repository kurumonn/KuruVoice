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

use crate::config::AppConfig;
use crate::dsp::meter::{peak, Meters};
use crate::dsp::DspChain;
use crate::error::{KuruError, Result};
use cpal::traits::{DeviceTrait, StreamTrait};
use ringbuf::HeapRb;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;
use std::time::Duration;

/// 処理スレッドへのパラメータ更新。
pub enum ParamUpdate {
    /// 設定全体を差し替える（チェーンを再構築）。
    Config(Box<AppConfig>),
}

const BLOCK: usize = 256;

/// 起動中のオーディオエンジン。drop / stop で安全に停止する。
pub struct Engine {
    _in_stream: cpal::Stream,
    _out_stream: cpal::Stream,
    stop: Arc<AtomicBool>,
    proc_handle: Option<JoinHandle<()>>,
    tx: Sender<ParamUpdate>,
    pub sample_rate: u32,
}

impl Engine {
    /// 入出力デバイスを開き、処理スレッドを起動する。
    pub fn start(
        in_device: &cpal::Device,
        out_device: &cpal::Device,
        config: &AppConfig,
        meters: Arc<Meters>,
    ) -> Result<Engine> {
        // --- 入力設定 ---
        let in_default = in_device
            .default_input_config()
            .map_err(|e| KuruError::DeviceError(format!("入力デフォルト設定取得失敗: {e}")))?;
        let in_channels = in_default.channels();
        let sample_rate = in_default.sample_rate();

        // --- 出力設定 ---
        let out_default = out_device
            .default_output_config()
            .map_err(|e| KuruError::DeviceError(format!("出力デフォルト設定取得失敗: {e}")))?;
        let out_channels = out_default.channels();

        if out_default.sample_rate().0 != sample_rate.0 {
            log::warn!(
                "入力 {}Hz と出力 {}Hz のサンプルレートが異なります。音程ずれが起きる場合があります。",
                sample_rate.0,
                out_default.sample_rate().0
            );
        }

        let in_cfg = cpal::StreamConfig {
            channels: in_channels,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };
        let out_cfg = cpal::StreamConfig {
            channels: out_channels,
            // 出力もこのレートを要求（多くの環境で 48000 一致）。
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        // リングバッファ（約 0.5 秒ぶん）。
        let cap = (sample_rate.0 as usize / 2).max(BLOCK * 8);
        let in_rb = HeapRb::<f32>::new(cap);
        let (mut in_prod, mut in_cons) = in_rb.split();
        let out_rb = HeapRb::<f32>::new(cap);
        let (mut out_prod, mut out_cons) = out_rb.split();

        // --- 入力ストリーム: チャンネルをモノラルに平均して push ---
        let in_err = |e| log::error!("入力ストリームエラー: {e}");
        let in_stream = in_device
            .build_input_stream(
                &in_cfg,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let ch = in_channels as usize;
                    if ch == 0 {
                        return;
                    }
                    for frame in data.chunks(ch) {
                        let mono = frame.iter().sum::<f32>() / ch as f32;
                        let _ = in_prod.push(mono); // 溢れたら破棄 (BufferOverrun 対応)
                    }
                },
                in_err,
                None,
            )
            .map_err(|e| KuruError::StreamBuildError(format!("入力: {e}")))?;

        // --- 出力ストリーム: モノラルを全チャンネルへ複製 ---
        let out_err = |e| log::error!("出力ストリームエラー: {e}");
        let out_stream = out_device
            .build_output_stream(
                &out_cfg,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let ch = out_channels as usize;
                    if ch == 0 {
                        return;
                    }
                    for frame in data.chunks_mut(ch) {
                        let mono = out_cons.pop().unwrap_or(0.0); // 足りなければ無音 (Underrun)
                        for s in frame.iter_mut() {
                            *s = mono;
                        }
                    }
                },
                out_err,
                None,
            )
            .map_err(|e| KuruError::StreamBuildError(format!("出力: {e}")))?;

        // --- 処理スレッド ---
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx): (Sender<ParamUpdate>, Receiver<ParamUpdate>) = mpsc::channel();
        let mut chain = DspChain::from_config(config, sample_rate.0 as f32, BLOCK);
        let stop_thread = stop.clone();
        let sr = sample_rate.0 as f32;

        let proc_handle = std::thread::Builder::new()
            .name("kuruvoice-dsp".to_string())
            .spawn(move || {
                let mut buf = vec![0.0f32; BLOCK];
                while !stop_thread.load(Ordering::Relaxed) {
                    // パラメータ更新を反映
                    while let Ok(update) = rx.try_recv() {
                        match update {
                            ParamUpdate::Config(cfg) => {
                                chain = DspChain::from_config(&cfg, sr, BLOCK);
                            }
                        }
                    }

                    if in_cons.len() >= BLOCK {
                        for slot in buf.iter_mut() {
                            *slot = in_cons.pop().unwrap_or(0.0);
                        }
                        meters.set_input(peak(&buf));
                        chain.process(&mut buf);
                        meters.set_output(peak(&buf));
                        for &s in buf.iter() {
                            let _ = out_prod.push(s);
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
            sample_rate: sample_rate.0,
        })
    }

    /// パラメータ（設定）を更新する。
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
