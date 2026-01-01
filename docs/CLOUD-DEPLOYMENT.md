# Multi-Cloud P2P Deployment Architecture

> **Purpose**: Deploy Mycelial P2P network across multiple cloud providers to test real-world peer discovery, geographic distribution, and network resilience.

## Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     MYCELIAL MULTI-CLOUD TOPOLOGY                           │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐                  │
│   │   ORACLE    │     │    AZURE    │     │     AWS     │                  │
│   │  (Primary)  │     │ (Secondary) │     │ (Tertiary)  │                  │
│   │             │     │             │     │             │                  │
│   │ ┌─────────┐ │     │ ┌─────────┐ │     │ ┌─────────┐ │                  │
│   │ │Bootstrap│ │◄───►│ │  Peer   │ │◄───►│ │  Peer   │ │                  │
│   │ │  Node   │ │     │ │  Node   │ │     │ │  Node   │ │                  │
│   │ └────┬────┘ │     │ └─────────┘ │     │ └─────────┘ │                  │
│   │      │      │     │             │     │             │                  │
│   │ ┌────▼────┐ │     │             │     │             │                  │
│   │ │Dashboard│ │     │             │     │             │                  │
│   │ │  (Web)  │ │     │             │     │             │                  │
│   │ └─────────┘ │     │             │     │             │                  │
│   └─────────────┘     └─────────────┘     └─────────────┘                  │
│                                                                             │
│   Ports:                                                                    │
│   • 8080 - HTTP/WebSocket (Dashboard API)                                   │
│   • 9000 - P2P TCP (libp2p)                                                 │
│   • 80/443 - Dashboard Web UI                                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Cloud Provider Selection

### Oracle Cloud (Free Tier) - Primary Bootstrap
- **Instance**: VM.Standard.A1.Flex (ARM, 4 OCPU, 24GB RAM - Always Free)
- **Role**: Bootstrap node + Dashboard hosting
- **Why Primary**: Best free tier resources, no time limit

### Azure (Free Tier) - Secondary Peer
- **Instance**: B1s (1 vCPU, 1GB RAM - 750 hours/month free for 12 months)
- **Role**: Peer node for cross-cloud discovery testing
- **Why**: Good global presence, GitHub integration

### AWS (Free Tier) - Tertiary Peer
- **Instance**: t2.micro (1 vCPU, 1GB RAM - 750 hours/month free for 12 months)
- **Role**: Additional peer for 3-node network validation
- **Why**: Most common cloud, reliable networking

## Deployment Strategy

### Phase 1: Container Registry Setup
```yaml
# Use GitHub Container Registry (GHCR) for all images
ghcr.io/univrs/mycelial-node:latest
ghcr.io/univrs/mycelial-dashboard:latest
```

### Phase 2: Oracle Cloud Bootstrap
1. Create ARM VM (Always Free tier)
2. Install Docker
3. Deploy bootstrap node with public IP
4. Deploy dashboard behind nginx proxy

### Phase 3: Azure Peer Deployment
1. Create B1s VM
2. Install Docker
3. Deploy peer node connecting to Oracle bootstrap

### Phase 4: AWS Peer Deployment
1. Create t2.micro EC2 instance
2. Install Docker
3. Deploy peer node connecting to Oracle bootstrap

## Network Configuration

### Required Ports (All Providers)
| Port | Protocol | Purpose | Source |
|------|----------|---------|--------|
| 22 | TCP | SSH | Admin IP only |
| 80 | TCP | Dashboard HTTP | Public |
| 443 | TCP | Dashboard HTTPS | Public |
| 8080 | TCP | P2P HTTP/WS API | Public |
| 9000 | TCP | libp2p P2P | Public |

### Security Groups

```hcl
# Example Terraform security rules (applicable to all clouds)
ingress_rules = [
  { port = 22,   cidr = "YOUR_IP/32",  description = "SSH Admin" },
  { port = 80,   cidr = "0.0.0.0/0",   description = "HTTP" },
  { port = 443,  cidr = "0.0.0.0/0",   description = "HTTPS" },
  { port = 8080, cidr = "0.0.0.0/0",   description = "P2P API" },
  { port = 9000, cidr = "0.0.0.0/0",   description = "libp2p" },
]
```

