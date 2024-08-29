// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {OwnableUpgradeable} from "@openzeppelin-upgrades/contracts/access/OwnableUpgradeable.sol";
import {AttestationVerifier} from "src/core/AttestationVerifier.sol";
import {IProverRegistry} from "src/interfaces/IProverRegistry.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";

contract ProverRegistry is OwnableUpgradeable, IProverRegistry {

    AttestationVerifier public verifier;
    uint256 public attestValiditySeconds = 3600;
    uint256 public maxBlockNumberDiff = 25;
    uint256 public nextInstanceId = 0;
    uint256 public chainID = 0;

    mapping(bytes32 reportHash => bool used) public attestedReports;
    mapping(uint256 proverInstanceID => ProverInstance) public attestedProvers;

    uint256[43] private __gap;

    constructor() {
        _disableInitializers();
    }

    function initialize(
        address _initialOwner,
        address _verifierAddr,
        uint256 _chainID
    ) public initializer {
        verifier = AttestationVerifier(_verifierAddr);
        chainID = _chainID;
        _transferOwnership(_initialOwner);
    }

    function reinitialize(
        uint8 i,
        address _initialOwner,
        address _verifierAddr,
        uint256 _chainID
    ) public reinitializer(i) {
        verifier = AttestationVerifier(_verifierAddr);
        chainID = _chainID;
        _transferOwnership(_initialOwner);
    }

    /// @notice register prover instance with quote
    function register(
        bytes calldata _report,
        ReportData calldata _data
    ) external {
        _checkBlockNumber(_data.referenceBlockNumber, _data.referenceBlockHash);
        bytes32 dataHash = keccak256(abi.encode(_data));

        verifier.verifyAttestation(_report, dataHash);

        bytes32 reportHash = keccak256(_report);
        if (attestedReports[reportHash]) revert REPORT_USED();
        attestedReports[reportHash] = true;

        uint256 instanceID = nextInstanceId;
        nextInstanceId += 1;

        uint256 validUnitl = block.timestamp + attestValiditySeconds;
        attestedProvers[instanceID] = ProverInstance(
            _data.addr,
            validUnitl,
            _data.teeType
        );

        emit InstanceAdded(instanceID, _data.addr, address(0), validUnitl);
    }

    /// TODO: should we need to add teeType?
    /// @notice validate whether the prover with (instanceID, address)
    function isProverValid(
        uint256 _id,
        address _instance
    ) external view returns (bool) {
        (bool valid, ) = _isProverValid(_id, _instance);
        return valid;
    }

    /// TODO: each proof should coming from different teeType
    /// @notice verify multiple proofs in one call
    function verifyProofs(Proof[] calldata _proofs) external {
        for (uint i = 0; i < _proofs.length; i++) {
            address oldInstance = ECDSA.recover(
                getSignedHash(
                    _proofs[i].poe.poe,
                    _proofs[i].poe.newInstance,
                    _proofs[i].ctx.prover,
                    _proofs[i].ctx.metaHash
                ),
                _proofs[i].poe.signature
            );

            _replaceInstance(
                _proofs[i].poe.id,
                _proofs[i].poe.teeType,
                oldInstance,
                _proofs[i].poe.newInstance
            );
        }
    }

    /// @notice key rotation for a prover instance
    function _replaceInstance(
        uint256 _id,
        TEEType _type,
        address _old,
        address _new
    ) private {
        (bool valid, ProverInstance memory prover) = _isProverValid(_id, _old);
        if (!valid) revert INVALID_PROVER_INSTANCE();
        if (_type != prover.teeType) revert PROVER_TYPE_MISMATCH();
        if (_old != _new) {
            attestedProvers[_id].addr = _new;
            emit InstanceAdded(_id, _old, _new, prover.validUntil);
        }
    }

    function getSignedHash(
        Poe memory _poe,
        address _newInstance,
        address _prover,
        bytes32 _metaHash
    ) public view returns (bytes32) {
        return
            keccak256(
                abi.encode(
                    "VERIFY_PROOF",
                    chainID,
                    address(this),
                    _poe,
                    _newInstance,
                    _prover,
                    _metaHash
                )
            );
    }

    // Due to the inherent unpredictability of blockHash, it mitigates the risk of mass-generation 
    //   of attestation reports in a short time frame, preventing their delayed and gradual exploitation.
    // This function will make sure the attestation report generated in recent ${maxBlockNumberDiff} blocks
    function _checkBlockNumber(
        uint256 blockNumber,
        bytes32 blockHash
    ) private view {
        if (blockNumber >= block.number) revert INVALID_BLOCK_NUMBER();
        if (block.number - blockNumber >= maxBlockNumberDiff)
            revert BLOCK_NUMBER_OUT_OF_DATE();
        if (blockhash(blockNumber) != blockHash) revert BLOCK_NUMBER_MISMATCH();
    }

    function _isProverValid(
        uint256 _instanceID,
        address _proverAddr
    ) private view returns (bool, ProverInstance memory) {
        ProverInstance memory prover;
        if (_instanceID == 0) return (false, prover);
        if (_proverAddr == address(0)) return (false, prover);
        prover = attestedProvers[_instanceID];
        return (
            prover.addr == _proverAddr && prover.validUntil >= block.timestamp,
            prover
        );
    }
}
