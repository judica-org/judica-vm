#!/usr/bin/env bash
# enable common error handling options
set -o errexit
set -o nounset
set -o pipefail
GAME_HOST_CONFIG_JSON=$(cat game_host_config.json.template | envsubst)
export GAME_HOST_CONFIG_JSON
echo "$GAME_HOST_CONFIG_JSON" | jq
../target/release/game-host
