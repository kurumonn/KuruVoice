//! KuruVoice ダッシュボード GUI。設計書 Phase 3 / F-020 / F-016。
//!
//! 左ペイン: デバイス選択・起動/停止・シグナルチェーン表示
//! 中央    : 入出力メーター・プリセット・各 DSP パラメータのスライダー・設定保存/読込
//!
//! 「イケメン」イメージに合わせたダーク + シアン基調のテーマ。

use crate::audio::virtual_cable::{self, VirtualCable};
use crate::audio::{device, Engine, VirtualMic};
use crate::config::AppConfig;
use crate::dsp::meter::Meters;
use crate::preset::{PresetManager, VoicePreset};
use crate::voice_character::VoiceCharacter;
use eframe::egui;
use std::ops::RangeInclusive;
use std::sync::Arc;

/// GUI を起動する（ブロッキング）。
pub fn run(initial: AppConfig) -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 720.0])
            .with_min_inner_size([760.0, 560.0])
            .with_title("KuruVoice — イケメンボイスチェンジャー"),
        ..Default::default()
    };

    eframe::run_native(
        "KuruVoice",
        options,
        Box::new(move |cc| {
            install_japanese_font(&cc.egui_ctx);
            apply_theme(&cc.egui_ctx);
            Ok(Box::new(Dashboard::new(initial)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("GUI 起動失敗: {e}"))
}

struct Dashboard {
    config: AppConfig,
    input_devices: Vec<String>,
    output_devices: Vec<String>,
    selected_input: String,
    selected_output: String,
    cables: Vec<VirtualCable>,
    virtual_mic: Option<VirtualMic>,
    character: VoiceCharacter,
    engine: Option<Engine>,
    meters: Arc<Meters>,
    status: String,
    config_path: String,
}

impl Dashboard {
    fn new(config: AppConfig) -> Self {
        let input_devices = device::input_device_names();
        let output_devices = device::output_device_names();
        let selected_input = pick_device(&config.audio.input_device, &input_devices);
        let selected_output = pick_device(&config.audio.output_device, &output_devices);
        let cables = virtual_cable::detect_from(&output_devices);
        Self {
            config,
            input_devices,
            output_devices,
            selected_input,
            selected_output,
            cables,
            virtual_mic: None,
            character: VoiceCharacter::default(),
            engine: None,
            meters: Arc::new(Meters::default()),
            status: "停止中".to_string(),
            config_path: "kuruvoice_config.toml".to_string(),
        }
    }

    fn is_running(&self) -> bool {
        self.engine.is_some()
    }

    fn start_engine(&mut self) {
        let in_dev = device::find_input(&self.selected_input);
        let out_dev = device::find_output(&self.selected_output);
        let (Some(in_dev), Some(out_dev)) = (in_dev, out_dev) else {
            self.status = "デバイスが見つかりません".to_string();
            return;
        };
        self.config.audio.input_device = self.selected_input.clone();
        self.config.audio.output_device = self.selected_output.clone();
        match Engine::start(&in_dev, &out_dev, &self.config, self.meters.clone()) {
            Ok(engine) => {
                self.status = format!("実行中 ({} Hz)", engine.sample_rate);
                self.engine = Some(engine);
            }
            Err(e) => {
                self.status = format!("起動失敗: {e}");
            }
        }
    }

    fn stop_engine(&mut self) {
        if let Some(engine) = self.engine.take() {
            engine.stop();
        }
        self.status = "停止中".to_string();
    }

    fn push_config(&self) {
        if let Some(engine) = &self.engine {
            engine.update_config(self.config.clone());
        }
    }

    fn refresh_devices(&mut self) {
        self.input_devices = device::input_device_names();
        self.output_devices = device::output_device_names();
        self.selected_input = pick_device(&self.selected_input, &self.input_devices);
        self.selected_output = pick_device(&self.selected_output, &self.output_devices);
        self.cables = virtual_cable::detect_from(&self.output_devices);
    }

    fn apply_preset(&mut self, preset: VoicePreset) {
        PresetManager::apply(&mut self.config, preset);
        self.status = format!("プリセット: {}", preset.label());
        self.push_config();
    }
}

impl eframe::App for Dashboard {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // メーターを滑らかに更新するため再描画を要求。
        ctx.request_repaint_after(std::time::Duration::from_millis(33));

        let mut dirty = false;

        self.top_bar(ctx);
        self.left_panel(ctx, &mut dirty);

        egui::CentralPanel::default().show(ctx, |ui| {
            self.meters_section(ui);
            ui.add_space(8.0);
            self.preset_section(ui);
            ui.add_space(8.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.character_section(ui, &mut dirty);
                self.voice_section(ui, &mut dirty);
                self.denoise_section(ui, &mut dirty);
                self.auto_gain_section(ui, &mut dirty);
                self.noise_gate_section(ui, &mut dirty);
                self.eq_section(ui, &mut dirty);
                self.harmonic_section(ui, &mut dirty);
                self.compressor_section(ui, &mut dirty);
                self.limiter_section(ui, &mut dirty);
                ui.add_space(8.0);
                self.config_io_section(ui);
            });
        });

        if dirty {
            self.push_config();
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.stop_engine();
        self.virtual_mic = None; // Drop で仮想マイクを破棄
    }
}

// ---- UI セクション ----

impl Dashboard {
    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("🎙 KuruVoice").color(ACCENT).strong());
                ui.label(
                    egui::RichText::new("イケメンボイスチェンジャー")
                        .color(egui::Color32::from_gray(180)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // 起動 / 停止
                    if self.is_running() {
                        if ui
                            .add(egui::Button::new(egui::RichText::new("■ 停止").strong()))
                            .clicked()
                        {
                            self.stop_engine();
                        }
                    } else if ui
                        .add(
                            egui::Button::new(egui::RichText::new("▶ 開始").strong())
                                .fill(ACCENT_DARK),
                        )
                        .clicked()
                    {
                        self.start_engine();
                    }

                    // バイパス
                    let mut bypass = self.config.app.bypass;
                    if ui.toggle_value(&mut bypass, "バイパス (素通し)").changed() {
                        self.config.app.bypass = bypass;
                        self.push_config();
                    }

                    // ステータスランプ
                    let (col, txt) = if self.is_running() {
                        (egui::Color32::from_rgb(80, 220, 120), self.status.clone())
                    } else {
                        (egui::Color32::from_gray(120), self.status.clone())
                    };
                    ui.label(egui::RichText::new(format!("● {txt}")).color(col));
                });
            });
            ui.add_space(6.0);
        });
    }

    fn left_panel(&mut self, ctx: &egui::Context, _dirty: &mut bool) {
        egui::SidePanel::left("devices")
            .resizable(false)
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.label(egui::RichText::new("デバイス").color(ACCENT).strong());
                ui.separator();

                let running = self.is_running();
                ui.add_enabled_ui(!running, |ui| {
                    ui.label("入力 (マイク)");
                    egui::ComboBox::from_id_salt("in_dev")
                        .width(250.0)
                        .selected_text(short(&self.selected_input))
                        .show_ui(ui, |ui| {
                            for d in &self.input_devices {
                                ui.selectable_value(&mut self.selected_input, d.clone(), short(d));
                            }
                        });

                    ui.add_space(4.0);
                    ui.label("出力 (スピーカー / 仮想デバイス)");
                    egui::ComboBox::from_id_salt("out_dev")
                        .width(250.0)
                        .selected_text(short(&self.selected_output))
                        .show_ui(ui, |ui| {
                            for d in &self.output_devices {
                                ui.selectable_value(
                                    &mut self.selected_output,
                                    d.clone(),
                                    short(d),
                                );
                            }
                        });
                });

                ui.add_space(6.0);
                if ui.button("🔄 デバイス再取得").clicked() {
                    self.refresh_devices();
                }
                if running {
                    ui.label(
                        egui::RichText::new("※ 実行中はデバイス変更不可。停止してから変更。")
                            .small()
                            .color(egui::Color32::from_gray(150)),
                    );
                }

                ui.add_space(14.0);
                ui.label(egui::RichText::new("シグナルチェーン").color(ACCENT).strong());
                ui.separator();
                for (i, name) in [
                    "① DC カット",
                    "② ノイズゲート",
                    "③ ピッチシフト",
                    "④ フォルマント補正",
                    "⑤ EQ",
                    "⑥ コンプレッサー",
                    "⑦ リミッター",
                ]
                .iter()
                .enumerate()
                {
                    let _ = i;
                    ui.label(egui::RichText::new(format!("  {name}")).monospace());
                }

                ui.add_space(14.0);
                ui.label(egui::RichText::new("🎧 仮想マイク (配信連携)").color(ACCENT).strong());
                ui.separator();
                if self.cables.is_empty() {
                    ui.label(
                        egui::RichText::new(
                            "仮想ケーブル未検出。VB-CABLE / VoiceMeeter を導入すると、\nKuruVoice の声を OBS・Discord・VRChat のマイクとして使えます。",
                        )
                        .small(),
                    );
                    ui.label(
                        egui::RichText::new("VB-CABLE : vb-audio.com/Cable/")
                            .small()
                            .monospace(),
                    );
                } else {
                    for cable in self.cables.clone() {
                        let active = self.selected_output == cable.output_device;
                        let label = if active {
                            format!("✅ {} に出力中", cable.kind)
                        } else {
                            format!("▶ {} へ出力", cable.kind)
                        };
                        let mut btn = egui::Button::new(label);
                        if active {
                            btn = btn.fill(ACCENT_DARK);
                        }
                        if ui
                            .add_enabled(!running, btn)
                            .on_hover_text(cable.output_device.clone())
                            .clicked()
                        {
                            self.selected_output = cable.output_device.clone();
                        }
                        ui.label(
                            egui::RichText::new("受け側アプリのマイクで↓を選択:")
                                .small()
                                .color(egui::Color32::from_gray(170)),
                        );
                        ui.label(egui::RichText::new(&cable.mic_hint).monospace());
                        ui.add_space(6.0);
                    }
                }

                // Linux: ネイティブ仮想マイク（外部ソフト不要）。Windows では非表示。
                if cfg!(target_os = "linux") {
                    ui.add_space(8.0);
                    let exists = self.virtual_mic.is_some();
                    let label = if exists {
                        "🗑 KuruVoice 仮想マイクを削除"
                    } else {
                        "🎙 KuruVoice 仮想マイクを作成"
                    };
                    if ui.add_enabled(!running, egui::Button::new(label)).clicked() {
                        if exists {
                            self.virtual_mic = None;
                            self.status = "仮想マイクを削除しました".to_string();
                            self.refresh_devices();
                        } else {
                            match VirtualMic::create() {
                                Ok(vm) => {
                                    self.selected_output = vm.sink_name().to_string();
                                    self.status =
                                        format!("仮想マイク作成: {} へ出力", vm.sink_name());
                                    self.virtual_mic = Some(vm);
                                    self.refresh_devices();
                                }
                                Err(e) => self.status = format!("作成失敗: {e}"),
                            }
                        }
                    }
                    if exists {
                        ui.label(
                            egui::RichText::new("受け側アプリのマイクで↓を選択:")
                                .small()
                                .color(egui::Color32::from_gray(170)),
                        );
                        ui.label(
                            egui::RichText::new(crate::audio::virtual_mic::MIC_NAME).monospace(),
                        );
                    }
                }
            });
    }

    fn meters_section(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.label(egui::RichText::new("音量メーター").color(ACCENT).strong());
        ui.group(|ui| {
            meter_row(ui, "入力 ", self.meters.input());
            meter_row(ui, "出力 ", self.meters.output());
        });
    }

    fn preset_section(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("プリセット").color(ACCENT).strong());
        ui.horizontal_wrapped(|ui| {
            for preset in VoicePreset::all() {
                let selected = self.config.app.preset == preset.key();
                let mut btn = egui::Button::new(preset.label());
                if selected {
                    btn = btn.fill(ACCENT_DARK);
                }
                let resp = ui.add(btn).on_hover_text(preset.description());
                if resp.clicked() {
                    self.apply_preset(preset);
                }
            }
        });
    }

    fn character_section(&mut self, ui: &mut egui::Ui, dirty: &mut bool) {
        egui::CollapsingHeader::new(
            egui::RichText::new("🎨 声の印象（グラフでかんたん調整）").strong(),
        )
        .default_open(true)
        .show(ui, |ui| {
            let mut changed = false;
            ui.horizontal(|ui| {
                changed |= draw_radar(ui, &mut self.character);
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("グラフをドラッグ、または↓のスライダーで調整")
                            .small()
                            .color(egui::Color32::from_gray(170)),
                    );
                    changed |= char_slider(ui, "明瞭さ", &mut self.character.clarity);
                    changed |= char_slider(ui, "かわいさ", &mut self.character.cuteness);
                    changed |= char_slider(ui, "かっこよさ", &mut self.character.coolness);
                    changed |= char_slider(ui, "怖さ", &mut self.character.fear);
                    ui.label(
                        egui::RichText::new(
                            "※ プリセット/下の詳細スライダーとは独立。動かすと声づくり・EQ に反映されます。",
                        )
                        .small()
                        .color(egui::Color32::from_gray(140)),
                    );
                });
            });
            if changed {
                self.character.apply_to_config(&mut self.config);
                *dirty = true;
            }
        });
    }

    fn voice_section(&mut self, ui: &mut egui::Ui, dirty: &mut bool) {
        section(ui, "声づくり (ピッチ / フォルマント)", |ui| {
            slider(
                ui,
                dirty,
                "ピッチ (半音)",
                &mut self.config.voice.pitch_semitones,
                -24.0..=24.0,
                " st",
            );
            slider(
                ui,
                dirty,
                "フォルマント (声の太さ)",
                &mut self.config.voice.formant_shift,
                -12.0..=12.0,
                "",
            );
            slider(
                ui,
                dirty,
                "ミックス",
                &mut self.config.voice.mix,
                0.0..=1.0,
                "",
            );
        });
    }

    fn denoise_section(&mut self, ui: &mut egui::Ui, dirty: &mut bool) {
        section(
            ui,
            "ノイズキャンセル (背景ノイズ低減)",
            |ui| {
                *dirty |= ui
                    .checkbox(&mut self.config.denoise.enabled, "有効")
                    .changed();
                ui.add_enabled_ui(self.config.denoise.enabled, |ui| {
                    slider(
                        ui,
                        dirty,
                        "強度",
                        &mut self.config.denoise.amount,
                        0.0..=1.0,
                        "",
                    );
                    ui.label(
                        egui::RichText::new(
                            "ファン/ホワイトノイズ等を低減。強すぎると声が不自然に。",
                        )
                        .small()
                        .color(egui::Color32::from_gray(150)),
                    );
                });
            },
        );
    }

    fn auto_gain_section(&mut self, ui: &mut egui::Ui, dirty: &mut bool) {
        section(ui, "オートゲイン (音量自動調整)", |ui| {
            *dirty |= ui
                .checkbox(&mut self.config.auto_gain.enabled, "有効")
                .changed();
            ui.add_enabled_ui(self.config.auto_gain.enabled, |ui| {
                slider(
                    ui,
                    dirty,
                    "目標レベル",
                    &mut self.config.auto_gain.target_db,
                    -40.0..=-6.0,
                    " dB",
                );
                slider(
                    ui,
                    dirty,
                    "最大ブースト",
                    &mut self.config.auto_gain.max_gain_db,
                    0.0..=30.0,
                    " dB",
                );
                slider(
                    ui,
                    dirty,
                    "無音ゲート",
                    &mut self.config.auto_gain.gate_db,
                    -70.0..=-30.0,
                    " dB",
                );
                ui.label(
                    egui::RichText::new("小さい声/大きい声を自動で均す。無音は持ち上げない。")
                        .small()
                        .color(egui::Color32::from_gray(150)),
                );
            });
        });
    }

    fn noise_gate_section(&mut self, ui: &mut egui::Ui, dirty: &mut bool) {
        section(ui, "ノイズゲート", |ui| {
            *dirty |= ui
                .checkbox(&mut self.config.noise_gate.enabled, "有効")
                .changed();
            ui.add_enabled_ui(self.config.noise_gate.enabled, |ui| {
                slider(
                    ui,
                    dirty,
                    "しきい値",
                    &mut self.config.noise_gate.threshold_db,
                    -80.0..=0.0,
                    " dB",
                );
                slider(
                    ui,
                    dirty,
                    "アタック",
                    &mut self.config.noise_gate.attack_ms,
                    0.1..=50.0,
                    " ms",
                );
                slider(
                    ui,
                    dirty,
                    "リリース",
                    &mut self.config.noise_gate.release_ms,
                    5.0..=500.0,
                    " ms",
                );
            });
        });
    }

    fn eq_section(&mut self, ui: &mut egui::Ui, dirty: &mut bool) {
        section(ui, "EQ", |ui| {
            *dirty |= ui.checkbox(&mut self.config.eq.enabled, "有効").changed();
            ui.add_enabled_ui(self.config.eq.enabled, |ui| {
                slider(
                    ui,
                    dirty,
                    "ローカット",
                    &mut self.config.eq.high_pass_hz,
                    20.0..=300.0,
                    " Hz",
                );
                slider(
                    ui,
                    dirty,
                    "低音ブースト",
                    &mut self.config.eq.low_boost_db,
                    -6.0..=6.0,
                    " dB",
                );
                slider(
                    ui,
                    dirty,
                    "こもりカット",
                    &mut self.config.eq.mud_cut_db,
                    -12.0..=6.0,
                    " dB",
                );
                slider(
                    ui,
                    dirty,
                    "明瞭感",
                    &mut self.config.eq.presence_boost_db,
                    -6.0..=8.0,
                    " dB",
                );
                slider(
                    ui,
                    dirty,
                    "歯擦音抑制",
                    &mut self.config.eq.de_esser_db,
                    -12.0..=0.0,
                    " dB",
                );
            });
        });
    }

    fn harmonic_section(&mut self, ui: &mut egui::Ui, dirty: &mut bool) {
        section(ui, "ハーモニック (倍音で芯/艶を補強)", |ui| {
            *dirty |= ui
                .checkbox(&mut self.config.harmonic.enabled, "有効")
                .changed();
            ui.add_enabled_ui(self.config.harmonic.enabled, |ui| {
                slider(
                    ui,
                    dirty,
                    "効き",
                    &mut self.config.harmonic.amount,
                    0.0..=1.0,
                    "",
                );
                slider(
                    ui,
                    dirty,
                    "芯/太さ",
                    &mut self.config.harmonic.warmth,
                    0.0..=1.0,
                    "",
                );
                slider(
                    ui,
                    dirty,
                    "艶/明るさ",
                    &mut self.config.harmonic.brightness,
                    0.0..=1.0,
                    "",
                );
                ui.label(
                    egui::RichText::new(
                        "上げた声の「細い・芯がない」を倍音で補強。強すぎると歪み。",
                    )
                    .small()
                    .color(egui::Color32::from_gray(150)),
                );
            });
        });
    }

    fn compressor_section(&mut self, ui: &mut egui::Ui, dirty: &mut bool) {
        section(ui, "コンプレッサー", |ui| {
            *dirty |= ui
                .checkbox(&mut self.config.compressor.enabled, "有効")
                .changed();
            ui.add_enabled_ui(self.config.compressor.enabled, |ui| {
                slider(
                    ui,
                    dirty,
                    "しきい値",
                    &mut self.config.compressor.threshold_db,
                    -40.0..=0.0,
                    " dB",
                );
                slider(
                    ui,
                    dirty,
                    "レシオ",
                    &mut self.config.compressor.ratio,
                    1.0..=10.0,
                    " :1",
                );
                slider(
                    ui,
                    dirty,
                    "アタック",
                    &mut self.config.compressor.attack_ms,
                    0.1..=50.0,
                    " ms",
                );
                slider(
                    ui,
                    dirty,
                    "リリース",
                    &mut self.config.compressor.release_ms,
                    10.0..=500.0,
                    " ms",
                );
                slider(
                    ui,
                    dirty,
                    "メイクアップ",
                    &mut self.config.compressor.makeup_gain_db,
                    0.0..=12.0,
                    " dB",
                );
            });
        });
    }

    fn limiter_section(&mut self, ui: &mut egui::Ui, dirty: &mut bool) {
        section(ui, "リミッター (最終段・音割れ防止)", |ui| {
            *dirty |= ui
                .checkbox(&mut self.config.limiter.enabled, "有効")
                .changed();
            ui.add_enabled_ui(self.config.limiter.enabled, |ui| {
                slider(
                    ui,
                    dirty,
                    "天井",
                    &mut self.config.limiter.ceiling_db,
                    -12.0..=0.0,
                    " dB",
                );
                slider(
                    ui,
                    dirty,
                    "リリース",
                    &mut self.config.limiter.release_ms,
                    5.0..=300.0,
                    " ms",
                );
            });
        });
    }

    fn config_io_section(&mut self, ui: &mut egui::Ui) {
        section(ui, "設定ファイル (TOML)", |ui| {
            ui.horizontal(|ui| {
                ui.label("パス:");
                ui.add(egui::TextEdit::singleline(&mut self.config_path).desired_width(360.0));
            });
            ui.horizontal(|ui| {
                if ui.button("💾 保存").clicked() {
                    match self.config.save(&self.config_path) {
                        Ok(_) => self.status = format!("保存しました: {}", self.config_path),
                        Err(e) => self.status = format!("保存失敗: {e}"),
                    }
                }
                if ui.button("📂 読み込み").clicked() {
                    match AppConfig::load(&self.config_path) {
                        Ok(cfg) => {
                            self.config = cfg;
                            self.status = format!("読み込みました: {}", self.config_path);
                            self.push_config();
                        }
                        Err(e) => self.status = format!("読込失敗: {e}"),
                    }
                }
            });
        });
    }
}

