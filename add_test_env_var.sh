#!/usr/bin/env bash
# Test MySQL / Redis URLs for rivetx-sql and anyproxy wasm plugins.
#
# Linux / macOS / Git Bash (current shell):
#   source ./add_test_env_var.sh
#
# Windows Git Bash:
#   source ./add_test_env_var.sh
#   also writes User env via setx (new cmd / PowerShell / Cursor terminals).
#
# Windows PowerShell (current session, then persist for new terminals):
#   bash ./add_test_env_var.sh

TEST_IP="127.0.0.1"
TEST_PASSWORD="Yfygz@389"

add_test_env() {
    local name="$1"
    local value="$2"
    export "${name}=${value}"
    echo "${name}=${value}"

    case "$(uname -s 2>/dev/null)" in
        MINGW*|MSYS*|CYGWIN*)
            if command -v setx.exe >/dev/null 2>&1; then
                setx.exe "$name" "$value" >/dev/null
            fi
            ;;
    esac
}

add_test_env TEST_RIVETX_MYSQL_HOST "mysql://root:${TEST_PASSWORD}@${TEST_IP}:3306?pool_min=100&pool_max=1000"
add_test_env TEST_RIVETX_MYSQL_URL "mysql://root:${TEST_PASSWORD}@${TEST_IP}:3306/test_db?pool_min=100&pool_max=1000"
add_test_env TEST_RIVETX_REDIS_URL "redis://${TEST_IP}:6379"

add_test_env TEST_ANYPROXY_MYSQL_HOST "mysql://root:${TEST_PASSWORD}@${TEST_IP}:3306?pool_min=100&pool_max=1000"
add_test_env TEST_ANYPROXY_MYSQL_URL "mysql://root:${TEST_PASSWORD}@${TEST_IP}:3306/test_db?pool_min=100&pool_max=1000"
add_test_env TEST_ANYPROXY_REDIS_URL "redis://${TEST_IP}:6379"
