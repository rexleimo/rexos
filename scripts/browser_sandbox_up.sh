#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/browser_sandbox_up.sh [up|down|logs|ps] [--build] [--no-detach]

Commands:
  up      Start sandbox stack (default)
  down    Stop and remove sandbox stack
  logs    Tail sandbox logs
  ps      Show sandbox containers

Options:
  --build      Build image before starting (for `up`)
  --no-detach  Run compose in foreground (default is detached)
  -h, --help   Show this help
USAGE
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
COMPOSE_FILE="${REPO_ROOT}/docker/sandbox-browser/compose.yml"

if [[ ! -f "${COMPOSE_FILE}" ]]; then
  echo "compose file not found: ${COMPOSE_FILE}" >&2
  exit 1
fi

cmd="up"
build_flag=""
detach_flag="-d"

for arg in "$@"; do
  case "${arg}" in
    up|down|logs|ps)
      cmd="${arg}"
      ;;
    --build)
      build_flag="--build"
      ;;
    --no-detach)
      detach_flag=""
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: ${arg}" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if ! command -v docker >/dev/null 2>&1; then
  echo "docker command not found" >&2
  exit 1
fi

case "${cmd}" in
  up)
    docker compose -f "${COMPOSE_FILE}" up ${build_flag} ${detach_flag}
    cat <<'ENV_HINT'

Sandbox is up.
Set these before running RexOS browser tools:
  export REXOS_BROWSER_CDP_HTTP="http://127.0.0.1:9222"

Optional (only if you intentionally connect to non-loopback CDP):
  export REXOS_BROWSER_CDP_ALLOW_REMOTE=1

Observer UI (noVNC):
  http://127.0.0.1:6080/vnc.html
ENV_HINT
    ;;
  down)
    docker compose -f "${COMPOSE_FILE}" down
    ;;
  logs)
    docker compose -f "${COMPOSE_FILE}" logs -f
    ;;
  ps)
    docker compose -f "${COMPOSE_FILE}" ps
    ;;
esac
