#!/usr/bin/env sh
export POSTGRES_PASSWORD=postgres

if [ "$1" = "k" ]; then
    docker compose -f dev/docker-compose.yml down
fi

docker compose -f dev/docker-compose.yml up -d
sleep 1
RUST_LOG=info,minibooru=debug MINIBOORU_CONFIG_PATH=./dev/minibooru.conf cargo run

# mkdir -p /tmp/minibooru/instance && ln -s /tmp/minibooru/instance /home/i/Code/minibooru/dev/instance
# PGPASSWORD=postgres psql -Upostgres -h0.0.0.0
# let anons post:
# PGPASSWORD=postgres psql -Upostgres -h0.0.0.0 -c "INSERT INTO permissions (operation, resource) VALUES ('create', 'posts')"