// ---- 補助 UI ----

const ACCENT: egui::Color32 = egui::Color32::from_rgb(86, 204, 242); // シアン
const ACCENT_DARK: egui::Color32 = egui::Color32::from_rgb(28, 86, 110);

fn section<R>(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui) -> R) {
    egui::CollapsingHeader::new(egui::RichText::new(title).strong())
        .default_open(true)
        .show(ui, |ui| {
            add(ui);
        });
}

/// 「声の印象」軸のスライダー（0..1 を % 表示）。変更があれば true。
fn char_slider(ui: &mut egui::Ui, label: &str, value: &mut f32) -> bool {
    ui.horizontal(|ui| {
        ui.add_sized([84.0, 18.0], egui::Label::new(label));
        let mut pct = *value * 100.0;
        let resp = ui.add(egui::Slider::new(&mut pct, 0.0..=100.0).suffix("%"));
        if resp.changed() {
            *value = pct / 100.0;
        }
        resp.changed()
    })
    .inner
}

/// 「声の印象」レーダーチャート。ドラッグで最寄り軸の値を変更。変更があれば true。
fn draw_radar(ui: &mut egui::Ui, ch: &mut VoiceCharacter) -> bool {
    use std::f32::consts::{PI, TAU};
    let size = 200.0;
    let (resp, painter) =
        ui.allocate_painter(egui::vec2(size, size), egui::Sense::click_and_drag());
    let center = resp.rect.center();
    let radius = size * 0.36;
    let n = 4usize;
    let dir = |i: usize| -> egui::Vec2 {
        let a = -PI / 2.0 + i as f32 * TAU / n as f32;
        egui::vec2(a.cos(), a.sin())
    };
    let grid = egui::Color32::from_gray(80);
    let txt = egui::Color32::from_gray(205);

    // 同心リング
    for r in [0.25_f32, 0.5, 0.75, 1.0] {
        let pts: Vec<egui::Pos2> = (0..=n).map(|i| center + dir(i % n) * radius * r).collect();
        painter.add(egui::Shape::line(pts, egui::Stroke::new(1.0, grid)));
    }
    // 軸線 + ラベル
    for i in 0..n {
        painter.line_segment(
            [center, center + dir(i) * radius],
            egui::Stroke::new(1.0, grid),
        );
        painter.text(
            center + dir(i) * (radius + 18.0),
            egui::Align2::CENTER_CENTER,
            VoiceCharacter::AXES[i],
            egui::FontId::proportional(13.0),
            txt,
        );
    }
    // 現在値ポリゴン
    let vals = ch.values();
    let vpts: Vec<egui::Pos2> = (0..n)
        .map(|i| center + dir(i) * radius * vals[i].max(0.02))
        .collect();
    painter.add(egui::Shape::convex_polygon(
        vpts.clone(),
        egui::Color32::from_rgba_unmultiplied(86, 204, 242, 60),
        egui::Stroke::new(2.0, ACCENT),
    ));
    for p in &vpts {
        painter.circle_filled(*p, 4.0, ACCENT);
    }

    // ドラッグ操作: 最寄り軸の値を投影で更新
    let mut changed = false;
    if resp.dragged() || resp.clicked() {
        if let Some(ptr) = resp.interact_pointer_pos() {
            let rel = ptr - center;
            if rel.length() > 2.0 {
                let ang = rel.y.atan2(rel.x);
                let mut best = 0usize;
                let mut best_d = f32::MAX;
                for i in 0..n {
                    let axis_a = -PI / 2.0 + i as f32 * TAU / n as f32;
                    let mut d = (ang - axis_a).abs() % TAU;
                    if d > PI {
                        d = TAU - d;
                    }
                    if d < best_d {
                        best_d = d;
                        best = i;
                    }
                }
                let proj = rel.dot(dir(best)) / radius;
                ch.set(best, proj.clamp(0.0, 1.0));
                changed = true;
            }
        }
    }
    changed
}

