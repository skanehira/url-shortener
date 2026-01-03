# ArgoCD セットアップガイド

Helm を使用した ArgoCD のインストールと App-of-Apps パターンによるアプリケーション管理の手順です。

## 概要

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              ArgoCD                                         │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                     Root Application                                │    │
│  │                    (infra/argocd/apps/)                             │    │
│  └───────────────────────────┬─────────────────────────────────────────┘    │
│              ┌───────────────┼───────────────┐                              │
│              ▼               ▼               ▼                              │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐                    │
│  │  Operators    │  │  Operators    │  │ url-shortener │                    │
│  │  (Wave 0)     │  │  (Wave 0)     │  │  (Wave 1)     │                    │
│  │               │  │               │  │               │                    │
│  │ - CloudNative │  │ - Sealed      │  │ - staging     │                    │
│  │   PG          │  │   Secrets     │  │ - prod        │                    │
│  │ - Redis       │  │               │  │               │                    │
│  │ - RabbitMQ    │  │               │  │               │                    │
│  └───────────────┘  └───────────────┘  └───────────────┘                    │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Sync Wave の順序:**
1. Wave 0: Operators（CloudNativePG, Redis, RabbitMQ, Sealed Secrets）
2. Wave 1: url-shortener（staging, prod）

## 前提条件

- kubeadm で構築された Kubernetes クラスタ
- kubectl が Mac から実行可能
- Helm 3.x インストール済み

## Step 1: Helm リポジトリ追加

```bash
# ArgoCD Helm リポジトリ追加
helm repo add argo https://argoproj.github.io/argo-helm
helm repo update
```

## Step 2: ArgoCD Namespace 作成

```bash
kubectl create namespace argocd
```

## Step 3: ArgoCD Helm インストール

```bash
# ArgoCD インストール
helm install argocd argo/argo-cd \
  --namespace argocd \
  --set configs.params."server\.insecure"=true \
  --set server.service.type=LoadBalancer \
  --wait

# Pod が起動するまで待機
kubectl wait --namespace argocd \
  --for=condition=ready pod \
  --selector=app.kubernetes.io/name=argocd-server \
  --timeout=300s
```

### Helm values のカスタマイズ（オプション）

より細かくカスタマイズする場合は values ファイルを作成:

```bash
cat > argocd-values.yaml <<'EOF'
configs:
  params:
    server.insecure: true

server:
  service:
    type: LoadBalancer

# HA 構成にする場合
# controller:
#   replicas: 2
# repoServer:
#   replicas: 2
# server:
#   replicas: 2
EOF

helm install argocd argo/argo-cd \
  --namespace argocd \
  -f argocd-values.yaml \
  --wait
```

## Step 4: 初期パスワード取得

```bash
# 初期 admin パスワードを取得
kubectl -n argocd get secret argocd-initial-admin-secret \
  -o jsonpath="{.data.password}" | base64 -d | pbcopy
```

## Step 5: ArgoCD UI アクセス

```bash
# LoadBalancer IP 確認
kubectl get svc -n argocd argocd-server

# 出力例:
# NAME            TYPE           EXTERNAL-IP     PORT(S)
# argocd-server   LoadBalancer   192.168.2.202   80:xxxxx/TCP,443:xxxxx/TCP
```

ブラウザで `http://<EXTERNAL-IP>` にアクセスし、以下でログイン:
- ユーザー名: `admin`
- パスワード: Step 4 で取得したパスワード

## Step 6: Root Application 適用

App-of-Apps パターンの起点となる Root Application を適用します。

```bash
kubectl apply -f infra/argocd/root-app.yaml
```

これにより、ArgoCD が以下を自動的にデプロイします:

1. **Wave 0 (Operators)**
   - CloudNativePG Operator
   - Redis Operator
   - RabbitMQ Cluster Operator
   - Sealed Secrets Controller

2. **Wave 1 (Applications)**
   - url-shortener-staging
   - url-shortener-prod

## Step 7: ArgoCD CLI インストール（オプション）

CLI を使用する場合はインストールしてください。

```bash
brew install argocd
```

## Step 8: デプロイ状況確認

```bash
# ArgoCD CLI でログイン（オプション）
argocd login <EXTERNAL-IP> --username admin --password <password> --insecure

# Application 一覧
argocd app list

# または kubectl で確認
kubectl get applications -n argocd
```

### UI での確認

ArgoCD UI の Applications ページで以下が確認できます:
- root: Healthy, Synced
- cloudnative-pg: Healthy, Synced
- redis-operator: Healthy, Synced
- rabbitmq-operator: Healthy, Synced
- sealed-secrets: Healthy, Synced
- url-shortener-staging: Healthy, Synced
- url-shortener-prod: Healthy, Synced (手動同期)

## ディレクトリ構成

```
infra/argocd/
├── root-app.yaml              ← 唯一手動で適用
└── apps/
    ├── operators/
    │   ├── cloudnative-pg.yaml   ← Wave 0
    │   ├── redis-operator.yaml   ← Wave 0
    │   ├── rabbitmq-operator.yaml← Wave 0
    │   └── sealed-secrets.yaml   ← Wave 0
    └── url-shortener.yaml        ← Wave 1 (ApplicationSet)
```

## Sync Wave について

ArgoCD の Sync Wave は、リソースのデプロイ順序を制御する機能です。

```yaml
metadata:
  annotations:
    argocd.argoproj.io/sync-wave: "0"  # 数値が小さいほど先にデプロイ
```

- Wave 0: Operators（CRD や Controller が必要）
- Wave 1: Applications（Operators が提供する CRD を使用）

これにより、Operators が完全に起動してから url-shortener がデプロイされます。

## よく使うコマンド

| コマンド | 説明 |
|---------|------|
| `argocd app list` | アプリ一覧 |
| `argocd app get <app>` | アプリ詳細 |
| `argocd app sync <app>` | 手動同期 |
| `argocd app sync <app> --prune` | 手動同期（削除含む） |
| `argocd app history <app>` | デプロイ履歴 |
| `argocd app rollback <app> <id>` | ロールバック |
| `argocd app delete <app>` | アプリ削除 |

## トラブルシューティング

### Application が Sync されない

```bash
# Application の詳細確認
kubectl describe application root -n argocd

# Sync 強制実行
argocd app sync root --prune
```

### Operator のデプロイ失敗

```bash
# 特定の Application の状態確認
kubectl get application cloudnative-pg -n argocd -o yaml

# Events 確認
kubectl get events -n argocd --sort-by='.lastTimestamp'
```

### Pod の状態確認

```bash
# Operator Pod の状態
kubectl get pods -n cnpg-system
kubectl get pods -n redis-operator-system
kubectl get pods -n rabbitmq-system
kubectl get pods -n observability
```

## パスワード変更

```bash
# ArgoCD CLI でパスワード変更
argocd account update-password
```

## ArgoCD アップグレード

```bash
# リポジトリ更新
helm repo update

# アップグレード（--reuse-values で既存設定を維持）
helm upgrade argocd argo/argo-cd \
  --namespace argocd \
  --reuse-values
```

## クリーンアップ

ArgoCD と関連リソースを完全に削除する場合:

```bash
# Root Application 削除（子 Application も削除される）
kubectl delete application root -n argocd

# ArgoCD Helm リリース削除
helm uninstall argocd -n argocd

# Namespace 削除
kubectl delete namespace argocd
```
