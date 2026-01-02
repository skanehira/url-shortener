# Kubernetesリソースガイド

## リソース一覧

### ワークロード（アプリを動かす）

#### Pod
最小デプロイ単位。1つ以上のコンテナをまとめる。

```
┌─────────────────┐
│      Pod        │
│ ┌─────┐ ┌─────┐ │
│ │ App │ │ Log │ │  ← 複数コンテナも可
│ └─────┘ └─────┘ │
│   共有ネットワーク  │
└─────────────────┘
```

#### Deployment
Podのレプリカ管理 + ローリングアップデート。**最も使う**。

```
Deployment (replicas: 3)
    │
    ├── Pod-1
    ├── Pod-2
    └── Pod-3
```

**ユースケース:**
- Webアプリケーション (nginx, React, Vue, Next.js)
- APIサーバー (REST API, GraphQL)
- ワーカープロセス (キュー処理、バックグラウンドジョブ)
- プロキシ/ゲートウェイ (nginx, envoy, traefik)
- マイクロサービス全般

#### ReplicaSet
Podのレプリカ数を維持。通常はDeploymentが内部で使う。

#### StatefulSet
**ステートフル**なアプリ用（DB等）。Pod名が固定、順序付き起動。

```
StatefulSet: mysql
    │
    ├── mysql-0  ← 必ずこの順で起動
    ├── mysql-1
    └── mysql-2
```

**ユースケース:**

| カテゴリ | 例 |
|---------|-----|
| RDB | MySQL, PostgreSQL, MariaDB |
| NoSQL | MongoDB, Cassandra, CockroachDB |
| 検索エンジン | Elasticsearch, OpenSearch |
| メッセージキュー | Kafka, RabbitMQ, NATS |
| キャッシュ | Redis Cluster, Memcached |
| 分散ストレージ | MinIO, Ceph |
| 分散システム | ZooKeeper, etcd, Consul |

#### DaemonSet
**全ノードに1つずつ**Podを配置。

```
DaemonSet: fluentd
    │
    ├── Node1 → fluentd-xxx
    ├── Node2 → fluentd-yyy
    └── Node3 → fluentd-zzz
```

**ユースケース:**

| カテゴリ | 例 |
|---------|-----|
| ログ収集 | fluentd, filebeat, fluent-bit |
| 監視エージェント | node-exporter, datadog-agent, newrelic |
| ネットワークプラグイン | calico-node, flannel, cilium, **kube-proxy** |
| ストレージデーモン | ceph, glusterfs, longhorn |
| セキュリティ | falco, twistlock, aqua |
| デバイスプラグイン | NVIDIA GPU plugin, SR-IOV |

※ `kube-proxy`自体もDaemonSetで動いている

#### Job
**一度だけ**実行して終了するタスク。

**ユースケース:**
- データベースマイグレーション
- バックアップ/リストア
- 初期データ投入
- 機械学習モデルのトレーニング
- テスト/E2E実行
- 一括データ処理

#### CronJob
**定期実行**。Jobを定期的に生成。

```yaml
schedule: "0 3 * * *"  # 毎日3時
```

**ユースケース:**
- 定期バックアップ
- ログローテーション
- レポート生成・メール送信
- キャッシュクリア
- 古いデータの削除
- ヘルスチェック
- 証明書更新

---

### ネットワーク（通信）

#### Service
Podへの安定したアクセスを提供。

| Type | 用途 |
|------|------|
| ClusterIP | クラスタ内部のみ（デフォルト） |
| NodePort | ノードIP:ポートで外部公開 |
| LoadBalancer | 外部LB経由で公開 |
| ExternalName | 外部DNSへのエイリアス |
| Headless | StatefulSet用、Pod直接アクセス |

```
          Service (ClusterIP: 10.96.0.1)
               │
    ┌──────────┼──────────┐
    ▼          ▼          ▼
  Pod-1      Pod-2      Pod-3
```

