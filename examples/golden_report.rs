//! Golden audio regression harness.
//!
//! It can generate deterministic fixture WAVs, process every preset offline, emit
//! JSON metrics, and compare them with a saved baseline.
//!
//!   cargo run --release --example golden_report -- --generate-fixtures --write-baseline docs/golden_baseline.json
//!   cargo run --release --example golden_report -- --baseline docs/golden_baseline.json

use kuruvoice::config::AppConfig;
use kuruvoice::dsp::DspChain;
use kuruvoice::eval::metrics::{analyze_audio, AudioMetrics};
use kuruvoice::preset::{PresetManager, VoicePreset};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f32::consts::TAU;
use std::path::{Path, PathBuf};

const SAMPLE_RATE: u32 = 48_000;
const BLOCK: usize = 256;
const FIXTURE_SECONDS: f32 = 2.0;

#[derive(Debug, Clone)]
struct Args {
    fixtures_dir: PathBuf,
    out_path: PathBuf,
    baseline_path: Option<PathBuf>,
    write_baseline_path: Option<PathBuf>,
    generate_fixtures: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoldenReport {
    schema: u32,
    sample_rate: u32,
    block_size: usize,
    cases: Vec<GoldenCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoldenCase {
    fixture: String,
    preset: String,
    input: MetricsSnapshot,
    output: MetricsSnapshot,
    delta_rms_db: f32,
    delta_noise_floor_db: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct MetricsSnapshot {
    peak: f32,
    rms_db: f32,
    clip_rate: f32,
    noise_floor_db: f32,
}

impl From<AudioMetrics> for MetricsSnapshot {
    fn from(value: AudioMetrics) -> Self {
        Self {
            peak: round4(value.peak),
            rms_db: round4(value.rms_db),
            clip_rate: round6(value.clip_rate),
            noise_floor_db: round4(value.noise_floor_db),
        }
    }
}

#[derive(Debug)]
struct Regression {
    key: String,
    field: &'static str,
    expected: f32,
    actual: f32,
    tolerance: f32,
}

fn main() -> anyhow::Result<()> {
    let args = parse_args()?;
    if args.generate_fixtures || !has_wav_fixtures(&args.fixtures_dir)? {
        generate_fixtures(&args.fixtures_dir)?;
    }

    let report = build_report(&args.fixtures_dir)?;
    write_json(&args.out_path, &report)?;

    if let Some(path) = &args.write_baseline_path {
        write_json(path, &report)?;
        println!("baseline written: {}", path.display());
    }

    if let Some(path) = &args.baseline_path {
        let baseline: GoldenReport = serde_json::from_str(&std::fs::read_to_string(path)?)?;
        let regressions = compare_reports(&baseline, &report);
        if !regressions.is_empty() {
            eprintln!("golden regression detected:");
            for r in &regressions {
                eprintln!(
                    "- {} {} expected {:.4}, actual {:.4}, tolerance {:.4}",
                    r.key, r.field, r.expected, r.actual, r.tolerance
                );
            }
            anyhow::bail!("{} metric regression(s)", regressions.len());
        }
        println!("baseline comparison passed: {}", path.display());
    }

    // T-013: 絶対閾値チェック（ベースライン比較なしでも CI で失敗させる）。
    let threshold_failures = check_absolute_thresholds(&report);
    if !threshold_failures.is_empty() {
        eprintln!("absolute threshold violation:");
        for msg in &threshold_failures {
            eprintln!("  {msg}");
        }
        anyhow::bail!("{} threshold violation(s)", threshold_failures.len());
    }

    print_summary(&report);
    println!("report written: {}", args.out_path.display());
    Ok(())
}

fn parse_args() -> anyhow::Result<Args> {
    let mut fixtures_dir = PathBuf::from("tests/audio");
    let mut out_path = PathBuf::from("target/golden_report.json");
    let mut baseline_path = None;
    let mut write_baseline_path = None;
    let mut generate_fixtures = false;

    let mut iter = std::env::args().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--fixtures" => fixtures_dir = PathBuf::from(require_value(&arg, iter.next())?),
            "--out" => out_path = PathBuf::from(require_value(&arg, iter.next())?),
            "--baseline" => baseline_path = Some(PathBuf::from(require_value(&arg, iter.next())?)),
            "--write-baseline" => {
                write_baseline_path = Some(PathBuf::from(require_value(&arg, iter.next())?));
            }
            "--generate-fixtures" => generate_fixtures = true,
            "-h" | "--help" => {
                println!(
                    "Usage: golden_report [--fixtures DIR] [--out PATH] [--baseline PATH] [--write-baseline PATH] [--generate-fixtures]"
                );
                std::process::exit(0);
            }
            other => anyhow::bail!("unknown argument: {other}"),
        }
    }

    Ok(Args {
        fixtures_dir,
        out_path,
        baseline_path,
        write_baseline_path,
        generate_fixtures,
    })
}

fn require_value(flag: &str, value: Option<String>) -> anyhow::Result<String> {
    value.ok_or_else(|| anyhow::anyhow!("{flag} requires a value"))
}

fn build_report(fixtures_dir: &Path) -> anyhow::Result<GoldenReport> {
    let fixtures = fixture_paths(fixtures_dir)?;
    if fixtures.is_empty() {
        anyhow::bail!("no WAV fixtures found in {}", fixtures_dir.display());
    }

    let mut cases = Vec::new();
    for fixture in fixtures {
        let input = read_wav_mono(&fixture)?;
        let fixture_name = fixture
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let input_metrics = analyze_audio(&input, 0.01).into();

        for preset in VoicePreset::all() {
            let cfg = PresetManager::load(preset);
            let output = process(&cfg, &input);
            let output_metrics: MetricsSnapshot = analyze_audio(&output, 0.01).into();
            cases.push(GoldenCase {
                fixture: fixture_name.clone(),
                preset: preset.key().to_string(),
                input: input_metrics,
                output: output_metrics,
                delta_rms_db: round4(output_metrics.rms_db - input_metrics.rms_db),
                delta_noise_floor_db: round4(
                    output_metrics.noise_floor_db - input_metrics.noise_floor_db,
                ),
            });
        }
    }

    Ok(GoldenReport {
        schema: 1,
        sample_rate: SAMPLE_RATE,
        block_size: BLOCK,
        cases,
    })
}

fn process(cfg: &AppConfig, input: &[f32]) -> Vec<f32> {
    let mut chain = DspChain::from_config(cfg, SAMPLE_RATE as f32, BLOCK);
    let mut output = input.to_vec();
    for block in output.chunks_mut(BLOCK) {
        chain.process(block);
    }
    output
}

fn has_wav_fixtures(dir: &Path) -> anyhow::Result<bool> {
    Ok(dir.exists()
        && std::fs::read_dir(dir)?
            .filter_map(Result::ok)
            .any(|entry| is_wav(&entry.path())))
}

fn fixture_paths(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| is_wav(path))
        .collect();
    paths.sort();
    Ok(paths)
}

