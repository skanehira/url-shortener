# ArgoCD GitOps セットアップガイド

GitHubリポジトリの変更を自動でKubernetesクラスタに反映するGitOpsパイプラインを構築します。

## 前提条件

- Kubernetesクラスタが稼働していること
- `kubectl`がクラスタに接続できること
- Homebrewがインストールされていること（Mac）

## 構成図

```
┌─────────────────┐                    ┌─────────────────┐
│     GitHub      │                    │    開発者        │
│   Repository    │◄───── git push ────│                 │
└────────┬────────┘                    └─────────────────┘
         │
         │ ポーリング（デフォルト3分）
         ▼
┌─────────────────┐                    ┌─────────────────┐
│     ArgoCD      │────── deploy ─────►│   Kubernetes    │
│                 │                    │    Cluster      │
└─────────────────┘                    └─────────────────┘
```

## 手順

### Step 1: ArgoCDのインストール

```bash
# namespaceを作成
kubectl create namespace argocd

# ArgoCDをインストール
kubectl apply -n argocd -f https://raw.githubusercontent.com/argoproj/argo-cd/stable/manifests/install.yaml

# 全Podが起動するまで待機（2-3分かかる）
kubectl wait --for=condition=Ready pods --all -n argocd --timeout=300s

# 起動確認
kubectl get pods -n argocd
```

### Step 2: ArgoCD CLIのインストール

```bash
brew install argocd
```

### Step 3: 初期パスワードの取得

```bash
# 初期adminパスワードを取得
kubectl -n argocd get secret argocd-initial-admin-secret \
  -o jsonpath="{.data.password}" | base64 -d; echo
```

出力されたパスワードをメモしておく。

### Step 4: ArgoCD UIへのアクセス

```bash
# ポートフォワード（バックグラウンドで実行）
kubectl port-forward svc/argocd-server -n argocd 8443:443 &

# ブラウザでアクセス
open https://localhost:8443
```

- ユーザー名: `admin`
- パスワード: Step 3で取得した値

### Step 5: ArgoCD CLIでログイン

```bash
# CLIでログイン（証明書警告をスキップ）
argocd login localhost:8443 --insecure

# ユーザー名とパスワードを入力
```

### Step 6: GitHubリポジトリを登録

**パブリックリポジトリの場合:**
```bash
argocd repo add https://github.com/skanehira/multipass-k8s.git
```

**プライベートリポジトリの場合:**
```bash
# GitHub Personal Access Token を使用
argocd repo add https://github.com/skanehira/multipass-k8s.git \
  --username <GitHubユーザー名> \
  --password <Personal Access Token>
```

登録確認:
```bash
argocd repo list
```

### Step 7: Applicationを作成

```bash
# Applicationマニフェストを適用
kubectl apply -f argocd/nginx-app.yaml
```

`argocd/nginx-app.yaml` の内容:
```yaml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: nginx-app
  namespace: argocd
spec:
  project: default
  source:
    repoURL: https://github.com/skanehira/multipass-k8s.git
    targetRevision: main
    path: deployment
  destination:
    server: https://kubernetes.default.svc
    namespace: default
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
    syncOptions:
      - CreateNamespace=true
```

### Step 8: 同期状態を確認

```bash
# CLIで確認
argocd app list
argocd app get nginx-app

# または kubectl で確認
kubectl get application -n argocd
```

## ポーリング間隔の変更（オプション）

デフォルトは3分間隔。短くしたい場合:

```bash
# ConfigMapを編集
kubectl edit configmap argocd-cm -n argocd
```

以下を追加:
```yaml
data:
  timeout.reconciliation: 60s  # 1分に変更
```

設定を反映:
```bash
kubectl rollout restart deployment argocd-repo-server -n argocd
```

## 動作確認

1. `deployment/nginx.yaml` を編集してGitHubにpush
2. 数分後（ポーリング間隔）にArgoCDが変更を検知
3. 自動でクラスタに反映される

```bash
# 同期状態を監視
argocd app get nginx-app --refresh

# 手動で即時同期したい場合
argocd app sync nginx-app
```

## よく使うコマンド

| コマンド | 説明 |
|---------|------|
| `argocd app list` | アプリ一覧 |
| `argocd app get <app>` | アプリ詳細 |
| `argocd app sync <app>` | 手動同期 |
| `argocd app history <app>` | デプロイ履歴 |
| `argocd app rollback <app> <id>` | ロールバック |
| `argocd app delete <app>` | アプリ削除 |

## トラブルシューティング

### Podが起動しない
```bash
kubectl describe pod -n argocd
kubectl logs -n argocd -l app.kubernetes.io/name=argocd-server
```

### 同期が失敗する
```bash
argocd app get nginx-app
# SYNCステータスとHEALTHを確認

# 詳細なエラーを確認
argocd app sync nginx-app --dry-run
```

### リポジトリに接続できない
```bash
argocd repo list
# CONNECTION STATUSを確認

# 再登録
argocd repo rm https://github.com/skanehira/multipass-k8s.git
argocd repo add https://github.com/skanehira/multipass-k8s.git
```

## クリーンアップ

```bash
# Applicationを削除
argocd app delete nginx-app

# ArgoCD自体を削除
kubectl delete -n argocd -f https://raw.githubusercontent.com/argoproj/argo-cd/stable/manifests/install.yaml
kubectl delete namespace argocd
```
