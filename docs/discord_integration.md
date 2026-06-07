# Discord 連携手順（T-015）

KuruVoice の加工音を Discord 通話で使う手順です。
仮想オーディオデバイス（VB-CABLE / BlackHole / PipeWire loopback）が必要です。

仮想デバイスのセットアップは OS ごとのガイドを参照してください:
- Windows: [obs_integration.md](obs_integration.md) の「VB-CABLE のセットアップ」節
- macOS: [setup_macos.md](setup_macos.md)
- Linux: [setup_linux.md](setup_linux.md)

---

## KuruVoice の出力先を仮想デバイスに設定する

1. KuruVoice を起動
2. 左ペインの **出力デバイス** で仮想デバイスを選択:
   - Windows: `CABLE Input (VB-Audio Virtual Cable)`
   - macOS: `BlackHole 2ch`
   - Linux: `KuruVoice_Mic`
3. **▶ 開始** を押す

---

## Discord の設定

1. Discord を起動
2. **設定（⚙）→ 音声・ビデオ** を開く
3. **入力デバイス** のドロップダウンで仮想デバイスを選択:
   - Windows: `CABLE Output (VB-Audio Virtual Cable)`
   - macOS: `BlackHole 2ch`
   - Linux: `KuruVoice_Mic`
4. **入力音量** スライダーを 100% 付近に設定
5. **入力感度** の「自動で入力感度を設定する」を **オフ** にして手動で調整
   - KuruVoice のノイズゲートが既に機能しているため、Discord 側の感度は低め推奨
6. **エコーキャンセル** を **オン**（KuruVoice の処理との二重化を避けるために **必ず確認**）

---

## 推奨 Discord 音声設定

| 設定項目 | 推奨値 | 理由 |
|---------|--------|------|
| エコーキャンセル | オン | スピーカーからのフィードバックを防ぐ |
| ノイズ抑制 | Krisp or 標準 | KuruVoice のノイズ低減と重複しても問題なし |
| 自動ゲインコントロール | オフ | KuruVoice の AutoGain と競合を防ぐ |
| 高品質音声 (Opus 96kbps) | オン | 音質向上 |

---

## 通話テスト

1. Discord の設定画面にある **「マイクのテスト」** ボタンを使って確認
2. または「Echo Test」ボット（公式）を DM して自分の声を確認

---

## VoiceMeeter を使ったモニタリング（Windows）

KuruVoice の加工音をスピーカーでモニタリングしながら Discord にも送りたい場合:

1. [VoiceMeeter Banana](https://vb-audio.com/Voicemeeter/banana.htm)（無料）をインストール
2. KuruVoice の出力を VoiceMeeter の仮想入力に設定
3. VoiceMeeter からスピーカーと "CABLE Input" の両方に出力
4. Discord の入力を "CABLE Output" に設定

---

## トラブルシューティング

| 症状 | 対処法 |
|------|-------|
| Discord に声が届かない | KuruVoice が「▶ 開始」状態になっているか確認 |
| エコーが入る | Discord のエコーキャンセルをオンにする |
| 音量が低い | KuruVoice の出力ゲインを上げる、Discord の入力感度を上げる |
| 声が機械的に聞こえる | プリセットを "Natural Low" や "Narrator" に変更する |
| 通話相手に雑音が聞こえる | KuruVoice のノイズゲート閾値を上げる（`config.toml`） |

---

## 関連ドキュメント

- [obs_integration.md](obs_integration.md) — OBS 連携
- [setup_macos.md](setup_macos.md) — macOS セットアップ
- [setup_linux.md](setup_linux.md) — Linux セットアップ
