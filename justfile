# .env ファイルを自動的に読み込む
set dotenv-load := true

# タスク一覧
default:
  @just --list

# ミドルウェア（PostgreSQL, Redis, RabbitMQ, OTEL, Jaeger）を起動
up:
  cd docker && docker compose up -d

# ミドルウェアを停止
down:
  cd docker && docker compose down

# アプリケーションを含む全サービスを起動
up-all:
  cd docker && docker compose --profile app up -d

# アプリケーションをビルドして起動
up-all-build:
  cd docker && docker compose --profile app up -d --build

# 全サービスを停止してボリュームも削除
[confirm]
down-all:
  cd docker && docker compose --profile app down -v

# コンテナのログを表示
logs *args:
  cd docker && docker compose --profile app logs {{args}}

# shortener-service のログを表示
logs-shortener:
  cd docker && docker compose logs -f shortener-service

# analytics-service のログを表示
logs-analytics:
  cd docker && docker compose logs -f analytics-service

# データベースをリセットしてマイグレーションを実行
[confirm]
sqlx-reset:
  sqlx database reset --source crates/shortener-service/migrations --force -y

# SQLx のクエリメタデータを生成（オフラインビルド用）
sqlx-prepare:
  @echo "SQLx クエリメタデータを生成中..."
  cargo sqlx prepare --workspace -- --all-targets
  @echo "生成完了: .sqlx/ ディレクトリにメタデータが保存されました"

# マイグレーションを実行
migrate:
  sqlx migrate run --source crates/shortener-service/migrations

# Lint, Format チェック, ビルドを実行
check-all:
  cargo clippy --workspace --all-targets -- -D warnings
  cargo fmt --all -- --check
  cargo build

# フォーマットを適用
fmt:
  cargo fmt --all

# Clippy を実行
lint:
  cargo clippy --workspace --all-targets

# テストを実行
test:
  cargo test --workspace

# ローカルでサービスを実行
run:
  cargo run -p shortener-service

# ローカルで analytics-service を実行
run-analytics:
  cargo run -p analytics-service

# 両サービスを並列実行（cargo-q が必要）
run-all:
  cargo q -v -p "run -p shortener-service" "run -p analytics-service"

# RabbitMQ 管理画面を開く
rabbitmq-ui:
  open http://localhost:15672

# Jaeger UI を開く
jaeger-ui:
  open http://localhost:16686

# Health check
health:
  @echo "=== shortener-service ===" && curl -s http://localhost:8080/health | jq .
  @echo "=== analytics-service ===" && curl -s http://localhost:8081/health | jq .

# Readiness check
ready:
  @echo "=== shortener-service ===" && curl -s http://localhost:8080/ready | jq .
  @echo "=== analytics-service ===" && curl -s http://localhost:8081/ready | jq .

# 短縮URLを作成
create-url url:
  curl -s -X POST http://localhost:8080/api/v1/urls \
    -H "Content-Type: application/json" \
    -d '{"url": "{{url}}"}' | jq .

# URL一覧を取得
list-urls:
  curl -s http://localhost:8080/api/v1/urls | jq .

# アナリティクス一覧を取得
list-analytics:
  curl -s http://localhost:8081/api/v1/analytics | jq .

# API 動作確認テストを実行
test-api:
  ./scripts/test-api.sh
