#!/bin/bash
set -euo pipefail

echo "=== Installing Kubernetes Operators ==="

# Add Helm repositories
echo "[1/6] Adding Helm repositories..."
helm repo add longhorn https://charts.longhorn.io
helm repo add cnpg https://cloudnative-pg.github.io/charts
helm repo add redis-operator https://spotahome.github.io/redis-operator
helm repo add sealed-secrets https://bitnami-labs.github.io/sealed-secrets
helm repo update

# Install Longhorn
echo "[2/6] Installing Longhorn..."
helm upgrade --install longhorn longhorn/longhorn \
  --namespace longhorn-system \
  --create-namespace \
  --set defaultSettings.defaultDataPath="/var/lib/longhorn" \
  --wait

# Install CloudNativePG
echo "[3/6] Installing CloudNativePG Operator..."
helm upgrade --install cnpg cnpg/cloudnative-pg \
  --namespace cnpg-system \
  --create-namespace \
  --wait

# Install Redis Operator (Spotahome)
echo "[4/6] Installing Redis Operator..."
helm upgrade --install redis-operator redis-operator/redis-operator \
  --namespace redis-operator \
  --create-namespace \
  --wait

# Install RabbitMQ Cluster Operator
echo "[5/6] Installing RabbitMQ Cluster Operator..."
kubectl apply -f "https://github.com/rabbitmq/cluster-operator/releases/latest/download/cluster-operator.yml"

# Install SealedSecrets
echo "[6/6] Installing SealedSecrets Controller..."
helm upgrade --install sealed-secrets sealed-secrets/sealed-secrets \
  --namespace kube-system \
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
echo ""
echo "Next steps:"
echo "  1. Verify operators are running: kubectl get pods -A | grep -E 'longhorn|cnpg|redis|rabbitmq|sealed'"
echo "  2. Apply application manifests: kubectl apply -k k8s/overlays/staging"
