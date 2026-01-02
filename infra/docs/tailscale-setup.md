# Tailscale Kubernetes Operator セットアップガイド

Tailscale Kubernetes Operatorを使って、クラスタ内のサービスをTailnetに公開したり、インターネットに公開（Funnel）したりできます。

## 構成図

```
┌────────────────────────────────────────────────────────────────┐
│  Tailnet（プライベートネットワーク）                           │
│                                                                │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────┐  │
│  │  Mac         │    │  iPhone      │    │  Kubernetes      │  │
│  │  (Tailscale) │◄──►│  (Tailscale) │◄──►│  (Operator)      │  │
│  └──────────────┘    └──────────────┘    └──────────────────┘  │
│                                                                │
└────────────────────────────────────────────────────────────────┘
                                                    │
                                                    │ Funnel
                                                    ▼
                                            ┌──────────────┐
                                            │  Internet    │
                                            └──────────────┘
```

## できること

| 機能 | 説明 |
|------|------|
| **Tailnet公開** | クラスタ内サービスをTailnetからアクセス可能に |
| **Funnel** | クラスタ内サービスをインターネットに公開 |
| **Egress** | Pod からTailnet内の他デバイスにアクセス |

## 前提条件

- Kubernetesクラスタが稼働中
- Helmがインストール済み
- Tailscaleアカウント（無料プランでOK）

## 手順

### Step 1: Tailscaleアカウントの準備

1. [Tailscale](https://tailscale.com/) にサインアップ
2. Admin console → DNS → **MagicDNS** を有効化
3. Admin console → DNS → **HTTPS Certificates** を有効化

### Step 2: ACLポリシーの設定

Admin console → Access controls で以下を追加：

```json
{
  "tagOwners": {
    "tag:k8s-operator": [],
    "tag:k8s": ["tag:k8s-operator"]
  },
  "acls": [
    {"action": "accept", "src": ["*"], "dst": ["*:*"]}
  ],
  "nodeAttrs": [
    {
      "target": ["tag:k8s"],
      "attr": ["funnel"]
    }
  ]
}
```

### Step 3: OAuthクライアントの作成

1. Admin console → Settings → OAuth clients
2. 「Generate OAuth client」をクリック
3. 以下を設定：
   - **Description**: `k8s-operator`
   - **Scopes**:
     - Devices: Core (Read & Write)
     - Auth Keys (Read & Write)
   - **Tags**: `tag:k8s-operator`
4. **Client ID** と **Client Secret** をメモ

### Step 4: Tailscale Operatorのインストール

```bash
# Helm リポジトリを追加
helm repo add tailscale https://pkgs.tailscale.com/helmcharts
helm repo update

# Operatorをインストール
helm install tailscale-operator tailscale/tailscale-operator \
  --namespace tailscale \
  --create-namespace \
  --set oauth.clientId="<CLIENT_ID>" \
  --set oauth.clientSecret="<CLIENT_SECRET>" \
  --wait

# 確認
kubectl get pods -n tailscale
```

### Step 5: 動作確認

```bash
# Tailscale Admin consoleで確認
# Machines に「tailscale-operator」が表示されていればOK
```

## 使用例

### 例1: サービスをTailnetに公開

```yaml
apiVersion: v1
kind: Service
metadata:
  name: my-app
  annotations:
    tailscale.com/expose: "true"
spec:
  selector:
    app: my-app
  ports:
    - port: 80
```

→ `my-app.tailxxxxx.ts.net` でTailnetからアクセス可能

### 例2: Ingressでインターネットに公開（Funnel）

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: my-app-public
  annotations:
    tailscale.com/funnel: "true"
spec:
  ingressClassName: tailscale
  defaultBackend:
    service:
      name: my-app
      port:
        number: 80
  tls:
    - hosts:
        - my-app-public
```

→ `https://my-app-public.tailxxxxx.ts.net` でインターネットからアクセス可能

### 例3: Tailnet内のサービスにPodからアクセス（Egress）

```yaml
apiVersion: v1
kind: Service
metadata:
  name: remote-server
  annotations:
    tailscale.com/tailnet-fqdn: "my-server.tailxxxxx.ts.net"
spec:
  type: ExternalName
  externalName: placeholder  # Operatorが自動設定
```

## トラブルシューティング

### Operatorが起動しない
```bash
kubectl logs -n tailscale -l app.kubernetes.io/name=tailscale-operator
kubectl describe pod -n tailscale -l app.kubernetes.io/name=tailscale-operator
```

### サービスが公開されない
```bash
# Serviceのアノテーションを確認
kubectl get svc <service-name> -o yaml

# Operatorのログを確認
kubectl logs -n tailscale -l app.kubernetes.io/name=tailscale-operator
```

### ACLエラー
- Admin console で `tag:k8s-operator` と `tag:k8s` が正しく設定されているか確認
- Funnelを使う場合は `nodeAttrs` に `funnel` 属性が必要

## クリーンアップ

```bash
# Operatorを削除
helm uninstall tailscale-operator -n tailscale

# Namespaceを削除
kubectl delete namespace tailscale
```

## 参考リンク

- [Tailscale Kubernetes Operator](https://tailscale.com/kb/1236/kubernetes-operator)
- [Cluster Ingress（Funnel）](https://tailscale.com/kb/1439/kubernetes-operator-cluster-ingress)
- [Cluster Egress](https://tailscale.com/kb/1438/kubernetes-operator-cluster-egress)
