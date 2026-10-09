// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;

/// @notice Numeric kinds and field order are the backend ABI contract.
struct CampaignConfig {
    bytes32 campaignKey;
    address creator;
    address refundRecipient;
    uint8 assetKind; // 0: ERC20, 1: ERC721, 2: ERC1155
    address rewardToken;
    uint256 rewardTokenId;
    uint256 targetQuantity;
    uint64 startsAt;
    uint64 cutoffAt;
    uint64 reviewDeadline;
    uint64 claimDeadline;
    uint8 mode; // 0: all eligible, 1: raffle (allocation policy enforced offchain)
    uint32 participantCapacity;
    uint32 winnerCount;
    bytes32 rulesHash;
    bytes32 raffleSeedCommitment;
}

struct Claim {
    uint256 index;
    address recipient;
    uint256 tokenId;
    uint256 quantity;
}
