# Multi-Cloud P2P Deployment Architecture

> **Purpose**: Deploy Mycelial P2P network across multiple cloud providers to test real-world peer discovery, geographic distribution, and network resilience.

## Overview

```
┌───────────────────────────────────────────────────────────────────────────────────────┐
│                         MYCELIAL MULTI-CLOUD TOPOLOGY                                 │
├───────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                       │
│   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐        │
│   │     GCP     │     │     AWS     │     │    AZURE    │     │   HETZNER   │        │
│   │ (Bootstrap) │     │  (US Peer)  │     │  (US Peer)  │     │  (EU Peer)  │        │
│   │             │     │             │     │             │     │  🇩🇪 / 🇫🇮    │        │
│   │ ┌─────────┐ │     │ ┌─────────┐ │     │ ┌─────────┐ │     │ ┌─────────┐ │        │
│   │ │Bootstrap│ │◄───►│ │  Peer   │ │◄───►│ │  Peer   │ │◄───►│ │EU Peer  │ │        │
│   │ │  Node   │ │     │ │  Node   │ │     │ │  Node   │ │     │ │  Node   │ │        │
│   │ └────┬────┘ │     │ └─────────┘ │     │ └─────────┘ │     │ └─────────┘ │        │
│   │      │      │     │             │     │             │     │             │        │
│   │ ┌────▼────┐ │     │             │     │             │     │             │        │
│   │ │Dashboard│ │     │             │     │             │     │             │        │
│   │ │  (Web)  │ │     │             │     │             │     │             │        │
│   │ └─────────┘ │     │             │     │             │     │             │        │
│   └─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘        │
│                                                                                       │
│   Ports:                              Locations:                                      │
│   • 8080 - HTTP/WebSocket API         • GCP: us-central1 (Free Tier)                  │
│   • 9000 - P2P TCP (libp2p)           • AWS: us-east-1 (Free Tier)                    │
│   • 80/443 - Dashboard Web UI         • Azure: eastus (Free Tier)                     │
│                                       • Hetzner: Falkenstein DE / Helsinki FI         │
└───────────────────────────────────────────────────────────────────────────────────────┘
```

## Cloud Provider Selection

### GCP (Free Tier) - Primary Bootstrap
- **Instance**: e2-micro (2 vCPU shared, 1GB RAM - Always Free in select regions)
- **Region**: us-central1, us-west1, or us-east1 (free tier eligible)
- **Role**: Bootstrap node + Dashboard hosting
- **Why Primary**: Always Free tier (no 12-month limit), good global network

### AWS (Free Tier) - US Peer
- **Instance**: t2.micro (1 vCPU, 1GB RAM - 750 hours/month free for 12 months)
- **Role**: Peer node for cross-cloud discovery testing
- **Why**: Most common cloud, excellent documentation

### Azure (Free Tier) - US Peer
- **Instance**: B1s (1 vCPU, 1GB RAM - 750 hours/month free for 12 months)
- **Role**: Additional peer for network validation
- **Why**: Good GitHub integration, different network backbone

### Hetzner Cloud - EU Peer
- **Instance**: CX22 (2 vCPU, 4GB RAM - €4.75/month)
- **Location**: Falkenstein (DE) or Helsinki (FI)
- **Role**: EU peer node for geographic distribution + GDPR-friendly data path
- **Why**: Excellent EU coverage, blazing fast network, dirt cheap

## Deployment Strategy

### Phase 1: Container Registry Setup
```yaml
# Use GitHub Container Registry (GHCR) for all images
ghcr.io/univrs/mycelial-node:latest
ghcr.io/univrs/mycelial-dashboard:latest
```

### Phase 2: GCP Bootstrap
1. Create e2-micro VM (Always Free tier)
2. Install Docker
3. Deploy bootstrap node with public IP
4. Deploy dashboard behind nginx proxy

### Phase 3: AWS Peer Deployment
1. Create t2.micro EC2 instance
2. Install Docker
3. Deploy peer node connecting to GCP bootstrap

### Phase 4: Azure Peer Deployment
1. Create B1s VM
2. Install Docker
3. Deploy peer node connecting to GCP bootstrap

### Phase 5: Hetzner EU Peer Deployment
1. Create CX22 VPS
2. Install Docker
3. Deploy peer node connecting to GCP bootstrap

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

## Manual Bootstrap Instructions

### GCP Bootstrap Node (Primary)

