// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;
import {Test} from "forge-std/Test.sol";
import {CampaignEscrow} from "../src/CampaignEscrow.sol";
import {CampaignConfig} from "../src/CampaignTypes.sol";
import {MockERC20, MockERC721, MockERC1155} from "./Mocks.sol";

contract SelectivelyFrozenNFT is MockERC721 {
    uint256 public frozenId = type(uint256).max;
    error TokenFrozen();

    function setFrozen(uint256 id) external {
        frozenId = id;
    }

    function _update(address to, uint256 id, address auth) internal override returns (address) {
        if (id == frozenId) revert TokenFrozen();
        return super._update(to, id, auth);
    }
}

contract AuditRegressionTest is Test {
    function config(address token, uint8 kind) private view returns (CampaignConfig memory) {
        return CampaignConfig(
            bytes32(uint256(777)),
            address(this),
            address(0xCAFE),
            kind,
            token,
            0,
            2,
            uint64(block.timestamp + 100),
            uint64(block.timestamp + 200),
            uint64(block.timestamp + 300),
            uint64(block.timestamp + 300 + 1 days),
            0,
            2,
            0,
            keccak256("rules"),
            bytes32(0)
        );
    }

    function testRejectImpossibleDivisibleAllocationCount() public {
        vm.warp(1_000_000);
        MockERC20 t = new MockERC20();
        CampaignConfig memory c = config(address(t), 0);
        CampaignEscrow e = new CampaignEscrow(c);
        t.mint(address(this), 2);
        t.approve(address(e), 2);
        e.fundERC20(2);
        e.activate();
        vm.warp(c.cutoffAt);
        // Three positive-quantity leaves cannot share only two base units.
        vm.expectRevert(CampaignEscrow.InvalidAllocation.selector);
        e.finalize(keccak256("root"), keccak256("manifest"), keccak256("snapshot"), 2, 3);
    }

    function testRejectImpossibleERC1155AllocationCount() public {
        vm.warp(1_000_000);
        MockERC1155 t = new MockERC1155();
        CampaignConfig memory c = config(address(t), 2);
        c.creator = address(0xA11CE);
        c.rewardTokenId = 42;
        CampaignEscrow e = new CampaignEscrow(c);
        t.mint(c.creator, 42, 2);
        vm.startPrank(c.creator);
        t.setApprovalForAll(address(e), true);
        e.fundERC1155(2);
        e.activate();
        vm.warp(c.cutoffAt);
        vm.expectRevert(CampaignEscrow.InvalidAllocation.selector);
        e.finalize(keccak256("root"), keccak256("manifest"), keccak256("snapshot"), 2, 3);
        vm.stopPrank();
    }

    function testOneFrozenNFTBlocksAtomicRefundUntilIssuerRestoresTransfers() public {
        vm.warp(1_000_000);
        SelectivelyFrozenNFT t = new SelectivelyFrozenNFT();
        CampaignConfig memory c = config(address(t), 1);
        CampaignEscrow e = new CampaignEscrow(c);
        t.mint(address(this), 7);
        t.mint(address(this), 8);
        t.setApprovalForAll(address(e), true);
        uint256[] memory ids = new uint256[](2);
        ids[0] = 7;
        ids[1] = 8;
        e.fundERC721(ids);
        e.activate();
        t.setFrozen(7);
        vm.warp(c.reviewDeadline);
        vm.expectRevert(SelectivelyFrozenNFT.TokenFrozen.selector);
        e.sweepRemaining();
        assertEq(uint8(e.state()), 1);
        assertEq(t.ownerOf(7), address(e));
        assertEq(t.ownerOf(8), address(e));
        assertFalse(e.deliveredNFT(7));
        assertFalse(e.deliveredNFT(8));
        // There is no permissionless recovery while the external collection keeps this ID frozen.
        t.setFrozen(type(uint256).max);
        e.sweepRemaining();
        assertEq(t.ownerOf(7), c.refundRecipient);
        assertEq(t.ownerOf(8), c.refundRecipient);
    }
}
