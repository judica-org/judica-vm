#!/usr/bin/env bash
# enable common error handling options
set -o errexit
set -o nounset
set -o pipefail
cd ../www/attest
export PORT=3002
export BROWSER=none

while true; do
    sleep 1 && curl -s -o /dev/null http://localhost:$PORT && break
done && echo $PORTS | xargs -I{} python3 -m webbrowser "http://localhost:3002?service_url=http%3A%2F%2F127.0.0.1%3A{}" &

yarn start react
