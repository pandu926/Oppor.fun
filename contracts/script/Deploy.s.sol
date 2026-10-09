// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;
import {Script} from "forge-std/Script.sol";
import {console2} from "forge-std/console2.sol";
import {CampaignFactory} from "../src/CampaignFactory.sol";

/// @notice Explicit chain/sender guards. Simulation is the default; --broadcast is a separate deployment step.
contract Deploy is Script {
    error WrongChain();
    error InvalidDeployer();

    function run(uint256 expectedChainId, address expectedDeployer) external returns (CampaignFactory factory) {
        if (block.chainid != expectedChainId) revert WrongChain();
        if (expectedDeployer == address(0)) revert InvalidDeployer();
        vm.startBroadcast(expectedDeployer);
        factory = new CampaignFactory();
        vm.stopBroadcast();
        console2.log("Factory:", address(factory));
        console2.logBytes32(address(factory).codehash);
    }
}
