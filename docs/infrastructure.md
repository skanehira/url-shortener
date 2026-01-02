# Production Infrastructure Architecture

## Overview

本番環境は数秒のダウンタイムを許容するHA構成で設計されています。

## System Architecture

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                              Kubernetes Cluster                                  │
│  ┌────────────────────────────────────────────────────────────────────────────┐  │
│  │                        url-shortener-prod namespace                        │  │
│  │                                                                            │  │
│  │  ┌──────────────────────────────────────────────────────────────────────┐  │  │
│  │  │                          Ingress Layer                               │  │  │
│  │  │                                                                      │  │  │
│  │  │   ┌────────────────────────────────────────────────────────────┐     │  │  │
│  │  │   │  Ingress (nginx)  url-shortener.example.com                │     │  │  │
│  │  │   │  - TLS termination                                         │     │  │  │
│  │  │   │  - Path-based routing                                      │     │  │  │
│  │  │   └────────────────────────────┬───────────────────────────────┘     │  │  │
│  │  │                                │                                     │  │  │
│  │  └────────────────────────────────┼─────────────────────────────────────┘  │  │
│  │                                   │                                        │  │
│  │  ┌────────────────────────────────┼─────────────────────────────────────┐  │  │
│  │  │                        Application Layer                             │  │  │
│  │  │                                ▼                                     │  │  │
│  │  │   ┌─────────────────────┐       ┌─────────────────────┐              │  │  │
│  │  │   │  shortener-service  │       │  analytics-service  │              │  │  │
│  │  │   │    (3 replicas)     │       │    (3 replicas)     │              │  │  │
│  │  │   │    PDB: min=1       │       │    PDB: min=1       │              │  │  │
│  │  │   │  ┌───┐ ┌───┐ ┌───┐  │       │  ┌───┐ ┌───┐ ┌───┐  │              │  │  │
│  │  │   │  │Pod│ │Pod│ │Pod│  │       │  │Pod│ │Pod│ │Pod│  │              │  │  │
│  │  │   │  └───┘ └───┘ └───┘  │       │  └───┘ └───┘ └───┘  │              │  │  │
│  │  │   └─────────┬───────────┘       └─────────┬───────────┘              │  │  │
│  │  │             │                             │                          │  │  │
│  │  └─────────────┼─────────────────────────────┼──────────────────────────┘  │  │
│  │                │                             │                             │  │
│  │  ┌─────────────┼─────────────────────────────┼──────────────────────────┐  │  │
│  │  │             │       Middleware Layer      │                          │  │  │
│  │  │             ▼                             ▼                          │  │  │
│  │  │  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐    │  │  │
│  │  │  │    PostgreSQL    │  │   RabbitMQ       │  │   Redis Sentinel │    │  │  │
│  │  │  │  (CloudNativePG) │  │   Cluster        │  │   (Spotahome)    │    │  │  │
│  │  │  │                  │  │                  │  │                  │    │  │  │
│  │  │  │  ┌────────────┐  │  │  ┌───┐┌───┐┌───┐ │  │  ┌─────────────┐ │    │  │  │
│  │  │  │  │  Primary   │  │  │  │ R ││ R ││ R │ │  │  │  Sentinel   │ │    │  │  │
│  │  │  │  └────────────┘  │  │  │ M ││ M ││ M │ │  │  │  (3 nodes)  │ │    │  │  │
│  │  │  │  ┌────────────┐  │  │  │ Q ││ Q ││ Q │ │  │  └─────────────┘ │    │  │  │
│  │  │  │  │  Replica   │  │  │  └───┘└───┘└───┘ │  │  ┌─────────────┐ │    │  │  │
│  │  │  │  └────────────┘  │  │                  │  │  │   Redis     │ │    │  │  │
│  │  │  │  ┌────────────┐  │  │                  │  │  │  (3 nodes)  │ │    │  │  │
│  │  │  │  │  Replica   │  │  │                  │  │  └─────────────┘ │    │  │  │
│  │  │  │  └────────────┘  │  │                  │  │                  │    │  │  │
│  │  │  └────────┬─────────┘  └────────┬─────────┘  └────────┬─────────┘    │  │  │
│  │  │           │                     │                     │              │  │  │
│  │  └───────────┼─────────────────────┼─────────────────────┼──────────────┘  │  │
│  │              │                     │                     │                 │  │
│  │  ┌───────────┼─────────────────────┼─────────────────────┼──────────────┐  │  │
│  │  │           ▼                     ▼                     ▼              │  │  │
│  │  │                         Storage Layer                                │  │  │
│  │  │                                                                      │  │  │
│  │  │   ┌─────────────────────────────────────────────────────────────┐    │  │  │
│  │  │   │                    Longhorn (Distributed Storage)           │    │  │  │
│  │  │   │                                                             │    │  │  │
│  │  │   │    ┌──────────┐     ┌──────────┐     ┌──────────┐           │    │  │  │
│  │  │   │    │  Node 1  │     │  Node 2  │     │  Node 3  │           │    │  │  │
│  │  │   │    │  ┌────┐  │     │  ┌────┐  │     │  ┌────┐  │           │    │  │  │
│  │  │   │    │  │ PV │  │ ◄─► │  │ PV │  │ ◄─► │  │ PV │  │           │    │  │  │
│  │  │   │    │  └────┘  │     │  └────┘  │     │  └────┘  │           │    │  │  │
│  │  │   │    └──────────┘     └──────────┘     └──────────┘           │    │  │  │
│  │  │   │                    (3-way replication)                      │    │  │  │
│  │  │   └─────────────────────────────────────────────────────────────┘    │  │  │
│  │  │                                                                      │  │  │
│  │  └──────────────────────────────────────────────────────────────────────┘  │  │
│  │                                                                            │  │
│  │  ┌──────────────────────────────────────────────────────────────────────┐  │  │
│  │  │                        Observability Layer                           │  │  │
│  │  │                                                                      │  │  │
│  │  │   ┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐    │  │  │
│  │  │   │  OTEL Collector │──►│     Jaeger      │──►│  Elasticsearch  │    │  │  │
│  │  │   │    (0.115.0)    │   │  (production)   │   │    (8.11.0)     │    │  │  │
│  │  │   │                 │   │   collector +   │   │                 │    │  │  │
│  │  │   │ traces, metrics │   │     query       │   │  trace storage  │    │  │  │
│  │  │   └─────────────────┘   └─────────────────┘   │   7 days TTL    │    │  │  │
│  │  │                                               └─────────────────┘    │  │  │
│  │  │                                                                      │  │  │
│  │  └──────────────────────────────────────────────────────────────────────┘  │  │
│  │                                                                            │  │
│  └────────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
└──────────────────────────────────────────────────────────────────────────────────┘
```

## Data Flow

```
                                 ┌─────────────────┐
                                 │     Ingress     │
                                 │  (nginx + TLS)  │
                                 └────────┬────────┘
                                          │
                                          ▼
