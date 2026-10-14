// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;
import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {ERC721} from "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import {ERC1155} from "@openzeppelin/contracts/token/ERC1155/ERC1155.sol";
import {IERC721Receiver} from "@openzeppelin/contracts/token/ERC721/IERC721Receiver.sol";
import {IERC1155Receiver} from "@openzeppelin/contracts/token/ERC1155/IERC1155Receiver.sol";
import {CampaignEscrow} from "../src/CampaignEscrow.sol";
import {Claim} from "../src/CampaignTypes.sol";

contract MockERC20 is ERC20 {
    bool public paused;
    bool public fee;
    address public callback;
    bytes public callbackData;
    bool public reentrySucceeded;
    constructor() ERC20("Test reward", "TST") {}

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }

    function setPaused(bool v) external {
        paused = v;
    }

    function setFee(bool v) external {
        fee = v;
    }

    function setCallback(address target, bytes calldata data) external {
        callback = target;
        callbackData = data;
    }

    function _update(address from, address to, uint256 amount) internal override {
        require(!paused, "paused");
        if (callback != address(0) && from != address(0)) {
            (bool success,) = callback.call(callbackData);
            reentrySucceeded = success;
        }
        if (fee && from != address(0) && to != address(0) && amount > 0) {
            super._update(from, address(0), 1);
            super._update(from, to, amount - 1);
        } else {
            super._update(from, to, amount);
        }
    }
}

contract MockERC721 is ERC721 {
    constructor() ERC721("Test NFT", "NFT") {}

    function mint(address to, uint256 id) external {
        _mint(to, id);
    }
}

contract MockERC1155 is ERC1155 {
    constructor() ERC1155("ipfs://test/{id}") {}

    function mint(address to, uint256 id, uint256 amount) external {
        _mint(to, id, amount, "");
    }
}

contract NoReturnERC20 {
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    function mint(address to, uint256 amount) external {
        balanceOf[to] += amount;
    }

    function approve(address to, uint256 amount) external {
        allowance[msg.sender][to] = amount;
    }

    function transfer(address to, uint256 amount) external {
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
    }

    function transferFrom(address from, address to, uint256 amount) external {
        allowance[from][msg.sender] -= amount;
        balanceOf[from] -= amount;
        balanceOf[to] += amount;
    }
}

contract FalseReturnERC20 {
    mapping(address => uint256) public balanceOf;

    function transferFrom(address, address, uint256) external pure returns (bool) {
        return false;
    }

    function transfer(address, uint256) external pure returns (bool) {
        return false;
    }
}

contract RewardWallet is IERC721Receiver, IERC1155Receiver {
    bool public reject;
    bool public reenter;
    bool public reentrySucceeded;
    CampaignEscrow private escrow;
    Claim private allocation;
    bytes32[] private proof;

    function setReject(bool value) external {
        reject = value;
    }

    function collect(CampaignEscrow e, Claim calldata c, bytes32[] calldata p, bool attack) external {
        escrow = e;
        allocation = c;
        proof = p;
        reenter = attack;
        e.claim(c, p);
    }

    function onERC721Received(address, address, uint256, bytes calldata) external returns (bytes4) {
        _callback();
        return IERC721Receiver.onERC721Received.selector;
    }

    function onERC1155Received(address, address, uint256, uint256, bytes calldata) external returns (bytes4) {
        _callback();
        return IERC1155Receiver.onERC1155Received.selector;
    }

    function onERC1155BatchReceived(address, address, uint256[] calldata, uint256[] calldata, bytes calldata)
        external
        pure
        returns (bytes4)
    {
        return IERC1155Receiver.onERC1155BatchReceived.selector;
    }

    function supportsInterface(bytes4 id) external pure returns (bool) {
        return id == 0x01ffc9a7 || id == type(IERC1155Receiver).interfaceId || id == type(IERC721Receiver).interfaceId;
    }

    function _callback() private {
        require(!reject, "reject");
        if (reenter) {
            (bool success,) = address(escrow).call(abi.encodeCall(CampaignEscrow.claim, (allocation, proof)));
            reentrySucceeded = success;
        }
    }
}
