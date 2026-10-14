// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;

import {Test} from "forge-std/Test.sol";
import {OpporEarlyAdopter} from "../collections/OpporEarlyAdopter.sol";

contract EarlyAdopterCollectionTest is Test {
    address private creator = address(0xA11CE);
    string private constant URI = "data:application/json;base64,eyJuYW1lIjoiT3Bwb3IifQ==";

    function testFixedEditionAndMetadata() public {
        OpporEarlyAdopter token = new OpporEarlyAdopter(creator, URI);
        assertEq(token.balanceOf(creator, 1), 1000);
        assertEq(token.totalSupply(1), 1000);
        assertEq(token.totalSupply(), 1000);
        assertEq(token.uri(1), URI);
        assertTrue(token.supportsInterface(0xd9b67a26));
        assertTrue(token.supportsInterface(0x0e89341c));
    }

    function testTransferPreservesEditionSupply() public {
        OpporEarlyAdopter token = new OpporEarlyAdopter(creator, URI);
        vm.prank(creator);
        token.safeTransferFrom(creator, address(0xB0B), 1, 1, "");
        assertEq(token.balanceOf(address(0xB0B), 1), 1);
        assertEq(token.balanceOf(creator, 1), 999);
        assertEq(token.totalSupply(1), 1000);
    }

    function testNoExternalMintOrMetadataMutation() public {
        OpporEarlyAdopter token = new OpporEarlyAdopter(creator, URI);
        (bool minted,) = address(token).call(abi.encodeWithSignature("mint(address,uint256,uint256)", creator, 1, 1));
        (bool changed,) = address(token).call(abi.encodeWithSignature("setURI(string)", "https://invalid.example"));
        assertFalse(minted);
        assertFalse(changed);
        assertEq(token.totalSupply(1), 1000);
        assertEq(token.uri(1), URI);
    }

    function testRejectsInvalidConstructorArguments() public {
        vm.expectRevert("Invalid recipient");
        new OpporEarlyAdopter(address(0), URI);
        vm.expectRevert("Empty metadata");
        new OpporEarlyAdopter(creator, "");
    }
}
