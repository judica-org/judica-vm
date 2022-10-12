#!/usr/bin/env bash
# enable common error handling options
set -o errexit
set -o nounset
set -o pipefail
cd ../ux
export PORT=3000
export BROWSER=none
yarn start