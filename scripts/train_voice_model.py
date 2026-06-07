#!/usr/bin/env python3
"""
T-017: KuruVoice 自声補正モデル 学習スクリプト

前提:
  pip install torch torchaudio onnx onnxruntime numpy librosa soundfile tqdm

使い方:
  # 1. ペアデータ収録 (元声 → 目標声)
  python scripts/train_voice_model.py prepare \
    --source  data/source/        # 元声の wav ディレクトリ
    --target  data/target/        # 目標声の wav ディレクトリ

  # 2. 学習
  python scripts/train_voice_model.py train \
    --data    data/prepared/      \
    --output  assets/models/      \
    --epochs  100

  # 3. ONNX エクスポート
  python scripts/train_voice_model.py export \
    --checkpoint assets/models/best.pt  \
    --output     assets/models/voice_enhance_v1.onnx

安全方針:
  - このスクリプトは「自分の声を整える」用途にのみ使ってください。
  - 他人の声を無断で学習に使うこと、なりすましを目的とした利用は禁止です。
  - 詳細: docs/safety.md
"""

import argparse
import sys
import os
import time


# ---------------------------------------------------------------------------
# モデル定義
# ---------------------------------------------------------------------------

def build_model(input_size: int = 64, hidden: int = 128) -> "torch.nn.Module":
    """軽量エンコーダ・デコーダ（1D Conv + GRU）。"""
    import torch.nn as nn

    class VoiceEnhanceNet(nn.Module):
        def __init__(self):
            super().__init__()
            self.encoder = nn.Sequential(
                nn.Conv1d(1, 32, kernel_size=5, padding=2),
                nn.ReLU(),
                nn.Conv1d(32, 64, kernel_size=5, padding=2),
                nn.ReLU(),
            )
            self.gru = nn.GRU(64, hidden, batch_first=True, bidirectional=True)
            self.decoder = nn.Sequential(
                nn.Conv1d(hidden * 2, 64, kernel_size=5, padding=2),
                nn.ReLU(),
                nn.Conv1d(64, 1, kernel_size=5, padding=2),
                nn.Tanh(),
            )

        def forward(self, x):
            # x: (batch, 1, time)
            h = self.encoder(x)               # (batch, 64, time)
            h = h.permute(0, 2, 1)            # (batch, time, 64)
            h, _ = self.gru(h)                # (batch, time, hidden*2)
            h = h.permute(0, 2, 1)            # (batch, hidden*2, time)
            return self.decoder(h)             # (batch, 1, time)

    return VoiceEnhanceNet()


# ---------------------------------------------------------------------------
# データ準備
# ---------------------------------------------------------------------------

def cmd_prepare(args):
    """元声・目標声ペアを mel スペクトログラムに変換して保存する。"""
    try:
        import numpy as np
        import librosa
        import soundfile as sf
        from tqdm import tqdm
    except ImportError as e:
        sys.exit(f"[ERROR] 依存ライブラリが不足しています: {e}\n  pip install librosa soundfile tqdm")

    os.makedirs(args.output, exist_ok=True)
    src_files = sorted(f for f in os.listdir(args.source) if f.endswith(".wav"))
    tgt_files = sorted(f for f in os.listdir(args.target) if f.endswith(".wav"))

    if len(src_files) != len(tgt_files):
        sys.exit(f"[ERROR] source({len(src_files)}) と target({len(tgt_files)}) のファイル数が違います")

    print(f"preparing {len(src_files)} pairs...")
    pairs = []
    for sf_name, tf_name in tqdm(zip(src_files, tgt_files)):
        src, sr = librosa.load(os.path.join(args.source, sf_name), sr=22050, mono=True)
        tgt, _  = librosa.load(os.path.join(args.target, tf_name), sr=22050, mono=True)
        # 長さを揃える
        length = min(len(src), len(tgt))
        src, tgt = src[:length], tgt[:length]
        pairs.append((src, tgt))

    out_path = os.path.join(args.output, "dataset.npz")
    np.savez(out_path,
             sources=np.array([p[0] for p in pairs], dtype=object),
             targets=np.array([p[1] for p in pairs], dtype=object))
    print(f"saved: {out_path}")


# ---------------------------------------------------------------------------
# 学習
# ---------------------------------------------------------------------------

