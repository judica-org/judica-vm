#!/usr/bin/env sh
PIPENAME=$(mktemp -u)
mkfifo -m 600 "$PIPENAME"
export LITIGATOR_LOGFILE=$PIPENAME
export LITIGATOR_CONFIG_JSON=$(cat litigator_config.json.template | envsubst)
echo $LITIGATOR_CONFIG_JSON | jq

../target/release/sapio-litigator