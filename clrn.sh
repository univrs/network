# Stop all nodes first (Ctrl+C in each terminal)

# Delete ALL database files
rm -f *.db *.db-shm *.db-wal

# Verify clean
ls -la *.db* 2>/dev/null || echo "All clean!"
# START
# npx pnpm dev -- --host
