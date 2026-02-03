# EventLedger

**Serverless event log with Kafka semantics and EventBridge simplicity.**

EventLedger provides ordered, immutable event streams with consumer offset tracking, key-based compaction, and bootstrap capability—all via a simple REST API, no Kafka expertise required.

## Features

- **Ordered, immutable log** per stream
- **Consumer offset tracking** (pull-based)
- **Key-based compaction** for latest-per-entity state
- **Bootstrap from compacted** for new subscribers
- **REST/HTTP API** (no Kafka client needed)
- **Serverless** (Lambda + DynamoDB + API Gateway)

## Prerequisites

- **Rust** - `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **cargo-lambda** - `cargo install cargo-lambda`
- **just** - `cargo install just` (command runner, like make)
- **OpenTofu** - [opentofu.org/docs/intro/install](https://opentofu.org/docs/intro/install/)
- **AWS CLI** - configured with credentials

## Quick Start

```bash

# 1. Build
just build-lambda

# 2. Bootstrap state backend (once)
just bootstrap-init && just bootstrap-apply

# 3. Deploy
just tf-init && just deploy

# 4. Get API URL
just tf-output
```

## API

### Streams

```bash
# Create stream
curl -X POST $API_URL/streams \
  -H "Content-Type: application/json" \
  -d '{"stream_id": "orders", "partition_count": 3}'

# List streams
curl $API_URL/streams

# Delete stream
curl -X DELETE $API_URL/streams/orders
```

### Events

```bash
# Publish event
curl -X POST $API_URL/streams/orders/events \
  -H "Content-Type: application/json" \
  -d '{"key": "order-123", "type": "order.created", "data": {"total": 99.99}}'

# Publish batch
curl -X POST $API_URL/streams/orders/events \
  -H "Content-Type: application/json" \
  -d '{"events": [{"key": "order-1", "type": "order.created", "data": {}}]}'
```

### Subscriptions

```bash
# Create subscription
curl -X POST $API_URL/streams/orders/subscriptions \
  -H "Content-Type: application/json" \
  -d '{"subscription_id": "shipping-service", "start_from": "earliest"}'

# Poll for events
curl "$API_URL/streams/orders/subscriptions/shipping-service/poll?limit=100"

# Commit offset
curl -X POST $API_URL/streams/orders/subscriptions/shipping-service/commit \
  -H "Content-Type: application/json" \
  -d '{"cursor": "eyJv..."}'
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    API Gateway (HTTP API)                       │
└──────────────────────────┬──────────────────────────────────────┘
                           │
          ┌────────────────┼────────────────┐
          ▼                ▼                ▼
  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
  │   Lambda:    │ │   Lambda:    │ │   Lambda:    │
  │   admin      │ │   publish    │ │   poll       │
  └──────┬───────┘ └──────┬───────┘ └──────┬───────┘
         │                │                │
         └────────────────┼────────────────┘
                          ▼
  ┌─────────────────────────────────────────────────┐
  │                   DynamoDB                       │
  │            (Single Table Design)                 │
  └─────────────────────────────────────────────────┘
                          │ DynamoDB Streams
                          ▼
  ┌─────────────────────────────────────────────────┐
  │           Lambda: compactor                      │
  └─────────────────────────────────────────────────┘
```

## Development

```bash
just --list          # Show all commands
just test            # Run unit tests
just check           # Check code
just fmt             # Format code
just lint            # Lint code
```

## Testing

```bash
# Unit tests
just test

# Integration tests (requires deployed API)
export EVENTLEDGER_API_URL=https://xxx.execute-api.us-west-2.amazonaws.com
cd tests/integration && cargo test
```

## Project Structure

```
eventledger/
├── lambdas/                    # Rust Lambda functions
│   ├── shared/                 # Core library
│   ├── admin/                  # Stream/subscription CRUD
│   ├── publish/                # Event publishing
│   ├── poll/                   # Poll and commit
│   └── compactor/              # DynamoDB Streams processor
├── infra/                      # OpenTofu/Terraform
│   ├── modules/                # Reusable modules
│   └── environments/           # Dev/prod configs
├── schemas/                    # JSON schemas
└── tests/integration/          # Integration tests
```

## License

MIT
