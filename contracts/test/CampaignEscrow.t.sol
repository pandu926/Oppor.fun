// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;
import {Test} from "forge-std/Test.sol";
import {CampaignConfig, Claim} from "../src/CampaignTypes.sol";
import {CampaignFactory} from "../src/CampaignFactory.sol";
import {CampaignEscrow} from "../src/CampaignEscrow.sol";
import {MockERC20, MockERC721, MockERC1155, NoReturnERC20, FalseReturnERC20, RewardWallet} from "./Mocks.sol";

contract CampaignEscrowTest is Test {
    CampaignFactory factory;
    MockERC20 token;
    MockERC721 nft;
    MockERC1155 multi;
    address creator = address(0xA11CE);
    address alice = address(0xB0B);
    address refund = address(0xCAFE);
    CampaignConfig c;
    CampaignEscrow e;

    function setUp() public {
        vm.warp(1_000_000);
        factory = new CampaignFactory();
        token = new MockERC20();
        nft = new MockERC721();
        multi = new MockERC1155();
        c = CampaignConfig(
            bytes32(uint256(1)),
            creator,
            refund,
            0,
            address(token),
            0,
            1000,
            uint64(block.timestamp + 100),
            uint64(block.timestamp + 200),
            uint64(block.timestamp + 300),
            uint64(block.timestamp + 300 + 1 days),
            0,
            0,
            0,
            keccak256("rules"),
            bytes32(0)
        );
        e = create(c);
        token.mint(creator, 2000);
        vm.prank(creator);
        token.approve(address(e), type(uint256).max);
    }

    function create(CampaignConfig memory conf) internal returns (CampaignEscrow) {
        vm.prank(creator);
        return CampaignEscrow(factory.createCampaign(conf));
    }

    function funded() internal {
        vm.prank(creator);
        e.fundERC20(1000);
    }

    function active() internal {
        funded();
        vm.prank(creator);
        e.activate();
    }

    function finalized(Claim memory cl) internal {
        active();
        vm.warp(c.cutoffAt);
        bytes32 root = e.claimLeaf(cl);
        vm.prank(creator);
        e.finalize(root, keccak256("manifest"), keccak256("eligibility"), cl.quantity, cl.index + 1);
    }

    function allocation() internal view returns (Claim memory) {
        return Claim(0, alice, 0, 600);
    }

    function pair(bytes32 a, bytes32 b) internal pure returns (bytes32) {
        return a < b ? keccak256(abi.encodePacked(a, b)) : keccak256(abi.encodePacked(b, a));
    }

    function nftEscrow(address recipient) internal returns (Claim memory cl) {
        c.campaignKey = bytes32(uint256(2));
        c.assetKind = 1;
        c.rewardToken = address(nft);
        c.targetQuantity = 2;
        c.participantCapacity = 2;
        e = create(c);
        nft.mint(creator, 7);
        nft.mint(creator, 8);
        vm.startPrank(creator);
        nft.setApprovalForAll(address(e), true);
        uint256[] memory ids = new uint256[](2);
        ids[0] = 7;
        ids[1] = 8;
        e.fundERC721(ids);
        e.activate();
        vm.stopPrank();
        cl = Claim(0, recipient, 7, 1);
        vm.warp(c.cutoffAt);
        bytes32 root = e.claimLeaf(cl);
        vm.prank(creator);
        e.finalize(root, keccak256("m"), keccak256("s"), 1, 1);
    }

    function multiEscrow(address recipient) internal returns (Claim memory cl) {
        c.campaignKey = bytes32(uint256(2));
        c.assetKind = 2;
        c.rewardToken = address(multi);
        c.rewardTokenId = 42;
        e = create(c);
        multi.mint(creator, 42, 1000);
        vm.startPrank(creator);
        multi.setApprovalForAll(address(e), true);
        e.fundERC1155(1000);
        e.activate();
        vm.stopPrank();
        cl = Claim(0, recipient, 42, 600);
        vm.warp(c.cutoffAt);
        bytes32 root = e.claimLeaf(cl);
        vm.prank(creator);
        e.finalize(root, keccak256("m"), keccak256("s"), 600, 1);
    }

    function testFactoryRegistryAndConfigHash() public view {
        assertEq(factory.campaignOf(creator, c.campaignKey), address(e));
        assertEq(e.configHash(), keccak256(abi.encode(c)));
    }

    function testFactoryDuplicateRejected() public {
        vm.expectRevert(CampaignFactory.CampaignExists.selector);
        create(c);
    }

    function testFactoryUnauthorizedCreator() public {
        vm.expectRevert(CampaignFactory.Unauthorized.selector);
        factory.createCampaign(c);
    }

    function testUniformRuntimeCodeHash() public {
        c.campaignKey = bytes32(uint256(2));
        c.targetQuantity = 333;
        CampaignEscrow second = create(c);
        assertEq(address(e).codehash, address(second).codehash);
        c.campaignKey = bytes32(uint256(3));
        c.assetKind = 2;
        c.rewardToken = address(multi);
        CampaignEscrow third = create(c);
        assertEq(address(e).codehash, address(third).codehash);
    }

    function testInvalidDeadlines() public {
        c.campaignKey = bytes32(uint256(2));
        c.cutoffAt = c.startsAt;
        vm.expectRevert(CampaignEscrow.InvalidConfig.selector);
        create(c);
    }

    function testShortClaimWindow() public {
        c.campaignKey = bytes32(uint256(2));
        --c.claimDeadline;
        vm.expectRevert(CampaignEscrow.InvalidConfig.selector);
        create(c);
    }

    function testInvalidAssetInterface() public {
        c.campaignKey = bytes32(uint256(2));
        c.assetKind = 2;
        vm.expectRevert(CampaignEscrow.InvalidConfig.selector);
        create(c);
    }

    function testInvalidRaffleConfig() public {
        c.campaignKey = bytes32(uint256(2));
        c.mode = 1;
        vm.expectRevert(CampaignEscrow.InvalidConfig.selector);
        create(c);
    }

    function testValidRaffleConfig() public {
        c.campaignKey = bytes32(uint256(2));
        c.mode = 1;
        c.winnerCount = 2;
        c.raffleSeedCommitment = keccak256("seed");
        assertEq(create(c).configHash(), keccak256(abi.encode(c)));
    }

    function testFundingPartialThenActivation() public {
        vm.startPrank(creator);
        e.fundERC20(400);
        vm.expectRevert(CampaignEscrow.FundingIncomplete.selector);
        e.activate();
        e.fundERC20(600);
        e.activate();
        vm.stopPrank();
        assertEq(uint8(e.state()), 1);
        assertEq(token.balanceOf(address(e)), 1000);
    }

    function testFundingUnauthorized() public {
        vm.expectRevert(CampaignEscrow.Unauthorized.selector);
        e.fundERC20(1);
    }

    function testFundingExceedsTarget() public {
        vm.prank(creator);
        vm.expectRevert(CampaignEscrow.FundingExceeded.selector);
        e.fundERC20(1001);
    }

    function testFundingZero() public {
        vm.prank(creator);
        vm.expectRevert(CampaignEscrow.FundingExceeded.selector);
        e.fundERC20(0);
    }

    function testFundingWrongKind() public {
        vm.prank(creator);
        vm.expectRevert(CampaignEscrow.WrongAssetKind.selector);
        e.fundERC1155(1);
    }

    function testFundingAtStartClosed() public {
        vm.warp(c.startsAt);
        vm.prank(creator);
        vm.expectRevert(CampaignEscrow.FundingClosed.selector);
        e.fundERC20(1);
    }

    function testDirectTransferDoesNotFund() public {
        vm.prank(creator);
        token.transfer(address(e), 1000);
        assertEq(e.fundedQuantity(), 0);
        vm.prank(creator);
        vm.expectRevert(CampaignEscrow.FundingIncomplete.selector);
        e.activate();
    }

    function testFeeFundingRevertsAtomically() public {
        token.setFee(true);
        vm.prank(creator);
        vm.expectRevert(CampaignEscrow.UnsupportedTokenBehavior.selector);
        e.fundERC20(1000);
        assertEq(e.fundedQuantity(), 0);
        assertEq(token.balanceOf(creator), 2000);
        assertEq(token.balanceOf(address(e)), 0);
    }

    function testClaimAndDuplicate() public {
        Claim memory cl = allocation();
        finalized(cl);
        vm.prank(alice);
        e.claim(cl, new bytes32[](0));
        assertEq(token.balanceOf(alice), 600);
        assertEq(e.claimedQuantity(), 600);
        assertEq(e.claimedLeafCount(), 1);
        assertTrue(e.isClaimed(0));
        vm.prank(alice);
        vm.expectRevert(CampaignEscrow.AlreadyClaimed.selector);
        e.claim(cl, new bytes32[](0));
    }

    function testWrongRecipientRejected() public {
        Claim memory cl = allocation();
        finalized(cl);
        vm.expectRevert(CampaignEscrow.Unauthorized.selector);
        e.claim(cl, new bytes32[](0));
    }

    function testForgedQuantityRejected() public {
        Claim memory cl = allocation();
        finalized(cl);
        cl.quantity = 700;
        vm.prank(alice);
        vm.expectRevert(CampaignEscrow.InvalidProof.selector);
        e.claim(cl, new bytes32[](0));
    }

    function testCrossEscrowProofRejected() public {
        Claim memory cl = allocation();
        finalized(cl);
        bytes32 root = e.distributionRoot();
        vm.warp(c.startsAt - 1);
        c.campaignKey = bytes32(uint256(2));
        CampaignEscrow second = create(c);
        vm.startPrank(creator);
        token.approve(address(second), 1000);
        second.fundERC20(1000);
        second.activate();
        vm.warp(c.cutoffAt);
        second.finalize(root, keccak256("m"), keccak256("s"), 600, 1);
        vm.stopPrank();
        vm.prank(alice);
        vm.expectRevert(CampaignEscrow.InvalidProof.selector);
        second.claim(cl, new bytes32[](0));
    }

    function testCrossChainProofRejected() public {
        Claim memory cl = allocation();
        finalized(cl);
        vm.chainId(block.chainid + 1);
        vm.prank(alice);
        vm.expectRevert(CampaignEscrow.InvalidProof.selector);
        e.claim(cl, new bytes32[](0));
    }

    function testFeeClaimRetainsClaimRight() public {
        Claim memory cl = allocation();
        finalized(cl);
        token.setFee(true);
        vm.prank(alice);
        vm.expectRevert(CampaignEscrow.UnsupportedTokenBehavior.selector);
        e.claim(cl, new bytes32[](0));
        assertFalse(e.isClaimed(0));
        assertEq(e.claimedQuantity(), 0);
        assertEq(token.balanceOf(address(e)), 1000);
        token.setFee(false);
        vm.prank(alice);
        e.claim(cl, new bytes32[](0));
    }

    function testPausedTransferRetainsClaimRight() public {
        Claim memory cl = allocation();
        finalized(cl);
        token.setPaused(true);
        vm.prank(alice);
        vm.expectRevert();
        e.claim(cl, new bytes32[](0));
        assertFalse(e.isClaimed(0));
        assertEq(e.claimedQuantity(), 0);
        token.setPaused(false);
        vm.prank(alice);
        e.claim(cl, new bytes32[](0));
    }

    function testReentrantTokenBlocked() public {
        Claim memory cl = allocation();
        finalized(cl);
        token.setCallback(address(e), abi.encodeCall(CampaignEscrow.sweepRemaining, ()));
        vm.prank(alice);
        e.claim(cl, new bytes32[](0));
        assertFalse(token.reentrySucceeded());
        assertEq(e.claimedQuantity(), 600);
    }

    function testClaimDeadlineExclusive() public {
        Claim memory cl = allocation();
        finalized(cl);
        vm.warp(c.claimDeadline);
        vm.prank(alice);
        vm.expectRevert(CampaignEscrow.ClaimExpired.selector);
        e.claim(cl, new bytes32[](0));
    }

    function testFinalizationCannotReplaceRoot() public {
        finalized(allocation());
        vm.prank(creator);
        vm.expectRevert(CampaignEscrow.InvalidState.selector);
        e.finalize(keccak256("bad"), keccak256("m"), keccak256("s"), 600, 1);
    }

    function testFinalizeBeforeCutoff() public {
        active();
        vm.prank(creator);
        vm.expectRevert(CampaignEscrow.CutoffNotReached.selector);
        e.finalize(bytes32(0), keccak256("m"), keccak256("s"), 0, 0);
    }

    function testFinalizeAtReviewDeadlineRejected() public {
        active();
        vm.warp(c.reviewDeadline);
        vm.prank(creator);
        vm.expectRevert(CampaignEscrow.ReviewExpired.selector);
        e.finalize(bytes32(0), keccak256("m"), keccak256("s"), 0, 0);
    }

    function testEmptyFinalization() public {
        active();
        vm.warp(c.cutoffAt);
        vm.prank(creator);
        e.finalize(bytes32(0), keccak256("m"), keccak256("s"), 0, 0);
        assertEq(uint8(e.state()), 2);
    }

    function testInvalidEmptyFinalization() public {
        active();
        vm.warp(c.cutoffAt);
        vm.prank(creator);
        vm.expectRevert(CampaignEscrow.InvalidAllocation.selector);
        e.finalize(keccak256("r"), keccak256("m"), keccak256("s"), 0, 0);
    }

    function testDeclaredAllocationLimit() public {
        active();
        vm.warp(c.cutoffAt);
        vm.prank(creator);
        vm.expectRevert(CampaignEscrow.InvalidAllocation.selector);
        e.finalize(keccak256("r"), keccak256("m"), keccak256("s"), 1001, 1);
    }

    function testAggregatePayoutCannotExceedDeclaredAmount() public {
        active();
        Claim memory a = Claim(0, alice, 0, 600);
        Claim memory b = Claim(1, refund, 0, 600);
        bytes32 la = e.claimLeaf(a);
        bytes32 lb = e.claimLeaf(b);
        vm.warp(c.cutoffAt);
        vm.prank(creator);
        e.finalize(pair(la, lb), keccak256("m"), keccak256("s"), 1000, 2);
        bytes32[] memory p = new bytes32[](1);
        p[0] = lb;
        vm.prank(alice);
        e.claim(a, p);
        p[0] = la;
        vm.prank(refund);
        vm.expectRevert(CampaignEscrow.AllocationExceeded.selector);
        e.claim(b, p);
        assertFalse(e.isClaimed(1));
        assertEq(e.claimedQuantity(), 600);
    }

    function testCancelRefundOnlyToConfiguredRecipient() public {
        funded();
        vm.prank(creator);
        e.cancel();
        assertEq(token.balanceOf(refund), 1000);
        assertEq(uint8(e.state()), 3);
        vm.prank(creator);
        vm.expectRevert(CampaignEscrow.InvalidState.selector);
        e.cancel();
    }

    function testActiveCannotCancel() public {
        active();
        vm.prank(creator);
        vm.expectRevert(CampaignEscrow.InvalidState.selector);
        e.cancel();
    }

    function testEarlySweepRejectedEvenAfterAllClaims() public {
        Claim memory cl = allocation();
        finalized(cl);
        vm.prank(alice);
        e.claim(cl, new bytes32[](0));
        vm.expectRevert(CampaignEscrow.RefundNotAvailable.selector);
        e.sweepRemaining();
    }

    function testPermissionlessSweepAfterDeadline() public {
        Claim memory cl = allocation();
        finalized(cl);
        vm.prank(alice);
        e.claim(cl, new bytes32[](0));
        vm.warp(c.claimDeadline);
        vm.prank(alice);
        e.sweepRemaining();
        assertEq(token.balanceOf(refund), 400);
        assertEq(token.balanceOf(alice), 600);
        assertEq(uint8(e.state()), 4);
        vm.expectRevert(CampaignEscrow.RefundNotAvailable.selector);
        e.sweepRemaining();
    }

    function testUnfinalizedSweepAtReviewDeadline() public {
        active();
        vm.warp(c.reviewDeadline);
        e.sweepRemaining();
        assertEq(token.balanceOf(refund), 1000);
    }

    function testAbandonedDraftSweep() public {
        funded();
        vm.warp(c.reviewDeadline);
        e.sweepRemaining();
        assertEq(token.balanceOf(refund), 1000);
    }

    function testRefundFailureRollsBackState() public {
        funded();
        token.setPaused(true);
        vm.prank(creator);
        vm.expectRevert();
        e.cancel();
        assertEq(uint8(e.state()), 0);
        token.setPaused(false);
        vm.prank(creator);
        e.cancel();
    }

    function testERC721ClaimAndRefundInventory() public {
        Claim memory cl = nftEscrow(alice);
        vm.prank(alice);
        e.claim(cl, new bytes32[](0));
        assertEq(nft.ownerOf(7), alice);
        vm.warp(c.claimDeadline);
        e.sweepRemaining();
        assertEq(nft.ownerOf(8), refund);
        assertEq(e.claimedQuantity(), 1);
    }

    function testERC721DuplicateFundingAtomic() public {
        c.campaignKey = bytes32(uint256(2));
        c.assetKind = 1;
        c.rewardToken = address(nft);
        c.targetQuantity = 2;
        c.participantCapacity = 2;
        e = create(c);
        nft.mint(creator, 7);
        vm.startPrank(creator);
        nft.setApprovalForAll(address(e), true);
        uint256[] memory ids = new uint256[](2);
        ids[0] = 7;
        ids[1] = 7;
        vm.expectRevert(CampaignEscrow.DuplicateNFT.selector);
        e.fundERC721(ids);
        vm.stopPrank();
        assertEq(nft.ownerOf(7), creator);
        assertEq(e.fundedQuantity(), 0);
    }

    function testERC721UnexpectedSafeTransferRejected() public {
        nft.mint(creator, 99);
        vm.prank(creator);
        vm.expectRevert(CampaignEscrow.UnexpectedNFTTransfer.selector);
        nft.safeTransferFrom(creator, address(e), 99);
        assertEq(nft.ownerOf(99), creator);
    }

    function testERC1155ClaimAndRefund() public {
        Claim memory cl = multiEscrow(alice);
        vm.prank(alice);
        e.claim(cl, new bytes32[](0));
        assertEq(multi.balanceOf(alice, 42), 600);
        vm.warp(c.claimDeadline);
        e.sweepRemaining();
        assertEq(multi.balanceOf(refund, 42), 400);
    }

    function testERC1155UnsolicitedAndBatchRejected() public {
        multi.mint(creator, 42, 5);
        vm.prank(creator);
        vm.expectRevert(CampaignEscrow.UnexpectedNFTTransfer.selector);
        multi.safeTransferFrom(creator, address(e), 42, 1, "");
        uint256[] memory ids = new uint256[](1);
        uint256[] memory amounts = new uint256[](1);
        ids[0] = 42;
        amounts[0] = 1;
        vm.prank(creator);
        vm.expectRevert(CampaignEscrow.UnexpectedNFTTransfer.selector);
        multi.safeBatchTransferFrom(creator, address(e), ids, amounts, "");
    }

    function testERC721WalletCallbackReentrancyBlocked() public {
        RewardWallet wallet = new RewardWallet();
        Claim memory cl = nftEscrow(address(wallet));
        wallet.collect(e, cl, new bytes32[](0), true);
        assertFalse(wallet.reentrySucceeded());
        assertEq(nft.ownerOf(7), address(wallet));
        assertEq(e.claimedLeafCount(), 1);
    }

    function testERC1155WalletCallbackReentrancyBlocked() public {
        RewardWallet wallet = new RewardWallet();
        Claim memory cl = multiEscrow(address(wallet));
        wallet.collect(e, cl, new bytes32[](0), true);
        assertFalse(wallet.reentrySucceeded());
        assertEq(multi.balanceOf(address(wallet), 42), 600);
    }

    function testRejectingNFTWalletRetainsClaimRight() public {
        RewardWallet wallet = new RewardWallet();
        Claim memory cl = nftEscrow(address(wallet));
        wallet.setReject(true);
        vm.expectRevert();
        wallet.collect(e, cl, new bytes32[](0), false);
        assertFalse(e.isClaimed(0));
        assertFalse(e.deliveredNFT(7));
        assertEq(nft.ownerOf(7), address(e));
        wallet.setReject(false);
        wallet.collect(e, cl, new bytes32[](0), false);
    }

    function testNoReturnERC20Supported() public {
        NoReturnERC20 legacy = new NoReturnERC20();
        c.campaignKey = bytes32(uint256(2));
        c.rewardToken = address(legacy);
        e = create(c);
        legacy.mint(creator, 1000);
        vm.startPrank(creator);
        legacy.approve(address(e), 1000);
        e.fundERC20(1000);
        e.cancel();
        vm.stopPrank();
        assertEq(legacy.balanceOf(refund), 1000);
    }

    function testFalseReturnERC20Rejected() public {
        c.campaignKey = bytes32(uint256(2));
        c.rewardToken = address(new FalseReturnERC20());
        e = create(c);
        vm.prank(creator);
        vm.expectRevert();
        e.fundERC20(1000);
        assertEq(e.fundedQuantity(), 0);
    }

    function testFuzzBitmapWordBoundary(uint32 rawIndex) public {
        uint256 index = bound(rawIndex, 0, 99_999);
        c.campaignKey = bytes32(uint256(2));
        c.targetQuantity = index + 1;
        e = create(c);
        token.mint(creator, c.targetQuantity);
        vm.startPrank(creator);
        token.approve(address(e), c.targetQuantity);
        e.fundERC20(c.targetQuantity);
        e.activate();
        vm.stopPrank();
        Claim memory cl = Claim(index, alice, 0, c.targetQuantity);
        bytes32 root = e.claimLeaf(cl);
        vm.warp(c.cutoffAt);
        vm.prank(creator);
        e.finalize(root, keccak256("manifest"), keccak256("snapshot"), c.targetQuantity, index + 1);
        vm.prank(alice);
        e.claim(cl, new bytes32[](0));
        assertTrue(e.isClaimed(index));
        assertFalse(e.isClaimed(index + 1));
        if (index > 0) assertFalse(e.isClaimed(index - 1));
    }

    function testFuzzPartialFundingConservation(uint256 raw) public {
        uint256 amount = bound(raw, 1, 1000);
        vm.startPrank(creator);
        e.fundERC20(amount);
        e.cancel();
        vm.stopPrank();
        assertEq(token.balanceOf(refund), amount);
        assertEq(token.balanceOf(address(e)), 0);
        assertEq(token.balanceOf(creator) + token.balanceOf(refund), 2000);
    }

    function testFuzzClaimThenRefundConservation(uint256 raw) public {
        uint256 amount = bound(raw, 1, 1000);
        Claim memory cl = Claim(0, alice, 0, amount);
        finalized(cl);
        vm.prank(alice);
        e.claim(cl, new bytes32[](0));
        vm.warp(c.claimDeadline);
        e.sweepRemaining();
        assertEq(token.balanceOf(alice) + token.balanceOf(refund), 1000);
        assertEq(token.balanceOf(address(e)), 0);
    }

    function testUnauthorizedLifecycleChanges() public {
        vm.expectRevert(CampaignEscrow.Unauthorized.selector);
        e.activate();
        vm.expectRevert(CampaignEscrow.Unauthorized.selector);
        e.cancel();
        vm.expectRevert(CampaignEscrow.Unauthorized.selector);
        e.finalize(bytes32(0), keccak256("m"), keccak256("s"), 0, 0);
    }

    function testClaimInvalidIndexAndTokenId() public {
        Claim memory cl = allocation();
        finalized(cl);
        cl.index = 1;
        vm.prank(alice);
        vm.expectRevert(CampaignEscrow.InvalidAllocation.selector);
        e.claim(cl, new bytes32[](0));
        cl.index = 0;
        cl.tokenId = 1;
        vm.prank(alice);
        vm.expectRevert(CampaignEscrow.InvalidAllocation.selector);
        e.claim(cl, new bytes32[](0));
    }

    function testProofLengthBound() public {
        Claim memory cl = allocation();
        finalized(cl);
        vm.prank(alice);
        vm.expectRevert(CampaignEscrow.InvalidProof.selector);
        e.claim(cl, new bytes32[](33));
    }

    function testERC721InventoryCap() public {
        c.campaignKey = bytes32(uint256(2));
        c.assetKind = 1;
        c.rewardToken = address(nft);
        c.targetQuantity = 101;
        c.participantCapacity = 1;
        vm.expectRevert(CampaignEscrow.InvalidConfig.selector);
        create(c);
    }

    function testERC721MaximumInventoryRefund() public {
        c.campaignKey = bytes32(uint256(2));
        c.assetKind = 1;
        c.rewardToken = address(nft);
        c.targetQuantity = 100;
        c.participantCapacity = 100;
        e = create(c);
        uint256[] memory ids = new uint256[](100);
        for (uint256 i; i < 100; ++i) {
            ids[i] = i;
            nft.mint(creator, i);
        }
        vm.startPrank(creator);
        nft.setApprovalForAll(address(e), true);
        e.fundERC721(ids);
        e.cancel();
        vm.stopPrank();
        for (uint256 i; i < 100; ++i) {
            assertEq(nft.ownerOf(i), refund);
        }
        assertEq(uint8(e.state()), 3);
        assertEq(e.nftInventory().length, 100);
    }

    function testERC1155CancelPartialFunding() public {
        c.campaignKey = bytes32(uint256(2));
        c.assetKind = 2;
        c.rewardToken = address(multi);
        c.rewardTokenId = 0;
        e = create(c);
        multi.mint(creator, 0, 1000);
        vm.startPrank(creator);
        multi.setApprovalForAll(address(e), true);
        e.fundERC1155(400);
        e.cancel();
        vm.stopPrank();
        assertEq(multi.balanceOf(refund, 0), 400);
        assertEq(multi.balanceOf(creator, 0), 600);
    }

    function testERC721RefundReceiverFailureAtomic() public {
        RewardWallet wallet = new RewardWallet();
        c.refundRecipient = address(wallet);
        Claim memory cl = nftEscrow(alice);
        vm.prank(alice);
        e.claim(cl, new bytes32[](0));
        wallet.setReject(true);
        vm.warp(c.claimDeadline);
        vm.expectRevert();
        e.sweepRemaining();
        assertEq(uint8(e.state()), 2);
        assertFalse(e.deliveredNFT(8));
        assertEq(nft.ownerOf(8), address(e));
        wallet.setReject(false);
        e.sweepRemaining();
        assertEq(nft.ownerOf(8), address(wallet));
    }

    function testNativePaymentRejected() public {
        vm.deal(address(this), 1 ether);
        (bool success,) = address(e).call{value: 1}("");
        assertFalse(success);
        assertEq(e.fundedQuantity(), 0);
    }

    function testDirectReceiverSpoofRejected() public {
        vm.expectRevert(CampaignEscrow.UnexpectedNFTTransfer.selector);
        e.onERC721Received(address(e), creator, 7, "");
        vm.expectRevert(CampaignEscrow.UnexpectedNFTTransfer.selector);
        e.onERC1155Received(address(e), creator, 42, 1, "");
    }
}
