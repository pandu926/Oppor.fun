// Generated from Foundry artifacts. Run python3 scripts/generate-abi.py.

export const factoryAbi = [
  {
    type: "function",
    name: "createCampaign",
    inputs: [
      {
        name: "config",
        type: "tuple",
        internalType: "struct CampaignConfig",
        components: [
          {
            name: "campaignKey",
            type: "bytes32",
            internalType: "bytes32",
          },
          {
            name: "creator",
            type: "address",
            internalType: "address",
          },
          {
            name: "refundRecipient",
            type: "address",
            internalType: "address",
          },
          {
            name: "assetKind",
            type: "uint8",
            internalType: "uint8",
          },
          {
            name: "rewardToken",
            type: "address",
            internalType: "address",
          },
          {
            name: "rewardTokenId",
            type: "uint256",
            internalType: "uint256",
          },
          {
            name: "targetQuantity",
            type: "uint256",
            internalType: "uint256",
          },
          {
            name: "startsAt",
            type: "uint64",
            internalType: "uint64",
          },
          {
            name: "cutoffAt",
            type: "uint64",
            internalType: "uint64",
          },
          {
            name: "reviewDeadline",
            type: "uint64",
            internalType: "uint64",
          },
          {
            name: "claimDeadline",
            type: "uint64",
            internalType: "uint64",
          },
          {
            name: "mode",
            type: "uint8",
            internalType: "uint8",
          },
          {
            name: "participantCapacity",
            type: "uint32",
            internalType: "uint32",
          },
          {
            name: "winnerCount",
            type: "uint32",
            internalType: "uint32",
          },
          {
            name: "rulesHash",
            type: "bytes32",
            internalType: "bytes32",
          },
          {
            name: "raffleSeedCommitment",
            type: "bytes32",
            internalType: "bytes32",
          },
        ],
      },
    ],
    outputs: [
      {
        name: "escrow",
        type: "address",
        internalType: "address",
      },
    ],
    stateMutability: "nonpayable",
  },
] as const;

export const escrowAbi = [
  {
    type: "function",
    name: "activate",
    inputs: [],
    outputs: [],
    stateMutability: "nonpayable",
  },
  {
    type: "function",
    name: "cancel",
    inputs: [],
    outputs: [],
    stateMutability: "nonpayable",
  },
  {
    type: "function",
    name: "claim",
    inputs: [
      {
        name: "allocation",
        type: "tuple",
        internalType: "struct Claim",
        components: [
          {
            name: "index",
            type: "uint256",
            internalType: "uint256",
          },
          {
            name: "recipient",
            type: "address",
            internalType: "address",
          },
          {
            name: "tokenId",
            type: "uint256",
            internalType: "uint256",
          },
          {
            name: "quantity",
            type: "uint256",
            internalType: "uint256",
          },
        ],
      },
      {
        name: "proof",
        type: "bytes32[]",
        internalType: "bytes32[]",
      },
    ],
    outputs: [],
    stateMutability: "nonpayable",
  },
  {
    type: "function",
    name: "finalize",
    inputs: [
      {
        name: "root",
        type: "bytes32",
        internalType: "bytes32",
      },
      {
        name: "allocationManifestHash",
        type: "bytes32",
        internalType: "bytes32",
      },
      {
        name: "eligibilitySnapshotHash",
        type: "bytes32",
        internalType: "bytes32",
      },
      {
        name: "declaredAllocatedQuantity",
        type: "uint256",
        internalType: "uint256",
      },
      {
        name: "declaredLeafCount",
        type: "uint256",
        internalType: "uint256",
      },
    ],
    outputs: [],
    stateMutability: "nonpayable",
  },
  {
    type: "function",
    name: "fundERC1155",
    inputs: [
      {
        name: "amount",
        type: "uint256",
        internalType: "uint256",
      },
    ],
    outputs: [],
    stateMutability: "nonpayable",
  },
  {
    type: "function",
    name: "fundERC20",
    inputs: [
      {
        name: "amount",
        type: "uint256",
        internalType: "uint256",
      },
    ],
    outputs: [],
    stateMutability: "nonpayable",
  },
  {
    type: "function",
    name: "fundERC721",
    inputs: [
      {
        name: "tokenIds",
        type: "uint256[]",
        internalType: "uint256[]",
      },
    ],
    outputs: [],
    stateMutability: "nonpayable",
  },
  {
    type: "function",
    name: "sweepRemaining",
    inputs: [],
    outputs: [],
    stateMutability: "nonpayable",
  },
] as const;
