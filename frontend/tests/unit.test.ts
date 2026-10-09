import { encodeFunctionData } from "viem";
import { escrowAbi } from "../src/lib/abi";
import { test, mock } from "node:test";
import assert from "node:assert/strict";
import { secureUrl } from "../src/lib/config";
import {
  uint256,
  validatePrepared,
  executePrepared,
  publicClient,
  type Provider,
} from "../src/lib/wallet";
import { samples } from "../src/lib/demo";
import { decimalLabel } from "../src/lib/reward";
import type { PreparedTransaction } from "../src/lib/types";
const wallet = "0x1111111111111111111111111111111111111111";
const campaign = {
  ...samples[0],
  escrow_address: "0x4444444444444444444444444444444444444444",
};
const tx: PreparedTransaction = {
  chain_id: "5042002",
  to: campaign.escrow_address,
  data: encodeFunctionData({
    abi: escrowAbi,
    functionName: "fundERC20",
    args: [1n],
  }),
  value: "0",
  expected_sender: wallet,
  intent: "FUND",
  expires_at: "2099-01-01T00:00:00Z",
};
test("uint256 rejects overflow, fractional, negative and noncanonical values", () => {
  assert.equal(uint256("0"), "0");
  assert.equal(uint256(String(2n ** 256n - 1n)), String(2n ** 256n - 1n));
  for (const n of ["01", "-1", "1.1", "1e18", "", String(2n ** 256n)])
    assert.throws(() => uint256(n));
});
test("URLs reject executable protocols, embedded credentials and nonlocal HTTP", () => {
  for (const u of [
    "javascript:alert(1)",
    "http://example.com",
    "https://user:password@example.com",
  ])
    assert.throws(() => secureUrl(u));
  assert.equal(
    secureUrl("http://127.0.0.1:8080", true),
    "http://127.0.0.1:8080/",
  );
  assert.throws(() => secureUrl("http://example.com", true));
});
test("prepared transactions bind intent, sender, network, destination, value and expiry", () => {
  validatePrepared(tx, wallet, campaign, "FUND");
  for (const replacement of [
    { chain_id: "1" },
    { to: wallet },
    { expected_sender: campaign.escrow_address },
    { value: "1" },
    { expires_at: "2000-01-01T00:00:00Z" },
    { data: "0x1" },
    { intent: "CLAIM" },
  ])
    assert.throws(() =>
      validatePrepared({ ...tx, ...replacement }, wallet, campaign, "FUND"),
    );
});
test("formatting token quantities preserves integer precision", () => {
  assert.equal(decimalLabel("2500000000", 6), "2,500");
  assert.equal(
    decimalLabel("123456789123456789123456789", 18),
    "123,456,789.123456",
  );
});
test("token approvals bind spender and reject excessive amounts", () => {
  const approved = encodeFunctionData({
    abi: [
      {
        type: "function",
        name: "approve",
        stateMutability: "nonpayable",
        inputs: [
          { name: "spender", type: "address" },
          { name: "amount", type: "uint256" },
        ],
        outputs: [{ type: "bool" }],
      },
    ],
    functionName: "approve",
    args: [campaign.escrow_address as `0x${string}`, 1n],
  });
  const approval = {
    ...tx,
    to: campaign.reward.token_address,
    intent: "APPROVE",
    data: approved,
  };
  validatePrepared(approval, wallet, campaign, "APPROVE");
  const wrong = encodeFunctionData({
    abi: [
      {
        type: "function",
        name: "approve",
        stateMutability: "nonpayable",
        inputs: [
          { name: "spender", type: "address" },
          { name: "amount", type: "uint256" },
        ],
        outputs: [{ type: "bool" }],
      },
    ],
    functionName: "approve",
    args: [wallet, 2n ** 256n - 1n],
  });
  assert.throws(() =>
    validatePrepared({ ...approval, data: wrong }, wallet, campaign, "APPROVE"),
  );
});

test("connector provider broadcasts only validated transactions and requires a successful receipt", async () => {
  const hash = ("0x" + "b".repeat(64)) as `0x${string}`;
  let sent = 0;
  let account = wallet;
  const provider = {
    request: async ({
      method,
      params,
    }: {
      method: string;
      params?: unknown[];
    }) => {
      if (method === "eth_accounts") return [account];
      if (method === "eth_chainId") return "0x4cef52";
      if (method === "eth_sendTransaction") {
        sent++;
        const body = params![0] as {
          from: string;
          to: string;
          data: string;
          value: string;
        };
        assert.equal(body.from.toLowerCase(), wallet);
        assert.equal(body.to.toLowerCase(), tx.to);
        assert.equal(body.data, tx.data);
        assert.equal(body.value, "0x0");
        return hash;
      }
      throw new Error("Unexpected provider request: " + method);
    },
  } as unknown as Provider;
  const receipt = mock.method(
    publicClient,
    "waitForTransactionReceipt",
    async ({ hash: received }: { hash: string }) => {
      assert.equal(received, hash);
      return { status: "success" };
    },
  );
  try {
    let broadcast: string | undefined;
    assert.equal(
      await executePrepared(provider, tx, wallet, campaign, "FUND", (h) => {
        broadcast = h;
      }),
      hash,
    );
    assert.equal(broadcast, hash);
    assert.equal(sent, 1);
    account = "0x2222222222222222222222222222222222222222";
    await assert.rejects(
      executePrepared(provider, tx, wallet, campaign, "FUND"),
      /Wallet or network changed/,
    );
    assert.equal(sent, 1);
    account = wallet;
    receipt.mock.mockImplementation(async () => ({ status: "reverted" }));
    await assert.rejects(
      executePrepared(provider, tx, wallet, campaign, "FUND"),
      /Transaction reverted/,
    );
  } finally {
    receipt.mock.restore();
  }
});
