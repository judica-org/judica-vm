#!/usr/bin/env bash
# enable common error handling options
set -o errexit
set -o nounset
set -o pipefail
ATTEST_CONFIG_JSON=$(cat attest_config.json.template | envsubst)
export ATTEST_CONFIG_JSON
echo "$ATTEST_CONFIG_JSON" | jq
../target/release/attest
