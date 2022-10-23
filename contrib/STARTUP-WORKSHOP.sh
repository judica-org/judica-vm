#!/usr/bin/env bash
tmux start-server
BTCPORT=${BTCPORT:-"18443"}

export USE_RELEASE_TAURI=${USE_RELEASE_TAURI:-"release"}

export BTCPORT
SCRIPT_LOCATION=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd $SCRIPT_LOCATION
export RUST_LOG=${RUST_LOG:-"info"}
ROOTPATH=${ROOTPATH:-"$HOME/demo-app-dir"}
export TOR_DIR="$HOME/demo-app-dir"

case "$(uname -s)" in
Darwin)

  export BITCOIN_DIR="$HOME/Library/Application Support/Bitcoin"
  BTCCOOKIE=${BTCCOOKIE:-"$HOME/Library/Application Support/Bitcoin/signet/.cookie"}
  ;;

Linux)
  export BITCOIN_DIR="$HOME/.bitcoin"
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
  cargo build --release || exit 255
  popd

  # Build WASM Module
  $SCRIPT_LOCATION/build_wasm.sh || exit 255
  export WASM_MODULE="$SCRIPT_LOCATION/../contracts/modules/target/wasm32-unknown-unknown/release/mining_game_contract.wasm"

  case "$USE_RELEASE_TAURI" in
  dev) ;;

  debug)
    pushd ../ux
    yarn tauri build --debug || exit 255
    popd
    ;;
  release)
    pushd ../ux
    yarn tauri build
    popd
    ;;
  esac
  # create a session with five panes
  tmux new-session -d -s MySession -n "www"
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

  if [[ -n $START_HOST ]]; then
    START_HOST_ADMINDB=15532
  fi
  export START_HOST_ADMINDB
  tmux split-window -t MySession:0 "export PORTS=\"$START_HOST_ADMINDB\"; $PWD/start_attest_www.sh; /usr/bin/env $SHELL -i"

  # Player 1
  tmux new-window -t MySession: -n "player-1" "export PLAYER=\"p1\" SOCKS_PORT=24402 ADMINDB_PORT=15533; $PWD/start_tauri.sh; /usr/bin/env $SHELL -i"
  tmux split-window -t MySession:1 "export PLAYER=\"p1\" SOCKS_PORT=14458 APP_PORT=13329 CONTROL_PORT=15533; $PWD/start_attest.sh; /usr/bin/env $SHELL -i"
  tmux split-window -t MySession:1 "export PLAYER=\"p1\"; $PWD/start_litigator.sh; /usr/bin/env $SHELL -i"

  if [[ -n $START_HOST ]]; then
    tmux new-window -t MySession: -n "host" "export PLAYER=\"host\"; $PWD/start_host.sh; /usr/bin/env $SHELL -i"
    tmux split-window -t MySession:2 "export PLAYER=\"host\" SOCKS_PORT=14457 APP_PORT=13328 CONTROL_PORT=15532; $PWD/start_attest.sh; /usr/bin/env $SHELL -i"
    tmux split-window -t MySession:2 "export PLAYER=\"host\"; $PWD/start_litigator.sh; /usr/bin/env $SHELL -i"
  fi

  # change layout to tiled
  tmux select-layout -t MySession:0 tiled
  tmux select-layout -t MySession:1 tiled
  if [[ -n $START_HOST ]]; then
    tmux select-layout -t MySession:2 tiled
  fi

  tmux attach -t MySession
fi
