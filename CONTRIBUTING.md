# Contributing to KuruVoice

ご協力ありがとうございます！

## 開発フロー

1. Issue で議論 → fork → ブランチ作成
2. 変更を実装
3. 下記チェックを通す
4. Pull Request

## チェック

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

## コーディング方針

- 既存コードのスタイル・コメント密度に合わせる（日本語コメント可）。
- DSP は `AudioProcessor` トレイトを実装し、`DspChain` の処理順序（設計書 4.3）を守る。
- **リミッターは必ず最終段**（NF-005）。
- オーディオコールバック内でアロケーション・ロックを避ける（RT 安全性）。
- panic でアプリ全体が落ちないこと（NF-004）。

## ライセンス

コントリビュートは `MIT OR Apache-2.0` のデュアルライセンスに同意したものとみなされます。
