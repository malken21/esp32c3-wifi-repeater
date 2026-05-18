# メンテナンスガイド

## 環境要件
- Rust ツールチェーン (rustup)
- espup (ESP32 Rust 開発用)
- espflash (書き込み・監視)

```bash
espup install
cargo install espflash
```

## 開発タスク
### コマンドの追加
1. `src/main.rs`: コマンド解析ロジックの追加
2. `src/config.rs`: 必要に応じて `RepeaterConfig` のフィールド追加
3. `README.md`: CLIリファレンスの更新

### メモリ最適化
- スタックサイズ制限の遵守
- 可能な限り `heapless` クレートの利用を検討
- 動的確保の最小化

## デバッグ
- ログ出力: `log::info!`, `log::warn!`, `log::error!`
- リアルタイム監視: `espflash flash --monitor`
- 状態確認: CLIより `GET` コマンドを実行

## リリース管理
- バージョン管理: [Semantic Versioning](https://semver.org/) に準拠
- `Cargo.toml` の `version` フィールドを更新
- 重要変更は `CHANGELOG.md` に追記
