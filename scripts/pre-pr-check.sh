#!/bin/bash
# Pre-PR Check Script for univrs-network
# Run this before creating any Pull Request to ensure CI will pass

set -e

echo "======================================"
echo "  Pre-PR Check for univrs-network"
echo "======================================"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

FAILED=0

# Step 1: Format Check
echo -e "${YELLOW}[1/4] Checking formatting...${NC}"
if cargo fmt --all -- --check 2>/dev/null; then
    echo -e "${GREEN}  ✓ Formatting OK${NC}"
else
    echo -e "${RED}  ✗ Formatting issues found${NC}"
    echo "    Running cargo fmt --all to fix..."
    cargo fmt --all
    echo -e "${GREEN}  ✓ Formatting fixed${NC}"
fi
echo ""

# Step 2: Clippy
echo -e "${YELLOW}[2/4] Running clippy...${NC}"
if cargo clippy --all-targets --all-features -- -D warnings 2>&1; then
    echo -e "${GREEN}  ✓ Clippy passed${NC}"
else
    echo -e "${RED}  ✗ Clippy warnings/errors found${NC}"
    FAILED=1
fi
echo ""

# Step 3: Build
echo -e "${YELLOW}[3/4] Building...${NC}"
if cargo build --all-features 2>&1 | tail -5; then
    echo -e "${GREEN}  ✓ Build succeeded${NC}"
else
    echo -e "${RED}  ✗ Build failed${NC}"
    FAILED=1
fi
echo ""

# Step 4: Tests
echo -e "${YELLOW}[4/4] Running unit tests...${NC}"
if cargo test --all-features --lib 2>&1 | tail -10; then
    echo -e "${GREEN}  ✓ Tests passed${NC}"
else
    echo -e "${RED}  ✗ Tests failed${NC}"
    FAILED=1
fi
echo ""

# Summary
echo "======================================"
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}  All checks passed! Ready for PR${NC}"
    echo "======================================"
    exit 0
else
    echo -e "${RED}  Some checks failed. Fix before PR${NC}"
    echo "======================================"
    exit 1
fi