**ユースケース:**
- **ClusterIP**: マイクロサービス間通信、DB接続
- **NodePort**: 開発環境、オンプレミス公開
- **LoadBalancer**: 本番環境の外部公開
- **ExternalName**: 外部SaaS (RDS, Cloud SQL等) へのプロキシ
- **Headless**: StatefulSetの各Podに個別アクセス (kafka-0, kafka-1...)

#### Ingress
**L7ロードバランサー**。HTTPルーティング、TLS終端。

```
                    Ingress
                       │
    ┌──────────────────┼──────────────────┐
    │                  │                  │
/api/*            /web/*            /admin/*
    │                  │                  │
 api-svc            web-svc          admin-svc
```

**ユースケース:**
- パスベースルーティング (`/api`, `/web`, `/admin`)
- ホストベースルーティング (`api.example.com`, `web.example.com`)
- TLS/SSL終端 (HTTPS対応)
- Basic認証、OAuth連携
- レート制限
- リダイレクト (HTTP→HTTPS, www→non-www)
- Canaryデプロイ (トラフィック分割)

**主要なIngress Controller:**
- nginx-ingress
- traefik
- HAProxy
- AWS ALB Ingress Controller
- GCE Ingress

#### NetworkPolicy
Pod間の通信制御（ファイアウォール）。

**ユースケース:**
- マルチテナント環境での通信分離
- PCI DSS等のコンプライアンス対応
- DBへのアクセス制限 (特定Podからのみ許可)
- 名前空間間の通信制御
- Egress制御 (外部通信の制限)

---

### 設定・機密情報

#### ConfigMap
設定情報を保存（平文）。

```yaml
data:
  DATABASE_HOST: "mysql.default"
  LOG_LEVEL: "info"
```

**ユースケース:**
- 環境変数の外部化
- 設定ファイル (nginx.conf, application.yml)
- スクリプトの格納
- 環境ごとの設定切り替え (dev/stg/prod)

#### Secret
機密情報を保存（Base64エンコード）。

```yaml
data:
  password: cGFzc3dvcmQxMjM=  # base64
```

**ユースケース:**
- DBパスワード、接続文字列
- APIキー、アクセストークン
- TLS証明書・秘密鍵
- SSH秘密鍵
- Dockerレジストリ認証情報 (imagePullSecrets)
- OAuth クライアントシークレット

---

### ストレージ

#### PersistentVolume (PV) / PersistentVolumeClaim (PVC)
永続ストレージを管理。

```
Pod → PVC → PV → 実ストレージ（NFS, EBS等）
```

**ユースケース:**
- DBのデータ永続化 (MySQL, PostgreSQL)
- ファイルアップロード保存
- 共有ストレージ (複数Podで共有)
- ログの永続保存
- 機械学習のモデル・データセット保存

**アクセスモード:**

| モード | 説明 |
|--------|------|
| ReadWriteOnce (RWO) | 単一ノードからR/W |
| ReadOnlyMany (ROX) | 複数ノードから読み取り専用 |
| ReadWriteMany (RWX) | 複数ノードからR/W (NFS等) |

#### StorageClass
動的にPVを作成するためのテンプレート。

**ユースケース:**
- クラウドストレージの自動プロビジョニング (EBS, GCE PD, Azure Disk)
- ストレージ性能の選択 (SSD/HDD, IOPS)
- 暗号化設定
- バックアップポリシー

---

### クラスタ管理

#### Namespace
リソースの論理的な分離。

```
├── default        # デフォルト
├── kube-system    # システムコンポーネント
├── kube-public    # 公開情報
└── production     # 自分で作成
```

**ユースケース:**
- 環境分離 (dev / staging / production)
- チーム分離 (team-a / team-b)
- マイクロサービス分離 (frontend / backend / data)
- 顧客分離 (マルチテナント)

#### Node
ワーカーマシン（読み取り専用）。

#### ResourceQuota
Namespace単位のリソース制限。

**ユースケース:**
- チームごとのリソース上限設定
- 暴走Podによるリソース枯渇防止
- コスト管理

#### LimitRange
Pod/コンテナのデフォルトリソース制限。

