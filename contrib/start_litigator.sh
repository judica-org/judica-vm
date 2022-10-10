#!/usr/bin/env sh
PIPENAME=$(mktemp -u)
mkfifo -m 600 "$PIPENAME"
export LITIGATOR_LOGFILE=$PIPENAME
if [[ -z "${SEQUENCER_KEY}" ]]; then
    echo "Paste in a key for the sequencer"
    read -r SEQUENCER_KEY
else
    echo "Key Already Selected $SEQUENCER_KEY"
fi
export SEQUENCER_KEY
export LITIGATOR_CONFIG_JSON=$(cat litigator_config.json.template | envsubst)

echo $LITIGATOR_CONFIG_JSON | jq

../target/release/sapio-litigator run