## Deployment Commands

### Bootstrap Node (Oracle)
```bash
# Pull and run bootstrap node
docker pull ghcr.io/univrs/mycelial-node:latest
docker run -d \
  --name mycelial-bootstrap \
  --restart unless-stopped \
  -p 8080:8080 \
  -p 9000:9000 \
  -v mycelial-data:/app/data \
  ghcr.io/univrs/mycelial-node:latest \
  --bootstrap --name "OracleBootstrap" --port 9000 --http-port 8080

# Pull and run dashboard
docker pull ghcr.io/univrs/mycelial-dashboard:latest
docker run -d \
  --name mycelial-dashboard \
  --restart unless-stopped \
  -p 80:80 \
  -e VITE_P2P_WS_URL=ws://ORACLE_PUBLIC_IP:8080/ws \
  -e VITE_P2P_API_URL=http://ORACLE_PUBLIC_IP:8080 \
  ghcr.io/univrs/mycelial-dashboard:latest
```

### Peer Node (Azure/AWS)
```bash
# Pull and run peer node
docker pull ghcr.io/univrs/mycelial-node:latest
docker run -d \
  --name mycelial-peer \
  --restart unless-stopped \
  -p 8080:8080 \
  -p 9000:9000 \
  -v mycelial-data:/app/data \
  ghcr.io/univrs/mycelial-node:latest \
  --name "AzurePeer" --connect "/ip4/ORACLE_PUBLIC_IP/tcp/9000"
```

## GitHub Actions CD Workflow

The CD workflow (`.github/workflows/cd.yml`) handles:

1. **Build & Push Images**
   - Build multi-arch images (AMD64 + ARM64)
   - Push to GitHub Container Registry

2. **Deploy to Oracle (Bootstrap)**
   - SSH into Oracle VM
   - Pull latest images
   - Restart containers

3. **Deploy to Azure (Peer)**
   - SSH into Azure VM
   - Pull latest images
   - Restart with bootstrap connection

4. **Deploy to AWS (Peer)**
   - SSH into AWS EC2
   - Pull latest images
   - Restart with bootstrap connection

## Secrets Required

Add these secrets to GitHub repository settings:

| Secret | Description |
|--------|-------------|
| `GHCR_TOKEN` | GitHub token for container registry |
| `ORACLE_SSH_KEY` | SSH private key for Oracle VM |
| `ORACLE_HOST` | Oracle VM public IP |
| `AZURE_SSH_KEY` | SSH private key for Azure VM |
| `AZURE_HOST` | Azure VM public IP |
| `AWS_SSH_KEY` | SSH private key for AWS EC2 |
| `AWS_HOST` | AWS EC2 public IP |

## Health Monitoring

### Health Check Endpoints
- Bootstrap: `http://ORACLE_IP:8080/health`
- Dashboard: `http://ORACLE_IP/health`
- Azure Peer: `http://AZURE_IP:8080/health`
- AWS Peer: `http://AWS_IP:8080/health`

### P2P Network Status
Access the dashboard at `http://ORACLE_IP` to view:
- Connected peers across all clouds
- Gossipsub mesh topology
- Message propagation between regions

## Rollback Strategy

If deployment fails:
```bash
# On any node, rollback to previous version
docker stop mycelial-node
docker run -d --name mycelial-node \
  ghcr.io/univrs/mycelial-node:previous-tag \
  [same args as before]
```

## Cost Estimation

| Provider | Instance | Monthly Cost | Notes |
|----------|----------|--------------|-------|
| Oracle | A1.Flex | $0 | Always Free (ARM) |
| Azure | B1s | $0* | 750 hrs/month (12 months) |
| AWS | t2.micro | $0* | 750 hrs/month (12 months) |
| **Total** | | **$0** | *During free tier period |

## Future Enhancements

1. **Kubernetes Deployment**: Migrate to K8s for auto-scaling
2. **Terraform/Pulumi IaC**: Infrastructure as code for reproducibility
3. **Prometheus/Grafana**: Metrics and alerting
4. **Let's Encrypt**: HTTPS certificates via Certbot
5. **Geographic DNS**: Route users to nearest peer
