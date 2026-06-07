# OBS Studio 連携手順（T-015）

KuruVoice の加工音を OBS Studio に送る手順です。
仮想オーディオデバイス（VB-CABLE / BlackHole）が必要です。

---

## 必要なもの

| OS | 仮想デバイス |
|----|------------|
| Windows | [VB-CABLE Virtual Audio Device](https://vb-audio.com/Cable/)（無料） |
| macOS | [BlackHole 2ch](https://existential.audio/blackhole/)（無料）|
| Linux | PipeWire loopback（`pw-loopback`、ビルトイン）|

---

## Windows: VB-CABLE のセットアップ

1. [https://vb-audio.com/Cable/](https://vb-audio.com/Cable/) から VB-CABLE をダウンロード
2. `VBCABLE_Setup_x64.exe` を **管理者として実行** してインストール
3. PC を再起動
4. `サウンド` 設定でデバイスが追加されたことを確認:
   - 再生: "CABLE Input"
   - 録音: "CABLE Output"

---

## KuruVoice の出力先を仮想デバイスに設定する

1. KuruVoice を起動
2. 左ペインの **出力デバイス** で仮想デバイスを選択:
   - Windows: `CABLE Input (VB-Audio Virtual Cable)`
   - macOS: `BlackHole 2ch`
   - Linux: `KuruVoice_Mic`（pw-loopback）
3. **▶ 開始** を押す

> 出力デバイスにスピーカーが含まれない場合、KuruVoice の加工音はモニタリングできません。
> モニタリングしたい場合は VoiceMeeter（Windows）で仮想バスを介してスピーカーに分岐させてください。

---

## OBS Studio の設定

### 音声入力キャプチャソースの追加

1. OBS を起動
2. ソースパネルの **＋** → **音声入力キャプチャ** を選択
3. ソース名（例: "KuruVoice"）を入力して OK
4. デバイスを選択:
   - Windows: `CABLE Output (VB-Audio Virtual Cable)`
   - macOS: `BlackHole 2ch`
   - Linux: `KuruVoice_Mic`
5. OK を押す

### 音量確認

音声ミキサーパネルに追加したソースが表示されます。  
KuruVoice で声を出すと緑のメーターが動けば成功です。

### 推奨フィルタ設定（OBS 側）

音声ミキサーの歯車アイコン → フィルタ から以下を追加できます（任意）:

| フィルタ | 目的 |
|---------|------|
| ゲイン | 入力音量のオフセット調整 |
| ノイズ抑制 | KuruVoice の後段で追加ノイズを除去 |
| コンプレッサー | 声量のばらつきを均一にする |

KuruVoice 側で同様の処理を既に行っているため、OBS 側のフィルタは軽くかけるのが適切です。

---

## 配信プロファイル例（OBS の設定）

```
エンコーダ:  x264 / NVENC H.264
映像ビットレート: 4000〜6000 kbps
音声ビットレート: 128 kbps（AAC）
音声サンプリングレート: 48 kHz
```

---

## トラブルシューティング

| 症状 | 対処法 |
|------|-------|
| OBS に音声ソースが表示されない | VB-CABLE を再インストール / PC 再起動 |
| 音量が小さい | KuruVoice の出力ゲインを上げる、または OBS の「ゲイン」フィルタを使う |
| 音がプチプチ途切れる | KuruVoice の `config.toml` の `buffer_size` を 512 に増やす |
| エコーが入る | OBS の音声設定でマイクデバイスを「なし」または KuruVoice 仮想デバイス以外にする |

---

## 関連ドキュメント

- [discord_integration.md](discord_integration.md) — Discord 連携
- [setup_macos.md](setup_macos.md) — macOS での BlackHole セットアップ
- [setup_linux.md](setup_linux.md) — Linux での仮想ソース作成
