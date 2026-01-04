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
│  │  │                          Gateway Layer                              │  │  │
│  │  │                                                                      │  │  │
│  │  │   ┌────────────────────────────────────────────────────────────┐     │  │  │
│  │  │   │  Gateway API + NGINX Gateway Fabric                        │     │  │  │
│  │  │   │  - HTTPRoute: url-shortener.prod.k8s.local                 │     │  │  │
│  │  │   │  - Host-based routing                                      │     │  │  │
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
│  │  │   ┌─────────────────┐                         ┌─────────────────┐    │  │  │
│  │  │   │  OTEL Collector │────────────────────────►│  Elasticsearch  │    │  │  │
│  │  │   │    (0.115.0)    │                         │    (8.11.0)     │    │  │  │
│  │  │   │                 │                         │                 │    │  │  │
│  │  │   │ traces, metrics │                         │  trace storage  │    │  │  │
│  │  │   │      logs       │                         │   7 days TTL    │    │  │  │
│  │  │   └─────────────────┘                         └─────────────────┘    │  │  │
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
                                 │  Gateway API    │
                                 │  (HTTPRoute)    │
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

### Gateway API

| Setting | Value |
|---------|-------|
| Controller | NGINX Gateway Fabric |
| GatewayClass | nginx |
| Host (prod) | url-shortener.prod.k8s.local |
| Host (staging) | url-shortener.staging.k8s.local |

### PostgreSQL (CloudNativePG)

| Setting | Value |
|---------|-------|
| Instances | 2 (1 Primary + 1 Replica) |
| Storage | 3Gi (Longhorn) |
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
| Storage | 3Gi (Longhorn) |
| CPU Request | 500m |
| Memory Request | 1Gi |
| CPU Limit | 1000m |
| Memory Limit | 2Gi |

**Failover**: Quorum queues による自動フェイルオーバー

### Observability

| Component | Version | Storage | Retention |
|-----------|---------|---------|-----------|
| OTEL Collector | 0.141.0 | - | - |
| Elasticsearch | 8.11.0 | 3Gi (Longhorn) | 7 days |

### Storage (Longhorn)

| Setting | Value |
|---------|-------|
| Replication Factor | 1 |
| Data Locality | best-effort |

## HA Guarantees

| Component | Failure Scenario | Recovery Time | Data Loss |
|-----------|------------------|---------------|-----------|
| PostgreSQL | Primary failure | 5-10 seconds | None (sync replication) |
| Redis | Master failure | 1-3 seconds | Minimal (async replication) |
| RabbitMQ | Node failure | Immediate | None (quorum queues) |
| App Pod | Pod crash | Immediate | None (stateless) |
| Storage Node | Node failure | Automatic | None (3-way replication) |
| Trace Data | OTEL restart | None | None (Elasticsearch) |

## Network Architecture

```
                                 ┌─────────────────┐
                                 │  Gateway API    │
                                 │  (HTTPRoute)    │
                                 │                 │
                                 │ url-shortener.  │
                                 │ prod.k8s.local  │
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
                               │ analytics-service│               │  elasticsearch   │
                               │    :8081         │               │     :9200        │
                               │  (ClusterIP)     │               └──────────────────┘
                               └────────┬─────────┘
                                        │
                                        ▼
                               ┌──────────────────┐
                               │rfs-url-shortener-│
                               │redis:26379       │
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
| Gateway API CRDs | Gateway API resources | - |
| NGINX Gateway Fabric | Gateway controller | nginx-gateway |
| Longhorn | Distributed storage | longhorn-system |
| CloudNativePG | PostgreSQL HA | cnpg-system |
| Redis Operator (Spotahome) | Redis Sentinel | redis-operator |
| RabbitMQ Cluster Operator | RabbitMQ HA | rabbitmq-system |
| SealedSecrets | Secrets encryption | kube-system |

## Deployment

```bash
# Production deployment
kubectl apply -k k8s/overlays/prod

# Verify
kubectl get pods -n url-shortener-prod
kubectl get gateway -n url-shortener-prod       # Gateway
kubectl get httproute -n url-shortener-prod     # HTTPRoute
kubectl get cluster -n url-shortener-prod       # PostgreSQL
kubectl get redisfailover -n url-shortener-prod # Redis
kubectl get rabbitmqcluster -n url-shortener-prod # RabbitMQ
kubectl get pdb -n url-shortener-prod           # PodDisruptionBudgets
```

## Environment Comparison

| Component | Staging | Prod |
|-----------|---------|------|
| App Replicas | 1 | 3 |
| PostgreSQL Instances | 1 | 3 |
| Redis Replicas | 1 | 3 |
| Redis Sentinel | 1 | 3 |
| RabbitMQ Replicas | 1 | 3 |
| HTTPRoute Host | url-shortener.staging.k8s.local | url-shortener.prod.k8s.local |
