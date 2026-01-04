# CoreDNS 内部DNS設定ガイド

ローカルKubernetesクラスタで内部ドメイン（`*.k8s.local`）を使用するための設定ガイドです。

## 概要

```
┌─────────────────┐     ┌─────────────────────────────────────────────────┐
│       Mac       │     │              Kubernetes Cluster                 │
│                 │     │                                                 │
│  curl           │     │   ┌─────────────┐     ┌─────────────────────┐   │
│  url-shortener. │────►│   │   CoreDNS   │────►│   NGINX Gateway     │   │
│  staging.k8s.   │ DNS │   │             │     │   Fabric            │   │
│  local          │     │   │  k8s.local  │     │                     │   │
│                 │     │   │  ゾーン管理 │     │   192.168.x.x       │   │
│                 │     │   └─────────────┘     └─────────────────────┘   │
└─────────────────┘     └─────────────────────────────────────────────────┘
```

## 前提条件

- kubeadm で構築された Kubernetes クラスタ
- MetalLB が導入済み
- kubectl が Mac から実行可能

## Step 1: 前提確認

```bash
# MetalLB の IP レンジを確認
kubectl get ipaddresspool -n metallb-system -o yaml

# NGINX Gateway Fabric が入っているか確認（未インストールならエラー）
kubectl get pods -n nginx-gateway
```

## Step 2: NGINX Gateway Fabric インストール

既に導入済みの場合はスキップしてください。

```bash
# 1. Gateway API CRDs インストール (Kubernetes SIG 公式)
kubectl apply --server-side -f https://github.com/kubernetes-sigs/gateway-api/releases/download/v1.4.1/standard-install.yaml

# 2. NGINX Gateway Fabric CRDs インストール
kubectl apply --server-side -f https://raw.githubusercontent.com/nginx/nginx-gateway-fabric/v2.3.0/deploy/crds.yaml

# 3. NGINX Gateway Fabric Control Plane インストール
kubectl apply -f https://raw.githubusercontent.com/nginx/nginx-gateway-fabric/v2.3.0/deploy/default/deploy.yaml

# Control Plane Pod が起動するまで待機
kubectl wait --namespace nginx-gateway \
  --for=condition=ready pod \
  --selector=app.kubernetes.io/name=nginx-gateway \
  --timeout=120s

# 確認
kubectl get pods -n nginx-gateway
```

出力例:
```
NAME                             READY   STATUS    RESTARTS   AGE
nginx-gateway-5c765bc7b6-xxxxx   1/1     Running   0          30s
```

この時点では Control Plane のみが起動しています。Data Plane (実際の NGINX) は Gateway リソースを作成すると動的に起動します。

## Step 3: アプリケーションデプロイ & Gateway LoadBalancer IP 確認

Gateway リソースはアプリケーションマニフェスト (`k8s/overlays/staging`) で管理されています。

```bash
# Staging 環境をデプロイ
kubectl apply -k k8s/overlays/staging

# Data Plane Pod が起動するまで待機
kubectl wait --namespace url-shortener-staging \
  --for=condition=ready pod \
  --selector=app.kubernetes.io/name=shortener-gateway-nginx \
  --timeout=120s

# Gateway の LoadBalancer IP 確認
kubectl get svc -n url-shortener-staging shortener-gateway-nginx
```

出力例:
```
NAME                      TYPE           EXTERNAL-IP     PORT(S)
shortener-gateway-nginx   LoadBalancer   192.168.2.203   80:xxxxx/TCP
```

この `EXTERNAL-IP` を控えておきます。全てのサービスはこの Gateway を経由してアクセスします。

## Step 4: CoreDNS ConfigMap 編集

```bash
kubectl edit configmap coredns -n kube-system
```

既存の Corefile に以下のブロックを追加します（`.:53` ブロックの前に追加）:

```
k8s.local:53 {
    errors
    cache 30
    hosts {
        192.168.2.203 url-shortener.staging.k8s.local
        192.168.2.203 url-shortener.prod.k8s.local
        192.168.2.203 analytics.staging.k8s.local
        192.168.2.203 analytics.prod.k8s.local
        fallthrough
    }
}
```

※ `192.168.2.203` は Step 3 で確認した Gateway の EXTERNAL-IP に置き換えてください

### 編集後の Corefile 例

```
apiVersion: v1
kind: ConfigMap
metadata:
  name: coredns
  namespace: kube-system
data:
  Corefile: |
    k8s.local:53 {
        errors
        cache 30
        hosts {
            192.168.2.203 url-shortener.staging.k8s.local
            192.168.2.203 url-shortener.prod.k8s.local
            192.168.2.203 analytics.staging.k8s.local
            192.168.2.203 analytics.prod.k8s.local
            fallthrough
        }
    }
    .:53 {
        errors
        health {
           lameduck 5s
        }
        ready
        kubernetes cluster.local in-addr.arpa ip6.arpa {
           pods insecure
           fallthrough in-addr.arpa ip6.arpa
           ttl 30
        }
        prometheus :9153
        forward . /etc/resolv.conf {
           max_concurrent 1000
        }
        cache 30
        loop
        reload
        loadbalance
    }
```

## Step 5: CoreDNS 再起動

設定を反映するため CoreDNS を再起動します。

```bash
kubectl rollout restart deployment coredns -n kube-system

# 再起動完了を待機
kubectl rollout status deployment coredns -n kube-system
```

