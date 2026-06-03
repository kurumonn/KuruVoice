# プリセット

設計書 4.4 の値に基づきます。プリセットは「声づくり・EQ・コンプ・リミッター・ゲート」を
まとめて設定します（デバイス設定は維持されます）。

| プリセット | pitch | formant | 圧縮 | 主な EQ | 用途 |
| --- | --- | --- | --- | --- | --- |
| Natural Low | -2.0 | -0.5 | 弱 | ナチュラル | 自然に少し低く |
| Ikemen Soft | -3.0 | -0.8 | 中 | presence +1.5 / mud -2.0 | 爽やか柔らか低音 |
| Ikemen Deep | -4.0 | -1.2 | 中 | low +2.0 / presence +1.0 | 深く落ち着いた声 |
| Narrator | -2.0 | -0.6 | 強 | presence +2.5 / limiter strict | ナレーション |
| Clear Streaming | -1.0 | -0.3 | 中 | presence +3.0 / de-esser | 配信の明瞭感 |
| Radio Voice | -3.0 | -1.0 | 強 | low +3.0 / high mild | ラジオ風の太い声 |

## カスタムプリセット

プリセットを適用後、スライダーで微調整し、設定ファイルとして保存すれば
自分専用プリセットになります。`--config my.toml` で読み込めます。
