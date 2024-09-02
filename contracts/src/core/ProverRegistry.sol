// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {OwnableUpgradeable} from "@openzeppelin-upgrades/contracts/access/OwnableUpgradeable.sol";
import {AttestationVerifier} from "src/core/AttestationVerifier.sol";
import {IProverRegistry} from "src/interfaces/IProverRegistry.sol";
import "@openzeppelin/contracts/utils/cryptography/ECDSA.sol";

contract ProverRegistry is OwnableUpgradeable, IProverRegistry {

    AttestationVerifier public verifier;
    uint256 public attestValiditySeconds;
    uint256 public maxBlockNumberDiff;
    uint256 public chainID;
    uint256 public nextInstanceId = 0;

    mapping(bytes32 reportHash => bool used) public attestedReports;
    mapping(uint256 proverInstanceID => ProverInstance) public attestedProvers;

    uint256[43] private __gap;

    constructor() {
        _disableInitializers();
    }

    function initialize(
        address _initialOwner,
        address _verifierAddr,
        uint256 _chainID,
        uint256 _attestValiditySeconds,
        uint256 _maxBlockNumberDiff
    ) public initializer {
        verifier = AttestationVerifier(_verifierAddr);
        chainID = _chainID;
        attestValiditySeconds = _attestValiditySeconds;
        maxBlockNumberDiff = _maxBlockNumberDiff;
        _transferOwnership(_initialOwner);
    }

    function reinitialize(
        uint8 i,
        address _initialOwner,
        address _verifierAddr,
        uint256 _chainID,
        uint256 _attestValiditySeconds,
        uint256 _maxBlockNumberDiff
    ) public reinitializer(i) {
        verifier = AttestationVerifier(_verifierAddr);
        chainID = _chainID;
        attestValiditySeconds = _attestValiditySeconds;
        maxBlockNumberDiff = _maxBlockNumberDiff;
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

        uint256 instanceID = nextInstanceId+1;
        nextInstanceId += 1;

        uint256 validUnitl = block.timestamp + attestValiditySeconds;
        attestedProvers[instanceID] = ProverInstance(
            _data.addr,
            validUnitl,
            _data.teeType
        );

        emit InstanceAdded(instanceID, _data.addr, address(0), validUnitl);
    }

    /// TODO: each proof should coming from different teeType
    /// @notice verify multiple proofs in one call
    function verifyProofs(Proof[] calldata _proofs) external {
        require(_proofs.length >= 1, "missing proofs");
        for (uint i = 0; i < _proofs.length; i++) {
            IProverRegistry.SignedPoe calldata poe = _proofs[i].poe;
            address oldInstance = recoverOldInstance(
                poe.poe,
                poe.newInstance,
                _proofs[i].ctx.prover,
                _proofs[i].ctx.metaHash,
                poe.signature
            );


            ProverInstance memory prover = checkProver(poe.id, oldInstance);
            if (poe.teeType != prover.teeType) revert PROVER_TYPE_MISMATCH();
            if (oldInstance != poe.newInstance) {
                attestedProvers[poe.id].addr = poe.newInstance;
                emit InstanceAdded(poe.id, oldInstance, poe.newInstance, prover.validUntil);
            }
        }

        emit VerifyProof(_proofs.length);
    }

    function recoverOldInstance(
        Poe memory _poe,
        address _newInstance,
        address _prover,
        bytes32 _metaHash,
        bytes memory _signature
    ) public view returns (address) {
        return ECDSA.recover(
            getSignedHash(
                _poe,
                _newInstance,
                _prover,
                _metaHash
            ),
            _signature
        );
    }

    function checkProver(
        uint256 _instanceID,
        address _proverAddr
    ) public view returns (ProverInstance memory) {
        ProverInstance memory prover;
        if (_instanceID == 0) revert PROVER_INVALID_INSTANCE_ID(_instanceID);
        if (_proverAddr == address(0)) revert PROVER_INVALID_ADDR(_proverAddr);
        prover = attestedProvers[_instanceID];
        if (prover.addr != _proverAddr) revert PROVER_ADDR_MISMATCH(prover.addr, _proverAddr);
        if (prover.validUntil < block.timestamp) revert PROVER_OUT_OF_DATE(prover.validUntil);
        return prover;
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
}
