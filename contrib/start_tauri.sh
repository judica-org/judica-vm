#!/usr/bin/env sh

export CONF=$(cat disable_tauri_front.json)
echo $CONF | jq
cd ../ux
yarn tauri dev -c "$CONF"