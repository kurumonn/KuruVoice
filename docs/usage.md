# 使い方

## 起動

```bash
# GUI ダッシュボード（引数なし）
cargo run --release

# デバイス一覧
cargo run --release -- --list-devices

# 設定 + プリセット指定
cargo run --release -- --config config.toml --preset ikemen_deep

# ヘッドレス（GUI なし）
cargo run --release -- --no-gui

# テスト録音（秒数）
cargo run --release -- --record-test 10
```

## GUI の使い方

1. 左ペインで **入力（マイク）** と **出力** を選ぶ。
2. 上部の **▶ 開始** を押す。
3. 中央の **プリセット** ボタンで雰囲気を決める。
4. スライダーで微調整（変更は実行中でも即反映）。
5. **バイパス** で加工前後を聴き比べ。
6. **設定ファイル** セクションで TOML を保存/読込。

> 実行中はデバイスを変更できません。変更したい場合は一度 **■ 停止** してから。

## 設定キー

| セクション | キー | 説明 |
| --- | --- | --- |
| voice | pitch_semitones | ピッチ（半音）。-3 で自然な低音 |
| voice | formant_shift | フォルマント。負で太く |
| eq | high_pass_hz | ローカット周波数 |
| eq | presence_boost_db | 明瞭感ブースト |
| compressor | ratio | 圧縮比 |
| limiter | ceiling_db | 最大音量（音割れ防止） |

詳細は `config.example.toml` を参照。

## トラブルシュート

- **音が出ない**: 出力デバイスが正しいか、`--list-devices` で名前確認。
- **遅延が大きい**: `buffer_size` を小さく（負荷とトレードオフ）。
- **入出力でサンプルレートが違う**: 警告が出たら、両デバイスを 48000Hz に揃える。
- **日本語が □ になる**: 日本語フォント（meiryo / Noto Sans CJK）が見つからない環境です。