fn slider(
    ui: &mut egui::Ui,
    dirty: &mut bool,
    label: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    suffix: &str,
) {
    ui.horizontal(|ui| {
        ui.add_sized([130.0, 18.0], egui::Label::new(label));
        let resp = ui.add(egui::Slider::new(value, range).suffix(suffix));
        *dirty |= resp.changed();
    });
}

/// 線形ピーク値を dB スケールのメーターバーとして描画する。
fn meter_row(ui: &mut egui::Ui, label: &str, level: f32) {
    let db = if level <= 1e-6 {
        -90.0
    } else {
        20.0 * level.log10()
    };
    let norm = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
    let color = if db > -3.0 {
        egui::Color32::from_rgb(235, 87, 87) // 赤（クリップ近辺）
    } else if db > -12.0 {
        egui::Color32::from_rgb(242, 201, 76) // 黄
    } else {
        egui::Color32::from_rgb(80, 220, 120) // 緑
    };
    ui.horizontal(|ui| {
        ui.add_sized([40.0, 18.0], egui::Label::new(label));
        let bar = egui::ProgressBar::new(norm)
            .desired_width(420.0)
            .fill(color)
            .text(format!("{db:5.1} dB"));
        ui.add(bar);
    });
}

/// "default" 指定や空文字を実デバイス名に解決する。
fn pick_device(want: &str, list: &[String]) -> String {
    if !want.is_empty() && !want.eq_ignore_ascii_case("default") && list.iter().any(|d| d == want) {
        want.to_string()
    } else {
        list.first()
            .cloned()
            .unwrap_or_else(|| "default".to_string())
    }
}

