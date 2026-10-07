set shell := ["powershell.exe", "-NoProfile", "-Command"]

# IMAP read-throughput benchmark, e.g. `just bench 16 1000`
bench clients="4" messages="200":
    docker compose up -d --wait db
    $env:SQLX_OFFLINE = 'true'; $env:TEST_DATABASE_URL = 'postgres://user:password@localhost:5432/postgres'; $env:IMAP_BENCH_CLIENTS = '{{clients}}'; $env:IMAP_BENCH_MESSAGES = '{{messages}}'; cargo bench -p imap --bench client_read
