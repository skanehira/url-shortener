#!/bin/bash
set -euo pipefail

METALLB_VERSION="${METALLB_VERSION:-0.14.9}"
IP_RANGE="${1:-}"

usage() {
  cat <<EOF
Usage: $0 <IP_RANGE>

MetalLBをインストールし、L2モードで設定します。

Arguments:
  IP_RANGE  LoadBalancerに割り当てるIPアドレス範囲 (例: 192.168.2.200-192.168.2.250)

Environment Variables:
  METALLB_VERSION  MetalLBのバージョン (デフォルト: ${METALLB_VERSION})

Example:
  $0 192.168.2.200-192.168.2.250
EOF
  exit 1
}

if [[ -z "${IP_RANGE}" ]]; then
  usage
fi

echo "==> MetalLB v${METALLB_VERSION} をインストール中..."
kubectl apply -f "https://raw.githubusercontent.com/metallb/metallb/v${METALLB_VERSION}/config/manifests/metallb-native.yaml"

echo "==> MetalLB Podの起動を待機中..."
kubectl -n metallb-system wait --for=condition=Ready pod -l app=metallb --timeout=120s

echo "==> IPアドレスプールを設定中 (${IP_RANGE})..."
kubectl apply -f - <<EOF
apiVersion: metallb.io/v1beta1
kind: IPAddressPool
metadata:
  name: default-pool
  namespace: metallb-system
spec:
  addresses:
  - ${IP_RANGE}
---
apiVersion: metallb.io/v1beta1
kind: L2Advertisement
metadata:
  name: default
  namespace: metallb-system
spec:
  ipAddressPools:
  - default-pool
EOF

echo "==> MetalLBのインストールが完了しました"
echo ""
echo "確認:"
echo "  kubectl get pods -n metallb-system"
echo "  kubectl get ipaddresspools -n metallb-system"
