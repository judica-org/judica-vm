#!/usr/bin/env sh
cd ../ux-attest
export PORT=3002
export BROWSER=none
$(
    while true; do
        sleep 1 && curl http://localhost:$PORT && break
    done
    $(
        echo $PORTS | xargs -I{} python3 -m webbrowser "http://localhost:3002?service_url=http%3A%2F%2F127.0.0.1%3A{}"
    )
) &
yarn start react