┌──────────┐     ┌─────────────────────┐     ┌──────────────────┐
│  Client  │────►│  shortener-service  │────►│    PostgreSQL    │
└──────────┘     │                     │     │  (Primary: RW)   │
                 │  - Create short URL │     │  (Replica: RO)   │
                 │  - Redirect         │     └──────────────────┘
                 └──────────┬──────────┘
                            │
                            │ Publish AccessEvent
                            ▼
                 ┌──────────────────────┐
                 │      RabbitMQ        │
                 │                      │
                 │  Exchange: events    │
                 │  Queue: access_logs  │
                 └──────────┬───────────┘
                            │
                            │ Consume
                            ▼
                 ┌─────────────────────┐     ┌──────────────────┐
                 │  analytics-service  │────►│      Redis       │
                 │                     │     │  (via Sentinel)  │
                 │  - Count accesses   │     │                  │
                 │  - Store analytics  │     └──────────────────┘
                 └─────────────────────┘
```

## Component Specifications

### Application Services

| Service | Replicas | CPU Request | Memory Request | CPU Limit | Memory Limit | PDB |
|---------|----------|-------------|----------------|-----------|--------------|-----|
| shortener-service | 3 | 250m | 256Mi | 1000m | 1Gi | minAvailable: 1 |
| analytics-service | 3 | 250m | 256Mi | 1000m | 1Gi | minAvailable: 1 |

### Ingress

| Setting | Value |
|---------|-------|
| Controller | nginx-ingress |
| Host (prod) | url-shortener.example.com |
| Host (staging) | url-shortener.staging.example.com |
| Host (dev) | url-shortener.dev.example.com |
| TLS | Enabled (secretName: shortener-tls) |

### PostgreSQL (CloudNativePG)

| Setting | Value |
|---------|-------|
| Instances | 3 (1 Primary + 2 Replicas) |
| Storage | 10Gi (Longhorn) |
| CPU Request | 500m |
| Memory Request | 1Gi |
| CPU Limit | 2000m |
| Memory Limit | 2Gi |
| max_connections | 200 |
| shared_buffers | 256MB |

**Failover**: 自動フェイルオーバー (5-10秒)

### Redis (Sentinel)

| Component | Replicas | CPU Request | Memory Request |
|-----------|----------|-------------|----------------|
| Redis | 3 | 200m | 512Mi |
| Sentinel | 3 | 100m | 128Mi |

**Failover**: Sentinel による自動フェイルオーバー (数秒)

### RabbitMQ

| Setting | Value |
|---------|-------|
| Replicas | 3 (Quorum) |
| Storage | 20Gi (Longhorn) |
| CPU Request | 500m |
| Memory Request | 1Gi |
| CPU Limit | 1000m |
| Memory Limit | 2Gi |

**Failover**: Quorum queues による自動フェイルオーバー

### Observability

| Component | Version | Storage | Retention |
|-----------|---------|---------|-----------|
| OTEL Collector | 0.115.0 | - | - |
| Jaeger | 1.62.0 (production strategy) | Elasticsearch | 7 days |
| Elasticsearch | 8.11.0 | 10Gi (Longhorn) | - |

### Storage (Longhorn)

| Setting | Value |
|---------|-------|
| Replication Factor | 3 |
| Data Locality | best-effort |

## HA Guarantees

| Component | Failure Scenario | Recovery Time | Data Loss |
|-----------|------------------|---------------|-----------|
| PostgreSQL | Primary failure | 5-10 seconds | None (sync replication) |
| Redis | Master failure | 1-3 seconds | Minimal (async replication) |
| RabbitMQ | Node failure | Immediate | None (quorum queues) |
| App Pod | Pod crash | Immediate | None (stateless) |
| Storage Node | Node failure | Automatic | None (3-way replication) |
| Trace Data | Jaeger restart | None | None (Elasticsearch) |

## Network Architecture

```
                                 ┌─────────────────┐
                                 │    Ingress      │
                                 │  (nginx + TLS)  │
                                 │                 │
                                 │ url-shortener.  │
                                 │ example.com     │
                                 └────────┬────────┘
                                          │
                                          ▼
                               ┌──────────────────┐
                               │ shortener-service│
                               │    :8080         │
                               │  (ClusterIP)     │
                               └──────────────────┘
                                          │
                                          │
    ┌─────────────────────────────────────┼───────────────────────────────────┐
    │                                     │                                   │
    ▼                                     ▼                                   ▼
