#!/usr/bin/env bash
export ATTEST_CONFIG_JSON=$(cat attest_config.json.template | envsubst)
echo $ATTEST_CONFIG_JSON | jq
../target/release/attest
