# Secrets Management

このディレクトリは SealedSecrets を使用して暗号化されたシークレットを管理します。

## 暗号化が必要なシークレット一覧

| シークレット名 | 用途 | キー |
|---------------|------|------|
| `postgres-credentials` | CloudNativePG の認証情報 | `username`, `password` |
| `rabbitmq-credentials` | RabbitMQ の認証情報 | `username`, `password` |
| `url-shortener-secrets` | アプリケーション接続情報 | `database-url`, `redis-url`, `rabbitmq-url` |

### 依存関係

```
┌─────────────────────────┐
│   postgres-credentials  │◄── CloudNativePG Cluster
└─────────────────────────┘

┌─────────────────────────┐
│   rabbitmq-credentials  │◄── RabbitMQ Cluster
└─────────────────────────┘

┌─────────────────────────┐
│  url-shortener-secrets  │◄── shortener-service (database-url, rabbitmq-url)
│                         │◄── analytics-service (redis-url, rabbitmq-url)
└─────────────────────────┘
```

## 前提条件

### 1. kubeseal CLI のインストール

```bash
# macOS
brew install kubeseal
```

### 2. Sealed Secrets Controller の確認

ArgoCD App-of-Apps でデプロイ済みであることを確認:

```bash
kubectl get pods -n kube-system -l app.kubernetes.io/name=sealed-secrets
```

## シークレット作成手順

### Step 1: Sealed Secrets Controller の公開鍵を取得

```bash
# 公開鍵を取得（オフラインで暗号化するため）
kubeseal --fetch-cert \
  --controller-name=sealed-secrets \
  --controller-namespace=kube-system \
  > sealed-secrets-cert.pem
```

### Step 2: シークレットの値を準備

以下の値を準備してください:

| 変数 | 説明 | 例 |
|------|------|-----|
| `POSTGRES_USER` | PostgreSQL ユーザー名 | `urlshortener` |
| `POSTGRES_PASSWORD` | PostgreSQL パスワード | (強力なパスワード) |
| `RABBITMQ_USER` | RabbitMQ ユーザー名 | `urlshortener` |
| `RABBITMQ_PASSWORD` | RabbitMQ パスワード | (強力なパスワード) |

### Step 3: 環境ごとにシークレットを暗号化

SealedSecrets は **namespace に紐づく** ため、環境ごとに生成が必要です。

#### Staging 環境

```bash
NAMESPACE="url-shortener-staging"
POSTGRES_USER="urlshortener"
POSTGRES_PASSWORD="your-staging-password"
RABBITMQ_USER="urlshortener"
RABBITMQ_PASSWORD="your-staging-password"

# postgres-credentials
kubectl create secret generic postgres-credentials \
  --namespace ${NAMESPACE} \
  --from-literal=username=${POSTGRES_USER} \
  --from-literal=password=${POSTGRES_PASSWORD} \
  --dry-run=client -o yaml | \
  kubeseal --cert sealed-secrets-cert.pem --format yaml \
  > staging-sealed-postgres-credentials.yaml

# rabbitmq-credentials
kubectl create secret generic rabbitmq-credentials \
  --namespace ${NAMESPACE} \
  --from-literal=username=${RABBITMQ_USER} \
  --from-literal=password=${RABBITMQ_PASSWORD} \
  --dry-run=client -o yaml | \
  kubeseal --cert sealed-secrets-cert.pem --format yaml \
  > staging-sealed-rabbitmq-credentials.yaml

# url-shortener-secrets
kubectl create secret generic url-shortener-secrets \
  --namespace ${NAMESPACE} \
  --from-literal=database-url="postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@staging-url-shortener-db-rw:5432/urlshortener" \
  --from-literal=redis-url="redis://staging-url-shortener-redis-master:6379" \
  --from-literal=rabbitmq-url="amqp://${RABBITMQ_USER}:${RABBITMQ_PASSWORD}@staging-url-shortener-rabbitmq:5672/" \
  --dry-run=client -o yaml | \
  kubeseal --cert sealed-secrets-cert.pem --format yaml \
  > staging-sealed-url-shortener-secrets.yaml
```

