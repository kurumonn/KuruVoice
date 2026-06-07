#!/usr/bin/env python3
"""
T-012: 音声評価コーパス生成スクリプト

合成音声サンプルを生成して tests/corpus/synthetic/ に書き出す。
実声サンプルは CC0 素材を手動で tests/corpus/cc0/ に配置してください。

使い方:
  pip install numpy soundfile
  python tests/corpus/generate.py --output tests/corpus/synthetic/
"""

import argparse
import os
import math

SAMPLE_RATE = 48_000
DURATION = 3.0  # 秒


def noise_lcg(seed: int) -> tuple:
    """LCG による再現性ある疑似乱数ノイズ生成器（外部依存なし）。"""
    while True:
        seed = (seed * 1_664_525 + 1_013_904_223) & 0xFFFF_FFFF
        yield (seed >> 9) / (1 << 23) - 1.0


def synth_voice(f0: float, noise_amp: float = 0.0, n_harmonics: int = 36) -> list:
    """調波合成による擬似音声（Klatt 倍音モデル）。"""
    n = int(SAMPLE_RATE * DURATION)
    gen = noise_lcg(0x5EED_0000 + int(f0))
    samples = []
    fade_len = int(SAMPLE_RATE * 0.02)

    for i in range(n):
        t = i / SAMPLE_RATE
        v = 0.0
        for h in range(1, n_harmonics + 1):
            hz = f0 * h
            if hz >= SAMPLE_RATE * 0.48:
                break
            v += math.sin(2 * math.pi * hz * t) / h
        noise = next(gen) * noise_amp
        s = v * 0.24 + noise
        # fade in/out
        env = min(min(i, fade_len), min(n - i - 1, fade_len)) / max(fade_len, 1)
        samples.append(s * env)

    # ピーク正規化
    peak = max(abs(s) for s in samples) or 1.0
    return [s / peak * 0.45 for s in samples]


def synth_sibilant() -> list:
    """歯擦音（高域ノイズ + 4kHz 強調）。"""
    n = int(SAMPLE_RATE * DURATION)
    gen = noise_lcg(0xABCD_1234)
    hp_state = 0.0
    coeff = math.exp(-2 * math.pi * 3500 / SAMPLE_RATE)
    samples = []
    fade_len = int(SAMPLE_RATE * 0.02)

    for i in range(n):
        raw = next(gen)
        # high-pass: y = x - coeff*prev
        hp = raw - coeff * hp_state
        hp_state = raw
        env = min(min(i, fade_len), min(n - i - 1, fade_len)) / max(fade_len, 1)
        samples.append(hp * 0.3 * env)

    peak = max(abs(s) for s in samples) or 1.0
    return [s / peak * 0.4 for s in samples]


def synth_keyboard_noise() -> list:
    """キーボードクリック音（インパルス列 + ホワイトノイズ）。"""
    n = int(SAMPLE_RATE * DURATION)
    gen = noise_lcg(0x1234_5678)
    samples = [next(gen) * 0.01 for _ in range(n)]
    click_times = [0.25, 0.63, 0.91, 1.34, 1.62, 2.05, 2.41, 2.78]
    click_gen = noise_lcg(0xFEDC_BA98)
    for t in click_times:
        start = int(t * SAMPLE_RATE)
        for j in range(240):
            idx = start + j
            if idx >= n:
                break
            env = math.exp(-j / 40.0)
            samples[idx] += next(click_gen) * 0.28 * env
    peak = max(abs(s) for s in samples) or 1.0
    return [s / peak * 0.35 for s in samples]


def synth_clipping_risk(f0: float = 150.0) -> list:
    """クリッピング寸前の高音量サンプル（limiter テスト用）。"""
    samples = synth_voice(f0)
    return [s * 2.1 for s in samples]  # 意図的にオーバーゲイン


def write_wav(path: str, samples: list, sample_rate: int = SAMPLE_RATE):
    try:
        import soundfile as sf
        import numpy as np
        sf.write(path, np.array(samples, dtype="float32"), sample_rate, subtype="PCM_16")
        print(f"  wrote: {path} ({len(samples)} samples)")
    except ImportError:
        # soundfile がない場合は手書きの WAV writer を使う
        import struct
        n = len(samples)
        data = struct.pack(f"<{n}h", *(max(-32768, min(32767, int(s * 32767))) for s in samples))
        header = struct.pack(
            "<4sI4s4sIHHIIHH4sI",
            b"RIFF", 36 + len(data), b"WAVE",
            b"fmt ", 16, 1, 1, sample_rate, sample_rate * 2, 2, 16,
            b"data", len(data),
        )
        with open(path, "wb") as f:
            f.write(header + data)
        print(f"  wrote (raw): {path} ({n} samples)")


CASES = [
    ("male_low.wav",      lambda: synth_voice(110.0)),
    ("male_mid.wav",      lambda: synth_voice(150.0)),
    ("male_high.wav",     lambda: synth_voice(200.0)),
    ("female_mid.wav",    lambda: synth_voice(220.0)),
    ("female_high.wav",   lambda: synth_voice(300.0)),
    ("sibilant.wav",      synth_sibilant),
    ("fast_speech.wav",   lambda: synth_voice(150.0, noise_amp=0.02)),
    ("noisy_room.wav",    lambda: synth_voice(150.0, noise_amp=0.04)),
    ("keyboard.wav",      synth_keyboard_noise),
    ("clipping_risk.wav", synth_clipping_risk),
]


def main():
    parser = argparse.ArgumentParser(description="KuruVoice 評価コーパス生成")
    parser.add_argument("--output", default="tests/corpus/synthetic/")
    args = parser.parse_args()

    os.makedirs(args.output, exist_ok=True)
    print(f"generating {len(CASES)} samples → {args.output}")
    for name, fn in CASES:
        samples = fn()
        write_wav(os.path.join(args.output, name), samples)
    print("done.")


if __name__ == "__main__":
    main()
