# ACTORIS Free Cloud Deployment Guide

## Free Tier Strategy

### Option 1: Railway.app (Recommended for Quick Start)
- **Cost**: Free tier with $5/month credit
- **Services**: Containers, PostgreSQL, Redis
- **Limits**: 500 execution hours/month

### Option 2: Render.com
- **Cost**: Free tier available
- **Services**: Web services, static sites, PostgreSQL, Redis
- **Limits**: 750 hours/month, spins down after inactivity

### Option 3: Fly.io
- **Cost**: Free tier with 3 shared VMs
- **Services**: Containers, Postgres, Redis
- **Limits**: 2340 hours/month shared

### Option 4: Oracle Cloud Free Tier (Most Generous)
- **Cost**: Always Free
- **Services**: 4 ARM Ampere cores, 24GB RAM, 200GB storage
- **Best for**: Full production deployment

---

## Quick Start (5 Minutes)

### 1. Deploy to Railway

[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/template/actoris)

```bash
# Install Railway CLI
npm install -g @railway/cli

# Login
railway login

# Deploy
cd good
railway up
```

### 2. Deploy to Render

```bash
# Use render.yaml in the repo
# Push to GitHub, connect to Render
```

### 3. Deploy to Fly.io

```bash
# Install Fly CLI
curl -L https://fly.io/install.sh | sh

# Login and deploy
fly auth login
fly launch
fly deploy
```

---

## Service Architecture for Free Tier

```
┌─────────────────────────────────────────────────────────────┐
│                         FRONTEND                             │
│                    Vercel (Free Tier)                        │
│                   Next.js React App                          │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                      API GATEWAY                             │
│              Railway/Render (Free Tier)                      │
│                   Rust Axum Server                           │
└─────────────────────────┬───────────────────────────────────┘
                          │
          ┌───────────────┼───────────────┐
          ▼               ▼               ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│IdentityCloud│  │ TrustLedger │  │   OneBill   │
│   (Go)      │  │   (Rust)    │  │   (Rust)    │
│  Railway    │  │   Railway   │  │   Railway   │
└──────┬──────┘  └──────┬──────┘  └──────┬──────┘
       │                │                │
       ▼                ▼                ▼
┌─────────────────────────────────────────────────────────────┐
│                      DATABASES                               │
│  Neo4j Aura Free │ Upstash Redis │ Supabase (Events)        │
└─────────────────────────────────────────────────────────────┘
```

---

## Free Database Services

### 1. Neo4j Aura Free
- **URL**: https://neo4j.com/cloud/aura-free/
- **Limits**: 1 database, 200K nodes, 400K relationships
- **Setup**:
  1. Create account at neo4j.com
  2. Create free instance
  3. Copy connection URI

### 2. Upstash Redis (Free)
- **URL**: https://upstash.com/
- **Limits**: 10K commands/day, 256MB storage
- **Setup**:
  1. Create account
  2. Create Redis database
  3. Copy REST API URL and token

### 3. Upstash Kafka (NATS alternative)
- **URL**: https://upstash.com/kafka
- **Limits**: 10K messages/day
- **Use for**: Event streaming

### 4. Supabase (PostgreSQL + Realtime)
- **URL**: https://supabase.com/
- **Limits**: 500MB database, 2GB bandwidth
- **Use for**: EventStore alternative

---

## Environment Variables

Create `.env` file:

```env
# Neo4j Aura
NEO4J_URI=neo4j+s://xxxxx.databases.neo4j.io
NEO4J_USER=neo4j
NEO4J_PASSWORD=your-password

# Upstash Redis
UPSTASH_REDIS_REST_URL=https://xxxxx.upstash.io
UPSTASH_REDIS_REST_TOKEN=your-token

# Supabase
SUPABASE_URL=https://xxxxx.supabase.co
SUPABASE_ANON_KEY=your-anon-key

# API Gateway
API_URL=https://your-api.railway.app
NEXT_PUBLIC_API_URL=https://your-api.railway.app
```

---

## Local Development

### Using Docker Compose

```bash
cd good
docker-compose up -d

# Frontend at http://localhost:3000
# API at http://localhost:8080
# Neo4j at http://localhost:7474
```

### Manual Setup

```bash
# Terminal 1: API Gateway
cd api-gateway
cargo run

# Terminal 2: Frontend
cd ui
npm install
npm run dev

# Terminal 3: Services (using Docker)
docker-compose up neo4j redis nats
```

---

## Deployment Steps

### Step 1: Set Up Free Databases

1. **Neo4j Aura**: https://neo4j.com/cloud/aura-free/
2. **Upstash Redis**: https://upstash.com/
3. **Supabase**: https://supabase.com/

### Step 2: Deploy Backend to Railway

```bash
# Clone and setup
git clone https://github.com/your-repo/actoris
cd actoris/good

# Deploy to Railway
railway init
railway add
railway up
```

### Step 3: Deploy Frontend to Vercel

```bash
cd ui
npx vercel --prod
```

### Step 4: Configure Environment

Set environment variables in Railway/Vercel dashboards.

---

## Cost Breakdown (Free Tier)

| Service | Provider | Monthly Cost |
|---------|----------|--------------|
| Frontend | Vercel | $0 |
| API Gateway | Railway | $0 (500hrs) |
| Backend Services | Railway | $0 |
| Neo4j | Aura Free | $0 |
| Redis | Upstash | $0 |
| Events | Supabase | $0 |
| **Total** | | **$0** |

---

## Scaling Beyond Free Tier

When ready to scale:

1. **Railway Pro**: $20/month for more resources
2. **Neo4j Aura Pro**: $65/month for production
3. **Dedicated K8s**: Use Helm charts in `deploy/helm/`
