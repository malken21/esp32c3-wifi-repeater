# ESP32-C3 WiFi Repeater (Rust)

## 概要
ESP32-C3を用いたL3 NAT (NAPT) 対応無線LANリピーターのRust実装

## 技術仕様
| 項目 | 内容 |
| :--- | :--- |
| 言語・ツールチェーン | Rust (esp-idf-svc) |
| ネットワーク機能 | IPv4 NAPT, DNS Propagation |
| 省電力機能 | Light Sleep, 動的周波数制御 (10-160MHz) |
| 設定保持 | NVS (JSON形式) |

## セットアップ
1. **ツールチェーンインストール**: `espup install`
2. **書き込みツール**: `cargo install espflash`
3. **ビルド・実行**: `cargo build` && `espflash flash --monitor`

## CLIリファレンス
| コマンド | 機能 |
| :--- | :--- |
| `HELP` | コマンド一覧表示 |
| `GET` | 現状の設定値およびステータスの取得 |
| `SET <key> <val>` | 設定値の一時変更（`SAVE`で永続化） |
| `SAVE` | 現在の設定をNVSへ保存 |
| `RESTART` | デバイスの再起動 |

## 設定パラメータ
- **STA (上流接続)**: `sta_ssid`, `sta_pass`, `sta_static` (bool), `sta_ip`, `sta_gw`, `sta_nm`
- **AP (本機)**: `ap_ssid`, `ap_pass`, `ap_chan`

## 開発・運用
- [システム設計概要](docs/ARCHITECTURE.md)
- [メンテナンスガイド](docs/MAINTENANCE.md)

## ライセンス
[MIT License](LICENSE)
