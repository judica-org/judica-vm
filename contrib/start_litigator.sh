#!/usr/bin/env bash
# enable common error handling options
set -o errexit
set -o nounset
set -o pipefail
LITIGATOR_CONFIG_JSON=$(cat litigator_config.json.template | envsubst)
export LITIGATOR_CONFIG_JSON

echo "$LITIGATOR_CONFIG_JSON" | jq

../target/release/sapio-litigator run
