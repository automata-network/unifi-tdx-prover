#!/bin/bash -e

. $(dirname $0)/env.sh

function deploy() {
    if [[ "$ATTEST_VALIDITY_SECONDS" == "" ]]; then
        ATTEST_VALIDITY_SECONDS=3600
    fi
    if [[ "$MAX_BLOCK_NUMBER_DIFF" == "" ]]; then
        MAX_BLOCK_NUMBER_DIFF=25
    fi 
    CHAIN_ID=$CHAIN_ID \
    ATTEST_VALIDITY_SECONDS=$ATTEST_VALIDITY_SECONDS \
    MAX_BLOCK_NUMBER_DIFF=$MAX_BLOCK_NUMBER_DIFF \
    DEPLOY_KEY_SUFFIX=DEPLOY_KEY \
    ENV=$ENV \
    _script script/Deploy.s.sol --sig 'deployAll()'
}

"$@"