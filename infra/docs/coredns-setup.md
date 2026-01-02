# CoreDNS 内部DNS設定ガイド

ローカルKubernetesクラスタで内部ドメイン（`*.k8s.local`）を使用するための設定ガイドです。

## 概要

```
┌─────────────────┐     ┌─────────────────────────────────────────────────┐
│       Mac       │     │              Kubernetes Cluster                 │
│                 │     │                                                 │
│  curl           │     │   ┌─────────────┐     ┌─────────────────────┐   │
│  staging.k8s.   │────►│   │   CoreDNS   │────►│   Ingress Controller│   │
│  local          │ DNS │   │             │     │   (nginx)           │   │
│                 │     │   │  k8s.local  │     │                     │   │
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

# nginx-ingress が入っているか確認
kubectl get pods -n ingress-nginx
```

## Step 2: nginx-ingress Controller インストール

既に導入済みの場合はスキップしてください。

```bash
kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/controller-v1.12.0/deploy/static/provider/cloud/deploy.yaml

# Pod が起動するまで待機
kubectl wait --namespace ingress-nginx \
  --for=condition=ready pod \
  --selector=app.kubernetes.io/component=controller \
  --timeout=120s

# LoadBalancer IP 確認
kubectl get svc -n ingress-nginx ingress-nginx-controller
```

出力例:
```
NAME                       TYPE           EXTERNAL-IP     PORT(S)
ingress-nginx-controller   LoadBalancer   192.168.2.200   80:xxxxx/TCP,443:xxxxx/TCP
```

この `EXTERNAL-IP` を控えておきます。

## Step 3: CoreDNS ConfigMap 編集

```bash
kubectl edit configmap coredns -n kube-system
```

既存の Corefile に以下のブロックを追加します（`.:53` ブロックの前に追加）:

```
k8s.local:53 {
    errors
    cache 30
    hosts {
        192.168.2.200 staging.k8s.local
        192.168.2.200 prod.k8s.local
        fallthrough
    }
}
```

※ `192.168.2.200` は Step 2 で確認した EXTERNAL-IP に置き換えてください

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
            192.168.2.200 staging.k8s.local
            192.168.2.200 prod.k8s.local
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

## Step 4: CoreDNS 再起動

設定を反映するため CoreDNS を再起動します。

```bash
kubectl rollout restart deployment coredns -n kube-system

# 再起動完了を待機
kubectl rollout status deployment coredns -n kube-system
```

## Step 5: CoreDNS を外部公開

Mac から CoreDNS にアクセスできるようにします。

### 方法A: 既存の kube-dns を NodePort 化

```bash
kubectl patch svc kube-dns -n kube-system -p '{"spec": {"type": "NodePort", "ports": [{"name": "dns", "port": 53, "protocol": "UDP", "nodePort": 30053}, {"name": "dns-tcp", "port": 53, "protocol": "TCP", "nodePort": 30053}]}}'
```

### 方法B: 専用 Service 作成（推奨）

```bash
kubectl apply -f - <<EOF
apiVersion: v1
kind: Service
metadata:
  name: coredns-external
  namespace: kube-system
spec:
  type: NodePort
  selector:
    k8s-app: kube-dns
  ports:
    - name: dns-udp
      port: 53
      targetPort: 53
      protocol: UDP
      nodePort: 30053
    - name: dns-tcp
      port: 53
      targetPort: 53
      protocol: TCP
      nodePort: 30053
EOF
```

## Step 6: Mac 側の resolver 設定

Mac の DNS 解決設定を追加します。

```bash
# Master VM の IP を確認
MASTER_IP=$(multipass info k8s-master --format json | jq -r '.info["k8s-master"].ipv4[0]')
echo "Master IP: $MASTER_IP"

# resolver ディレクトリ作成
sudo mkdir -p /etc/resolver

# k8s.local 用の resolver 設定
sudo tee /etc/resolver/k8s.local <<EOF
nameserver $MASTER_IP
port 30053
EOF

# 設定確認
cat /etc/resolver/k8s.local
```

## Step 7: 動作確認

### DNS 解決確認

```bash
# dig で確認
dig staging.k8s.local @$(multipass info k8s-master --format json | jq -r '.info["k8s-master"].ipv4[0]') -p 30053

# 期待する出力:
# ;; ANSWER SECTION:
# staging.k8s.local.    30    IN    A    192.168.2.200
```

### Mac からの名前解決確認

```bash
# Mac のローカル resolver 経由で確認
dscacheutil -q host -a name staging.k8s.local

# ping で確認
ping -c 1 staging.k8s.local
```

### Ingress 経由のアクセス確認

Ingress リソースを作成してテストします。

```bash
# テスト用 Deployment と Service
kubectl create deployment nginx --image=nginx
kubectl expose deployment nginx --port=80

# テスト用 Ingress
kubectl apply -f - <<EOF
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: nginx-test
spec:
  ingressClassName: nginx
  rules:
    - host: staging.k8s.local
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: nginx
                port:
                  number: 80
EOF

# アクセス確認
curl http://staging.k8s.local

# クリーンアップ
kubectl delete ingress nginx-test
kubectl delete svc nginx
kubectl delete deployment nginx
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

### NodePort 接続確認

```bash
# Master VM で直接確認
multipass exec k8s-master -- dig staging.k8s.local @127.0.0.1 -p 53
```

## ドメイン追加方法

新しいドメインを追加する場合は、CoreDNS の ConfigMap を編集します。

```bash
kubectl edit configmap coredns -n kube-system
```

`hosts` ブロックに追加:

```
hosts {
    192.168.2.200 staging.k8s.local
    192.168.2.200 prod.k8s.local
    192.168.2.200 new-app.k8s.local    # 追加
    fallthrough
}
```

編集後、CoreDNS を再起動:

```bash
kubectl rollout restart deployment coredns -n kube-system
```

## 次のステップ

手動管理から自動化に進む場合は、External DNS の導入を検討してください。
External DNS を使用すると、Ingress リソースの作成時に自動的に DNS レコードが登録されます。
