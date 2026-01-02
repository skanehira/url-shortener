# Secrets Management

This directory contains SealedSecrets for the url-shortener application.

## Prerequisites

Install `kubeseal` CLI:

```bash
# macOS
brew install kubeseal

# Linux
wget https://github.com/bitnami-labs/sealed-secrets/releases/download/v0.24.0/kubeseal-0.24.0-linux-amd64.tar.gz
tar -xvzf kubeseal-0.24.0-linux-amd64.tar.gz
sudo mv kubeseal /usr/local/bin/
```

## Generating Sealed Secrets

### 1. PostgreSQL Credentials

```bash
kubectl create secret generic postgres-credentials \
  --namespace url-shortener \
  --from-literal=username=urlshortener \
  --from-literal=password='YOUR_PASSWORD_HERE' \
  --dry-run=client -o yaml | \
  kubeseal --format yaml > sealed-postgres-credentials.yaml
```

### 2. RabbitMQ Credentials

```bash
kubectl create secret generic rabbitmq-credentials \
  --namespace url-shortener \
  --from-literal=username=urlshortener \
  --from-literal=password='YOUR_PASSWORD_HERE' \
  --dry-run=client -o yaml | \
  kubeseal --format yaml > sealed-rabbitmq-credentials.yaml
```

### 3. Application Secrets

```bash
kubectl create secret generic url-shortener-secrets \
  --namespace url-shortener \
  --from-literal=database-url='postgres://urlshortener:PASSWORD@url-shortener-db-rw:5432/urlshortener' \
  --from-literal=redis-url='redis+sentinel://rfs-url-shortener-redis:26379/mymaster' \
  --from-literal=rabbitmq-url='amqp://urlshortener:PASSWORD@url-shortener-rabbitmq:5672/' \
  --dry-run=client -o yaml | \
  kubeseal --format yaml > sealed-url-shortener-secrets.yaml
```

## Notes

- SealedSecrets are cluster-specific. If you move to a new cluster, you need to regenerate them.
- Never commit unencrypted secrets to version control.
- The sealed secrets in this directory are encrypted and safe to commit.