**ユースケース:**
- リソース指定忘れ防止 (デフォルト値設定)
- 単一Podの過剰リソース消費防止
- 最小リソース保証

---

### 認証・認可

#### ServiceAccount
Pod用のアイデンティティ。

**ユースケース:**
- CI/CDパイプラインからのクラスタ操作
- Operator/Controller (他リソースを操作するPod)
- 外部サービス連携 (AWS IAM Role for Service Accounts等)
- 最小権限の原則適用

#### Role / ClusterRole
権限の定義。

#### RoleBinding / ClusterRoleBinding
権限をユーザー/ServiceAccountに紐付け。

```
ServiceAccount ──RoleBinding──▶ Role
                               (pods: get, list)
```

**ユースケース:**
- 開発者に特定Namespaceのみ権限付与
- CI/CDにデプロイ権限付与
- 監視ツールに読み取り権限付与
- 管理者とオペレーターの権限分離

---

### その他

#### HorizontalPodAutoscaler (HPA)
CPU/メモリ使用率でPod数を自動スケール。

**ユースケース:**
- Webトラフィックの増減対応
- バッチ処理のスケールアウト
- コスト最適化 (低負荷時にスケールイン)
- カスタムメトリクスによるスケール (リクエスト数、キュー長)

#### PodDisruptionBudget (PDB)
メンテナンス時の最小稼働Pod数を保証。

**ユースケース:**
- ローリングアップデート時の可用性確保
- ノードメンテナンス (drain) 時のサービス継続
- クラスタアップグレード時の保護

#### CustomResourceDefinition (CRD)
独自のリソースを定義。

**ユースケース:**
- Operator パターン (アプリ固有の運用自動化)
- MetalLB (`IPAddressPool`, `L2Advertisement`)
- cert-manager (`Certificate`, `Issuer`)
- Prometheus (`ServiceMonitor`, `PrometheusRule`)
- Argo CD (`Application`, `AppProject`)

---

## シナリオ別構成

### 1. シンプルなWebアプリ

```
┌─────────────────────────────────────────────┐
│  外部からアクセスできるWebサイト             │
└─────────────────────────────────────────────┘

Deployment (nginx/React/Vue)
    │
Service (LoadBalancer or NodePort)
    │
外部ユーザー
```

**使うリソース:**
- `Deployment`: アプリのレプリカ管理
- `Service`: 外部公開

---

### 2. マイクロサービス（複数サービス連携）

```
┌─────────────────────────────────────────────────────────────┐
│  ECサイト: フロント + API + 決済 + 認証                      │
└─────────────────────────────────────────────────────────────┘

                         Ingress
                            │
        ┌───────────────────┼───────────────────┐
        │ /                 │ /api              │ /pay
        ▼                   ▼                   ▼
   ┌─────────┐        ┌─────────┐        ┌─────────┐
   │ front   │        │ api     │        │ payment │
   │ Service │        │ Service │        │ Service │
   └────┬────┘        └────┬────┘        └────┬────┘
        │                  │                  │
   Deployment         Deployment         Deployment
        │                  │                  │
        │             ConfigMap           Secret
        │             (DB設定)           (API Key)
        │                  │
        │                  ▼
        │             ┌─────────┐
        │             │ db      │
        │             │ Service │
        │             └────┬────┘
        │                  │
        │             StatefulSet (MySQL)
        │                  │
        │                 PVC
        └──────────────────┘
```

**使うリソース:**

| リソース | 用途 |
|---------|------|
| `Ingress` | パスベースでサービス振り分け |
| `Deployment` | 各マイクロサービス |
| `Service (ClusterIP)` | サービス間通信 |
| `StatefulSet` | DB（状態を持つ） |
| `ConfigMap` | 環境ごとの設定 |
| `Secret` | APIキー、DBパスワード |
| `PVC` | DBのデータ永続化 |

---

### 3. バッチ処理システム