## Step 6: CoreDNS を外部公開

Mac から CoreDNS にアクセスできるようにするため、LoadBalancer Service を作成します。

```bash
# CoreDNS 外部公開用 Service を適用
kubectl apply -f infra/k8s/coredns-external-service.yaml

# LoadBalancer IP 確認
kubectl get svc -n kube-system coredns-external
```

出力例:
```
NAME               TYPE           CLUSTER-IP     EXTERNAL-IP     PORT(S)
coredns-external   LoadBalancer   10.x.x.x       192.168.2.201   53:xxxxx/UDP,53:xxxxx/TCP
```

この `EXTERNAL-IP` を控えておきます。

## Step 7: Mac 側の resolver 設定

Mac の DNS 解決設定を追加します。

```bash
# CoreDNS の LoadBalancer IP を確認
DNS_IP=$(kubectl get svc -n kube-system coredns-external -o jsonpath='{.status.loadBalancer.ingress[0].ip}')
echo "DNS IP: $DNS_IP"

# resolver ディレクトリ作成
sudo mkdir -p /etc/resolver

# k8s.local 用の resolver 設定
sudo tee /etc/resolver/k8s.local <<EOF
nameserver $DNS_IP
EOF

# 設定確認
cat /etc/resolver/k8s.local
```

## Step 8: 動作確認

### DNS 解決確認

```bash
# CoreDNS の LoadBalancer IP を取得
DNS_IP=$(kubectl get svc -n kube-system coredns-external -o jsonpath='{.status.loadBalancer.ingress[0].ip}')

# dig で確認
dig url-shortener.staging.k8s.local @$DNS_IP

# 期待する出力:
# ;; ANSWER SECTION:
# url-shortener.staging.k8s.local.    30    IN    A    192.168.2.203
```

### Mac からの名前解決確認

```bash
# Mac のローカル resolver 経由で確認
dscacheutil -q host -a name url-shortener.staging.k8s.local
```

期待する出力:
```
name: url-shortener.staging.k8s.local
ip_address: 192.168.2.203
```

### HTTP アクセス確認

テスト用の nginx をデプロイして、CoreDNS 経由でアクセスできることを確認します。

```bash
# テスト用 nginx Deployment をデプロイ
kubectl apply -f infra/example/nginx.yaml -n url-shortener-staging

# ClusterIP Service を作成
kubectl expose deployment nginx -n url-shortener-staging --port=80

# HTTPRoute を作成（Gateway 経由でルーティング）
kubectl apply -n url-shortener-staging -f - <<'EOF'
apiVersion: gateway.networking.k8s.io/v1
kind: HTTPRoute
metadata:
  name: nginx-test
spec:
  parentRefs:
    - name: staging-shortener-gateway
  hostnames:
    - "url-shortener.staging.k8s.local"
  rules:
    - matches:
        - path:
            type: PathPrefix
            value: /
      backendRefs:
        - name: nginx
          port: 80
EOF

# Pod が起動するまで待機
kubectl wait --namespace url-shortener-staging \
  --for=condition=ready pod \
  --selector=app=nginx \
  --timeout=60s

# アクセス確認
curl http://url-shortener.staging.k8s.local
```

期待する出力:
```html
<!DOCTYPE html>
<html>
<head>
<title>Welcome to nginx!</title>
...
```

確認後、テストリソースをクリーンアップ:

```bash
kubectl delete httproute nginx-test -n url-shortener-staging
kubectl delete svc nginx -n url-shortener-staging
kubectl delete deployment nginx -n url-shortener-staging
```

## トラブルシューティング

### CoreDNS ログ確認

```bash
kubectl logs -n kube-system -l k8s-app=kube-dns -f
```

### ConfigMap 確認

```bash
kubectl get configmap coredns -n kube-system -o yaml
```

### Mac の DNS キャッシュクリア

```bash
sudo dscacheutil -flushcache; sudo killall -HUP mDNSResponder
```

### resolver 設定確認

```bash
scutil --dns | grep -A 5 "k8s.local"
```

### CoreDNS Service 確認

```bash
# LoadBalancer IP が割り当てられているか確認
kubectl get svc -n kube-system coredns-external

# CoreDNS Pod が起動しているか確認
kubectl get pods -n kube-system -l k8s-app=kube-dns
```

### Gateway/HTTPRoute 確認

```bash
# Gateway 状態確認
kubectl get gateway -n url-shortener-staging

# HTTPRoute 状態確認
kubectl get httproute -n url-shortener-staging
```

## ドメイン追加方法

新しいドメインを追加する場合は、CoreDNS の ConfigMap を編集します。

```bash
kubectl edit configmap coredns -n kube-system
```

`hosts` ブロックに追加:

```
hosts {
    192.168.2.203 url-shortener.staging.k8s.local
    192.168.2.203 url-shortener.prod.k8s.local
    192.168.2.203 analytics.staging.k8s.local
    192.168.2.203 analytics.prod.k8s.local
    192.168.2.203 new-app.staging.k8s.local    # 追加
    fallthrough
}
```

編集後、CoreDNS を再起動:

```bash
kubectl rollout restart deployment coredns -n kube-system
```

## 次のステップ

手動管理から自動化に進む場合は、External DNS の導入を検討してください。
External DNS を使用すると、Gateway/HTTPRoute リソースの作成時に自動的に DNS レコードが登録されます。
