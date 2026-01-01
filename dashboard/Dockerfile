# =============================================================================
# Mycelial Dashboard - Multi-stage Dockerfile
# =============================================================================
# Build: docker build -t mycelial-dashboard .
# Run:   docker run -p 80:80 mycelial-dashboard
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build the React application
# -----------------------------------------------------------------------------
FROM node:20-alpine AS builder

WORKDIR /app

# Install pnpm
RUN corepack enable && corepack prepare pnpm@8 --activate

# Copy package files first for better caching
COPY package.json pnpm-lock.yaml ./

# Install dependencies
RUN pnpm install --frozen-lockfile

# Copy source code
COPY . .

# Build arguments for environment configuration
ARG VITE_P2P_WS_URL=ws://localhost:8080/ws
ARG VITE_P2P_API_URL=http://localhost:8080
ARG VITE_ORCHESTRATOR_WS_URL=ws://localhost:9090/api/v1/events
ARG VITE_ORCHESTRATOR_API_URL=http://localhost:9090
ARG VITE_USE_MOCK_DATA=false

# Set environment variables for build
ENV VITE_P2P_WS_URL=$VITE_P2P_WS_URL
ENV VITE_P2P_API_URL=$VITE_P2P_API_URL
ENV VITE_ORCHESTRATOR_WS_URL=$VITE_ORCHESTRATOR_WS_URL
ENV VITE_ORCHESTRATOR_API_URL=$VITE_ORCHESTRATOR_API_URL
ENV VITE_USE_MOCK_DATA=$VITE_USE_MOCK_DATA

# Build production bundle
RUN pnpm build

# -----------------------------------------------------------------------------
# Stage 2: Serve with nginx
# -----------------------------------------------------------------------------
FROM nginx:alpine AS runtime

# Copy custom nginx configuration
COPY nginx.conf /etc/nginx/conf.d/default.conf

# Copy built assets from builder
COPY --from=builder /app/dist /usr/share/nginx/html

# Expose port 80
EXPOSE 80

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget -q --spider http://localhost/health || exit 1

# Start nginx
CMD ["nginx", "-g", "daemon off;"]
