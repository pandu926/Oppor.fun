// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {IERC721} from "@openzeppelin/contracts/token/ERC721/IERC721.sol";
import {IERC721Receiver} from "@openzeppelin/contracts/token/ERC721/IERC721Receiver.sol";
import {IERC1155} from "@openzeppelin/contracts/token/ERC1155/IERC1155.sol";
import {IERC1155Receiver} from "@openzeppelin/contracts/token/ERC1155/IERC1155Receiver.sol";
import {IERC165} from "@openzeppelin/contracts/utils/introspection/IERC165.sol";
import {ERC165Checker} from "@openzeppelin/contracts/utils/introspection/ERC165Checker.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {MerkleProof} from "@openzeppelin/contracts/utils/cryptography/MerkleProof.sol";
import {CampaignConfig, Claim} from "./CampaignTypes.sol";

/// @notice Non-upgradeable, single-asset campaign escrow. No platform withdrawal authority.
/// @dev Creator attests allocations. A Merkle proof does not prove total sums or fair eligibility.
contract CampaignEscrow is ReentrancyGuard, IERC721Receiver, IERC1155Receiver {
    using SafeERC20 for IERC20;
    using ERC165Checker for address;

    enum CampaignState {
        DRAFT,
        ACTIVE,
        FINALIZED,
        CANCELLED,
        CLOSED
    }
    uint256 public constant MAX_NFTS = 100;
    uint256 public constant MAX_LEAVES = 100_000;
    uint256 public constant MAX_PROOF_LENGTH = 32;
    uint256 public constant MIN_CLAIM_WINDOW = 1 days;

    // Storage configuration makes runtime bytecode identical across all campaigns:
    // backend admission verifies one deployed-code hash, independent of config values.
    CampaignConfig public configuration;
    bytes32 public configHash;
    CampaignState public state;
    uint256 public fundedQuantity;
    uint256 public allocatedQuantity;
    uint256 public claimedQuantity;
    uint256 public leafCount;
    uint256 public claimedLeafCount;
    bytes32 public distributionRoot;
    bytes32 public manifestHash;
    bytes32 public eligibilityHash;
    mapping(uint256 => uint256) private claimedBitmap;
    mapping(uint256 => bool) private depositedNFT;
    mapping(uint256 => bool) public deliveredNFT;
    uint256[] private inventory;
    bool private expectingDeposit;
    uint256 private expectedId;
    uint256 private expectedQuantity;

    error Unauthorized();
    error InvalidConfig();
    error InvalidState();
    error WrongAssetKind();
    error FundingClosed();
    error FundingExceeded();
    error FundingIncomplete();
    error UnexpectedNFTTransfer();
    error DuplicateNFT();
    error CutoffNotReached();
    error ReviewExpired();
    error ClaimExpired();
    error AlreadyClaimed();
    error InvalidProof();
    error InvalidAllocation();
    error AllocationExceeded();
    error UnsupportedTokenBehavior();
    error RefundNotAvailable();

    event RewardFunded(uint8 assetKind, address indexed token, uint256 tokenId, uint256 quantity);
    event CampaignActivated(bytes32 indexed configHash);
    event DistributionFinalized(
        bytes32 indexed root,
        bytes32 manifestHash,
        bytes32 eligibilityHash,
        uint256 allocatedQuantity,
        uint256 leafCount
    );
    event RewardClaimed(uint256 indexed index, address indexed recipient, uint256 tokenId, uint256 quantity);
    event CampaignCancelled(address indexed refundRecipient);
    event RemainingSwept(address indexed refundRecipient, uint256 quantity);

    constructor(CampaignConfig memory c) {
        if (
            c.campaignKey == bytes32(0) || c.creator == address(0) || c.refundRecipient == address(0)
                || c.refundRecipient == address(this) || c.rewardToken.code.length == 0
                || c.rewardToken == address(this) || c.targetQuantity == 0 || c.rulesHash == bytes32(0)
                || c.assetKind > 2 || c.mode > 1
                || !(block.timestamp < c.startsAt
                    && c.startsAt < c.cutoffAt
                    && c.cutoffAt < c.reviewDeadline
                    && c.reviewDeadline < c.claimDeadline)
                || uint256(c.claimDeadline) - c.reviewDeadline < MIN_CLAIM_WINDOW || c.participantCapacity > MAX_LEAVES
                || c.winnerCount > MAX_LEAVES
                || (c.mode == 0 && (c.winnerCount != 0 || c.raffleSeedCommitment != bytes32(0)))
                || (c.mode == 1 && (c.winnerCount == 0 || c.raffleSeedCommitment == bytes32(0)))
                || (c.assetKind != 2 && c.rewardTokenId != 0)
        ) revert InvalidConfig();
        if (c.assetKind == 1) {
            if (
                c.targetQuantity > MAX_NFTS || !c.rewardToken.supportsInterface(type(IERC721).interfaceId)
                    || (c.mode == 0 && (c.participantCapacity == 0 || c.participantCapacity > c.targetQuantity))
                    || (c.mode == 1 && c.winnerCount > c.targetQuantity)
            ) revert InvalidConfig();
        } else if (c.assetKind == 2 && !c.rewardToken.supportsInterface(type(IERC1155).interfaceId)) {
            revert InvalidConfig();
        }
        configuration = c;
        configHash = keccak256(abi.encode(c));
    }

    modifier onlyCreator() {
        if (msg.sender != configuration.creator) revert Unauthorized();
        _;
    }

    function isClaimed(uint256 index) public view returns (bool) {
        return claimedBitmap[index >> 8] & (uint256(1) << (index & 255)) != 0;
    }

    function isDepositedNFT(uint256 tokenId) external view returns (bool) {
        return depositedNFT[tokenId];
    }

    function nftInventory() external view returns (uint256[] memory) {
        return inventory;
    }

    function fundERC20(uint256 amount) external onlyCreator nonReentrant {
        _fundingCheck(0, amount);
        IERC20 token = IERC20(configuration.rewardToken);
        uint256 beforeEscrow = token.balanceOf(address(this));
        uint256 beforeCreator = token.balanceOf(msg.sender);
        fundedQuantity += amount;
        token.safeTransferFrom(msg.sender, address(this), amount);
        _checkDelta(beforeEscrow, token.balanceOf(address(this)), amount, true);
        _checkDelta(beforeCreator, token.balanceOf(msg.sender), amount, false);
        emit RewardFunded(0, address(token), 0, amount);
    }

    function fundERC721(uint256[] calldata tokenIds) external onlyCreator nonReentrant {
        _fundingCheck(1, tokenIds.length);
        IERC721 token = IERC721(configuration.rewardToken);
        fundedQuantity += tokenIds.length;
        for (uint256 i; i < tokenIds.length; ++i) {
            uint256 id = tokenIds[i];
            if (depositedNFT[id]) revert DuplicateNFT();
            if (token.ownerOf(id) != msg.sender) revert Unauthorized();
            depositedNFT[id] = true;
            inventory.push(id);
            _expectNFT(id, 1);
            token.safeTransferFrom(msg.sender, address(this), id);
            if (expectingDeposit || token.ownerOf(id) != address(this)) revert UnsupportedTokenBehavior();
            emit RewardFunded(1, address(token), id, 1);
        }
    }

    function fundERC1155(uint256 amount) external onlyCreator nonReentrant {
        _fundingCheck(2, amount);
        IERC1155 token = IERC1155(configuration.rewardToken);
        uint256 id = configuration.rewardTokenId;
        uint256 beforeEscrow = token.balanceOf(address(this), id);
        uint256 beforeCreator = token.balanceOf(msg.sender, id);
        fundedQuantity += amount;
        _expectNFT(id, amount);
        token.safeTransferFrom(msg.sender, address(this), id, amount, "");
        if (expectingDeposit) revert UnsupportedTokenBehavior();
        _checkDelta(beforeEscrow, token.balanceOf(address(this), id), amount, true);
        _checkDelta(beforeCreator, token.balanceOf(msg.sender, id), amount, false);
        emit RewardFunded(2, address(token), id, amount);
    }

    function activate() external onlyCreator nonReentrant {
        if (state != CampaignState.DRAFT) revert InvalidState();
        if (block.timestamp >= configuration.startsAt) revert FundingClosed();
        if (fundedQuantity != configuration.targetQuantity) revert FundingIncomplete();
        state = CampaignState.ACTIVE;
        emit CampaignActivated(configHash);
    }

    function finalize(
        bytes32 root,
        bytes32 allocationManifestHash,
        bytes32 eligibilitySnapshotHash,
        uint256 declaredAllocatedQuantity,
        uint256 declaredLeafCount
    ) external onlyCreator nonReentrant {
        if (state != CampaignState.ACTIVE) revert InvalidState();
        if (block.timestamp < configuration.cutoffAt) revert CutoffNotReached();
        if (block.timestamp >= configuration.reviewDeadline) revert ReviewExpired();
        if (
            allocationManifestHash == bytes32(0) || eligibilitySnapshotHash == bytes32(0)
                || declaredLeafCount > MAX_LEAVES || declaredAllocatedQuantity > fundedQuantity
                // Every payable leaf requires at least one base unit (one NFT for ERC721).
                || declaredLeafCount > declaredAllocatedQuantity
                || (declaredLeafCount == 0 && (root != bytes32(0) || declaredAllocatedQuantity != 0))
                || (declaredLeafCount != 0 && (root == bytes32(0) || declaredAllocatedQuantity == 0))
                || (configuration.assetKind == 1 && declaredAllocatedQuantity != declaredLeafCount)
        ) revert InvalidAllocation();
        distributionRoot = root;
        manifestHash = allocationManifestHash;
        eligibilityHash = eligibilitySnapshotHash;
        allocatedQuantity = declaredAllocatedQuantity;
        leafCount = declaredLeafCount;
        state = CampaignState.FINALIZED;
        emit DistributionFinalized(
            root, allocationManifestHash, eligibilitySnapshotHash, declaredAllocatedQuantity, declaredLeafCount
        );
    }

    /// @notice The payout wallet itself calls claim. Relayers may execute through that wallet.
    function claim(Claim calldata allocation, bytes32[] calldata proof) external nonReentrant {
        if (state != CampaignState.FINALIZED) revert InvalidState();
        if (block.timestamp >= configuration.claimDeadline) revert ClaimExpired();
        if (msg.sender != allocation.recipient) revert Unauthorized();
        if (
            allocation.recipient == address(0) || allocation.recipient == address(this) || allocation.index >= leafCount
                || allocation.quantity == 0
        ) revert InvalidAllocation();
        if (isClaimed(allocation.index)) revert AlreadyClaimed();
        uint8 kind = configuration.assetKind;
        if (
            (kind == 0 && allocation.tokenId != 0)
                || (kind == 1
                    && (allocation.quantity != 1
                        || !depositedNFT[allocation.tokenId]
                        || deliveredNFT[allocation.tokenId]))
                || (kind == 2 && allocation.tokenId != configuration.rewardTokenId)
        ) revert InvalidAllocation();
        if (
            proof.length > MAX_PROOF_LENGTH
                || !MerkleProof.verifyCalldata(proof, distributionRoot, claimLeaf(allocation))
        ) revert InvalidProof();
        if (allocation.quantity > allocatedQuantity - claimedQuantity) revert AllocationExceeded();
        claimedBitmap[allocation.index >> 8] |= uint256(1) << (allocation.index & 255);
        claimedQuantity += allocation.quantity;
        ++claimedLeafCount;
        if (kind == 1) deliveredNFT[allocation.tokenId] = true;
        _pay(allocation.recipient, allocation.tokenId, allocation.quantity);
        emit RewardClaimed(allocation.index, allocation.recipient, allocation.tokenId, allocation.quantity);
    }

    /// @dev Double-hashed, ABI-encoded domain agrees with backend crypto::leaf.
    function claimLeaf(Claim calldata allocation) public view returns (bytes32) {
        return keccak256(
            bytes.concat(
                keccak256(
                    abi.encode(
                        block.chainid,
                        address(this),
                        configuration.campaignKey,
                        allocation.index,
                        allocation.recipient,
                        configuration.assetKind,
                        configuration.rewardToken,
                        allocation.tokenId,
                        allocation.quantity
                    )
                )
            )
        );
    }

    function cancel() external onlyCreator nonReentrant {
        if (state != CampaignState.DRAFT) revert InvalidState();
        state = CampaignState.CANCELLED;
        _refund();
        emit CampaignCancelled(configuration.refundRecipient);
    }

    /// @notice Permissionless execution; all refunds go to the immutable configured recipient.
    function sweepRemaining() external nonReentrant {
        bool expired =
            ((state == CampaignState.DRAFT || state == CampaignState.ACTIVE)
                    && block.timestamp >= configuration.reviewDeadline)
                || (state == CampaignState.FINALIZED && block.timestamp >= configuration.claimDeadline);
        if (!expired) revert RefundNotAvailable();
        state = CampaignState.CLOSED;
        uint256 remaining = fundedQuantity - claimedQuantity;
        _refund();
        emit RemainingSwept(configuration.refundRecipient, remaining);
    }

    function onERC721Received(address operator, address from, uint256 tokenId, bytes calldata)
        external
        returns (bytes4)
    {
        _receiveNFT(1, operator, from, tokenId, 1);
        return IERC721Receiver.onERC721Received.selector;
    }

    function onERC1155Received(address operator, address from, uint256 id, uint256 value, bytes calldata)
        external
        returns (bytes4)
    {
        _receiveNFT(2, operator, from, id, value);
        return IERC1155Receiver.onERC1155Received.selector;
    }

    function onERC1155BatchReceived(address, address, uint256[] calldata, uint256[] calldata, bytes calldata)
        external
        pure
        returns (bytes4)
    {
        revert UnexpectedNFTTransfer();
    }

    function supportsInterface(bytes4 interfaceId) external pure returns (bool) {
        return interfaceId == type(IERC165).interfaceId || interfaceId == type(IERC1155Receiver).interfaceId
            || interfaceId == type(IERC721Receiver).interfaceId;
    }

    function _fundingCheck(uint8 kind, uint256 amount) private view {
        if (configuration.assetKind != kind) revert WrongAssetKind();
        if (state != CampaignState.DRAFT) revert InvalidState();
        if (block.timestamp >= configuration.startsAt) revert FundingClosed();
        if (amount == 0 || amount > configuration.targetQuantity - fundedQuantity) revert FundingExceeded();
    }

    function _expectNFT(uint256 id, uint256 quantity) private {
        expectingDeposit = true;
        expectedId = id;
        expectedQuantity = quantity;
    }

    // Funding owns the reentrancy guard; callbacks only consume an exact deposit context.
    function _receiveNFT(uint8 kind, address operator, address from, uint256 id, uint256 quantity) private {
        if (
            !expectingDeposit || msg.sender != configuration.rewardToken || configuration.assetKind != kind
                || operator != address(this) || from != configuration.creator || id != expectedId
                || quantity != expectedQuantity
        ) revert UnexpectedNFTTransfer();
        expectingDeposit = false;
    }

    function _checkDelta(uint256 beforeBalance, uint256 afterBalance, uint256 quantity, bool increase) private pure {
        if (increase) {
            if (afterBalance < beforeBalance || afterBalance - beforeBalance != quantity) {
                revert UnsupportedTokenBehavior();
            }
        } else {
            if (beforeBalance < afterBalance || beforeBalance - afterBalance != quantity) {
                revert UnsupportedTokenBehavior();
            }
        }
    }

    function _pay(address recipient, uint256 id, uint256 quantity) private {
        if (configuration.assetKind == 0) {
            IERC20 token = IERC20(configuration.rewardToken);
            uint256 beforeEscrow = token.balanceOf(address(this));
            uint256 beforeRecipient = token.balanceOf(recipient);
            token.safeTransfer(recipient, quantity);
            _checkDelta(beforeEscrow, token.balanceOf(address(this)), quantity, false);
            _checkDelta(beforeRecipient, token.balanceOf(recipient), quantity, true);
        } else if (configuration.assetKind == 1) {
            IERC721 token = IERC721(configuration.rewardToken);
            token.safeTransferFrom(address(this), recipient, id);
            if (token.ownerOf(id) != recipient) revert UnsupportedTokenBehavior();
        } else {
            IERC1155 token = IERC1155(configuration.rewardToken);
            uint256 beforeEscrow = token.balanceOf(address(this), id);
            uint256 beforeRecipient = token.balanceOf(recipient, id);
            token.safeTransferFrom(address(this), recipient, id, quantity, "");
            _checkDelta(beforeEscrow, token.balanceOf(address(this), id), quantity, false);
            _checkDelta(beforeRecipient, token.balanceOf(recipient, id), quantity, true);
        }
    }

    function _refund() private {
        if (configuration.assetKind == 1) {
            for (uint256 i; i < inventory.length; ++i) {
                uint256 id = inventory[i];
                if (!deliveredNFT[id]) {
                    deliveredNFT[id] = true;
                    _pay(configuration.refundRecipient, id, 1);
                }
            }
        } else {
            uint256 remaining = fundedQuantity - claimedQuantity;
            if (remaining != 0) _pay(configuration.refundRecipient, configuration.rewardTokenId, remaining);
        }
    }
}
