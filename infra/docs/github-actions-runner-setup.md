# GitHub Actions Self-hosted Runner (ARC) セットアップガイド

Kubernetes上にGitHub Actions Self-hosted Runnerを構築します。
GitHub公式の Actions Runner Controller (ARC) v0.13.x を使用します。

## 構成図

```
┌─────────────┐                ┌─────────────────────────────────┐
│   GitHub    │    ポーリング  │  Kubernetes Cluster             │
│   Actions   │◄───────────────│                                 │
│             │                │  namespace: arc-systems         │
│  ┌───────┐  │   ジョブ取得   │  ┌───────────────────────────┐  │
│  │ Repo  │  │◄───────────────│  │  ARC Controller           │  │
│  └───────┘  │                │  │  (gha-runner-scale-set)   │  │
│             │   結果送信     │  └─────────────┬─────────────┘  │
│             │◄───────────────│                │ 管理           │
│             │                │  namespace: arc-runners         │
└─────────────┘                │  ┌─────────────▼─────────────┐  │
                               │  │  Runner Pod               │  │
                               │  │  (ワークフロー実行)       │  │
                               │  └───────────────────────────┘  │
                               └─────────────────────────────────┘
```

## 動作確認済み環境

| コンポーネント | バージョン |
|--------------|-----------|
| Kubernetes | v1.32 |
| ARC | v0.13.x |
| Helm | v3.x |

## 前提条件

- Kubernetesクラスタが稼働していること
- Helmがインストールされていること
- GitHubアカウントがあること

## 手順

### Step 1: Helmのインストール

```bash
brew install helm
```

### Step 2: GitHub Appの作成

GitHub Appを使うことで、PATより細かい権限制御と監査が可能になります。

#### 2-1. GitHub Appを作成

1. GitHub → Settings → Developer settings → GitHub Apps → 「New GitHub App」
2. 以下を設定：
   - **GitHub App name**: `arc-runner`（任意、一意である必要あり）
   - **Homepage URL**: `https://github.com/actions/actions-runner-controller`
   - **Webhook**: Active のチェックを**外す**

3. Permissions を設定：
   - **Repository permissions**:
     - **Administration**: Read and write
     - **Metadata**: Read-only
   - **Organization permissions**（組織で使う場合）:
     - **Self-hosted runners**: Read and write

4. 「Create GitHub App」をクリック

#### 2-2. 必要な情報を取得

1. **App ID**: 作成後の画面上部に表示される数字をメモ: 123456
2. **Private Key**: 画面下部の「Generate a private key」をクリックし、`.pem`ファイルをダウンロード

#### 2-3. アプリをリポジトリにインストール

1. 作成したApp画面 → 左メニュー「Install App」
2. 対象のアカウント/組織を選択
3. 「Only select repositories」で対象リポジトリを選択
4. 「Install」をクリック
5. **Installation ID**: インストール後のURLから取得: 12345678
   ```
   https://github.com/settings/installations/12345678
                                              ^^^^^^^^
                                              この数字がInstallation ID
   ```

#### 2-4. 取得した情報の確認

以下の3つが揃っていることを確認：

| 項目 | 例 |
|------|-----|
| App ID | `123456` |
| Installation ID | `12345678` |
| Private Key | `arc-runner.2025-01-01.private-key.pem` |

### Step 3: ARC Controllerのインストール

```bash
# オペレータをインストール
helm install arc \
  --namespace arc-systems \
  --create-namespace \
  oci://ghcr.io/actions/actions-runner-controller-charts/gha-runner-scale-set-controller

# Podが起動するまで待機
kubectl wait --for=condition=Ready pods --all -n arc-systems --timeout=300s

# 確認
kubectl get pods -n arc-systems
```

### Step 4: Runner Scale Setのインストール

#### 4-1. GitHub App用のSecretを作成

```bash
# 変数を設定
GITHUB_APP_ID=
GITHUB_APP_INSTALLATION_ID=
GITHUB_APP_PRIVATE_KEY_PATH=

# Namespaceを作成
kubectl create namespace arc-runners

# Secretを作成
kubectl create secret generic github-app-secret \
  --namespace arc-runners \
  --from-literal=github_app_id="${GITHUB_APP_ID}" \
  --from-literal=github_app_installation_id="${GITHUB_APP_INSTALLATION_ID}" \
  --from-file=github_app_private_key="${GITHUB_APP_PRIVATE_KEY_PATH}"
```

#### 4-2. Runner Scale Setをインストール

