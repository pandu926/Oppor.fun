// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;

import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {CampaignEscrow} from "./CampaignEscrow.sol";
import {CampaignConfig} from "./CampaignTypes.sol";

/// @notice Permissionless registry. No owner, upgrade, platform signer, or withdrawal capability.
contract CampaignFactory is ReentrancyGuard {
    mapping(address => mapping(bytes32 => address)) public campaignOf;
    error Unauthorized();
    error CampaignExists();
    event CampaignCreated(
        address indexed creator, bytes32 indexed campaignKey, address indexed escrow, bytes32 configHash
    );

    function createCampaign(CampaignConfig calldata config) external nonReentrant returns (address escrow) {
        if (config.creator != msg.sender) revert Unauthorized();
        if (campaignOf[msg.sender][config.campaignKey] != address(0)) revert CampaignExists();
        escrow = address(new CampaignEscrow(config));
        campaignOf[msg.sender][config.campaignKey] = escrow;
        emit CampaignCreated(msg.sender, config.campaignKey, escrow, keccak256(abi.encode(config)));
    }
}
