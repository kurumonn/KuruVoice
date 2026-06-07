# tests/corpus — 音声評価コーパス（T-012）

このディレクトリは品質評価・回帰テスト用の標準音声サンプルを管理します。

## ディレクトリ構成

```
tests/corpus/
├── README.md              (このファイル)
├── generate.py            サンプル生成スクリプト（合成音声 / CC0 素材）
├── synthetic/             合成音声サンプル（generate.py で生成）
│   ├── male_low.wav       男性低音: 110 Hz 基音
│   ├── male_mid.wav       男性中音: 150 Hz 基音
│   ├── male_high.wav      男性高音: 200 Hz 基音
│   ├── female_mid.wav     女性中音: 220 Hz 基音
│   ├── female_high.wav    女性高音: 300 Hz 基音
│   ├── sibilant.wav       サ行歯擦音（高域エネルギー集中）
│   ├── fast_speech.wav    早口ランダム音節シミュレーション
│   ├── noisy_room.wav     ルームノイズ混入 (SNR ~20dB)
│   ├── keyboard.wav       キーボードノイズ混入
│   └── clipping_risk.wav  クリッピング手前の高音量サンプル
└── cc0/                   CC0 ライセンスの実声サンプル（手動配置）
    └── .gitkeep
```

## サンプル生成

```bash
python tests/corpus/generate.py --output tests/corpus/synthetic/
```

Python 3.10+ と numpy が必要です:

```bash
pip install numpy soundfile
```

## 評価閾値（T-013）

| 指標 | 閾値 | 計測箇所 |
|------|------|---------|
| クリップ率 | < 0.01% | golden_report.rs |
| ピーク | ≤ 1.0 (0 dBFS) | golden_report.rs |
| ドロップアウト率 | < 0.1% | examples/perf.rs |

## CC0 素材の追加方法

[freesound.org](https://freesound.org/) から CC0 ライセンスの音声をダウンロードし、
`tests/corpus/cc0/` に配置してください。ファイル名のルール:

```
{性別}_{基音Hz}_{特徴}.wav  例: male_150_clear.wav
```

**重要**: CC0 以外のライセンスの素材はリポジトリに含めないでください。
