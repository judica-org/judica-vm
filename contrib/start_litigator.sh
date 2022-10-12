#!/usr/bin/env bash
export LITIGATOR_CONFIG_JSON=$(cat litigator_config.json.template | envsubst)

echo $LITIGATOR_CONFIG_JSON | jq

../target/release/sapio-litigator run