```bash
# リポジトリURLを設定
GITHUB_CONFIG_URL="https://github.com/skanehira/k8s.nvim"

# Runner Scale Setをインストール
helm install arc-runner-set \
  --namespace arc-runners \
  --set githubConfigUrl="${GITHUB_CONFIG_URL}" \
  --set githubConfigSecret=github-app-secret \
  oci://ghcr.io/actions/actions-runner-controller-charts/gha-runner-scale-set

# Podが起動するまで待機
kubectl wait --for=condition=Ready pods --all -n arc-runners --timeout=300s

# 確認
kubectl get pods -n arc-runners
```

### Step 5: 動作確認

#### GitHubで確認
1. リポジトリの Settings → Actions → Runners
2. 「arc-runner-set」が表示されていればOK

#### ワークフローをトリガー
```bash
# テスト用ワークフローをpush（.github/workflows/self-hosted-test.yaml）
git add .github/workflows/self-hosted-test.yaml
git commit -m "Add self-hosted runner test workflow"
git push
```

#### ログを確認
```bash
# Controllerのログ
kubectl logs -n arc-systems -l app.kubernetes.io/name=gha-runner-scale-set-controller

# Runnerのログ
kubectl logs -n arc-runners -l app.kubernetes.io/component=runner
```

## ワークフローでの使用

`.github/workflows/self-hosted-test.yaml`:
```yaml
name: Test on Self-hosted Runner

on:
  push:
    branches: [main]
  workflow_dispatch:

jobs:
  test:
    runs-on: arc-runner-set  # ← インストール時の名前を指定
    steps:
      - uses: actions/checkout@v4
      - name: Show runner info
        run: |
          echo "Running on self-hosted runner!"
          echo "Hostname: $(hostname)"
```

## オプション設定

### スケーリング設定

`values.yaml` を作成してカスタマイズ:

```yaml
# values.yaml
minRunners: 1
maxRunners: 5
```

```bash
helm upgrade arc-runner-set \
  --namespace arc-runners \
  --set githubConfigUrl="${GITHUB_CONFIG_URL}" \
  --set githubConfigSecret=github-app-secret \
  -f values.yaml \
  oci://ghcr.io/actions/actions-runner-controller-charts/gha-runner-scale-set
```

### リソース制限

```yaml
# values.yaml
template:
  spec:
    containers:
      - name: runner
        resources:
          limits:
            cpu: "2"
            memory: "4Gi"
          requests:
            cpu: "500m"
            memory: "1Gi"
```

### Docker-in-Docker (DinD) モード

コンテナビルドが必要な場合:

```yaml
# values.yaml
containerMode:
  type: dind
```

```bash
helm upgrade arc-runner-set \
  --namespace arc-runners \
  --set githubConfigUrl="${GITHUB_CONFIG_URL}" \
  --set githubConfigSecret=github-app-secret \
  --set containerMode.type=dind \
  oci://ghcr.io/actions/actions-runner-controller-charts/gha-runner-scale-set
```

### カスタムRunnerイメージの使用

公式イメージに追加のツール（make, gcc等）をインストールしたカスタムイメージを使用できます。

#### 1. Dockerfileの作成

`runner-image/Dockerfile`:

```dockerfile
FROM ghcr.io/actions/actions-runner:latest

# rootに切り替えてパッケージインストール
USER root
RUN apt-get update && apt-get install -y \
    make \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# runnerユーザーに戻す（重要）
USER runner
WORKDIR /home/runner
```

#### 2. イメージのビルドとプッシュ

```bash
cd runner-image

# イメージをビルド
docker build -t ghcr.io/<your-username>/arc-runner:latest .

# ghcr.ioにログイン
echo $GITHUB_TOKEN | docker login ghcr.io -u <your-username> --password-stdin

# イメージをプッシュ
docker push ghcr.io/<your-username>/arc-runner:latest
```

> **Note**: ghcr.ioのイメージはデフォルトでprivateです。publicにするか、imagePullSecretsを設定してください。

#### 3. values.yamlの作成

`runner-image/values.yaml`:

```yaml
template:
  spec:
    containers:
      - name: runner
        image: ghcr.io/<your-username>/arc-runner:latest
        imagePullPolicy: Always
        command: ["/home/runner/run.sh"]
        securityContext:
          capabilities:
            add:
              - SYS_RESOURCE
```