```
┌─────────────────────────────────────────────┐
│  毎日深夜にデータ集計、レポート生成          │
└─────────────────────────────────────────────┘

CronJob (schedule: "0 3 * * *")
    │
    │ 毎日3時に起動
    ▼
   Job
    │
   Pod (集計処理)
    │
    ├── ConfigMap (S3バケット名等)
    └── Secret (AWS認証情報)
```

**使うリソース:**

| リソース | 用途 |
|---------|------|
| `CronJob` | 定期実行スケジュール |
| `Job` | 1回限りの処理 |
| `ConfigMap/Secret` | 外部サービス設定 |

---

### 4. ログ収集・監視

```
┌─────────────────────────────────────────────┐
│  全ノードからログを収集                      │
└─────────────────────────────────────────────┘

DaemonSet (fluentd/filebeat)
    │
    ├── Node1 → fluentd Pod → Elasticsearch
    ├── Node2 → fluentd Pod → Elasticsearch
    └── Node3 → fluentd Pod → Elasticsearch

DaemonSet (node-exporter)
    │
    └── 全ノードでメトリクス収集 → Prometheus
```

**使うリソース:**

| リソース | 用途 |
|---------|------|
| `DaemonSet` | 全ノードにエージェント配置 |
| `ConfigMap` | 収集設定 |

---

### 5. マルチテナント環境

```
┌─────────────────────────────────────────────┐
│  チームごとに環境を分離                      │
└─────────────────────────────────────────────┘

Namespace: team-a          Namespace: team-b
    │                          │
    ├── Deployment             ├── Deployment
    ├── Service                ├── Service
    ├── ResourceQuota          ├── ResourceQuota
    │   (CPU: 4, Mem: 8Gi)     │   (CPU: 8, Mem: 16Gi)
    └── NetworkPolicy          └── NetworkPolicy
        (team-b通信禁止)           (team-a通信禁止)
```

**使うリソース:**

| リソース | 用途 |
|---------|------|
| `Namespace` | チーム/環境の分離 |
| `ResourceQuota` | リソース使用量制限 |
| `NetworkPolicy` | チーム間通信制御 |
| `Role/RoleBinding` | チームごとの権限管理 |

---

### 6. 本番環境（高可用性）

```
┌─────────────────────────────────────────────┐
│  ダウンタイムを最小化したい                  │
└─────────────────────────────────────────────┘

Deployment (replicas: 3)
    │
    ├── Pod (node1)
    ├── Pod (node2)
    └── Pod (node3)

+ PodDisruptionBudget
    minAvailable: 2    ← 最低2つは常に稼働

+ HorizontalPodAutoscaler
    minReplicas: 3
    maxReplicas: 10
    targetCPU: 70%     ← CPU70%超えたらスケールアウト
```

**使うリソース:**

| リソース | 用途 |
|---------|------|
| `Deployment` | レプリカ + ローリングアップデート |
| `PodDisruptionBudget` | メンテナンス時の可用性保証 |
| `HorizontalPodAutoscaler` | 負荷に応じた自動スケール |

---

## リソース選択フローチャート

```
アプリをデプロイしたい
    │
    ├─ ステートレス？ → Deployment
    │
    ├─ ステートフル（DB等）？ → StatefulSet
    │
    ├─ 全ノードで動かす？ → DaemonSet
    │
    ├─ 1回だけ実行？ → Job
    │
    └─ 定期実行？ → CronJob


外部公開したい
    │
    ├─ TCP/UDPレベル？ → Service (LoadBalancer/NodePort)
    │
    └─ HTTPルーティング？ → Ingress


設定を渡したい
    │
    ├─ 機密情報？ → Secret
    │
    └─ 通常設定？ → ConfigMap


データを永続化したい → PVC + PV (+ StorageClass)
```

---

## よく使う組み合わせ

```
┌─────────────────────────────────────────────────┐
│  典型的なWebアプリ構成                           │
│                                                 │
│  Ingress                                        │
│     │                                           │
│  Service (ClusterIP)                            │
│     │                                           │
│  Deployment ─── ConfigMap                       │
│     │           Secret                          │
│     │                                           │
│  Pod ─── PVC ─── PV                             │
└─────────────────────────────────────────────────┘
```
