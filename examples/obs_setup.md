# OBS / 配信ソフト連携

KuruVoice の加工後音声を OBS・Discord・VRChat などへ流すには、
**仮想オーディオデバイス** を経由します。

## Windows (VB-CABLE)

1. [VB-CABLE](https://vb-audio.com/Cable/) をインストールして PC を再起動。
2. KuruVoice を起動し、左ペインの **出力** を `CABLE Input (VB-Audio Virtual Cable)` に設定。
3. **入力** は実際のマイクを選択。
4. **▶ 開始**。
5. 受け側アプリのマイク入力を `CABLE Output (VB-Audio Virtual Cable)` に設定:
   - OBS: ソース → 「音声入力キャプチャ」→ デバイス `CABLE Output`
   - Discord: 設定 → 音声・ビデオ → 入力デバイス `CABLE Output`
   - VRChat: Settings → Audio → Microphone `CABLE Output`

> 自分でモニターしたい場合は VoiceMeeter を使うと、仮想出力と実スピーカーの両方へ送れます。

## macOS (BlackHole)

1. [BlackHole](https://existential.audio/blackhole/) をインストール。
2. KuruVoice の出力を `BlackHole 2ch` に設定。
3. 受け側アプリのマイクを `BlackHole 2ch` に設定。

## Linux (PipeWire / JACK)

1. `pw-loopback` や `qpwgraph` で仮想シンクを作成。
2. KuruVoice の出力をそのシンクへ、受け側アプリの入力を同シンクのモニターへ接続。

## チェックリスト

- KuruVoice の **入力 = 実マイク** / **出力 = 仮想デバイス**
- 受け側アプリの **マイク = 仮想デバイスの出力**
- サンプルレートは 48000Hz に揃えると安定
