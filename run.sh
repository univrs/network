# Delete all databases
rm -f *.db bootstrap.db alice.db bob.db mycelial.db

# Terminal 1: Bootstrap
cargo run --release --bin mycelial-node -- \
  --bootstrap --name "Bootstrap" --port 9000 --http-port 8080 --db bootstrap.db

# Terminal 2: Alice  
cargo run --release --bin mycelial-node -- \
  --name "Alice" --port 9001 --http-port 8081 --db alice.db \
  --connect "/ip4/127.0.0.1/tcp/9000"

# Terminal 3: Bob
cargo run --release --bin mycelial-node -- \
  --name "Bob" --port 9002 --http-port 8082 --db bob.db \
  --connect "/ip4/127.0.0.1/tcp/9000"

# START 
npx pnpm dev -- --host
