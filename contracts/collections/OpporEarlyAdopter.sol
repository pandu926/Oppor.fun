// SPDX-License-Identifier: MIT
pragma solidity 0.8.30;

import {ERC1155} from "@openzeppelin/contracts/token/ERC1155/ERC1155.sol";
import {ERC1155Supply} from "@openzeppelin/contracts/token/ERC1155/extensions/ERC1155Supply.sol";

/// @notice Fixed-edition commemorative collection. All 1,000 units are minted once at deployment.
/// @dev No owner, additional minting, metadata setter, pause, or upgrade mechanism.
contract OpporEarlyAdopter is ERC1155Supply {
    string public constant name = "Oppor.fun Early Adopter";
    string public constant symbol = "OPPOR-EA";
    uint256 public constant TOKEN_ID = 1;
    uint256 public constant EDITION_SIZE = 1000;

    constructor(address recipient, string memory metadataURI) ERC1155(metadataURI) {
        require(recipient != address(0), "Invalid recipient");
        require(bytes(metadataURI).length != 0, "Empty metadata");
        _mint(recipient, TOKEN_ID, EDITION_SIZE, "");
    }
}
