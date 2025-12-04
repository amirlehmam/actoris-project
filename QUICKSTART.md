# ACTORIS Quick Start Guide

## Local Development (5 minutes)

```bash
# 1. Start all services with Docker
docker-compose up -d

# 2. Open the UI
open http://localhost:3000

# 3. API is available at
curl http://localhost:8080/health
```

## Free Cloud Deployment Options

### Option 1: Vercel + Railway (Recommended)

**Frontend on Vercel (Free):**
```bash
cd ui
npx vercel --prod
# Note the URL, e.g., https://actoris-ui.vercel.app
```

**Backend on Railway (Free $5/month credit):**
```bash
npm install -g @railway/cli
railway login
railway init
railway up
# Note the API URL
```

**Connect them:**
- Go to Vercel dashboard > Project Settings > Environment Variables
- Add `NEXT_PUBLIC_API_URL` = your Railway API URL

### Option 2: Render (Free Tier)

```bash
# Push to GitHub, then:
# 1. Go to render.com
# 2. New > Blueprint
# 3. Connect your repo
# 4. Render reads render.yaml automatically
```

### Option 3: Fly.io (Free Tier)

```bash
# Install Fly CLI
curl -L https://fly.io/install.sh | sh

# Deploy API
fly auth login
fly launch --copy-config
fly deploy

# Deploy UI separately on Vercel
cd ui && npx vercel --prod
```

## Using the UI

### 1. Create an Agent
- Click "New Agent" button
- Enter a name (e.g., "My AI Agent")
- Select type: Human, AI, Hybrid, or Contract
- Click Create

### 2. Deposit HC (Harness Credits)
- Go to Wallet page
- Select your agent
- Enter amount and click Deposit

### 3. Submit an Action
- Go to Actions page
- Click "Submit Action"
- Select Producer (who provides the service)
- Select Consumer (who pays for the service)
- Enter input data
- Submit

### 4. Verify an Action
- Find your pending action
- Click the eye icon to view details
- Enter output data
- Click "Submit Verification"
- Watch the FROST signatures confirm!

### 5. Check Trust Score
- Go to Agents page
- Select an agent
- View trust score (increases with successful verifications)

## API Endpoints

```
GET  /health              - Service health check
GET  /stats               - Platform statistics
GET  /agents              - List all agents
POST /agents              - Create new agent
GET  /agents/:id          - Get agent details
GET  /agents/:id/trust    - Get trust score
GET  /agents/:id/wallet   - Get wallet
POST /agents/:id/wallet/deposit - Deposit HC
GET  /actions             - List all actions
POST /actions             - Submit action
GET  /actions/:id         - Get action details
POST /actions/:id/verify  - Verify action
GET  /ws                  - WebSocket for real-time events
```

## Free Database Services

For production, replace in-memory storage with:

| Service | Free Tier | Setup |
|---------|-----------|-------|
| Neo4j Aura | 200K nodes | neo4j.com/cloud/aura-free |
| Upstash Redis | 10K/day | upstash.com |
| Supabase | 500MB | supabase.com |

## Environment Variables

```env
# API Gateway
RUST_LOG=info
REDIS_URL=redis://localhost:6379
IDENTITY_CLOUD_URL=http://identity-cloud:50051
TRUSTLEDGER_URL=http://trustledger:50052
ONEBILL_URL=http://onebill:50053

# Frontend
NEXT_PUBLIC_API_URL=http://localhost:8080
```

## Troubleshooting

**CORS errors?**
- Make sure API_URL matches exactly (no trailing slash)
- API Gateway has CORS enabled for all origins

**WebSocket not connecting?**
- Check if your host supports WebSockets
- Vercel supports WebSockets, Railway supports them too

**Trust score not updating?**
- Verify actions must be successful
- Check the actions page for status

## Architecture

```
┌────────────────┐     ┌────────────────┐
│   Vercel       │     │   Railway      │
│   (UI)         │────▶│   (API)        │
└────────────────┘     └────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
       ┌──────────┐    ┌──────────┐    ┌──────────┐
       │ Identity │    │  Trust   │    │  OneBill │
       │  Cloud   │    │  Ledger  │    │          │
       └──────────┘    └──────────┘    └──────────┘
```

## Cost Summary (All Free)

| Component | Provider | Cost |
|-----------|----------|------|
| Frontend | Vercel | $0 |
| API Gateway | Railway/Fly.io | $0 |
| Database | Upstash/Aura | $0 |
| **Total** | | **$0** |