┌────────────┐                     ┌────────────┐                     ┌────────────┐
│url-        │                     │url-        │                     │otel-       │
│shortener-  │                     │shortener-  │                     │collector   │
│db-rw:5432  │                     │rabbitmq    │                     │:4317       │
│            │                     │:5672       │                     │            │
└────────────┘                     └─────┬──────┘                     └─────┬──────┘
 PostgreSQL                              │                                  │
                                         ▼                                  ▼
                               ┌──────────────────┐               ┌──────────────────┐
                               │ analytics-service│               │  jaeger-collector│
                               │    :8081         │               │     :4317        │
                               │  (ClusterIP)     │               └────────┬─────────┘
                               └────────┬─────────┘                        │
                                        │                                  ▼
                                        ▼                         ┌──────────────────┐
                               ┌──────────────────┐               │  elasticsearch   │
                               │rfs-url-shortener-│               │     :9200        │
                               │redis:26379       │               └──────────────────┘
                               └──────────────────┘
                                Redis Sentinel
```

## Operators Required

以下の Operator を事前にインストールする必要があります:

```bash
./k8s/helm/install-operators.sh
```

| Operator | Purpose | Namespace |
|----------|---------|-----------|
| Longhorn | Distributed storage | longhorn-system |
| CloudNativePG | PostgreSQL HA | cnpg-system |
| Redis Operator (Spotahome) | Redis Sentinel | redis-operator |
| RabbitMQ Cluster Operator | RabbitMQ HA | rabbitmq-system |
| SealedSecrets | Secrets encryption | kube-system |
| Jaeger Operator | Distributed tracing | observability |

## Deployment

```bash
# Production deployment
kubectl apply -k k8s/overlays/prod

# Verify
kubectl get pods -n url-shortener-prod
kubectl get cluster -n url-shortener-prod       # PostgreSQL
kubectl get redisfailover -n url-shortener-prod # Redis
kubectl get rabbitmqcluster -n url-shortener-prod # RabbitMQ
kubectl get jaeger -n url-shortener-prod        # Jaeger
kubectl get ingress -n url-shortener-prod       # Ingress
kubectl get pdb -n url-shortener-prod           # PodDisruptionBudgets
```

## Environment Comparison

| Component | Dev | Staging | Prod |
|-----------|-----|---------|------|
| App Replicas | 1 | 2 | 3 |
| PostgreSQL Instances | 1 | 2 | 3 |
| Redis Replicas | 1 | 2 | 3 |
| Redis Sentinel | 1 | 3 | 3 |
| RabbitMQ Replicas | 1 | 2 | 3 |
| Ingress Host | *.dev.example.com | *.staging.example.com | *.example.com |