#### Production 環境

```bash
NAMESPACE="url-shortener-prod"
POSTGRES_USER="urlshortener"
POSTGRES_PASSWORD="your-prod-password"
RABBITMQ_USER="urlshortener"
RABBITMQ_PASSWORD="your-prod-password"

# postgres-credentials
kubectl create secret generic postgres-credentials \
  --namespace ${NAMESPACE} \
  --from-literal=username=${POSTGRES_USER} \
  --from-literal=password=${POSTGRES_PASSWORD} \
  --dry-run=client -o yaml | \
  kubeseal --cert sealed-secrets-cert.pem --format yaml \
  > prod-sealed-postgres-credentials.yaml

# rabbitmq-credentials
kubectl create secret generic rabbitmq-credentials \
  --namespace ${NAMESPACE} \
  --from-literal=username=${RABBITMQ_USER} \
  --from-literal=password=${RABBITMQ_PASSWORD} \
  --dry-run=client -o yaml | \
  kubeseal --cert sealed-secrets-cert.pem --format yaml \
  > prod-sealed-rabbitmq-credentials.yaml

# url-shortener-secrets
kubectl create secret generic url-shortener-secrets \
  --namespace ${NAMESPACE} \
  --from-literal=database-url="postgres://${POSTGRES_USER}:${POSTGRES_PASSWORD}@prod-url-shortener-db-rw:5432/urlshortener" \
  --from-literal=redis-url="redis://prod-url-shortener-redis-master:6379" \
  --from-literal=rabbitmq-url="amqp://${RABBITMQ_USER}:${RABBITMQ_PASSWORD}@prod-url-shortener-rabbitmq:5672/" \
  --dry-run=client -o yaml | \
  kubeseal --cert sealed-secrets-cert.pem --format yaml \
  > prod-sealed-url-shortener-secrets.yaml
```

### Step 4: SealedSecrets をマニフェストに統合

生成された SealedSecrets を overlay に配置:

```bash
# Staging
mv staging-sealed-*.yaml ../../../k8s/overlays/staging/

# Production
mv prod-sealed-*.yaml ../../../k8s/overlays/prod/
```

各 overlay の `kustomization.yaml` に追加:

```yaml
# k8s/overlays/staging/kustomization.yaml
resources:
  - ../../base
  - staging-sealed-postgres-credentials.yaml
  - staging-sealed-rabbitmq-credentials.yaml
  - staging-sealed-url-shortener-secrets.yaml
```

### Step 5: 公開鍵を削除

```bash
rm sealed-secrets-cert.pem
```

### Step 6: コミットしてプッシュ

```bash
git add k8s/overlays/
git commit -m "feat(k8s): add sealed secrets for staging and prod"
git push
```

## 注意事項

- **SealedSecrets はクラスタ固有**: クラスタを再構築した場合、Controller の秘密鍵が変わるため再生成が必要
- **Namespace に紐づく**: デフォルトでは同じ namespace でのみ復号化可能
- **パスワードは履歴に残さない**: シェル履歴に残らないよう `read -s` を使用するか、環境変数ファイルを使用

### パスワードを安全に入力する方法

```bash
read -s -p "Postgres Password: " POSTGRES_PASSWORD && echo
read -s -p "RabbitMQ Password: " RABBITMQ_PASSWORD && echo
```

## トラブルシューティング

### SealedSecret が復号化されない

```bash
# Controller のログを確認
kubectl logs -n kube-system -l app.kubernetes.io/name=sealed-secrets

# SealedSecret の状態を確認
kubectl get sealedsecret -n url-shortener-staging
kubectl describe sealedsecret postgres-credentials -n url-shortener-staging
```

### 公開鍵の取得に失敗する

```bash
# Controller が起動しているか確認
kubectl get pods -n kube-system -l app.kubernetes.io/name=sealed-secrets

# Service が存在するか確認
kubectl get svc -n kube-system sealed-secrets
```
