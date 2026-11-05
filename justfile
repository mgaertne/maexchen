default:
    @just --list

# Üben: Server und Zuschauer, bis du die GUI schließt; variant: mia21, blind, mia42[,blind], random[,blind]
warmup variant="mia21" port="9000": (_session variant port "0" "no")

# Werten: wie warmup, dazu simple-Bot als Maßstab; der Server endet nach <minutes> Minuten, die GUI bleibt stehen
finale variant="mia21" port="9000" minutes="5": (_session variant port minutes "yes")

# Server und Zuschauer im Release-Modus; minutes=0 heißt ohne Zeitlimit, benchmark=yes startet den simple-Bot
[private]
_session variant port minutes benchmark:
    #!/usr/bin/env bash
    set -euo pipefail
    variant="{{variant}}"
    [ "$variant" = "blind" ] && variant="mia21,blind"
    case "$variant" in
        mia21|mia21,blind|mia42|mia42,blind|random|random,blind) ;;
        *) echo "Unbekannte Variante '$variant'. Erlaubt: mia21, blind, mia42, mia42,blind, random, random,blind" >&2; exit 1 ;;
    esac
    server_args=(--rules "${variant%%,*}")
    case "$variant" in *,blind) server_args+=(--with-blind-roll) ;; esac
    cargo build --release -p maexchen_server -p maexchen_spectator
    if [ "{{benchmark}}" = "yes" ]; then
        uv sync --project clients/python
    fi
    bin="${CARGO_TARGET_DIR:-target}/release"
    pids=()
    trap 'kill "${pids[@]}" 2>/dev/null || true' EXIT
    echo "Regeln: $variant, Port {{port}}"
    if [ "{{minutes}}" = "0" ]; then
        "$bin/maexchen_server" -p {{port}} "${server_args[@]}" &
    else
        timeout "{{minutes}}m" "$bin/maexchen_server" -p {{port}} "${server_args[@]}" &
    fi
    server=$!
    pids+=("$server")
    sleep 1
    kill -0 "$server" || { echo "Der Server ist nicht gestartet (Port {{port}} belegt?)" >&2; exit 1; }
    if [ "{{benchmark}}" = "yes" ]; then
        log="${TMPDIR:-/tmp}/maexchen-benchmark-{{port}}.log"
        uv run --project clients/python maexchen-bot -i 127.0.0.1 -p {{port}} -n simple-benchmark -t simple >"$log" 2>&1 &
        pids+=("$!")
        echo "simple-Bot als Maßstab gestartet (Log: $log)"
    fi
    "$bin/maexchen_spectator" -p {{port}}

server *args:
    cargo run -p maexchen_server -- {{args}}

spectator *args:
    cargo run -p maexchen_spectator -- {{args}}

check:
    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    cd clients/rust && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
    cd clients/python && uv run --group lint --group test sh -c "ruff check src tests && ruff format --check src tests && mypy && ty check && pytest -q"
    cd clients/java && ./gradlew build

kit:
    git archive --format=zip --prefix=maexchen/ -o maexchen.zip HEAD:clients
