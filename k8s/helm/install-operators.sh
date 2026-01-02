#!/bin/bash
set -euo pipefail

echo "=== Installing Kubernetes Operators ==="

# Add Helm repositories
echo "[1/7] Adding Helm repositories..."
helm repo add longhorn https://charts.longhorn.io
helm repo add cnpg https://cloudnative-pg.github.io/charts
helm repo add redis-operator https://spotahome.github.io/redis-operator
helm repo add sealed-secrets https://bitnami-labs.github.io/sealed-secrets
helm repo add jaegertracing https://jaegertracing.github.io/helm-charts
helm repo update

# Install Longhorn
echo "[2/7] Installing Longhorn..."
helm upgrade --install longhorn longhorn/longhorn \
  --namespace longhorn-system \
  --create-namespace \
  --set defaultSettings.defaultDataPath="/var/lib/longhorn" \
  --wait

# Install CloudNativePG
echo "[3/7] Installing CloudNativePG Operator..."
helm upgrade --install cnpg cnpg/cloudnative-pg \
  --namespace cnpg-system \
  --create-namespace \
  --wait

# Install Redis Operator (Spotahome)
echo "[4/7] Installing Redis Operator..."
helm upgrade --install redis-operator redis-operator/redis-operator \
  --namespace redis-operator \
  --create-namespace \
  --wait

# Install RabbitMQ Cluster Operator
echo "[5/7] Installing RabbitMQ Cluster Operator..."
kubectl apply -f "https://github.com/rabbitmq/cluster-operator/releases/latest/download/cluster-operator.yml"

# Install SealedSecrets
echo "[6/7] Installing SealedSecrets Controller..."
helm upgrade --install sealed-secrets sealed-secrets/sealed-secrets \
  --namespace kube-system \
  --wait

# Install Jaeger Operator
echo "[7/7] Installing Jaeger Operator..."
kubectl create namespace observability --dry-run=client -o yaml | kubectl apply -f -
helm upgrade --install jaeger-operator jaegertracing/jaeger-operator \
  --namespace observability \
  --set rbac.clusterRole=true \
  --wait

echo ""
echo "=== All Operators Installed Successfully ==="
echo ""
echo "Installed components:"
echo "  - Longhorn (longhorn-system)"
echo "  - CloudNativePG (cnpg-system)"
echo "  - Redis Operator (redis-operator)"
echo "  - RabbitMQ Cluster Operator (rabbitmq-system)"
echo "  - SealedSecrets (kube-system)"
echo "  - Jaeger Operator (observability)"
echo ""
echo "Next steps:"
echo "  1. Verify operators are running: kubectl get pods -A | grep -E 'longhorn|cnpg|redis|rabbitmq|sealed|jaeger'"
echo "  2. Apply application manifests: kubectl apply -k k8s/overlays/dev"
