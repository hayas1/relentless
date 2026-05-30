#!/bin/bash
# Usage: $0 [relentless|server] [http|grpc]

JAEGER=${JAEGER_HOST:-http://localhost}
REPO=$(dirname "$(dirname "$(dirname "$0")")")
case "$1" in
    relentless)
        case "$2" in
            http)
                OTEL_EXPORTER_OTLP_ENDPOINT=${JAEGER}:4317 \
                    cargo run --manifest-path "$REPO"/relentless-http/Cargo.toml \
                    -- "$REPO"/relentless-http/examples/config/*.yaml
                ;;
            grpc)
                OTEL_EXPORTER_OTLP_ENDPOINT=${JAEGER}:4317 \
                    cargo run --manifest-path "$REPO"/relentless-grpc/Cargo.toml \
                    -- "$REPO"/relentless-grpc/examples/config/*.yaml
                ;;
            *)
                cat "$0" | grep -e "^# Usage" | sed 's/# //g' | sed "s|\$0|$0|" && exit 1
                ;;
        esac
        ;;
    server)
        case "$2" in
            http)
                OTEL_EXPORTER_OTLP_ENDPOINT=${JAEGER}:4318 \
                    cargo run --manifest-path "$REPO"/dev/server/http/Cargo.toml
                ;;
            grpc)
                OTEL_EXPORTER_OTLP_ENDPOINT=${JAEGER}:4317 \
                    cargo run --manifest-path "$REPO"/dev/server/grpc/Cargo.toml
                ;;
            *)
                cat "$0" | grep -e "^# Usage" | sed 's/# //g' | sed "s|\$0|$0|" && exit 1
                ;;
        esac
        ;;
    *)
        cat "$0" | grep -e "^# Usage" | sed 's/# //g' | sed "s|\$0|$0|" && exit 1
        ;;
esac
