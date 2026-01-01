# URL Shortener

RustによるマイクロサービスアーキテクチャのURL短縮サービス。

## アーキテクチャ

```
                                    ┌─────────────────┐
                                    │     Jaeger      │
                                    │   (トレース)    │
                                    └────────▲────────┘
                                             │
┌─────────────────────────────────────────────────────────────────────────┐
│                         OTEL Collector                                  │
└───────────────────────────▲─────────────────────▲───────────────────────┘
                            │                     │
┌───────────────────────────┴───────┐ ┌───────────┴───────────────────────┐
│        shortener-service          │ │        analytics-service          │
│         (Port: 8080)              │ │          (Port: 8081)             │
│                                   │ │                                   │
│  - URL CRUD API                   │ │  - アクセス統計 API               │
│  - リダイレクト                   │ │  - イベント消費                   │
└───────────┬───────────────────────┘ └───────────────────────┬───────────┘
            │                                                 │
            │              ┌───────────────┐                  │
            │              │   RabbitMQ    │                  │
            └─────────────►│ (メッセージ)  │─────────────────►│
                           └───────────────┘                  │
            │                                                 │
            ▼                                                 ▼
    ┌───────────────┐                                 ┌───────────────┐
    │  PostgreSQL   │                                 │     Redis     │
    │   (永続化)    │                                 │  (カウンター) │
    └───────────────┘                                 └───────────────┘
```

## 技術スタック

| カテゴリ | 技術 |
|---------|------|
| 言語 | Rust |
| Web Framework | Axum |
| データベース | PostgreSQL (SQLx) |
| キャッシュ/カウンター | Redis |
| メッセージキュー | RabbitMQ (lapin) |
| トレーシング | OpenTelemetry + Jaeger |
| コンテナ | Docker, Docker Compose |

## 前提条件

- Rust 1.75+
- Docker / Docker Compose
- [just](https://github.com/casey/just) (タスクランナー)
- [sqlx-cli](https://github.com/launchbadge/sqlx/tree/main/sqlx-cli) (マイグレーション用)

```bash
# just のインストール
cargo install just

# sqlx-cli のインストール
cargo install sqlx-cli --no-default-features --features postgres
```

## セットアップ

### 1. 環境変数の設定

```bash
cp .env.example .env
```

### 2. ミドルウェアの起動

```bash
just up
```

以下のサービスが起動します:
- PostgreSQL (5432)
- Redis (6379)
- RabbitMQ (5672, 管理画面: 15672)
- OTEL Collector (4317)
- Jaeger (16686)

### 3. マイグレーション実行

```bash
just migrate
```

### 4. アプリケーション起動

**ローカル実行:**
```bash
just run-all  # 両サービスを並列実行
```

**Docker で実行:**
```bash
just up-all-build
```

## API リファレンス

### shortener-service (Port: 8080)

#### ヘルスチェック
```bash
GET /health
GET /ready
```

#### URL 作成
```bash
POST /api/v1/urls
Content-Type: application/json

{"url": "https://example.com/very/long/path"}
```

Response:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "code": "abc123",
  "original_url": "https://example.com/very/long/path",
  "created_at": "2024-01-01T00:00:00Z",
  "updated_at": "2024-01-01T00:00:00Z"
}
```

#### URL 一覧取得
```bash
GET /api/v1/urls
```

#### URL 取得
```bash
GET /api/v1/urls/{code}
```

#### URL 更新
```bash
PUT /api/v1/urls/{code}
Content-Type: application/json

{"url": "https://example.com/new/path"}
```

#### URL 削除
```bash
DELETE /api/v1/urls/{code}
```

#### リダイレクト
```bash
GET /{code}
# → 307 Temporary Redirect
```

### analytics-service (Port: 8081)

#### ヘルスチェック
```bash
GET /health
GET /ready
```

#### アクセス統計取得
```bash
GET /api/v1/analytics/{code}
```

Response:
```json
{
  "code": "abc123",
  "access_count": 42,
  "last_accessed_at": "2024-01-01T12:00:00Z"
}
```

#### アクセス統計一覧
```bash
GET /api/v1/analytics
```

## 開発コマンド

```bash
just              # コマンド一覧を表示
just up           # ミドルウェア起動
just down         # ミドルウェア停止
just up-all-build # 全サービスをビルドして起動
just down-all     # 全停止 + ボリューム削除

just run          # shortener-service をローカル実行
just run-analytics # analytics-service をローカル実行
just run-all      # 両サービスを並列実行

just check-all    # Lint + Format チェック + ビルド
just fmt          # フォーマット適用
just lint         # Clippy 実行
just test         # テスト実行
just test-api     # API 動作確認テスト

just logs-shortener  # shortener-service ログ表示
just logs-analytics  # analytics-service ログ表示

just jaeger-ui    # Jaeger UI を開く (http://localhost:16686)
just rabbitmq-ui  # RabbitMQ 管理画面を開く (http://localhost:15672)

just create-url "https://example.com"  # 短縮URL作成
just list-urls                          # URL一覧取得
just list-analytics                     # アナリティクス一覧取得
```

## Observability

### 分散トレーシング

Jaeger UI: http://localhost:16686

shortener-service から analytics-service へのトレース伝播が確認できます:
1. shortener-service: リダイレクト処理
2. RabbitMQ: メッセージ送信
3. analytics-service: イベント処理・Redis 更新

### RabbitMQ 管理画面

URL: http://localhost:15672
- Username: `urlshortener`
- Password: `localdevpassword`

## プロジェクト構成

```
.
├── crates/
│   ├── shortener-core/      # 共通ライブラリ
│   │   ├── src/
│   │   │   ├── config.rs    # 設定
│   │   │   ├── error.rs     # エラー型
│   │   │   ├── messaging/   # イベント定義
│   │   │   ├── rabbitmq.rs  # RabbitMQ接続
│   │   │   └── telemetry.rs # OpenTelemetry設定
│   │   └── Cargo.toml
│   ├── shortener-service/   # URL短縮サービス
│   │   ├── migrations/      # SQLマイグレーション
│   │   ├── src/
│   │   │   ├── config.rs
│   │   │   ├── main.rs
│   │   │   ├── publisher/   # イベント発行
│   │   │   ├── repository/  # DB操作
│   │   │   └── routes/      # APIハンドラ
│   │   └── Cargo.toml
│   └── analytics-service/   # アナリティクスサービス
│       ├── src/
│       │   ├── config.rs
│       │   ├── main.rs
│       │   ├── consumer/    # イベント消費
│       │   ├── repository/  # Redis操作
│       │   └── routes/      # APIハンドラ
│       └── Cargo.toml
├── docker/
│   ├── compose.yaml
│   ├── Dockerfile
│   └── otel-collector-config.yaml
├── scripts/
│   └── test-api.sh          # API動作確認スクリプト
├── Cargo.toml               # ワークスペース設定
├── justfile                 # タスクランナー設定
└── README.md
```

## ライセンス

MIT
