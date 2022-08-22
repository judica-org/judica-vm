#!/usr/bin/env sh
tmux start-server
BTCPORT=${BTCPORT:-"18443"}
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
ROOTPATH=${ROOTPATH:-"$HOME/demo-app-dir"}
mkdir -p $ROOTPATH
mkdir -p $ROOTPATH/attest/tor/onion
mkdir -p $ROOTPATH/game-host/tor/onion
export RUST_LOG=debug
export ATTEST_CONFIG_JSON="{\
                \"bitcoin\": {\"url\": \"http://127.0.0.1:$BTCPORT\", \"auth\": {\"CookieFile\":\"$BTCCOOKIE\"}},\
                \"subname\": \"testing\",\
                \"tor\":{\"directory\":\"$ROOTPATH/attest/tor\",\
                         \"socks_port\":14457,\
                         \"application_port\":13328,\
                         \"application_path\": \"service1\"}\
                            }"
export GAME_HOST_CONFIG_JSON="{\
                \"tor\":{\"directory\":\"$ROOTPATH/game-host/tor\", \"socks_port\":14458,\"application_port\":13329 ,\"application_path\": \"service1\"},\
                \"key\":\"d4586672776d42aa3359acf585b7f7f40bb6a1fda027904a3cf07c2a07115c99\"\
                                }"

echo "Using Config"
echo $ATTEST_CONFIG_JSON | jq
echo $GAME_HOST_CONFIG_JSON | jq

if tmux attach -tMySession
then
    echo "Exiting";
else
    # create a session with five panes
    tmux new-session -d -s MySession -n Shell1 -d "/usr/bin/env $SHELL -c \"echo 'first shell'\"; /usr/bin/env $SHELL -i"
    tmux split-window -t MySession:0 "/usr/bin/env $SHELL -c \"cd ux; yarn tauri dev; /usr/bin/env $SHELL -i\""
    tmux split-window -t MySession:0 "/usr/bin/env $SHELL -c \"./target/release/game-host\"; /usr/bin/env $SHELL -i"
    tmux split-window -t MySession:0 "/usr/bin/env $SHELL -c \"./target/release/attest\"; /usr/bin/env $SHELL -i"

    # change layout to tiled
    tmux select-layout -t MySession:0 tiled

    tmux attach -tMySession
fi