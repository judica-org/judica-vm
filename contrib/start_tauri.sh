#!/usr/bin/env sh

export CONF=$(cat disable_tauri_front.json)
export MASTERMINE_CONFIG=$(cat mastermine_config.json.template | envsubst)
echo $MASTERMINE_CONFIG | jq
cd ../ux
case "$USE_RELEASE_TAURI" in
dev)
    echo "Building Tauri"
    echo $CONF | jq
    sleep 10
    yarn tauri dev -c "$CONF"
    ;;

debug)
    echo "Using Debug Tauri"
    ./src-tauri/target/debug/mine-with-friends
    ;;

release)
    echo "Using Release Tauri"
    ./src-tauri/target/release/mine-with-friends
    ;;
esac
