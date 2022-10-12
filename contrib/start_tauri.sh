#!/usr/bin/env bash
# enable common error handling options
set -o errexit
set -o nounset
set -o pipefail
CONF=$(cat disable_tauri_front.json)
export CONF
MASTERMINE_CONFIG=$(cat mastermine_config.json.template | envsubst)
export MASTERMINE_CONFIG
echo "$MASTERMINE_CONFIG" | jq
cd ../ux
case "$USE_RELEASE_TAURI" in
dev)
    echo "Building Tauri"
    echo "$CONF" | jq
    sleep 10
    yarn tauri dev -c "$CONF"
    ;;
debug)
    echo "Using Debug Tauri"
    ./src-tauri/target/debug/mastermine
    ;;
release)
    echo "Using Release Tauri"
    ./src-tauri/target/release/mastermine
    ;;
*)
    echo "Unknown Mode"
    ;;
esac
