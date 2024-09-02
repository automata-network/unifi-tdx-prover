// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {IAttestation} from "src/interfaces/IAttestation.sol";

contract AttestationVerifier {
    IAttestation public attestationVerifier;

    constructor(address _attestationVerifierAddr) {
        attestationVerifier = IAttestation(_attestationVerifierAddr);
    }

    error INVALID_REPORT();
    error INVALID_REPORT_DATA();
    error REPORT_DATA_MISMATCH();

    function verifyAttestation(
        bytes calldata _report,
        bytes32 _userData
    ) public {
        if (address(attestationVerifier) == address(0)) return;

        (bool succ, bytes memory output) = attestationVerifier
            .verifyAndAttestOnChain(_report);
        if (!succ) revert INVALID_REPORT();

        if (output.length < 32) revert INVALID_REPORT_DATA();

        bytes32 quoteBodyLast32;
        assembly {
            quoteBodyLast32 := mload(
                add(add(output, 0x20), sub(mload(output), 32))
            )
        }

        if (quoteBodyLast32 != _userData) revert REPORT_DATA_MISMATCH();
    }

    function extractQuoteBody(
        bytes memory data
    ) public pure returns (bytes memory) {
        uint256 offset = 0;

        // Skip quoteVersion (2 bytes)
        offset += 2;
        // Skip tee (4 bytes)
        offset += 4;
        // Skip tcbStatus (1 byte)
        offset += 1;
        // Skip fmspcBytes (6 bytes)
        offset += 6;

        // Extract quoteBody (remaining bytes)
        bytes memory quoteBody = new bytes(64);
        for (uint256 i = 0; i < quoteBody.length; i++) {
            quoteBody[i] = data[offset + i];
        }

        return quoteBody;
    }
}
