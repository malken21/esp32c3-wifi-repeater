# メンテナンスガイド

## 環境要件
- Rust ツールチェーン (rustup)
- espup (ESP32 Rust 開発用)
- espflash (書き込み・監視)

```bash
espup install
cargo install espflash
```

## ビルド時設定
`.cargo/config.toml.example` を `.cargo/config.toml` にコピーし、`[env]` セクションを編集する。
各変数は `option_env!` でビルド時に埋め込まれ、NVSに設定が存在しない場合のデフォルト値として使われる。

```toml
[env]
REPEATER_STA_SSID      = "your-upstream-ssid"
REPEATER_STA_PASS      = "your-upstream-password"
REPEATER_STA_USE_STATIC = "false"   # "true" で静的IP有効
REPEATER_STA_STATIC_IP = "192.168.10.2"
REPEATER_STA_GATEWAY   = "192.168.10.1"
REPEATER_STA_NETMASK   = "255.255.255.0"
REPEATER_AP_SSID       = "ESP32-C3-Repeater"
REPEATER_AP_PASS       = ""         # 空文字でオープンAP
REPEATER_AP_CHANNEL    = "1"
```

> **優先順位**: NVS保存値 > ビルド時環境変数のデフォルト値

## 開発タスク
### コマンドの追加
1. `src/cli/commands.rs`: コマンド解析ロジックの追加（`run()` 関数の match アームに追記）
2. `src/config.rs`: 必要に応じて `RepeaterConfig` のフィールド追加、`Default` に `option_env!` エントリを追加
3. `.cargo/config.toml.example`: 対応する環境変数エントリを追加
4. `README.md`: CLIリファレンスおよびビルド時環境変数テーブルの更新

### メモリ最適化
- スタックサイズ制限の遵守
- 可能な限り `heapless` クレートの利用を検討
- 動的確保の最小化

## デバッグ
- ログ出力: `log::info!`, `log::warn!`, `log::error!`
- リアルタイム監視: `espflash flash --monitor`
- 状態確認: CLIより `GET` コマンドを実行
- 省電力動作確認: 起動ログで以下を確認する
  - `cpu freq: 80000000 Hz` — CPU 80MHz 動作
  - `wifi:Set ps type: 1` — WiFiモデムスリープ有効
  - `Light sleep: ENABLED` — ライトスリープ有効
  - `PM configured: 80MHz max, 40MHz min, light sleep enabled.` — PM設定成功

## リリース管理
- バージョン管理: [Semantic Versioning](https://semver.org/) に準拠
- `Cargo.toml` の `version` フィールドを更新
