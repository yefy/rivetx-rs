#!/usr/bin/env bash
# Test MySQL / Redis URLs for rivetx-sql and anyproxy wasm plugins.
#
# Linux / macOS / Windows Git Bash (current shell):
#   source ./add_test_env_var.sh
#
# Must source (not bash ./...) so export stays in this shell.

TEST_IP="127.0.0.1"
TEST_PASSWORD="Yfygz@389"

add_test_env() {
    local name="$1"
    local value="$2"
    export "${name}=${value}"
    echo "${name}=${value}"
}

add_test_env TEST_RIVETX_MYSQL_HOST "mysql://root:${TEST_PASSWORD}@${TEST_IP}:3306?pool_min=100&pool_max=1000"
add_test_env TEST_RIVETX_MYSQL_URL "mysql://root:${TEST_PASSWORD}@${TEST_IP}:3306/test_db?pool_min=100&pool_max=1000"
add_test_env TEST_RIVETX_REDIS_URL "redis://${TEST_IP}:6379"

add_test_env TEST_ANYPROXY_MYSQL_HOST "mysql://root:${TEST_PASSWORD}@${TEST_IP}:3306?pool_min=100&pool_max=1000"
add_test_env TEST_ANYPROXY_MYSQL_URL "mysql://root:${TEST_PASSWORD}@${TEST_IP}:3306/test_db?pool_min=100&pool_max=1000"
add_test_env TEST_ANYPROXY_REDIS_URL "redis://${TEST_IP}:6379"

_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
case "$(uname -s 2>/dev/null)" in
    MINGW*|MSYS*|CYGWIN*)
        _script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -W 2>/dev/null || echo "${_script_dir}")"
        add_test_env TEST_HTTP_STATIC_PATH "C:/Users/yefy/Desktop/yefy/tools/nginx/nginx-1.18.0/html"
        add_test_env TEST_PROXY_CACHE_PATH "C:/Users/yefy/Desktop/proxy_cache"
        add_test_env TEST_PROXY_CACHE_PATH_2 "C:/Users/yefy/Desktop/proxy_cache_2"
        ;;
    *)
        add_test_env TEST_HTTP_STATIC_PATH "/root/Desktop/fdisk/nginx/nginx-1.18.0/nginx/html"
        add_test_env TEST_PROXY_CACHE_PATH "/root/Desktop/proxy_cache"
        add_test_env TEST_PROXY_CACHE_PATH_2 "/root/Desktop/proxy_cache_2"
        ;;
esac
add_test_env TEST_WASM_PATH "${_script_dir}/wasm-plugin-wit/target/wasm32-wasi/release/wasm_server.wasm"

