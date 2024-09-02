// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.12;

import "forge-std/Script.sol";
import {VmSafe} from "forge-std/Vm.sol";
import {AttestationVerifier} from "src/core/AttestationVerifier.sol";
import {ProverRegistry} from "src/core/ProverRegistry.sol";

import {TransparentUpgradeableProxy, ITransparentUpgradeableProxy} from "@openzeppelin/contracts/proxy/transparent/TransparentUpgradeableProxy.sol";
import {ProxyAdmin} from "@openzeppelin/contracts/proxy/transparent/ProxyAdmin.sol";
import {EmptyContract} from "./EmptyContract.sol";

contract Deploy is Script {
    function setUp() public {}

    function getOutputFilePath() private view returns (string memory) {
        string memory env = vm.envString("ENV");
        return
            string.concat(
                vm.projectRoot(),
                "/deployment/tee_deploy_",
                env,
                ".json"
            );
    }

    function readJson() private returns (string memory) {
        bytes32 remark = keccak256(abi.encodePacked("remark"));
        string memory output = vm.readFile(getOutputFilePath());
        string[] memory keys = vm.parseJsonKeys(output, ".");
        for (uint i = 0; i < keys.length; i++) {
            if (keccak256(abi.encodePacked(keys[i])) == remark) {
                continue;
            }
            string memory keyPath = string(abi.encodePacked(".", keys[i]));
            vm.serializeAddress(
                output,
                keys[i],
                vm.parseJsonAddress(output, keyPath)
            );
        }
        return output;
    }

    function saveJson(string memory json) private {
        string memory finalJson = vm.serializeString(
            json,
            "remark",
            "Deployment"
        );
        vm.writeJson(finalJson, getOutputFilePath());
    }

    function deployVerifier() public {
        address attestation = vm.envAddress("ATTESTATION");
        vm.startBroadcast();
        AttestationVerifier verifier = new AttestationVerifier(attestation);
        vm.stopBroadcast();
        string memory output = readJson();
        vm.serializeAddress(output, "AttestationVerifier", address(verifier));
        saveJson(output);
    }

    function deployProxyAdmin() public {
        string memory output = readJson();
        vm.startBroadcast();
        ProxyAdmin proxyAdmin = new ProxyAdmin();
        vm.stopBroadcast();
        vm.serializeAddress(output, "ProxyAdmin", address(proxyAdmin));
        saveJson(output);
    }

    function deployProverVerifier() public {
        uint256 chainID = vm.envUint("CHAIN_ID");
        uint256 version = vm.envUint("VERSION");
        uint256 attestValiditySeconds = vm.envUint("ATTEST_VALIDITY_SECONDS");
        uint256 maxBlockNumberDiff = vm.envUint("MAX_BLOCK_NUMBER_DIFF");
        require(version < 255, "version overflowed");

        string memory output = readJson();
        ProxyAdmin proxyAdmin = ProxyAdmin(
            vm.parseJsonAddress(output, ".ProxyAdmin")
        );
        address attestationAddr = vm.parseJsonAddress(
            output,
            ".AttestationVerifier"
        );

        address registryProxyAddr;
        vm.startBroadcast();
        ProverRegistry registryImpl = new ProverRegistry();
        bytes memory initializeCall;
        if (vm.keyExistsJson(output, ".ProxyRegistryProxy") && version > 1) {
            registryProxyAddr = vm.parseJsonAddress(
                output,
                ".ProxyRegistryProxy"
            );
            console.log("reuse proxy");
            console.logAddress(registryProxyAddr);
        } else {
            console.log("Deploy new proxy");
            EmptyContract emptyContract = new EmptyContract();
            registryProxyAddr = address(
                new TransparentUpgradeableProxy(
                    address(emptyContract),
                    address(proxyAdmin),
                    ""
                )
            );
        }
        if (version <= 1) {
            initializeCall = abi.encodeWithSelector(
                ProverRegistry.initialize.selector,
                msg.sender,
                address(attestationAddr),
                chainID,
                attestValiditySeconds,
                maxBlockNumberDiff
            );
        } else {
            initializeCall = abi.encodeWithSelector(
                ProverRegistry.reinitialize.selector,
                version,
                msg.sender,
                address(attestationAddr),
                chainID,
                attestValiditySeconds,
                maxBlockNumberDiff
            );
        }
        proxyAdmin.upgradeAndCall(
            ITransparentUpgradeableProxy(registryProxyAddr),
            address(registryImpl),
            initializeCall
        );
        vm.stopBroadcast();

        vm.serializeAddress(output, "ProxyRegistryProxy", registryProxyAddr);
        vm.serializeAddress(output, "ProxyRegistryImpl", address(registryImpl));
        saveJson(output);
    }

    function deployAll() public {
        deployProxyAdmin();
        deployVerifier();
        deployProverVerifier();
    }
}
