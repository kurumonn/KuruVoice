# プリセット

設計書 4.4 の値に基づきます。プリセットは「声づくり・EQ・コンプ・リミッター・ゲート」を
まとめて設定します（デバイス設定は維持されます）。

| プリセット | pitch | formant | 圧縮 | 主な EQ | 用途 |
| --- | --- | --- | --- | --- | --- |
| Neutral Clean | +2.5 | +0.7 | 中 | high-pass 100 / mud -2.5 / 動的De-esser | 性別感を薄めた自然声 |
| Soft Feminine | +5.0 | +1.2 | 中 | high-pass 115 / mud -3.0 / 強めDe-esser | 落ち着いた女性寄り声 |
| Bright Feminine | +7.0 | +1.6 | 中 | high-pass 130 / presence +3.6 / 強De-esser | 明るい女性寄り声 |
| Young Neutral | +4.0 | +0.9 | 中 | high-pass 110 / presence +2.6 / De-esser | 少年・中性寄り |
| Natural Low | -2.0 | -0.5 | 弱 | ナチュラル | 自然に少し低く |
| Ikemen Soft | -3.0 | -0.8 | 中 | presence +1.5 / mud -2.0 | 爽やか柔らか低音 |
| Ikemen Deep | -4.0 | -1.2 | 中 | low +2.0 / presence +1.0 | 深く落ち着いた声 |
| Narrator | -2.0 | -0.6 | 強 | presence +2.5 / limiter strict | ナレーション |
| Clear Streaming | -1.0 | -0.3 | 中 | presence +3.0 / de-esser | 配信の明瞭感 |
| Radio Voice | -3.0 | -1.0 | 強 | low +3.0 / high mild | ラジオ風の太い声 |
| Bright High | +6.0 | +2.0 | 中 | presence +3.5 / 高域寄り | 明るく軽い高めの声（高音キャラ方向） |
| Deep Cool | -6.0 | -2.0 | 強 | low +3.0 / presence +1.0 | 低く渋い太めの声（低音キャラ方向） |

> Bright High / Deep Cool は**個人名ではなく「方向性」のプリセット**です。高品質な
> 位相ボコーダ＋フォルマント処理により、±6 半音でも比較的自然な仕上がりになります。
> 「声の印象」グラフ（明瞭さ/かわいさ/かっこよさ/怖さ）と併用すると更に追い込めます。

Neutral Clean / Soft Feminine / Bright Feminine / Young Neutral は商用化設計のP0に沿った
「綺麗に入れる・刺さらず出す」系プリセットです。AI Voice Conversion を入れる前の
DSP-only baseline として使い、`cargo run --release --example voice_report` でAB比較します。

## カスタムプリセット

プリセットを適用後、スライダーで微調整し、設定ファイルとして保存すれば
自分専用プリセットになります。`--config my.toml` で読み込めます。
