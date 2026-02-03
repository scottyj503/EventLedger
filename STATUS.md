# EventLedger - Status Document

> **Date:** 2025-01-09 (Updated: 2025-02-03)
> **Status:** Decisions Finalized / Ready to Build
> **Region:** us-west-2

---

## Decisions

| Decision | Choice | Date |
|----------|--------|------|
| **Product Name** | EventLedger | 2025-02-03 |
| **Target Market** | Bounded Context Bus | 2025-02-03 |
| **MVP Scope** | See below | 2025-02-03 |

### Target Market: Bounded Context Bus

- **Customer:** Engineering teams building microservices/DDD systems
- **Use case:** Internal CDC between bounded contexts
- **Value prop:** "Publish your bounded context changes. Subscribers bootstrap and follow."
- **Key differentiator:** Compaction + bootstrap capability via REST

### MVP Scope

| Include | Exclude (Later) |
|---------|-----------------|
| Create/delete/list streams | Multi-region replication |
| Publish single + batch events | Transactions, exactly-once |
| Create subscriptions, choose start point | Filters, wildcards |
| Poll, commit offset | Push/webhook delivery |
| Key-based compaction (automatic) | Custom compaction rules |
| Hot storage (DynamoDB) | Cold archive (S3) |
| API key authentication | IAM, OAuth, fine-grained |
| CloudWatch metrics | Custom dashboards, tracing |
| REST only | Rust, TypeScript SDKs |

### MVP API Surface

```
POST   /streams                              # Create stream
GET    /streams                              # List streams
DELETE /streams/:id                          # Delete stream

POST   /streams/:id/events                   # Publish event(s)

POST   /streams/:id/subscriptions            # Create subscription
GET    /streams/:id/subscriptions/:sub/poll  # Poll for events
POST   /streams/:id/subscriptions/:sub/commit # Commit offset
```

---

## Executive Summary

**EventLedger** fills the gap between **Apache Kafka** (powerful but operationally complex) and **AWS EventBridge** (simple but lacks log semantics). The vision is:

> **"Serverless EventBridge with built-in ordered, immutable log semantics and consumer offset tracking"**

---

## The Problem

### Kafka Gives You
- Ordered, immutable, append-only log
- Consumer-controlled offset tracking (pull model)
- Replay from any point in time
- Key-based compaction
- Partitioned parallelism

### Kafka Costs You
- Cluster management (brokers, ZooKeeper/KRaft)
- Partition planning and rebalancing
- Capacity planning
- Consumer group coordination complexity
- Operational expertise requirement

### EventBridge Gives You
- True serverless (pay-per-event, zero infrastructure)
- Native cloud integrations
- Simple pub/sub model

### EventBridge Lacks
- Ordered delivery guarantees
- Consumer offset tracking
- Replay/bootstrap from log (archives are bolted-on)
- Compaction concept
- Pull-based consumption model

### The Gap
No solution offers **Kafka's log semantics** with **EventBridge's operational simplicity**.

---

## Target Use Case

**CDC (Change Data Capture) for Bounded Contexts**

- Bounded contexts publish their data changes
- Subscribers consume and build local materialized views
- New subscribers can bootstrap from compacted state
- Full history available in cold archive for replay/audit

### The Bootstrap Problem

With EventBridge alone, new subscribers cannot rebuild local copies:

```
New "Shipping" service needs current state of all Orders:

With Kafka/Log:
1. Read from compacted topic (or offset 0)
2. Build materialized view
3. Continue tailing live events
✓ Self-service, decoupled

With EventBridge:
1. Subscribe to events (from NOW only)
2. Current state? Unknown
✗ Cannot bootstrap without querying source system
```

---

## Proposed Solution

