#!/bin/bash
# start-full-cluster.sh
# Starts the full Univrs test environment:
# - Orchestrator cluster (4 nodes via Docker)
# - P2P network (bootstrap + 3 peers)
# - Optional: Dashboard
#
# Usage:
#   ./start-full-cluster.sh           # Start everything
#   ./start-full-cluster.sh --stop    # Stop everything
#   ./start-full-cluster.sh --status  # Check status

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NETWORK_DIR="$(dirname "$SCRIPT_DIR")"
ORCHESTRATOR_DIR="$HOME/repos/univrs-orchestration/orchestrator"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_section() { echo -e "\n${BLUE}═══════════════════════════════════════════════════════════${NC}"; echo -e "${BLUE}  $1${NC}"; echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"; }

# PID file locations
P2P_PIDS_FILE="/tmp/univrs-p2p-nodes.pids"

stop_all() {
    log_section "Stopping Full Cluster"

    # Stop P2P nodes
    if [ -f "$P2P_PIDS_FILE" ]; then
        log_info "Stopping P2P nodes..."
        while read -r pid; do
            if kill -0 "$pid" 2>/dev/null; then
                kill "$pid" 2>/dev/null || true
                log_info "  Stopped PID $pid"
            fi
        done < "$P2P_PIDS_FILE"
        rm -f "$P2P_PIDS_FILE"
    else
        # Fallback: kill by process name
        pkill -f "mycelial-node" 2>/dev/null || true
    fi

    # Stop orchestrator cluster
    if [ -d "$ORCHESTRATOR_DIR" ]; then
        log_info "Stopping orchestrator cluster..."
        cd "$ORCHESTRATOR_DIR" && docker-compose down 2>/dev/null || true
    fi

    log_info "All services stopped"
}

show_status() {
    log_section "Cluster Status"

    # Check orchestrator
    echo -e "\n${BLUE}Orchestrator Cluster:${NC}"
    if [ -d "$ORCHESTRATOR_DIR" ]; then
        cd "$ORCHESTRATOR_DIR" && docker-compose ps 2>/dev/null || echo "  Not running"
    fi

    # Check P2P nodes
    echo -e "\n${BLUE}P2P Nodes:${NC}"
    pgrep -fa "mycelial-node" 2>/dev/null || echo "  No P2P nodes running"

    # Check API endpoints
    echo -e "\n${BLUE}API Endpoints:${NC}"
    if curl -s http://localhost:9090/api/v1/nodes >/dev/null 2>&1; then
        nodes=$(curl -s http://localhost:9090/api/v1/nodes | grep -o '"count":[0-9]*' | cut -d: -f2)
        echo "  Orchestrator API (port 9090): ${GREEN}UP${NC} - $nodes nodes"
    else
        echo "  Orchestrator API (port 9090): ${RED}DOWN${NC}"
    fi

    if curl -s http://localhost:8080/api/status >/dev/null 2>&1; then
        echo "  P2P API (port 8080): ${GREEN}UP${NC}"
    else
        echo "  P2P API (port 8080): ${RED}DOWN${NC}"
    fi
}

start_orchestrator() {
    log_section "Starting Orchestrator Cluster"

    if [ ! -d "$ORCHESTRATOR_DIR" ]; then
        log_error "Orchestrator directory not found: $ORCHESTRATOR_DIR"
        return 1
    fi

    cd "$ORCHESTRATOR_DIR"

    # Check if already running
    if docker-compose ps 2>/dev/null | grep -q "healthy"; then
        log_warn "Orchestrator cluster already running"
    else
        log_info "Starting docker-compose..."
        docker-compose up -d

        # Wait for healthy
        log_info "Waiting for containers to be healthy..."
        for i in {1..30}; do
            if docker-compose ps 2>/dev/null | grep -c "healthy" | grep -q "4"; then
                break
            fi
            sleep 1
        done
    fi

    # Verify nodes
    sleep 2
    nodes=$(curl -s http://localhost:9090/api/v1/nodes 2>/dev/null | grep -o '"count":[0-9]*' | cut -d: -f2 || echo "0")
    log_info "Orchestrator cluster running with $nodes nodes"
    log_info "  API: http://localhost:9090/api/v1/nodes"
}

build_p2p() {
    log_section "Building P2P Node"

    cd "$NETWORK_DIR"
    log_info "Building mycelial-node (release)..."
    cargo build --release --bin mycelial-node 2>&1 | tail -3
}

start_p2p() {
    log_section "Starting P2P Network"

    cd "$NETWORK_DIR"

    # Check if binary exists
    if [ ! -f "target/release/mycelial-node" ]; then
        build_p2p
    fi

    # Clear old PIDs
    > "$P2P_PIDS_FILE"

    # Start bootstrap node
    log_info "Starting Bootstrap node (P2P: 9000, HTTP: 8080)..."
    cargo run --release --bin mycelial-node -- \
        --bootstrap --name "Bootstrap" --port 9000 --http-port 8080 \
        > /tmp/p2p-bootstrap.log 2>&1 &
    echo $! >> "$P2P_PIDS_FILE"
    sleep 3

    # Start peer nodes
    for peer in "Alice" "Bob" "Charlie"; do
        log_info "Starting $peer node..."
        cargo run --release --bin mycelial-node -- \
            --name "$peer" --connect "/ip4/127.0.0.1/tcp/9000" \
            > "/tmp/p2p-${peer,,}.log" 2>&1 &
        echo $! >> "$P2P_PIDS_FILE"
        sleep 1
    done

    log_info "P2P network running with 4 nodes"
    log_info "  API: http://localhost:8080"
    log_info "  WebSocket: ws://localhost:8080/ws"
    log_info "  Logs: /tmp/p2p-*.log"
}

start_dashboard() {
    log_section "Dashboard Info"
    log_info "To start the dashboard, run in a separate terminal:"
    echo ""
    echo "  cd $NETWORK_DIR/dashboard && pnpm dev"
    echo ""
    log_info "Dashboard will be available at: http://localhost:5173"
    log_info "It connects to:"
    log_info "  - P2P: ws://localhost:8080/ws"
    log_info "  - Orchestrator: http://localhost:9090/api/v1"
}

# Main logic
case "${1:-}" in
    --stop|-s)
        stop_all
        ;;
    --status)
        show_status
        ;;
    --help|-h)
        echo "Usage: $0 [option]"
        echo ""
        echo "Options:"
        echo "  (no args)   Start full cluster (orchestrator + P2P)"
        echo "  --stop      Stop all services"
        echo "  --status    Show status of all services"
        echo "  --help      Show this help"
        ;;
    *)
        # Start everything
        stop_all 2>/dev/null || true
        sleep 2
        start_orchestrator
        start_p2p
        echo ""
        show_status
        start_dashboard
        ;;
esac