/// 長いデバイス名を短縮表示する。
fn short(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    if chars.len() > 34 {
        format!("{}…", chars[..33].iter().collect::<String>())
    } else {
        name.to_string()
    }
}

// ---- テーマ / フォント ----

fn apply_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(18, 22, 28);
    visuals.window_fill = egui::Color32::from_rgb(22, 27, 34);
    visuals.extreme_bg_color = egui::Color32::from_rgb(12, 15, 19);
    visuals.selection.bg_fill = ACCENT_DARK;
    visuals.hyperlink_color = ACCENT;
    visuals.widgets.hovered.bg_fill = ACCENT_DARK;
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.slider_width = 200.0;
    ctx.set_style(style);
}

/// Windows / 一般的な環境の日本語フォントを読み込む。
/// 見つからなければ既定フォント（日本語は豆腐表示）にフォールバック。
fn install_japanese_font(ctx: &egui::Context) {
    let candidates = [
        r"C:\Windows\Fonts\meiryo.ttc",
        r"C:\Windows\Fonts\YuGothR.ttc",
        r"C:\Windows\Fonts\YuGothM.ttc",
        r"C:\Windows\Fonts\msgothic.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
        "/Library/Fonts/Arial Unicode.ttf",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("jp".to_owned(), egui::FontData::from_owned(bytes));
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "jp".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("jp".to_owned());
            ctx.set_fonts(fonts);
            log::info!("日本語フォント読み込み: {path}");
            return;
        }
    }
    log::warn!("日本語フォントが見つかりませんでした。日本語が正しく表示されない場合があります。");
}
