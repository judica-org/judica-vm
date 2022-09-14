#!/usr/bin/env sh
tmux start-server
BTCPORT=${BTCPORT:-"18443"}

# dev, debug, release
export USE_RELEASE_TAURI=${USE_RELEASE_TAURI:-"dev"}

export BTCPORT
SCRIPT_LOCATION=$(dirname -- "$(readlink -f -- "$BASH_SOURCE")")
cd $SCRIPT_LOCATION
export RUST_LOG=debug
ROOTPATH=${ROOTPATH:-"$HOME/demo-app-dir"}
export TOR_DIR="$HOME/demo-app-dir"

case "$(uname -s)" in
Darwin)

  export BITCOIN_DIR="$HOME/Library/Application Support/Bitcoin"
  BTCCOOKIE=${BTCCOOKIE:-"$HOME/Library/Application Support/Bitcoin/signet/.cookie"}
  ;;

Linux)
  export BITCOIN_DIR="$HOME/~/.bitcoin"
  BTCCOOKIE=${BTCCOOKIE:-"$HOME/.bitcoin/signet/.cookie"}
  ;;
*)
  echo "$(uname -s) Not Supported"
  exit 1
  ;;
esac

if tmux attach -t MySession; then
  echo "Exiting"
else

  pushd ..
  cargo build --release
  popd

  case "$USE_RELEASE_TAURI" in
  dev) ;;

  debug)
    pushd ../ux
    yarn tauri build --debug
    popd
    ;;
  release)
    pushd ../ux
    yarn tauri build
    popd
    ;;
  esac
  # create a session with five panes
  tmux new-session -d -s MySession -n "www" -d "$PWD/start_host_www.sh; /usr/bin/env $SHELL -i"
  case "$USE_RELEASE_TAURI" in
  dev)
    tmux split-window -t MySession:0 "$PWD/start_tauri_front.sh; /usr/bin/env $SHELL -i"
    ;;

  debug)
    echo "Not Starting Tauri Frontend, Debug Mode"
    ;;

  release)
    echo "Not Starting Tauri Frontend, Release Mode"
    ;;
  esac
  tmux split-window -t MySession:0 "export PORTS=\"15532\n15533\n15534\"; $PWD/start_attest_www.sh; /usr/bin/env $SHELL -i"

  tmux new-window -t MySession: -n "host" "export PLAYER=\"host\"; $PWD/start_host.sh; /usr/bin/env $SHELL -i"
  tmux split-window -t MySession:1 "export PLAYER=\"host\" SOCKS_PORT=14457 APP_PORT=13328 CONTROL_PORT=15532; $PWD/start_attest.sh; /usr/bin/env $SHELL -i"
  tmux split-window -t MySession:1 "export PLAYER=\"host\"; $PWD/start_litigator.sh; /usr/bin/env $SHELL -i"

  # Player 1
  tmux new-window -t MySession: -n "player-1" "$PWD/start_tauri.sh; /usr/bin/env $SHELL -i"
  tmux split-window -t MySession:2 "export PLAYER=\"p1\" SOCKS_PORT=14458 APP_PORT=13329 CONTROL_PORT=15533; $PWD/start_attest.sh; /usr/bin/env $SHELL -i"
  tmux split-window -t MySession:2 "export PLAYER=\"p1\"; $PWD/start_litigator.sh; /usr/bin/env $SHELL -i"

  # Player 2
  tmux new-window -t MySession: -n "player-2" "$PWD/start_tauri.sh; /usr/bin/env $SHELL -i"
  tmux split-window -t MySession:3 "export PLAYER=\"p2\" SOCKS_PORT=14459 APP_PORT=13330 CONTROL_PORT=15534; $PWD/start_attest.sh; /usr/bin/env $SHELL -i"
  tmux split-window -t MySession:3 "export PLAYER=\"p2\"; $PWD/start_litigator.sh; /usr/bin/env $SHELL -i"

  # change layout to tiled
  tmux select-layout -t MySession:0 tiled
  tmux select-layout -t MySession:1 tiled
  tmux select-layout -t MySession:2 tiled
  tmux select-layout -t MySession:3 tiled

  tmux attach -t MySession
fi
