#!/usr/bin/env sh

export CONF=$(cat disable_tauri_front.json)
echo $CONF | jq
cd ../ux
sleep 10
yarn tauri dev -c "$CONF"