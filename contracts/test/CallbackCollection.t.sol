// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;
import {Test} from "forge-std/Test.sol";
import {ERC721} from "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {CampaignConfig} from "../src/CampaignTypes.sol";
import {CampaignFactory} from "../src/CampaignFactory.sol";
import {CampaignEscrow} from "../src/CampaignEscrow.sol";

/// @dev The collection itself is the creator, so onlyCreator cannot mask the guard test.
contract CallbackCollection is ERC721 {
    CampaignEscrow public escrow;
    bool public attempted;
    bytes4 public failure;
    constructor() ERC721("Callback", "CALL") {}

    function createAndFund(CampaignFactory factory, CampaignConfig memory c) external {
        c.creator = address(this);
        c.rewardToken = address(this);
        escrow = CampaignEscrow(factory.createCampaign(c));
        _mint(address(this), 7);
        _mint(address(this), 8);
        _setApprovalForAll(address(this), address(escrow), true);
        uint256[] memory ids = new uint256[](2);
        ids[0] = 7;
        ids[1] = 8;
        escrow.fundERC721(ids);
    }

    function safeTransferFrom(address from, address to, uint256 id, bytes memory data) public override {
        attempted = true;
        (bool success, bytes memory result) = address(escrow).call(abi.encodeCall(CampaignEscrow.activate, ()));
        require(!success, "reentered");
        failure = bytes4(result);
        super.safeTransferFrom(from, to, id, data);
    }
}

contract CallbackCollectionTest is Test {
    function testCreatorCollectionCannotReenterFunding() public {
        vm.warp(1_000_000);
        CallbackCollection collection = new CallbackCollection();
        CampaignConfig memory c = CampaignConfig(
            bytes32(uint256(1)),
            address(0),
            address(0xCAFE),
            1,
            address(0),
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
        collection.createAndFund(new CampaignFactory(), c);
        assertTrue(collection.attempted());
        assertEq(collection.failure(), ReentrancyGuard.ReentrancyGuardReentrantCall.selector);
        assertEq(collection.escrow().fundedQuantity(), 2);
        assertEq(uint8(collection.escrow().state()), 0);
    }
}
