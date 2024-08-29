// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

interface IProverRegistry {
    struct Context {
        bytes32 metaHash;
        bytes32 blobHash;
        address prover;
        uint64 blockId;
        bool isContesting;
        bool blobUsed;
        address msgSender;
    }

    struct ProverInstance {
        address addr;
        uint256 validUntil;
        TEEType teeType;
    }

    struct Poe {
        bytes32 parentHash;
        bytes32 blockHash;
        bytes32 stateRoot;
        bytes32 graffiti;
    }

    struct SignedPoe {
        Poe poe;
        uint256 id;
        address newInstance;
        bytes signature;
        TEEType teeType;
    }

    struct Proof {
        SignedPoe poe;
        Context ctx;
    }

    enum TEEType {
        INTEL_TDX,
        AMD_SEV_SNP
    }

    struct ReportData {
        address addr;
        TEEType teeType;
        uint256 referenceBlockNumber;
        bytes32 referenceBlockHash;
    }

    error INVALID_BLOCK_NUMBER();
    error BLOCK_NUMBER_OUT_OF_DATE();
    error BLOCK_NUMBER_MISMATCH();
    error REPORT_USED();
    error INVALID_PROVER_INSTANCE();
    error PROVER_TYPE_MISMATCH();

    event InstanceAdded(
        uint256 indexed id,
        address indexed instance,
        address replaced,
        uint256 validUntil
    );

    /// @notice register prover instance with quote
    function register(
        bytes calldata _report,
        ReportData calldata _data
    ) external;

    /// TODO: should we need to add teeType?
    /// @notice validate whether the prover with (instanceID, address)
    function isProverValid(
        uint256 _id,
        address _instance
    ) external view returns (bool);

    /// TODO: each proof should coming from different teeType
    /// @notice verify multiple proofs in one call
    function verifyProofs(Proof[] calldata _proofs) external;
}