fn is_wav(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("wav"))
}

fn generate_fixtures(dir: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let cases = [
        ("male_low.wav", synth_voice(110.0, 0.0, 0.0)),
        ("male_mid.wav", synth_voice(150.0, 0.0, 0.0)),
        ("female.wav", synth_voice(220.0, 0.0, 0.0)),
        ("noisy_room.wav", synth_voice(150.0, 0.035, 0.0)),
        ("keyboard.wav", synth_keyboard_noise()),
    ];
    for (name, samples) in cases {
        write_wav(dir.join(name), &samples)?;
    }
    println!("fixtures generated: {}", dir.display());
    Ok(())
}

fn synth_voice(f0: f32, noise_amp: f32, breath_amp: f32) -> Vec<f32> {
    let n = (SAMPLE_RATE as f32 * FIXTURE_SECONDS) as usize;
    let mut seed = 0x5eed_1234_u32;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let env = fade_env(i, n);
        let mut voiced = 0.0;
        for h in 1..=36 {
            let hz = f0 * h as f32;
            if hz >= SAMPLE_RATE as f32 * 0.48 {
                break;
            }
            voiced += (TAU * hz * t).sin() / h as f32;
        }
        let noise = noise_sample(&mut seed) * noise_amp;
        let breath = noise_sample(&mut seed) * breath_amp * (TAU * 6200.0 * t).sin().abs();
        out.push((voiced * 0.24 + noise + breath) * env);
    }
    normalize_peak(&mut out, 0.45);
    out
}

fn synth_keyboard_noise() -> Vec<f32> {
    let n = (SAMPLE_RATE as f32 * FIXTURE_SECONDS) as usize;
    let mut seed = 0x45ab_cdef_u32;
    let mut out = vec![0.0; n];
    for sample in out.iter_mut() {
        *sample = noise_sample(&mut seed) * 0.015;
    }
    for click in [0.25_f32, 0.63, 0.91, 1.34, 1.62] {
        let start = (click * SAMPLE_RATE as f32) as usize;
        for i in 0..240 {
            let idx = start + i;
            if idx >= out.len() {
                break;
            }
            let env = (-(i as f32) / 40.0).exp();
            out[idx] += noise_sample(&mut seed) * 0.28 * env;
        }
    }
    out
}

fn fade_env(i: usize, n: usize) -> f32 {
    let fade = (SAMPLE_RATE as f32 * 0.03) as usize;
    let a = (i as f32 / fade as f32).clamp(0.0, 1.0);
    let b = ((n.saturating_sub(i + 1)) as f32 / fade as f32).clamp(0.0, 1.0);
    a.min(b)
}

fn noise_sample(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    ((*seed >> 9) as f32 / (1u32 << 23) as f32) - 1.0
}

