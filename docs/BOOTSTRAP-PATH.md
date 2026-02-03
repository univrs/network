# Univrs P2P Network Bootstrap Path

> **Philosophy:** Rawdog the architecture, train wheels on node ownership.  
> Cloud nodes first → prove stability → community nodes join → progressive decentralization.

---

## The Vision

```
Phase 0 (NOW)     Phase 1           Phase 2              Phase 3
─────────────     ───────────       ──────────────       ─────────────
[Cloud Nodes]  →  [Direct File]  →  [Hybrid Mesh]     →  [Community-Led]
 You own all      runs on P2P       Community joins      Cloud optional
 Debug/stabilize  Architecture ✓    alongside cloud      Full decentralization
```

**Direct Saints runs on P2P from day one.** The fact that you operate all the nodes initially doesn't change the architecture — it just means you can fix bugs without killing user enthusiasm.

---

## Phase 0: Cloud Bootstrap (TODAY)

### Step 1: Oracle Cloud Free Tier (Bootstrap Node)

Oracle's Always Free tier gives you a beefy ARM VM forever — perfect for bootstrap.

#### 1.1 Create Oracle Cloud Account
1. Go to: https://www.oracle.com/cloud/free/
2. Sign up (requires credit card, won't be charged for free tier)
3. Home region: Choose closest to you (e.g., `us-chicago-1` or `us-ashburn-1`)

#### 1.2 Create the VM
1. Console → Compute → Instances → Create Instance
2. **Name:** `univrs-bootstrap`
3. **Image:** Oracle Linux 8 (or Ubuntu 22.04)
4. **Shape:** VM.Standard.A1.Flex (Ampere ARM)
   - OCPUs: 2 (can go up to 4 free)
   - Memory: 12 GB (can go up to 24 free)
5. **Networking:** 
   - Create new VCN or use default
   - Assign public IP: Yes
6. **SSH Key:** Upload your public key or generate new
7. Click **Create**

#### 1.3 Configure Security List (Firewall)
1. Go to: Networking → Virtual Cloud Networks → [Your VCN] → Security Lists
2. Add Ingress Rules:

| Source CIDR | Protocol | Dest Port | Description |
|-------------|----------|-----------|-------------|
| 0.0.0.0/0 | TCP | 22 | SSH (consider restricting to your IP) |
| 0.0.0.0/0 | TCP | 80 | HTTP |
| 0.0.0.0/0 | TCP | 443 | HTTPS |
| 0.0.0.0/0 | TCP | 8080 | P2P HTTP/WebSocket API |
| 0.0.0.0/0 | TCP | 9000 | libp2p P2P |

#### 1.4 SSH In and Install Docker
```bash
# SSH into the VM
ssh -i ~/.ssh/your_key opc@<ORACLE_PUBLIC_IP>

# For Oracle Linux 8:
sudo dnf config-manager --add-repo https://download.docker.com/linux/centos/docker-ce.repo
sudo dnf install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
sudo systemctl enable --now docker
sudo usermod -aG docker opc

# Log out and back in for group change
exit
ssh -i ~/.ssh/your_key opc@<ORACLE_PUBLIC_IP>

# Verify
docker --version
```

#### 1.5 Open OS Firewall (Oracle Linux)
```bash
# Oracle Linux has iptables rules by default
sudo firewall-cmd --permanent --add-port=80/tcp
sudo firewall-cmd --permanent --add-port=443/tcp
sudo firewall-cmd --permanent --add-port=8080/tcp
sudo firewall-cmd --permanent --add-port=9000/tcp
sudo firewall-cmd --reload
```

#### 1.6 Deploy Bootstrap Node
```bash
# Create data directory
mkdir -p ~/mycelial-data

# Run bootstrap node (using local build for now, GHCR later)
# Option A: Pull from GitHub Container Registry (when available)
docker pull ghcr.io/univrs/mycelial-node:latest
docker run -d \
  --name mycelial-bootstrap \
  --restart unless-stopped \
  -p 8080:8080 \
  -p 9000:9000 \
  -v ~/mycelial-data:/app/data \
  ghcr.io/univrs/mycelial-node:latest \
  --bootstrap --name "OracleBootstrap" --port 9000 --http-port 8080

# Option B: Build locally (if images not published yet)
cd ~/repos/univrs-network
docker build -t mycelial-node .
# Then scp or rsync to Oracle VM and run
```

#### 1.7 Verify Bootstrap Node
```bash
# Check it's running
docker ps
docker logs mycelial-bootstrap

# Test health endpoint
curl http://localhost:8080/health

# From your local machine, test public access:
curl http://<ORACLE_PUBLIC_IP>:8080/health
```

**🎉 Checkpoint: Bootstrap node running on Oracle. Note the public IP.**

---

### Step 2: Azure Free Tier (Peer Node)

#### 2.1 Create Azure Account
1. Go to: https://azure.microsoft.com/free/
2. Sign up (12 months free tier for B1s)

#### 2.2 Create the VM
1. Portal → Virtual Machines → Create
2. **Name:** `univrs-peer-azure`
3. **Image:** Ubuntu 22.04 LTS
4. **Size:** B1s (1 vCPU, 1 GB RAM) — Free tier eligible
5. **Authentication:** SSH public key
6. **Inbound ports:** Allow SSH (22)

#### 2.3 Configure Network Security Group
Add inbound rules for ports: 80, 443, 8080, 9000 (same as Oracle)

#### 2.4 SSH In and Install Docker
```bash
ssh -i ~/.ssh/your_key azureuser@<AZURE_PUBLIC_IP>

# Install Docker
curl -fsSL https://get.docker.com | sudo sh
sudo usermod -aG docker azureuser
exit
ssh -i ~/.ssh/your_key azureuser@<AZURE_PUBLIC_IP>
```

#### 2.5 Deploy Peer Node
```bash
docker run -d \
  --name mycelial-peer \
  --restart unless-stopped \
  -p 8080:8080 \
  -p 9000:9000 \
  -v ~/mycelial-data:/app/data \
  ghcr.io/univrs/mycelial-node:latest \
  --name "AzurePeer" --port 9000 --http-port 8080 \
  --connect "/ip4/<ORACLE_PUBLIC_IP>/tcp/9000"
```

**🎉 Checkpoint: Azure peer connected to Oracle bootstrap.**

---

### Step 3: AWS Free Tier (Peer Node)

#### 3.1 Create AWS Account
1. Go to: https://aws.amazon.com/free/
2. Sign up (12 months free tier for t2.micro)

#### 3.2 Launch EC2 Instance
1. EC2 → Launch Instance
2. **Name:** `univrs-peer-aws`
3. **AMI:** Ubuntu 22.04 LTS
4. **Instance type:** t2.micro (Free tier)
5. **Key pair:** Create or select existing
6. **Security group:** Create new, allow SSH

#### 3.3 Configure Security Group
Add inbound rules for ports: 80, 443, 8080, 9000

#### 3.4 SSH In and Install Docker
```bash
ssh -i ~/.ssh/your_key ubuntu@<AWS_PUBLIC_IP>

curl -fsSL https://get.docker.com | sudo sh
sudo usermod -aG docker ubuntu
exit
ssh -i ~/.ssh/your_key ubuntu@<AWS_PUBLIC_IP>
```

#### 3.5 Deploy Peer Node
```bash
docker run -d \
  --name mycelial-peer \
  --restart unless-stopped \
  -p 8080:8080 \
  -p 9000:9000 \
  -v ~/mycelial-data:/app/data \
  ghcr.io/univrs/mycelial-node:latest \
  --name "AWSPeer" --port 9000 --http-port 8080 \
  --connect "/ip4/<ORACLE_PUBLIC_IP>/tcp/9000"
```

**🎉 Checkpoint: 3-node P2P network across 3 clouds.**

---

### Step 4: GitHub CI/CD Setup

#### 4.1 Add Repository Secrets
Go to: GitHub → univrs-network → Settings → Secrets and variables → Actions

| Secret Name | Value |
|-------------|-------|
| `GHCR_TOKEN` | GitHub PAT with `write:packages` scope |
| `ORACLE_HOST` | Oracle VM public IP |
| `ORACLE_SSH_KEY` | Private SSH key (entire contents) |
| `ORACLE_USER` | `opc` (Oracle Linux) or `ubuntu` |
| `AZURE_HOST` | Azure VM public IP |
| `AZURE_SSH_KEY` | Private SSH key |
| `AZURE_USER` | `azureuser` |
| `AWS_HOST` | AWS EC2 public IP |
| `AWS_SSH_KEY` | Private SSH key |
| `AWS_USER` | `ubuntu` |

#### 4.2 Verify Workflows
- `.github/workflows/ci.yml` — Builds and tests on PR/push
- `.github/workflows/cd.yml` — Deploys to cloud on merge to main

After secrets are set, push to main → images build → deploy to all nodes.

---

## Phase 0 Complete Checklist

- [ ] Oracle Cloud account created
- [ ] Oracle VM running (A1.Flex ARM)
- [ ] Docker installed on Oracle
- [ ] Firewall/security rules configured
- [ ] Bootstrap node running on Oracle
- [ ] Azure account created
- [ ] Azure VM running (B1s)
- [ ] Azure peer connected to Oracle bootstrap
- [ ] AWS account created
- [ ] AWS EC2 running (t2.micro)
- [ ] AWS peer connected to Oracle bootstrap
- [ ] GitHub secrets configured
- [ ] CI/CD pipeline tested
- [ ] Dashboard accessible at http://<ORACLE_IP>

---

## What's Next

### Phase 1: Direct Saints on P2P
- Deploy Direct File services as containers on the P2P network
- All nodes operator-controlled, architecture is decentralized
- Debug, stabilize, prove it works

### Phase 2: Community Nodes
- Open node registration
- Community members run nodes alongside cloud nodes
- Hybrid mesh with gradual trust building

### Phase 3: Progressive Decentralization
- Workloads migrate to community nodes
- Cloud nodes become fallback
- True "compute of the people"

---

## Quick Reference

### SSH Commands
```bash
# Oracle
ssh -i ~/.ssh/oracle_key opc@<ORACLE_IP>

# Azure
ssh -i ~/.ssh/azure_key azureuser@<AZURE_IP>

# AWS
ssh -i ~/.ssh/aws_key ubuntu@<AWS_IP>
```

### Docker Commands (on any node)
```bash
# View logs
docker logs -f mycelial-bootstrap  # or mycelial-peer

# Restart
docker restart mycelial-bootstrap

# Update to latest
docker pull ghcr.io/univrs/mycelial-node:latest
docker stop mycelial-bootstrap
docker rm mycelial-bootstrap
# Re-run docker run command from above
```

### Health Checks
```bash
curl http://<ORACLE_IP>:8080/health
curl http://<AZURE_IP>:8080/health
curl http://<AWS_IP>:8080/health
```

---

*Document created: 2026-02-03*  
*"Rawdog the architecture, train wheels on the ownership."*
