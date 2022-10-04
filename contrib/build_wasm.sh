#!/usr/bin/env sh

case "$(uname -s)" in
Darwin)
    export PATH="/opt/homebrew/opt/llvm/bin:$PATH" ─╯
    export CC=/opt/homebrew/opt/llvm/bin/clang
    export AR=/opt/homebrew/opt/llvm/bin/llvm-ar
    ;;
esac

pushd ../contracts/modules/mining_game

cargo build --target wasm32-unknown-unknown --release

popd
