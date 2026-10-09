export type DocSection = {
  id: string;
  title: string;
  paragraphs?: string[];
  steps?: string[];
  table?: { headers: string[]; rows: string[][] };
  code?: { language: string; text: string };
  note?: string;
};
export type DocPage = {
  slug: string;
  group: string;
  title: string;
  description: string;
  sections: DocSection[];
};
export const docPages: DocPage[] = [
  {
    slug: "",
    group: "Getting started",
    title: "Introduction",
    description:
      "Community campaigns. Creator-funded rewards. Clear rules from entry to claim.",
    sections: [
      {
        id: "what-is-oppor",
        title: "What is Oppor?",
        paragraphs: [
          "Oppor is a campaign platform for token communities on Arc. Creators publish tasks and commit a reward pool. Participants complete the tasks, submit evidence, and claim their allocation after review and finalization.",
          "A campaign can reward participants with an ERC-20 token, an NFT collection, or an ERC-1155 edition. Creators choose between a raffle and a distribution to all eligible entries.",
        ],
      },
      {
        id: "how-it-fits-together",
        title: "How it fits together",
        table: {
          headers: ["Component", "Responsibility"],
          rows: [
            [
              "Campaign workspace",
              "Define rewards, tasks, distribution rules, and deadlines.",
            ],
            [
              "Reward escrow",
              "Hold the campaign reward asset and enforce funding, claim, and refund conditions.",
            ],
            [
              "Entry review",
              "Let the creator evaluate submitted evidence after the entry cutoff.",
            ],
            [
              "Allocation manifest",
              "Record recipients and quantities, with a Merkle commitment published to the escrow.",
            ],
            [
              "Participant wallet",
              "Sign in, submit an entry, and initiate a reward claim.",
            ],
          ],
        },
      },
      {
        id: "choose-your-path",
        title: "Choose your path",
        paragraphs: [
          "For participants, start with Joining a campaign. It covers entry requirements, task evidence, and the difference between saving and submitting.",
          "For creators, start with Creating a campaign. It explains the configuration you lock before deploying and funding an escrow.",
          "For integrations, the API overview describes wallet authentication, request conventions, and transaction preparation.",
        ],
      },
      {
        id: "important-boundaries",
        title: "Understand the rules",
        paragraphs: [
          "Social tasks are reviewed manually. A task link or declared username is not automatic verification of a like, repost, comment, or Discord membership.",
          "Campaign rewards follow the locked distribution policy and deadlines. Raffle participation does not guarantee selection. A funded token pool does not guarantee the token’s market value.",
        ],
      },
    ],
  },
  {
    slug: "quickstart",
    group: "Getting started",
    title: "Quickstart",
    description:
      "Go from discovery to a submitted entry, or launch your first campaign.",
    sections: [
      {
        id: "for-participants",
        title: "For participants",
        steps: [
          "Open Discover and select a campaign. Read the reward asset, distribution policy, tasks, and deadlines.",
          "Connect the wallet that will receive your rewards. Sign the login message in your wallet.",
          "Join the campaign. Optional social usernames are self-declared.",
          "Complete each required task and save its evidence.",
          "Select Submit entry for review before the cutoff.",
          "Check the creator’s review decision. After finalization, view your allocation and claim before the claim deadline.",
        ],
        note: "Saving evidence does not submit an entry. If you edit evidence after submitting, submit the entry again.",
      },
      {
        id: "for-creators",
        title: "For creators",
        steps: [
          "Create a draft with one reward asset, a distribution policy, a schedule, and at least one required task.",
          "Check the complete configuration, then lock it.",
          "Deploy the campaign escrow, approve the reward asset where required, and fund the pool.",
          "Activate the campaign and share its page with your community.",
          "Review submitted entries after cutoff. Lock eligibility before the review deadline.",
          "Inspect the allocation preview and finalize it onchain. Participants can then claim their rewards.",
        ],
      },
      {
        id: "wallet-signatures",
        title: "Wallet signatures and transactions",
        paragraphs: [
          "Signing in uses a message signature. It identifies your wallet and does not authorize a token transfer.",
          "Deployment, approvals, funding, activation, finalization, cancellation, refunds, and claims are separate blockchain transactions. Review the action, destination, and network fee in your wallet before confirming.",
        ],
      },
    ],
  },
  {
    slug: "joining-a-campaign",
    group: "For participants",
    title: "Joining a campaign",
    description:
      "Review the campaign rules and register the wallet that receives your rewards.",
    sections: [
      {
        id: "before-you-join",
        title: "Before you join",
        paragraphs: [
          "Check the reward token’s contract address, the entry cutoff, the review deadline, and the claim deadline. The campaign page shows the distribution mode and allocation policy.",
          "For a raffle, check the declared winner count. For all-eligible campaigns, check the capacity and reward policy. An entry limit controls registration; it does not mean every registered entry receives a reward.",
        ],
      },
      {
        id: "register-your-wallet",
        title: "Register your wallet",
        steps: [
          "Connect your wallet and sign in.",
          "Select Join campaign while registration is open.",
          "Add your X or Discord username if the campaign benefits from that context.",
          "Confirm that the campaign appears in My entries.",
        ],
        paragraphs: [
          "One wallet can register once per campaign. The campaign creator cannot register in their own campaign. Registration can close when capacity is reached or the entry cutoff passes.",
        ],
      },
      {
        id: "payout-wallet",
        title: "Your payout wallet",
        paragraphs: [
          "Your authenticated wallet is the payout wallet for the entry. You cannot direct the entry’s reward to another address in the registration request.",
          "Changing the selected account in your wallet ends the current interface session. Connect and sign in with the intended account before continuing.",
        ],
      },
      {
        id: "finding-your-entry",
        title: "Finding an existing entry",
        paragraphs: [
          "My entries includes campaigns remembered in your browser and entries found in the loaded public catalog. Load more public campaigns to expand the search.",
          "If you use another device or clear browser storage, open the original campaign link or recover the entry by campaign ID. Your recorded entry remains associated with your wallet.",
        ],
      },
    ],
  },
  {
    slug: "submitting-evidence",
    group: "For participants",
    title: "Submitting evidence",
    description:
      "Make your submission easy for the campaign creator to review.",
    sections: [
      {
        id: "accepted-evidence",
        title: "Accepted evidence",
        table: {
          headers: ["Format", "Use"],
          rows: [
            [
              "Text",
              "Explain what you completed or provide the requested response.",
            ],
            [
              "HTTPS link",
              "Link to a post, comment, or another relevant public page.",
            ],
            ["Image", "Attach a PNG, JPEG, or WebP screenshot up to 5 MiB."],
          ],
        },
        paragraphs: [
          "Follow the instructions on each task. A clear link and a short explanation are usually easier to review than an unexplained screenshot.",
          "Evidence is available to the submitting participant and campaign creator. Avoid including private conversations, credentials, or unrelated personal information.",
        ],
      },
      {
        id: "save-and-submit",
        title: "Save, then submit",
        steps: [
          "Complete the task outside Oppor.",
          "Add the requested evidence to the matching task and select Save evidence.",
          "Repeat for every required task.",
          "Select Submit entry for review before the entry cutoff.",
        ],
        note: "Any evidence edit returns the entry to REGISTERED. Submit again so the creator reviews your latest evidence.",
      },
      {
        id: "entry-status",
        title: "Entry status",
        table: {
          headers: ["Status", "Meaning"],
          rows: [
            [
              "REGISTERED",
              "The entry exists but its current evidence has not been submitted.",
            ],
            ["SUBMITTED", "The current entry has been sent for review."],
            ["ELIGIBLE", "The creator has accepted the entry."],
            [
              "DISQUALIFIED",
              "The creator has rejected the entry and recorded a reason.",
            ],
            [
              "NOT_SUBMITTED",
              "The entry did not meet the submission requirement before cutoff.",
            ],
          ],
        },
      },
      {
        id: "manual-review",
        title: "Manual review",
        paragraphs: [
          "Oppor does not automatically check X engagement or Discord membership. The creator reviews the evidence after cutoff and records an eligibility decision with a reason.",
          "An image passing upload validation confirms its accepted format and size. It does not prove that a social task was completed.",
        ],
      },
    ],
  },
  {
    slug: "claiming-rewards",
    group: "For participants",
    title: "Claiming rewards",
    description:
      "Receive a finalized allocation directly in your registered wallet.",
    sections: [
      {
        id: "when-claims-open",
        title: "When claims open",
        paragraphs: [
          "Claims become available after the creator finalizes the allocation and the backend verifies the matching escrow event. An eligible review decision alone does not open claims.",
          "In a raffle, only selected eligible entries receive an allocation. For an all-eligible distribution, allocation follows the locked policy and the finalized recipient set.",
        ],
      },
      {
        id: "claim-your-allocation",
        title: "Claim your allocation",
        steps: [
          "Sign in with the wallet used to register.",
          "Open My rewards or the campaign’s reward section.",
          "Check the allocation quantity and token ID, where applicable.",
          "Select Claim reward and review the transaction details.",
          "Confirm in your wallet and wait for the claim to be indexed.",
        ],
        paragraphs: [
          "The claim transaction must be sent by the allocation recipient. A successful claim pays that recipient; the interface cannot substitute a different payout wallet.",
        ],
      },
      {
        id: "claim-proofs",
        title: "Export claim proofs",
        paragraphs: [
          "Export claim proofs to keep the escrow address, allocation index, recipient, token ID, quantity, and Merkle proof. The manifest and onchain commitment let you inspect the finalized distribution.",
          "A saved proof can be used for a direct contract claim if the interface becomes unavailable. The escrow still enforces the recipient, claim deadline, and whether the allocation has already been claimed.",
        ],
      },
      {
        id: "deadline-and-fees",
        title: "Deadlines and fees",
        paragraphs: [
          "Claim before the campaign’s claim deadline. Unclaimed assets follow the escrow’s refund rules after the deadline.",
          "Your wallet shows the network fee before confirmation. Keep sufficient native USDC for transactions on Arc. A token’s ERC-20 base-unit quantity and the native gas amount use their respective decimals.",
        ],
      },
    ],
  },
  {
    slug: "creating-a-campaign",
    group: "For creators",
    title: "Creating a campaign",
    description:
      "Define a reward and participation rules your community can understand.",
    sections: [
      {
        id: "campaign-details",
        title: "Campaign details",
        paragraphs: [
          "Give the campaign a specific name and a description that explains the community, the reward, and the requested participation. Avoid promises that the campaign configuration does not support.",
          "A draft is private to its creator. Review it before locking the configuration. Draft updates replace the editable configuration; a full replacement also regenerates task IDs.",
        ],
      },
      {
        id: "reward-asset",
        title: "Choose one reward asset",
        table: {
          headers: ["Asset", "Configuration"],
          rows: [
            [
              "ERC-20",
              "Token contract and total integer quantity in base units.",
            ],
            [
              "ERC-721",
              "Collection contract and a unique inventory of up to 100 token IDs.",
            ],
            [
              "ERC-1155",
              "Token contract, one token ID, and its total integer quantity.",
            ],
          ],
        },
        paragraphs: [
          "The pool contains one reward asset. For ERC-20 tokens, convert the displayed token amount using that contract’s decimals. The form accepts exact base units rather than floating-point amounts.",
        ],
      },
      {
        id: "tasks",
        title: "Write clear tasks",
        paragraphs: [
          "A campaign supports one to twenty tasks, with at least one required task. Supported types include X repost, like, comment, tag, Discord join, and custom tasks.",
          "Each task has a target HTTPS URL and instructions. Tell participants what evidence to provide. X tasks must link to X or Twitter, and Discord tasks must link to Discord.",
        ],
      },
      {
        id: "schedule",
        title: "Set the schedule",
        table: {
          headers: ["Deadline", "Purpose"],
          rows: [
            ["Start", "The scheduled beginning of participation."],
            ["Entry cutoff", "Registration and evidence submission close."],
            [
              "Review deadline",
              "Review, eligibility locking, and finalization must finish.",
            ],
            ["Claim deadline", "Recipients must complete their claims."],
          ],
        },
        paragraphs: [
          "Dates must be future and strictly ordered. The claim deadline must allow at least 24 hours after the review deadline. Local dates entered in the form are saved as UTC.",
        ],
      },
      {
        id: "lock-configuration",
        title: "Lock the configuration",
        paragraphs: [
          "Locking commits the reward asset, quantities, tasks, rules, and deadlines. These settings cannot be edited afterward.",
          "Check the refund recipient, capacity, winner count, and every token ID before locking. Deploy and fund the escrow before the scheduled start.",
        ],
      },
    ],
  },
  {
    slug: "reward-distribution",
    group: "For creators",
    title: "Reward distribution",
    description: "Choose who receives rewards and how the pool is allocated.",
    sections: [
      {
        id: "distribution-modes",
        title: "Distribution modes",
        table: {
          headers: ["Mode", "Recipients"],
          rows: [
            [
              "All eligible",
              "Every entry included in the final eligible recipient set.",
            ],
            [
              "Raffle",
              "A selected subset of the final eligible entries, bounded by the winner count.",
            ],
          ],
        },
        paragraphs: [
          "Registration, submission, eligibility, and allocation are separate stages. Joining a campaign does not make an entry eligible, and eligibility does not guarantee raffle selection.",
        ],
      },
      {
        id: "allocation-policies",
        title: "Allocation policies",
        table: {
          headers: ["Policy", "Behavior"],
          rows: [
            [
              "Equal pool",
              "Distribute the allocated pool across the eligible or selected recipients under the locked rules.",
            ],
            [
              "Fixed reward",
              "Declare a reward per recipient and a positive capacity; total funding equals reward per recipient × capacity.",
            ],
          ],
        },
        paragraphs: [
          "Fixed rewards require a divisible asset: ERC-20 or ERC-1155. ERC-721 campaigns use the configured NFT inventory.",
          "For an all-eligible NFT campaign, capacity must be positive and covered by inventory. For an NFT raffle, inventory must cover the declared winner count.",
        ],
      },
      {
        id: "entry-limits",
        title: "Entry limits and capacity",
        paragraphs: [
          "Registration limits are bounded at 100,000. A positive capacity becomes the effective registration cap; otherwise the registration limit applies. Capacity cannot exceed the registration limit.",
          "Configure limits you can review within the available time. A large entry cap creates a correspondingly large evidence-review workload.",
        ],
      },
      {
        id: "allocation-artifacts",
        title: "Allocation artifacts",
        paragraphs: [
          "Eligibility locking produces an immutable ordered snapshot and starts allocation processing. The allocation preview includes the Merkle root, manifest hash, recipient count, and allocated quantity.",
          "The creator reviews and publishes these commitments through finalization. Public results become available after the matching onchain event is confirmed.",
        ],
      },
    ],
  },
  {
    slug: "funding-and-launch",
    group: "For creators",
    title: "Funding & launch",
    description:
      "Deploy the escrow, deposit the complete reward pool, and activate your campaign.",
    sections: [
      {
        id: "deploy",
        title: "Deploy the escrow",
        paragraphs: [
          "After locking configuration, deploy a campaign escrow through the factory. Review the factory destination and the locked configuration commitment in the transaction dialog.",
          "Each campaign has its own escrow. The factory registers the creator and campaign key against the deployed escrow address.",
        ],
      },
      {
        id: "approve-and-fund",
        title: "Approve and fund",
        table: {
          headers: ["Asset", "Approval"],
          rows: [
            [
              "ERC-20",
              "Approve the required amount to the campaign escrow. A token may require resetting an existing allowance first.",
            ],
            ["ERC-721", "Approve each configured NFT token ID to the escrow."],
            [
              "ERC-1155",
              "Authorize collection access for the campaign escrow, then deposit the configured edition quantity.",
            ],
          ],
        },
        steps: [
          "Select Approve & fund reward pool.",
          "Confirm the required approval transactions individually.",
          "Prepare funding again after the approvals are confirmed.",
          "Confirm the funding transaction and wait for recognized escrow state.",
        ],
        note: "ERC-1155 collection approval grants the escrow operator access to the collection. Review the exact spender before approving.",
      },
      {
        id: "activate",
        title: "Activate the campaign",
        paragraphs: [
          "Activate only after the escrow recognizes the complete required funding. Share the campaign page once activation has been indexed. Participation follows the scheduled start and cutoff.",
          "A transaction hash is a tracking reference. The application confirms campaign funding and activation from verified escrow events, rather than accepting a submitted hash as proof.",
        ],
      },
      {
        id: "refunds",
        title: "Cancellation and unclaimed rewards",
        paragraphs: [
          "The escrow enforces when cancellation is allowed. Cancelling does not provide a way to rewrite a finalized distribution.",
          "After the claim deadline, use the refund action to return remaining assets to the configured refund recipient. Token restrictions, transfers, and NFT receivers can affect whether an asset transfer succeeds.",
        ],
      },
    ],
  },
  {
    slug: "review-and-finalize",
    group: "For creators",
    title: "Review & finalize",
    description:
      "Turn submitted evidence into an immutable recipient allocation.",
    sections: [
      {
        id: "review-period",
        title: "Review period",
        paragraphs: [
          "Review begins at the entry cutoff and ends at the review deadline. Open an entry to inspect the declared usernames, saved task evidence, links, and uploaded images.",
          "Record ELIGIBLE or DISQUALIFIED with a nonempty reason. Decisions are appended to review history. Check the latest version before changing a decision.",
        ],
      },
      {
        id: "lock-eligibility",
        title: "Lock eligibility",
        paragraphs: [
          "Every submitted entry must have a review decision before eligibility can be locked. Entries that were never submitted do not become eligible.",
          "Lock eligibility with at least two minutes remaining before the review deadline. This preserves time to produce the allocation artifacts and finalize the campaign.",
        ],
        note: "Do not leave finalization until the final minute. Eligibility locking starts allocation processing; it does not itself open claims.",
      },
      {
        id: "inspect-preview",
        title: "Inspect the allocation preview",
        steps: [
          "Check the recipient count and allocated quantity.",
          "Export and inspect the allocation manifest.",
          "Verify that the preview follows the configured distribution mode and policy.",
          "For a raffle, inspect the published selection artifacts when available.",
        ],
      },
      {
        id: "publish",
        title: "Finalize onchain",
        paragraphs: [
          "Finalize the distribution with its Merkle root, allocation manifest hash, and eligibility snapshot hash. The escrow binds future claims to the published allocation.",
          "The backend publishes matching results after it verifies the finalization event. Participants can then retrieve their proofs and claim.",
          "The creator reviews evidence and publishes the allocation. Escrow enforces the published commitments and payout conditions; it does not independently verify social activity or offchain review decisions.",
        ],
      },
    ],
  },
  {
    slug: "escrow-and-security",
    group: "Reference",
    title: "Escrow & security",
    description:
      "Understand custody, commitments, and the responsibilities of each actor.",
    sections: [
      {
        id: "custody",
        title: "Reward custody",
        paragraphs: [
          "Rewards are deposited into the campaign escrow. The factory and escrow are non-upgradeable and do not expose a platform withdrawal key or a platform signer that can spend participant allocations.",
          "Creators control campaign preparation, activation, review, and finalization within the contract’s state and deadline rules. Participants initiate their own claims.",
        ],
      },
      {
        id: "commitments",
        title: "Configuration and allocation commitments",
        paragraphs: [
          "The configuration hash binds the campaign’s escrow parameters. The rules hash identifies the locked offchain rules. Finalization publishes the distribution root and artifact hashes.",
          "Claim proofs bind an allocation to its recipient, token ID, quantity, and index. The contract checks the proof and prevents the same allocation from being claimed twice.",
        ],
      },
      {
        id: "wallet-checks",
        title: "Wallet transaction checks",
        paragraphs: [
          "The interface checks the prepared intent, network, sender, destination, expiry, and decoded function before requesting a wallet transaction. Creation calldata must match the locked configuration commitment.",
          "Approvals name the campaign escrow. ERC-20 approval amounts are bounded by the configured pool, and ERC-721 approvals reference configured inventory. Always verify the transaction shown by your wallet.",
        ],
      },
      {
        id: "trust-boundaries",
        title: "Trust boundaries",
        table: {
          headers: ["Actor or component", "Responsibility"],
          rows: [
            [
              "Creator",
              "Task accuracy, evidence review, eligibility decisions, and allocation publication.",
            ],
            [
              "Escrow",
              "Funding rules, state transitions, deadlines, committed proofs, and transfers.",
            ],
            [
              "Backend",
              "Session authorization, evidence access, artifact generation, and verified event indexing.",
            ],
            [
              "Reward asset",
              "Its own transfer behavior, restrictions, and economic value.",
            ],
            [
              "Participant",
              "Protect wallet access, verify the campaign, and claim before the deadline.",
            ],
          ],
        },
      },
      {
        id: "moderation",
        title: "Platform moderation",
        paragraphs: [
          "Operators can hide campaign discovery, suspend accounts, and revoke sessions. These controls do not rewrite escrow balances, eligibility commitments, or published allocations.",
          "Existing claim artifacts and contract-enforced claim rights remain separate from listing visibility. Keep an exported proof when you need direct contract access.",
        ],
      },
    ],
  },
  {
    slug: "api-overview",
    group: "Reference",
    title: "API overview",
    description:
      "Integrate campaign discovery, wallet authentication, and transaction preparation.",
    sections: [
      {
        id: "base-path",
        title: "Request conventions",
        paragraphs: [
          "The API is served under /v1. JSON responses use typed fields. Campaign IDs and entry IDs are UUIDs; token quantities are canonical decimal strings in base units.",
          "GET requests retrieve state. Session-authenticated mutations require the session cookie, X-CSRF-Token, an allowed Origin, and Idempotency-Key. Keep the CSRF token in memory.",
        ],
        code: {
          language: "HTTP",
          text: "GET /v1/campaigns?limit=25&sort=ending\nAccept: application/json",
        },
      },
      {
        id: "authentication",
        title: "Wallet authentication",
        steps: [
          "POST /auth/challenge with wallet and chain_id.",
          "Sign the exact returned message using the selected wallet.",
          "POST /auth/verify with message and signature.",
          "Send credentials on authenticated requests and the returned CSRF token on mutations.",
        ],
        paragraphs: [
          "Deployed contract wallets use EIP-1271 signature validation. Counterfactual wallets using EIP-6492 are not supported. Changing or revoking a contract-wallet signature can invalidate the session.",
        ],
        code: {
          language: "JSON",
          text: '{\n  "wallet": "<wallet-address>",\n  "chain_id": "<configured-chain-id>"\n}',
        },
      },
      {
        id: "endpoints",
        title: "Core endpoints",
        table: {
          headers: ["Endpoint", "Purpose"],
          rows: [
            [
              "GET /campaigns",
              "Paginated public discovery; filters and newest/ending sort.",
            ],
            ["GET /campaigns/:id", "Campaign configuration and ordered tasks."],
            ["POST /campaigns", "Create a private campaign draft."],
            [
              "POST /campaigns/:id/entries",
              "Register the authenticated payout wallet.",
            ],
            [
              "PUT /campaigns/:id/my-entry/submissions/:task_id",
              "Save versioned task evidence.",
            ],
            [
              "POST /campaigns/:id/my-entry/submit",
              "Submit the current entry for review.",
            ],
            [
              "GET /campaigns/:id/allocations/:wallet",
              "Retrieve allocations and claim proofs.",
            ],
          ],
        },
      },
      {
        id: "prepared-transactions",
        title: "Prepared transactions",
        paragraphs: [
          "Prepare endpoints return an unsigned transaction with chain_id, to, data, value, expected_sender, intent, and expires_at. The caller checks the payload and asks the designated wallet to execute it.",
          "Creation, funding, activation, finalization, claiming, cancellation, and sweeping each have a prepare endpoint. Funding also returns required approvals. Confirm approvals and prepare funding again before execution.",
          "POST /campaigns/:id/transactions records a tracking hint. Authoritative confirmation comes from the indexer’s verified escrow events.",
        ],
      },
      {
        id: "concurrency",
        title: "Versioning and retries",
        paragraphs: [
          "Versioned mutations carry expected_version. If state has changed, refresh and inspect it before repeating the action.",
          "For an ambiguous network failure or timeout, reuse the same idempotency key with the exact original payload. Do not retry a rejected business rule blindly.",
        ],
        code: {
          language: "HTTP",
          text: "X-CSRF-Token: <session-csrf-token>\nIdempotency-Key: <unique-request-key>\nContent-Type: application/json",
        },
      },
      {
        id: "errors",
        title: "Error responses",
        code: {
          language: "JSON",
          text: '{\n  "error": {\n    "code": "STALE_VERSION",\n    "message": "Refresh the campaign before editing.",\n    "request_id": "<request-reference>"\n  }\n}',
        },
        paragraphs: [
          "400 indicates an invalid request, 401 an unauthenticated session, 403 denied access, 404 an unavailable resource, 409 a state or version conflict, 422 business validation, and 429 a rate limit. Dependency failures use 503 or 504.",
        ],
      },
    ],
  },
  {
    slug: "faq",
    group: "Reference",
    title: "Frequently asked questions",
    description:
      "Answers to common questions about entries, reviews, and rewards.",
    sections: [
      {
        id: "social-accounts",
        title: "Do I need to connect X or Discord?",
        paragraphs: [
          "No. Campaign tasks link to the relevant platform, and you submit evidence in Oppor. Optional social usernames are self-declared. The creator reviews whether an entry meets the campaign requirements.",
        ],
      },
      {
        id: "guaranteed-reward",
        title: "Does joining guarantee a reward?",
        paragraphs: [
          "No. You must submit the required evidence and be accepted as eligible. In a raffle, only selected eligible entries receive an allocation. The locked reward policy and finalized allocation determine the payout.",
        ],
      },
      {
        id: "edit-entry",
        title: "Can I update an entry after submitting?",
        paragraphs: [
          "You can edit evidence while submissions are open. Editing returns the entry to REGISTERED. Submit it again before cutoff to put the updated evidence into review.",
        ],
      },
      {
        id: "pending-transaction",
        title: "Why is the campaign unchanged after a transaction?",
        paragraphs: [
          "Wallet broadcast, receipt confirmation, and indexed application state are separate steps. Check the transaction hash in the explorer, then refresh the campaign. Do not repeat a transfer just because indexing is delayed.",
        ],
      },
      {
        id: "missing-entry",
        title: "Why is an entry missing on another device?",
        paragraphs: [
          "The workspace combines remembered campaign IDs with entries in the loaded public catalog. Open the original campaign link, add its ID to the recovery field, or load more public campaigns. Entries remain recorded against your wallet.",
        ],
      },
      {
        id: "missed-claim",
        title: "What happens if I miss the claim deadline?",
        paragraphs: [
          "The escrow closes the claim window under its deadline rules. Remaining assets can be returned to the configured refund recipient. Claim early enough to resolve wallet or network issues.",
        ],
      },
      {
        id: "creator-changes",
        title: "Can a creator change tasks or rewards after locking?",
        paragraphs: [
          "No. Locking makes the campaign configuration immutable. Check the rules, reward contract, and deadlines on the campaign page before joining.",
        ],
      },
    ],
  },
];
export function docPath(slug: string) {
  return slug ? `/docs/${slug}` : "/docs";
}
export function findDoc(path: string) {
  const normalized = path.replace(/\/$/, "");
  return docPages.find((page) => docPath(page.slug) === normalized);
}
export const docGroups = [
  "Getting started",
  "For participants",
  "For creators",
  "Reference",
];