fn normalize_peak(samples: &mut [f32], target: f32) {
    let peak = samples
        .iter()
        .fold(0.0_f32, |max, sample| max.max(sample.abs()));
    if peak > 1e-9 {
        for sample in samples {
            *sample = *sample / peak * target;
        }
    }
}

fn write_wav(path: impl AsRef<Path>, samples: &[f32]) -> anyhow::Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for sample in samples {
        writer.write_sample(*sample)?;
    }
    writer.finalize()?;
    Ok(())
}

fn read_wav_mono(path: &Path) -> anyhow::Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    if spec.sample_rate != SAMPLE_RATE {
        anyhow::bail!(
            "{} has sample_rate={}, expected {}",
            path.display(),
            spec.sample_rate,
            SAMPLE_RATE
        );
    }
    let channels = spec.channels.max(1) as usize;
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().collect::<Result<Vec<_>, _>>()?,
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => reader
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
            .collect::<Result<Vec<_>, _>>()?,
        hound::SampleFormat::Int => reader
            .samples::<i32>()
            .map(|s| s.map(|v| v as f32 / i32::MAX as f32))
            .collect::<Result<Vec<_>, _>>()?,
    };
    Ok(samples
        .chunks(channels)
        .map(|frame| frame.iter().copied().sum::<f32>() / frame.len() as f32)
        .collect())
}

fn write_json(path: &Path, report: &GoldenReport) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(path, serde_json::to_string_pretty(report)? + "\n")?;
    Ok(())
}

fn compare_reports(expected: &GoldenReport, actual: &GoldenReport) -> Vec<Regression> {
    let expected_by_key: HashMap<String, &GoldenCase> = expected
        .cases
        .iter()
        .map(|case| (case_key(case), case))
        .collect();
    let mut regressions = Vec::new();
    for actual_case in &actual.cases {
        let key = case_key(actual_case);
        let Some(expected_case) = expected_by_key.get(&key) else {
            regressions.push(Regression {
                key,
                field: "case",
                expected: 1.0,
                actual: 0.0,
                tolerance: 0.0,
            });
            continue;
        };
        compare_field(
            &mut regressions,
            &key,
            "peak",
            expected_case.output.peak,
            actual_case.output.peak,
            0.03,
        );
        compare_field(
            &mut regressions,
            &key,
            "rms_db",
            expected_case.output.rms_db,
            actual_case.output.rms_db,
            1.5,
        );
        compare_field(
            &mut regressions,
            &key,
            "clip_rate",
            expected_case.output.clip_rate,
            actual_case.output.clip_rate,
            0.001,
        );
        compare_field(
            &mut regressions,
            &key,
            "noise_floor_db",
            expected_case.output.noise_floor_db,
            actual_case.output.noise_floor_db,
            3.0,
        );
    }
    regressions
}

fn compare_field(
    regressions: &mut Vec<Regression>,
    key: &str,
    field: &'static str,
    expected: f32,
    actual: f32,
    tolerance: f32,
) {
    if (actual - expected).abs() > tolerance {
        regressions.push(Regression {
            key: key.to_string(),
            field,
            expected,
            actual,
            tolerance,
        });
    }
}

fn case_key(case: &GoldenCase) -> String {
    format!("{}::{}", case.fixture, case.preset)
}

/// T-013: clip_rate < 0.01% / peak <= 1.0 の絶対閾値をすべてのケースに適用する。
fn check_absolute_thresholds(report: &GoldenReport) -> Vec<String> {
    const MAX_CLIP_RATE: f32 = 0.0001; // 0.01%
    const MAX_PEAK: f32 = 1.0;
    let mut failures = Vec::new();
    for case in &report.cases {
        let key = case_key(case);
        if case.output.peak > MAX_PEAK {
            failures.push(format!(
                "{key} peak {:.4} > {MAX_PEAK:.4} (リミッター不足)",
                case.output.peak
            ));
        }
        if case.output.clip_rate > MAX_CLIP_RATE {
            failures.push(format!(
                "{key} clip_rate {:.6} > {MAX_CLIP_RATE:.6} (過剰クリッピング)",
                case.output.clip_rate
            ));
        }
    }
    failures
}

fn print_summary(report: &GoldenReport) {
    let max_clip = report
        .cases
        .iter()
        .map(|case| case.output.clip_rate)
        .fold(0.0_f32, f32::max);
    let max_peak = report
        .cases
        .iter()
        .map(|case| case.output.peak)
        .fold(0.0_f32, f32::max);
    println!(
        "golden cases: {} / max_peak={:.4} / max_clip_rate={:.6}",
        report.cases.len(),
        max_peak,
        max_clip
    );
}

fn round4(value: f32) -> f32 {
    (value * 10_000.0).round() / 10_000.0
}

fn round6(value: f32) -> f32 {
    (value * 1_000_000.0).round() / 1_000_000.0
}