def cmd_train(args):
    try:
        import torch
        import torch.nn as nn
        import numpy as np
        from torch.utils.data import Dataset, DataLoader
        from tqdm import tqdm
    except ImportError as e:
        sys.exit(f"[ERROR] 依存ライブラリが不足しています: {e}\n  pip install torch numpy tqdm")

    dataset_path = os.path.join(args.data, "dataset.npz")
    if not os.path.exists(dataset_path):
        sys.exit(f"[ERROR] データセットが見つかりません: {dataset_path}\n  先に `prepare` を実行してください")

    data = np.load(dataset_path, allow_pickle=True)
    sources, targets = data["sources"], data["targets"]

    SEGMENT = 16000  # 約 0.7 秒 (22050Hz)

    class PairDataset(Dataset):
        def __init__(self):
            self.pairs = []
            for src, tgt in zip(sources, targets):
                for i in range(0, min(len(src), len(tgt)) - SEGMENT, SEGMENT // 2):
                    self.pairs.append((
                        torch.tensor(src[i:i+SEGMENT], dtype=torch.float32).unsqueeze(0),
                        torch.tensor(tgt[i:i+SEGMENT], dtype=torch.float32).unsqueeze(0),
                    ))

        def __len__(self): return len(self.pairs)
        def __getitem__(self, idx): return self.pairs[idx]

    dataset = PairDataset()
    loader = DataLoader(dataset, batch_size=16, shuffle=True, num_workers=0)
    model = build_model()
    optimizer = torch.optim.Adam(model.parameters(), lr=1e-3)
    scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=args.epochs)
    criterion = nn.L1Loss()

    os.makedirs(args.output, exist_ok=True)
    best_loss = float("inf")
    for epoch in range(1, args.epochs + 1):
        model.train()
        total_loss = 0.0
        for src_batch, tgt_batch in tqdm(loader, desc=f"epoch {epoch}/{args.epochs}", leave=False):
            pred = model(src_batch)
            loss = criterion(pred, tgt_batch)
            optimizer.zero_grad()
            loss.backward()
            optimizer.step()
            total_loss += loss.item()
        avg_loss = total_loss / max(len(loader), 1)
        scheduler.step()
        print(f"epoch {epoch:4d} / {args.epochs}  loss={avg_loss:.6f}")
        if avg_loss < best_loss:
            best_loss = avg_loss
            ckpt = os.path.join(args.output, "best.pt")
            torch.save({"epoch": epoch, "model_state": model.state_dict(), "loss": best_loss}, ckpt)
            print(f"  -> checkpoint saved: {ckpt}")

    print(f"training done. best_loss={best_loss:.6f}")


# ---------------------------------------------------------------------------
# ONNX エクスポート
# ---------------------------------------------------------------------------

def cmd_export(args):
    try:
        import torch
        import torch.onnx
    except ImportError as e:
        sys.exit(f"[ERROR] PyTorch が必要です: {e}\n  pip install torch onnx")

    ckpt = torch.load(args.checkpoint, map_location="cpu")
    model = build_model()
    model.load_state_dict(ckpt["model_state"])
    model.eval()

    dummy_input = torch.zeros(1, 1, 16000)
    os.makedirs(os.path.dirname(os.path.abspath(args.output)), exist_ok=True)
    torch.onnx.export(
        model,
        dummy_input,
        args.output,
        export_params=True,
        opset_version=17,
        input_names=["input"],
        output_names=["output"],
        dynamic_axes={"input": {2: "time"}, "output": {2: "time"}},
    )
    print(f"exported: {args.output}")


# ---------------------------------------------------------------------------
# エントリーポイント
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        prog="train_voice_model.py",
        description="KuruVoice 自声補正モデル 学習・エクスポートスクリプト",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    p_prepare = sub.add_parser("prepare", help="ペアデータを前処理する")
    p_prepare.add_argument("--source",  required=True, help="元声 wav ディレクトリ")
    p_prepare.add_argument("--target",  required=True, help="目標声 wav ディレクトリ")
    p_prepare.add_argument("--output",  default="data/prepared", help="出力ディレクトリ")

    p_train = sub.add_parser("train", help="モデルを学習する")
    p_train.add_argument("--data",    required=True, help="前処理済みデータのディレクトリ")
    p_train.add_argument("--output",  default="assets/models", help="チェックポイント出力先")
    p_train.add_argument("--epochs",  type=int, default=100, help="エポック数 (default: 100)")

    p_export = sub.add_parser("export", help="ONNX にエクスポートする")
    p_export.add_argument("--checkpoint", required=True, help="best.pt のパス")
    p_export.add_argument("--output",     default="assets/models/voice_enhance_v1.onnx")

    args = parser.parse_args()
    {"prepare": cmd_prepare, "train": cmd_train, "export": cmd_export}[args.command](args)


if __name__ == "__main__":
    main()
