# ESP32-C3 WiFi Repeater

## 概要
ESP32-C3を用いたL3 NAT (NAPT) 対応無線LANリピーターのRust実装

## 技術仕様
| 項目 | 内容 |
| :--- | :--- |
| 言語・ツールチェーン | Rust (esp-idf-svc) |
| ネットワーク機能 | IPv4 NAPT, DNS Propagation |
| 省電力機能 | WiFiモデムスリープ (MIN_MODEM) + CPU DFS 40〜80MHz + ライトスリープ |
| 設定保持 | NVS (JSON形式) |

## セットアップ
1. **ツールチェーンインストール**: `espup install`
2. **書き込みツール**: `cargo install espflash`
3. **設定ファイル準備**: `.cargo/config.toml.example` を `.cargo/config.toml` にコピーし、各環境変数を編集
4. **ビルド・実行**: `cargo build` && `espflash flash --monitor`

## CLIリファレンス
| コマンド | 機能 |
| :--- | :--- |
| `HELP` | コマンド一覧表示 |
| `GET` | 現状の設定値およびステータスの取得 |
| `SET <key> <val>` | 設定値の一時変更（`SAVE`で永続化） |
| `SAVE` | 現在の設定をNVSへ保存 |
| `RESTART` | デバイスの再起動 |
| `CURL <host>` | 指定ホストへHTTP GETリクエストを送信 |
| `PING` | 1.1.1.1/8.8.8.8/example.com へ port 80 TCP 疎通確認（IP直打ち・DNS不使用） |

## ビルド時環境変数
`.cargo/config.toml` の `[env]` セクションで設定。NVSに保存済みの値がある場合はそちらが優先される。

| 環境変数 | 対応フィールド | デフォルト |
| :--- | :--- | :--- |
| `REPEATER_STA_SSID` | `sta_ssid` | `""` |
| `REPEATER_STA_PASS` | `sta_password` | `""` |
| `REPEATER_STA_USE_STATIC` | `sta_use_static` | `"false"` |
| `REPEATER_STA_STATIC_IP` | `sta_static_ip` | `"192.168.10.2"` |
| `REPEATER_STA_GATEWAY` | `sta_gateway` | `"192.168.10.1"` |
| `REPEATER_STA_NETMASK` | `sta_netmask` | `"255.255.255.0"` |
| `REPEATER_AP_SSID` | `ap_ssid` | `"ESP32-C3-Repeater"` |
| `REPEATER_AP_PASS` | `ap_password` | `""` |
| `REPEATER_AP_CHANNEL` | `ap_channel` | `"1"` |

## 設定パラメータ (CLI)
- **STA (上流接続)**: `sta_ssid`, `sta_pass`, `sta_static` (bool), `sta_ip`, `sta_gw`, `sta_nm`
- **AP (本機)**: `ap_ssid`, `ap_pass`, `ap_chan`

## 開発・運用
- [システム設計概要](docs/ARCHITECTURE.md)
- [メンテナンスガイド](docs/MAINTENANCE.md)