### Core Capabilities
1. **Ordered, immutable log** per stream
2. **Consumer offset tracking** (pull-based)
3. **Key-based compaction** for latest-per-entity state
4. **Tiered storage**: Hot (DynamoDB) → Cold (S3)
5. **REST/HTTP API** (no Kafka client complexity)
6. **Hidden partitioning** (consumers don't manage partitions)
7. **Serverless** (Lambda + DynamoDB + S3)

### Retention Strategy

| Tier | Duration | Storage | Access | Purpose |
|------|----------|---------|--------|---------|
| **Hot** | 0-15 days (configurable) | DynamoDB | <10ms | Live operations |
| **Compacted** | Indefinite | DynamoDB | <100ms | Bootstrap new consumers |
| **Cold Archive** | Configurable (e.g., 365 days) | S3 | Seconds | Audit, replay, compliance |

---

## Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────────────┐
│                    API Gateway (HTTP API)                       │
│              /streams, /events, /subscriptions                  │
└──────────────────────────┬──────────────────────────────────────┘
                           │
          ┌────────────────┼────────────────┐
          ▼                ▼                ▼
  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
  │   Lambda:    │ │   Lambda:    │ │   Lambda:    │
  │   publish    │ │   poll       │ │   admin      │
  │   (Rust)     │ │   (Rust)     │ │   (Rust)     │
  └──────┬───────┘ └──────┬───────┘ └──────┬───────┘
         │                │                │
         ▼                ▼                ▼
  ┌─────────────────────────────────────────────────┐
  │                   DynamoDB                       │
  │            (Single Table Design)                 │
  │  Events │ Offsets │ Subscriptions │ Compacted   │
  └─────────────────────────────────────────────────┘
                           │
                           │ DynamoDB Streams
                           ▼
  ┌─────────────────────────────────────────────────┐
  │           Lambda: compactor (Rust)              │
  └──────────────────────┬──────────────────────────┘
                         │
                         ▼
  ┌─────────────────────────────────────────────────┐
  │                  S3 Bucket                       │
  │              (Cold Archive)                      │
  └─────────────────────────────────────────────────┘
```

### DynamoDB Single Table Design

| PK | SK | Purpose |
|----|-----|---------|
| `STREAM#orders` | `META` | Stream metadata |
| `STREAM#orders` | `SUB#shipping-svc` | Subscription config |
| `STREAM#orders#P0` | `SEQ#00000000000001` | Event in partition 0 |
| `STREAM#orders#SUB#shipping` | `OFFSET#P0` | Consumer offset |
| `STREAM#orders#COMPACT` | `KEY#order-123` | Compacted state |

### API Design

```
POST   /streams                              # Create stream
GET    /streams/:id                          # Get stream info
DELETE /streams/:id                          # Delete stream

POST   /streams/:id/events                   # Publish event(s)

POST   /streams/:id/subscriptions            # Create subscription
GET    /streams/:id/subscriptions/:sub/poll  # Poll for events
POST   /streams/:id/subscriptions/:sub/commit # Commit offset
DELETE /streams/:id/subscriptions/:sub       # Delete subscription

POST   /streams/:id/compact                  # Trigger compaction
GET    /streams/:id/compacted/:key           # Get compacted state
```

---

## Tech Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| **Language** | Rust | Cold starts (10-15ms), cost efficiency, performance |
| **Compute** | AWS Lambda | Serverless, scales to zero |
| **API** | API Gateway (HTTP API) | Low latency, cost effective |
| **Hot Storage** | DynamoDB | Serverless, single-digit ms latency |
| **Cold Storage** | S3 | Cheap, durable, lifecycle policies |
| **IaC** | OpenTofu/Terraform | S3 backend for state |
| **Region** | us-west-2 | User preference |

---

## Cost Analysis

### Lambda Runtime Comparison

| Runtime | Memory | Cold Start | Relative Execution Speed |
|---------|--------|------------|--------------------------|
| **Rust** | 128MB | 10-15ms | 1x (baseline) |
| **Node.js** | 256MB | 200-400ms | 2-3x slower |
| **Java+SnapStart** | 512MB | 200-500ms | 1.5-2x slower |

### Monthly Cost by Scale (Our Design - Rust)

| Scenario | Events/Month | Invocations/Month | Lambda | Other AWS | **Total** |
|----------|--------------|-------------------|--------|-----------|-----------|
| **Startup** | 3M | 18M | $300 | $35 | **$335** |
| **Growth** | 30M | 181M | $3,005 | $320 | **$3,325** |
| **Scale** | 300M | 1.8B | $30,050 | $2,950 | **$33,000** |

### Runtime Cost Comparison (Growth Scenario - 181M inv/month)

| Runtime | Monthly Cost | vs Rust |
|---------|--------------|---------|
| **Rust** | $3,325 | — |
| **Node.js** | $18,669 | 5.6x more |
| **Java+SnapStart** | $50,856 | 15.3x more |

**Annual savings with Rust vs Node.js at Growth scale: ~$184K**

---

## Competitive Landscape

### Feature Comparison (Feature-for-Feature)

| Feature | Our Design | MSK | Kinesis | Upstash | WarpStream |
|---------|------------|-----|---------|---------|------------|
| Ordered log | ✓ | ✓ | ✓ | ✓ | ✓ |
| Consumer offsets | ✓ | ✓ | ✓ (KCL) | ✓ | ✓ |
| Key-based compaction | ✓ | ✓ | **✗** | ✓ | ✓ |
| Cold archive (S3) | ✓ | Add-on | Add-on | **✗** | ✓ |
| Bootstrap from compacted | ✓ | ✓ | **✗** | ✓ | ✓ |
| REST/HTTP API | ✓ | Add-on | ~ | ✓ | Add-on |
| Hidden partitioning | ✓ | ✗ | ✗ | ✗ | ✗ |
| Serverless (scale to zero) | ✓ | ✗ | ~ | ✓ | ✗ |

### Cost Comparison (Growth Scenario - Feature Equivalent)

| Solution | Base Cost | Add-ons Needed | True Total | Can Match Features? |
|----------|-----------|----------------|------------|---------------------|
| **Our Design (Rust)** | $3,325 | — | **$3,325** | ✓ Baseline |
| MSK Serverless | $965 | +$300 (Connect+REST) | **$1,265** | ✓ Yes |
| MSK Provisioned | $1,650 | +$300 | **$1,950** | ✓ Yes |
| WarpStream | $1,100 | +$200 (Agents+REST) | **$1,300** | ✓ Yes |
| Kinesis | $450 | +$100 (Firehose) | **$550** | **✗ No compaction** |
| Upstash | $240 | — | **$240** | ~ No S3 archive |

**Note:** Our design is 2-3x more expensive than MSK/WarpStream when feature-equivalent, but provides simpler API, no Kafka ops, and full control.

---

## Blue Ocean Analysis

### Red Oceans (Avoid)
- Enterprise streaming (Confluent, AWS, Azure dominate)
- High-throughput pipelines (commoditizing)
- General event routing (cloud vendors bundle cheap)

### Potential Blue Oceans

#### 1. "Customer Event Feeds as a Service" ⭐ Top Recommendation
**Target:** B2B SaaS companies wanting to offer event APIs to customers

- Pain: Webhooks are unreliable, customers miss events, no replay
- Current: Everyone builds custom or tells customers "sorry, event is gone"
- Value prop: "Give your customers Kafka-quality event feeds via REST"
- Market: Every B2B SaaS with integration needs
- Differentiation: Multi-tenant, REST-native, replay built-in

#### 2. "Bounded Context Event Bus" ⭐ SELECTED
**Target:** DDD/Microservices teams doing CDC between contexts

- Pain: New services can't bootstrap, Kafka is overkill
- Current: Custom outbox patterns, ad-hoc sync
- Value prop: "Publish your bounded context changes. Subscribers bootstrap and follow."
- Differentiation: Compaction + bootstrap is killer feature

#### 3. "Kafka for the REST of Us"
**Target:** Teams needing log semantics without Kafka expertise

- Pain: Kafka learning curve, operational burden
- Value prop: "Log semantics without the learning curve"

#### 4. "Serverless Event Sourcing Backbone"
**Target:** Event sourcing practitioners on serverless

- Pain: EventStoreDB isn't serverless, DynamoDB streams only 24h retention
- Value prop: "Serverless event store with built-in snapshots"

#### 5. "Compliant Audit Log Infrastructure"
**Target:** Regulated industries (fintech, healthtech)

- Pain: 7-year retention, hot/cold tiering, tamper-evidence
- Value prop: "Immutable event log with configurable retention tiers"
- Premium: High willingness to pay

### Blue Ocean Scoring

| Segment | Market Size | Competition | Fit | WTP | Score |
|---------|-------------|-------------|-----|-----|-------|
| Customer Event Feeds | Large | Low | High | High | ⭐⭐⭐⭐⭐ |
| Bounded Context Bus | Medium | Low | Very High | Medium-High | ⭐⭐⭐⭐ |
| Kafka for REST of us | Large | Medium | High | Medium | ⭐⭐⭐ |
| Serverless Event Sourcing | Small | Low | Very High | Medium | ⭐⭐⭐ |
| Compliant Audit Logs | Medium | Medium | High | Very High | ⭐⭐⭐ |

---

## Product Evolution Path

```
Phase 1: Bounded Context Bus        ← CURRENT
   │     (Prove model, MVP)
   ▼
Phase 2: Cold Archive + SDKs
   │     (S3 tiering, Rust/TS SDKs)
   ▼
Phase 3: Multi-tenancy Primitives
   │     (Tenant isolation, quotas)
   ▼
Phase 4: Customer Event Feeds
   │     (Portal, usage billing)
   ▼
Phase 5: Compliance Add-ons
   │     (WORM, audit exports, retention policies)
   ▼
Phase 6: Platform Play
         (Marketplace of sources/sinks)
```

---

## Project Structure

```
eventledger/                        # (project-events → eventledger)
├── STATUS.md                       # This document
├── .gitignore
│
├── infra/                          # OpenTofu/Terraform
│   ├── main.tf
│   ├── variables.tf
│   ├── outputs.tf
│   ├── modules/
│   │   ├── api/                    # API Gateway
│   │   ├── lambdas/                # Lambda definitions
│   │   └── dynamodb/               # Table + GSIs
│   └── environments/
│       ├── dev/
│       └── prod/
│
├── lambdas/                        # Rust workspace
│   ├── Cargo.toml                  # Workspace root
│   ├── shared/                     # Shared library (eventledger-core)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── models.rs
│   │       ├── dynamo.rs
│   │       ├── partitioner.rs
│   │       └── errors.rs
│   ├── publish/                    # eventledger-publish
│   ├── poll/                       # eventledger-poll
│   ├── admin/                      # eventledger-admin
│   └── compactor/                  # eventledger-compactor
│
├── tests/                          # Integration tests
├── examples/                       # Producer/consumer examples
└── justfile                        # Task runner
```

---

## Open Questions

1. ~~**Market validation:** Which blue ocean segment to prioritize?~~ → **Bounded Context Bus**
2. ~~**Naming:** Product name that captures the value prop?~~ → **EventLedger**
3. **Pricing model:** Per-event? Per-GB? Tiered? *(deferred)*
4. **Multi-region:** Required for MVP or later phase? *(deferred — not in MVP)*
5. **Kafka compatibility:** Worth adding protocol support or stay REST-only? *(deferred — REST only for MVP)*

---

## Next Steps

### Phase 1: Foundation
- [ ] Initialize Rust workspace with cargo-lambda
- [ ] Set up OpenTofu structure with S3 backend
- [ ] Create DynamoDB table (single-table design)
- [ ] Create API Gateway (HTTP API)

### Phase 2: Core Lambdas
- [ ] Shared library (models, DynamoDB ops, partitioner, errors)
- [ ] Publish Lambda
- [ ] Poll Lambda
- [ ] Commit Lambda
- [ ] Admin Lambda (streams + subscriptions CRUD)

### Phase 3: Compaction
- [ ] DynamoDB Streams trigger
- [ ] Compactor Lambda

### Phase 4: Validation
- [ ] Integration tests
- [ ] Example producer/consumer
- [ ] Documentation

---

## References

- [Kafka Log Compaction](https://kafka.apache.org/documentation/#compaction)
- [DynamoDB Single Table Design](https://www.alexdebrie.com/posts/dynamodb-single-table/)
- [cargo-lambda](https://www.cargo-lambda.info/)
- [Blue Ocean Strategy](https://www.blueoceanstrategy.com/)

---

## Changelog

| Date | Update |
|------|--------|
| 2025-01-09 | Initial discovery and architecture design |
| 2025-02-03 | Finalized decisions: Product name (EventLedger), Target market (Bounded Context Bus), MVP scope |