**1. Create the VM via gcloud CLI:**
```bash
# Set project
gcloud config set project YOUR_PROJECT_ID

# Create firewall rules
gcloud compute firewall-rules create mycelial-ports \
  --allow tcp:22,tcp:80,tcp:443,tcp:8080,tcp:9000 \
  --source-ranges 0.0.0.0/0 \
  --description "Mycelial P2P ports"

# Create e2-micro VM (Always Free in us-central1, us-west1, us-east1)
gcloud compute instances create mycelial-bootstrap \
  --zone=us-central1-a \
  --machine-type=e2-micro \
  --image-family=ubuntu-2404-lts-amd64 \
  --image-project=ubuntu-os-cloud \
  --boot-disk-size=30GB \
  --tags=mycelial

# Get external IP
gcloud compute instances describe mycelial-bootstrap \
  --zone=us-central1-a \
  --format='get(networkInterfaces[0].accessConfigs[0].natIP)'
```

**2. SSH in and install Docker:**
```bash
gcloud compute ssh mycelial-bootstrap --zone=us-central1-a

# Install Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
newgrp docker

# Verify
docker --version
```

**3. Deploy bootstrap node + dashboard:**
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
  --bootstrap --name "GCPBootstrap" --port 9000 --http-port 8080

# Pull and run dashboard
docker pull ghcr.io/univrs/mycelial-dashboard:latest
docker run -d \
  --name mycelial-dashboard \
  --restart unless-stopped \
  -p 80:80 \
  -e VITE_P2P_WS_URL=ws://GCP_PUBLIC_IP:8080/ws \
  -e VITE_P2P_API_URL=http://GCP_PUBLIC_IP:8080 \
  ghcr.io/univrs/mycelial-dashboard:latest

# Verify
curl http://localhost:8080/health
curl http://localhost/health
```

### AWS Peer Node

**1. Create via AWS CLI:**
```bash
# Create security group
aws ec2 create-security-group \
  --group-name mycelial-sg \
  --description "Mycelial P2P ports"

aws ec2 authorize-security-group-ingress \
  --group-name mycelial-sg \
  --protocol tcp \
  --port 22 \
  --cidr YOUR_IP/32

aws ec2 authorize-security-group-ingress \
  --group-name mycelial-sg \
  --protocol tcp \
  --port 8080 \
  --cidr 0.0.0.0/0

aws ec2 authorize-security-group-ingress \
  --group-name mycelial-sg \
  --protocol tcp \
  --port 9000 \
  --cidr 0.0.0.0/0

# Create t2.micro instance (Free tier)
aws ec2 run-instances \
  --image-id ami-0c7217cdde317cfec \
  --instance-type t2.micro \
  --key-name YOUR_KEY_NAME \
  --security-groups mycelial-sg \
  --tag-specifications 'ResourceType=instance,Tags=[{Key=Name,Value=mycelial-peer}]'
```

**2. SSH in and deploy:**
```bash
ssh -i your-key.pem ubuntu@AWS_PUBLIC_IP

# Install Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
newgrp docker

# Deploy peer
docker run -d \
  --name mycelial-peer \
  --restart unless-stopped \
  -p 8080:8080 \
  -p 9000:9000 \
  -v mycelial-data:/app/data \
  ghcr.io/univrs/mycelial-node:latest \
  --name "AWSPeer" --connect "/ip4/GCP_PUBLIC_IP/tcp/9000"
```

### Azure Peer Node

**1. Create via Azure CLI:**
```bash
# Create resource group
az group create --name mycelial-rg --location eastus

# Create VM
az vm create \
  --resource-group mycelial-rg \
  --name mycelial-peer \
  --image Ubuntu2404 \
  --size Standard_B1s \
  --admin-username azureuser \
  --generate-ssh-keys \
  --public-ip-sku Standard

# Open ports
az vm open-port --resource-group mycelial-rg --name mycelial-peer --port 8080 --priority 1001
az vm open-port --resource-group mycelial-rg --name mycelial-peer --port 9000 --priority 1002
```

**2. SSH in and deploy:**
```bash
ssh azureuser@AZURE_PUBLIC_IP

# Install Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
newgrp docker

# Deploy peer
docker run -d \
  --name mycelial-peer \
  --restart unless-stopped \
  -p 8080:8080 \
  -p 9000:9000 \
  -v mycelial-data:/app/data \
  ghcr.io/univrs/mycelial-node:latest \
  --name "AzurePeer" --connect "/ip4/GCP_PUBLIC_IP/tcp/9000"
```

### Hetzner EU Peer Node

**1. Create via hcloud CLI:**
```bash
# Install hcloud if needed
# brew install hcloud  (macOS)
# sudo apt install hcloud-cli  (Ubuntu)

