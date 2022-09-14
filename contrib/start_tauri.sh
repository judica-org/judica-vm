#!/usr/bin/env sh

export CONF=$(cat disable_tauri_front.json)
cd ../ux
if [[ $USE_RELEASE_TAURI -eq 1 ]]; then
    echo "Using Release Tauri"
    ./src-tauri/target/release/mine-with-friends
else
    echo "Building Tauri"
    echo $CONF | jq
    sleep 10
    yarn tauri dev -c "$CONF"
fi
