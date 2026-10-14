// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;
import {Test} from "forge-std/Test.sol";
import {StdInvariant} from "forge-std/StdInvariant.sol";
import {CampaignFactory} from "../src/CampaignFactory.sol";
import {CampaignEscrow} from "../src/CampaignEscrow.sol";
import {CampaignConfig, Claim} from "../src/CampaignTypes.sol";
import {MockERC20} from "./Mocks.sol";

/// @dev Fuzz actions against a real, funded 32-leaf root. No mock escrow or replaced accounting.
contract EscrowHandler is Test {
    CampaignEscrow public escrow;
    MockERC20 public token;
    uint64 public deadline;
    bytes32 public initialRoot;
    bytes32 public initialConfigHash;
    bytes32[63] private tree;
    address public constant REFUND = address(0xCAFE);

    constructor() {
        token = new MockERC20();
        deadline = uint64(block.timestamp + 300 + 1 days);
        CampaignConfig memory c = CampaignConfig(
            bytes32(uint256(123)),
            address(this),
            REFUND,
            0,
            address(token),
            0,
            32_000,
            uint64(block.timestamp + 100),
            uint64(block.timestamp + 200),
            uint64(block.timestamp + 300),
            deadline,
            0,
            32,
            0,
            keccak256("rules"),
            bytes32(0)
        );
        escrow = CampaignEscrow(new CampaignFactory().createCampaign(c));
        token.mint(address(this), 32_000);
        token.approve(address(escrow), 32_000);
        escrow.fundERC20(32_000);
        escrow.activate();
        for (uint256 i; i < 32; ++i) {
            tree[31 + i] = escrow.claimLeaf(Claim(i, recipient(i), 0, 1000));
        }
        for (uint256 i = 31; i > 0;) {
            --i;
            bytes32 a = tree[2 * i + 1];
            bytes32 b = tree[2 * i + 2];
            tree[i] = a < b ? keccak256(abi.encodePacked(a, b)) : keccak256(abi.encodePacked(b, a));
        }
        initialRoot = tree[0];
        initialConfigHash = escrow.configHash();
        vm.warp(c.cutoffAt);
        escrow.finalize(initialRoot, keccak256("manifest"), keccak256("snapshot"), 32_000, 32);
    }

    function recipient(uint256 i) public pure returns (address) {
        return address(uint160(1000 + i));
    }

    function collect(uint8 raw) external {
        uint256 i = uint256(raw) % 32;
        if (
            escrow.state() != CampaignEscrow.CampaignState.FINALIZED || block.timestamp >= deadline
                || escrow.isClaimed(i)
        ) return;
        bytes32[] memory proof = new bytes32[](5);
        uint256 p = 31 + i;
        for (uint256 j; j < 5; ++j) {
            proof[j] = tree[p % 2 == 1 ? p + 1 : p - 1];
            p = (p - 1) / 2;
        }
        vm.prank(recipient(i));
        escrow.claim(Claim(i, recipient(i), 0, 1000), proof);
    }

    function advanceTime(uint32 raw) external {
        vm.warp(block.timestamp + uint256(raw) % 10000);
    }

    function sweep() external {
        if (escrow.state() == CampaignEscrow.CampaignState.FINALIZED && block.timestamp >= deadline) {
            escrow.sweepRemaining();
        }
    }
}

contract CampaignInvariantTest is StdInvariant, Test {
    EscrowHandler handler;

    function setUp() public {
        vm.warp(1_000_000);
        handler = new EscrowHandler();
        targetContract(address(handler));
    }

    function invariantAccountingAndConservation() public view {
        CampaignEscrow e = handler.escrow();
        MockERC20 t = handler.token();
        assertLe(e.claimedQuantity(), e.allocatedQuantity());
        assertLe(e.allocatedQuantity(), e.fundedQuantity());
        assertEq(e.fundedQuantity(), 32_000);
        uint256 participantTotal;
        uint256 count;
        for (uint256 i; i < 32; ++i) {
            uint256 balance = t.balanceOf(handler.recipient(i));
            assertLe(balance, 1000);
            participantTotal += balance;
            bool claimed = e.isClaimed(i);
            if (claimed) ++count;
            assertEq(balance, claimed ? 1000 : 0);
        }
        assertEq(count, e.claimedLeafCount());
        assertEq(participantTotal, e.claimedQuantity());
        assertEq(t.balanceOf(address(e)) + t.balanceOf(handler.REFUND()) + participantTotal, 32_000);
        if (e.state() == CampaignEscrow.CampaignState.CLOSED) {
            assertEq(t.balanceOf(address(e)), 0);
            assertGe(block.timestamp, handler.deadline());
        } else {
            assertEq(t.balanceOf(handler.REFUND()), 0);
        }
    }

    function invariantConfigurationAndRootNeverChange() public view {
        assertEq(handler.escrow().configHash(), handler.initialConfigHash());
        assertEq(handler.escrow().distributionRoot(), handler.initialRoot());
    }
}