> **重要**:
> - `command: ["/home/runner/run.sh"]` は必須です。これがないとコンテナが即座に終了します。
> - `SYS_RESOURCE` capabilityはOOMスコア調整の警告を抑制します（オプション）。

#### 4. カスタムイメージを適用

```bash
GITHUB_CONFIG_URL="https://github.com/<owner>/<repo>"

helm upgrade arc-runner-set \
  --namespace arc-runners \
  --set githubConfigUrl="${GITHUB_CONFIG_URL}" \
  --set githubConfigSecret=github-app-secret \
  -f runner-image/values.yaml \
  oci://ghcr.io/actions/actions-runner-controller-charts/gha-runner-scale-set
```

#### 5. 適用の確認

```bash
# AutoscalingRunnerSetの設定を確認
kubectl get autoscalingrunnerset arc-runner-set -n arc-runners -o yaml | grep -A 10 "containers:"
```

出力例:
```yaml
containers:
- command:
  - /home/runner/run.sh
  image: ghcr.io/<your-username>/arc-runner:latest
  imagePullPolicy: Always
  name: runner
  securityContext:
    capabilities:
      add:
      - SYS_RESOURCE
```

### 対象リポジトリの変更

Runnerを別のリポジトリで使用したい場合の手順です。

#### 1. GitHub Appのインストール先を更新

1. GitHub → Settings → Applications → 作成したGitHub App → 「Configure」
2. 「Repository access」で新しいリポジトリを追加
3. 「Save」をクリック

#### 2. Runner Scale Setを更新

```bash
# 新しいリポジトリURLを設定
GITHUB_CONFIG_URL="https://github.com/<owner>/<new-repo>"

# カスタムイメージを使用している場合
helm upgrade arc-runner-set \
  --namespace arc-runners \
  --set githubConfigUrl="${GITHUB_CONFIG_URL}" \
  --set githubConfigSecret=github-app-secret \
  -f runner-image/values.yaml \
  oci://ghcr.io/actions/actions-runner-controller-charts/gha-runner-scale-set

# 公式イメージを使用している場合
helm upgrade arc-runner-set \
  --namespace arc-runners \
  --set githubConfigUrl="${GITHUB_CONFIG_URL}" \
  --set githubConfigSecret=github-app-secret \
  oci://ghcr.io/actions/actions-runner-controller-charts/gha-runner-scale-set
```

#### 3. 確認

```bash
# 設定が更新されたか確認
kubectl get autoscalingrunnerset arc-runner-set -n arc-runners -o yaml | grep githubConfigUrl

# Listenerが再起動されるまで待機
kubectl get pods -n arc-systems -w
```

新しいリポジトリの Settings → Actions → Runners で「arc-runner-set」が表示されればOKです。

> **Note**: GitHub Appに新しいリポジトリへのアクセス権限がないと、Runnerが登録されません。

## トラブルシューティング

### Runnerが登録されない
```bash
# Controllerのログを確認
kubectl logs -n arc-systems -l app.kubernetes.io/name=gha-runner-scale-set-controller

# Secretを確認
kubectl get secrets -n arc-runners
```

### Podが起動しない
```bash
kubectl describe pod -n arc-runners <pod-name>
kubectl get events -n arc-runners
```

### GitHub Appの認証エラー

```bash
# Secretの内容を確認
kubectl get secret github-app-secret -n arc-runners -o yaml

# 必要なキーが存在するか確認
# - github_app_id
# - github_app_installation_id
# - github_app_private_key
```

- **App ID / Installation ID が間違っている**: GitHub App設定画面で再確認
- **Private Keyが無効**: 新しいキーを生成して再作成
- **権限不足**: Repository permissions で Administration: Read and write が必要

## クリーンアップ

```bash
# Runner Scale Setを削除
helm uninstall arc-runner-set -n arc-runners

# Controllerを削除
helm uninstall arc -n arc-systems

# Namespaceを削除
kubectl delete namespace arc-runners
kubectl delete namespace arc-systems
```

## 参考リンク

- [GitHub公式ドキュメント](https://docs.github.com/en/actions/hosting-your-own-runners/managing-self-hosted-runners-with-actions-runner-controller/quickstart-for-actions-runner-controller)
- [GitHub App認証](https://docs.github.com/en/actions/hosting-your-own-runners/managing-self-hosted-runners-with-actions-runner-controller/authenticating-to-the-github-api)
- [ARC GitHub リポジトリ](https://github.com/actions/actions-runner-controller)
- [ARC リリースノート](https://github.com/actions/actions-runner-controller/releases)
