# システム設計概要

## システム構造
各コンポーネントの責務を以下に定義する

| コンポーネント | 責務 | 該当ファイル |
| :--- | :--- | :--- |
| メイン制御 | 初期化、CLIループ、ライフサイクル管理 | `main.rs` |
| WiFi管理 | STA/APスタック制御、接続フロー、DNS構成 | `wifi.rs` |
| 設定管理 | NVSへのJSON永続化、シリアライズ、デフォルト値 | `config.rs` |
| NAPT | lwIP NAPT機能の有効化、パケットフォワーディング | `napt.rs` |

## データフロー
```mermaid
graph TD
    NVS[NVS Storage] -->|Load Config| Main[Main Controller]
    Main -->|Init| Wifi[Wifi Manager]
    Wifi -->|Connect| Upstream[Upstream AP]
    Upstream -->|Get DNS/IP| Wifi
    Wifi -->|Enable| NAPT[NAPT Engine]
    NAPT -->|Forward| Clients[Connected Clients]
```

## 設計上の決定事項
- **メモリ安全性**: Rustの所有権モデルにより、組み込み環境特有のメモリリークや競合を排除
- **設定フォーマット**: 汎用性とデバッグ性を考慮し、NVS内データはJSON形式を採用
- **同期実行**: 起動シーケンスの確実性を担保するため、`BlockingWifi` による同期的接続処理を実施