# Create context (first time)
hcloud context create univrs
# Paste API token from console.hetzner.cloud → Security → API Tokens

# Create SSH key (if not already)
hcloud ssh-key create --name mykey --public-key-from-file ~/.ssh/id_rsa.pub

# Create server
hcloud server create \
  --name mycelial-eu \
  --type cx22 \
  --image ubuntu-24.04 \
  --location fsn1 \
  --ssh-key mykey

# Create firewall
hcloud firewall create --name mycelial-fw
hcloud firewall add-rule mycelial-fw --direction in --protocol tcp --port 22 --source-ips 0.0.0.0/0
hcloud firewall add-rule mycelial-fw --direction in --protocol tcp --port 8080 --source-ips 0.0.0.0/0
hcloud firewall add-rule mycelial-fw --direction in --protocol tcp --port 9000 --source-ips 0.0.0.0/0
hcloud firewall apply-to-resource mycelial-fw --type server --server mycelial-eu

# Get IP
hcloud server ip mycelial-eu
```

**2. SSH in and deploy:**
```bash
ssh root@HETZNER_IP

# Install Docker
curl -fsSL https://get.docker.com | sh
systemctl enable --now docker

# Deploy peer
docker run -d \
  --name mycelial-peer \
  --restart unless-stopped \
  -p 8080:8080 \
  -p 9000:9000 \
  -v mycelial-data:/app/data \
  ghcr.io/univrs/mycelial-node:latest \
  --name "HetznerEU" --connect "/ip4/GCP_PUBLIC_IP/tcp/9000"

# Verify
curl localhost:8080/health
```

## GitHub Actions CD Workflow

The CD workflow (`.github/workflows/cd.yml`) handles:

1. **Build & Push Images**
   - Build multi-arch images (AMD64 + ARM64)
   - Push to GitHub Container Registry

2. **Deploy to GCP (Bootstrap)**
   - SSH into GCP VM
   - Pull latest images
   - Restart containers

3. **Deploy to AWS (Peer)**
   - SSH into AWS EC2
   - Pull latest images
   - Restart with bootstrap connection

4. **Deploy to Azure (Peer)**
   - SSH into Azure VM
   - Pull latest images
   - Restart with bootstrap connection

5. **Deploy to Hetzner (EU Peer)**
   - SSH into Hetzner VPS
   - Pull latest images
   - Restart with bootstrap connection

## Secrets Required

Add these secrets to GitHub repository settings:

| Secret | Description |
|--------|-------------|
| `GCP_SSH_KEY` | SSH private key for GCP VM |
| `GCP_HOST` | GCP VM external IP |
| `GCP_USER` | SSH user for GCP (usually your username) |
| `AWS_SSH_KEY` | SSH private key for AWS EC2 |
| `AWS_HOST` | AWS EC2 public IP |
| `AWS_USER` | SSH user for AWS (usually `ubuntu`) |
| `AZURE_SSH_KEY` | SSH private key for Azure VM |
| `AZURE_HOST` | Azure VM public IP |
| `AZURE_USER` | SSH user for Azure (usually `azureuser`) |
| `HETZNER_SSH_KEY` | SSH private key for Hetzner VPS |
| `HETZNER_HOST` | Hetzner VPS public IP |
| `HETZNER_USER` | SSH user for Hetzner (usually `root`) |

## Health Monitoring

### Health Check Endpoints
- GCP Bootstrap: `http://GCP_IP:8080/health`
- Dashboard: `http://GCP_IP/health`
- AWS Peer: `http://AWS_IP:8080/health`
- Azure Peer: `http://AZURE_IP:8080/health`
- Hetzner EU Peer: `http://HETZNER_IP:8080/health`

### P2P Network Status
Access the dashboard at `http://GCP_IP` to view:
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
| GCP | e2-micro | $0 | Always Free (select regions) |
| AWS | t2.micro | $0* | 750 hrs/month (12 months) |
| Azure | B1s | $0* | 750 hrs/month (12 months) |
| Hetzner | CX22 | €4.75 (~$5) | 2 vCPU, 4GB RAM, EU location |
| **Total** | | **~$5/mo** | *AWS/Azure free during tier period |

## Future Enhancements

1. **Kubernetes Deployment**: Migrate to K8s for auto-scaling
2. **Terraform/Pulumi IaC**: Infrastructure as code for reproducibility
3. **Prometheus/Grafana**: Metrics and alerting
4. **Let's Encrypt**: HTTPS certificates via Certbot
5. **Geographic DNS**: Route users to nearest peer
