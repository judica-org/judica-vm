#!/usr/bin/env bash
# enable common error handling options
set -o errexit
set -o nounset
set -o pipefail
cd ../www/game-host
export PORT=3001
export BROWSER=NONE
export PORTOFSERVICE=11409
export URL="http://localhost:$PORT?service_url=http%3A%2F%2F127.0.0.1%3A$PORTOFSERVICE"

while true; do
    sleep 1 && curl -s -o /dev/null "$URL" && break
done && python3 -m webbrowser "$URL" &

yarn start react
