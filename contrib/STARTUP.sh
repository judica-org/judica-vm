#!/usr/bin/env sh
tmux start-server
BTCPORT=${BTCPORT:-"18443"}
SCRIPT_LOCATION=$(dirname -- "$(readlink -f -- "$BASH_SOURCE")")
cd $SCRIPT_LOCATION
export RUST_LOG=debug
ROOTPATH=${ROOTPATH:-"$HOME/demo-app-dir"}

case "$(uname -s)" in
   Darwin)

        BTCCOOKIE=${BTCCOOKIE:-"$HOME/Library/Application Support/Bitcoin/signet/.cookie"}
     ;;

   Linux)
        BTCCOOKIE=${BTCCOOKIE:-"$HOME/.bitcoin/signet/.cookie"}
     ;;
   *)
     echo "$(uname -s) Not Supported" 
     exit 1
     ;;
esac




echo "Using Config"
echo $ATTEST_CONFIG_JSON | jq 
echo $GAME_HOST_CONFIG_JSON | jq
if tmux attach -tMySession
then
    echo "Exiting";
else
    # create a session with five panes
    tmux new-session -d -s MySession -n Shell1 -d "/usr/bin/env $SHELL -c \"echo 'first shell'\"; /usr/bin/env $SHELL -i"
    tmux split-window -t MySession:0 "$PWD/start_tauri.sh; /usr/bin/env $SHELL -i"
    tmux split-window -t MySession:0 "$PWD/start_host.sh; /usr/bin/env $SHELL -i"
    tmux split-window -t MySession:0 "$PWD/start_attest.sh; /usr/bin/env $SHELL -i"

    # change layout to tiled
    tmux select-layout -t MySession:0 tiled

    tmux attach -tMySession
fi