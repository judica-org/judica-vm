#!/usr/bin/env bash
set -e
export CI=false

yarn build
echo "#################"
echo "# BUILDING BINS #"
echo "#################"
pushd ..
cargo build --release
popd
echo "#################"
echo "# BINS BUILT OK #"
echo "#################"

echo "#################"
echo "# BUILDING WASM #"
echo "#################"
case "$(uname -s)" in
Darwin)

    export PATH_BACK=$PATH
    export CC_BACK=$CC
    export AR_BACK=$AR

    export PATH="/usr/local/opt/llvm/bin:$PATH"
    export CC=/usr/local/opt/llvm/bin/clang
    export AR=/usr/local/opt/llvm/bin/llvm-ar
    if test -f "$CC"; then
        echo "Using CC=$CC AR=$AR PATH=$PATH"
    else
        export PATH="/opt/homebrew/opt/llvm/bin:$PATH_BACK"
        export CC=/opt/homebrew/opt/llvm/bin/clang
        export AR=/opt/homebrew/opt/llvm/bin/llvm-ar
        if test -f "$CC"; then
            echo "Using CC=$CC AR=$AR PATH=$PATH"
        else
            echo "You may need to: $ brew install llvm" && exit -1
        fi
    fi

    ;;

esac

pushd ../contracts/modules/mining_game
cargo build --target wasm32-unknown-unknown --release
popd
case "$(uname -s)" in
Darwin)

    export PATH=$PATH_BACK
    export CC=$CC_BACK
    export AR=$AR_BACK
    ;;
esac
echo "#################"
echo "# WASM BUILD OK #"
echo "#################"

echo "######################"
echo "# BUILDING EXTRA UXs #"
echo "######################"

pushd ../www/attest
yarn build
cd ..
cd game-host
yarn build
popd

echo "#########################"
echo "# BUILDING EXTRA UXs OK #"
echo "#########################"
