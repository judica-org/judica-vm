#!/usr/bin/env bash
if [[ -n $START_HOST ]]; then
	export GAME_HOST_CONFIG_JSON=$(cat game_host_config.json.template | envsubst)
	echo $GAME_HOST_CONFIG_JSON | jq
	../target/release/game-host
fi
