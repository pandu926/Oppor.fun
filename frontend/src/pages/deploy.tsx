import { useState } from "react";
import { Link } from "react-router-dom";
import { useAccount } from "wagmi";
import { useConnectModal } from "@rainbow-me/rainbowkit";
import {
  createPublicClient,
  createWalletClient,
  custom,
  defineChain,
  http,
  formatUnits,
  keccak256,
  isAddress,
  type Hex,
} from "viem";
import { deploymentArtifact } from "../lib/deployment-artifact";
import { type Provider, shorten } from "../lib/wallet";
import { Dialog } from "../lib/context";
const mainnet = defineChain({
  id: 5042,
  name: "Arc",
  nativeCurrency: { name: "USDC", symbol: "USDC", decimals: 18 },
  rpcUrls: { default: { http: ["https://rpc.mainnet.arc.io"] } },
  blockExplorers: {
    default: { name: "Arc Explorer", url: "https://explorer.arc.io" },
  },
});
const client = createPublicClient({
  chain: mainnet,
  transport: http(mainnet.rpcUrls.default.http[0], { timeout: 15000 }),
});
type Review = {
  wallet: `0x${string}`;
  provider: Provider;
  gas: bigint;
  gasPrice: bigint;
};
export default function DeployFactory() {
  const account = useAccount();
  const { openConnectModal } = useConnectModal();
  const [review, setReview] = useState<Review | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [hash, setHash] = useState<Hex | null>(null);
  const [manifest, setManifest] = useState<Record<string, unknown> | null>(
    null,
  );
  async function prepare() {
    setBusy(true);
    setError("");
    try {
      if (!account.address || !account.connector)
        throw new Error("Connect a wallet first.");
      const provider = (await account.connector.getProvider()) as Provider;
      const wallet = createWalletClient({
        chain: mainnet,
        transport: custom(provider),
      });
      if ((await client.getChainId()) !== mainnet.id)
        throw new Error(
          "The RPC did not return the expected Arc mainnet chain.",
        );
      if ((await wallet.getChainId()) !== mainnet.id) {
        try {
          await wallet.switchChain({ id: mainnet.id });
        } catch (e) {
          if (
            (e as { code?: number }).code !== 4902 &&
            !String(e).includes("4902")
          )
            throw e;
          await wallet.addChain({ chain: mainnet });
          await wallet.switchChain({ id: mainnet.id });
        }
      }
      const [address] = await wallet.getAddresses();
      if (address?.toLowerCase() !== account.address.toLowerCase())
        throw new Error("Wallet account changed. Connect again.");
      const [gas, gasPrice, balance] = await Promise.all([
        client.estimateGas({
          account: address,
          data: deploymentArtifact.creationCode as Hex,
          value: 0n,
        }),
        client.getGasPrice(),
        client.getBalance({ address }),
      ]);
      const budget = gas + gas / 5n;
      if (balance < budget * gasPrice)
        throw new Error(
          "This wallet needs native USDC on Arc mainnet to cover the deployment fee.",
        );
      setReview({ wallet: address, provider, gas: budget, gasPrice });
    } catch (e) {
      setError(
        e instanceof Error ? e.message : "Deployment preparation failed.",
      );
    } finally {
      setBusy(false);
    }
  }
  async function verify(txHash: Hex) {
    const receipt = await client.waitForTransactionReceipt({
      hash: txHash,
      confirmations: 2,
      timeout: 180000,
    });
    if (
      receipt.status !== "success" ||
      !receipt.contractAddress ||
      !isAddress(receipt.contractAddress)
    )
      throw new Error("The factory deployment did not succeed.");
    const code = await client.getBytecode({ address: receipt.contractAddress });
    if (!code || keccak256(code) !== deploymentArtifact.factoryCodeHash)
      throw new Error(
        "Deployed runtime bytecode does not match the Oppor release.",
      );
    setManifest({
      schema_version: 1,
      chain_id: "5042",
      rpc_url: mainnet.rpcUrls.default.http[0],
      factory_address: receipt.contractAddress,
      factory_code_hash: deploymentArtifact.factoryCodeHash,
      escrow_code_hash: deploymentArtifact.escrowCodeHash,
      deployment_block: receipt.blockNumber.toString(),
      transaction_hash: txHash,
      deployer: receipt.from,
    });
  }
  async function send() {
    if (!review) return;
    setBusy(true);
    setError("");
    try {
      const wallet = createWalletClient({
        chain: mainnet,
        transport: custom(review.provider),
      });
      const [addresses, id] = await Promise.all([
        wallet.getAddresses(),
        wallet.getChainId(),
      ]);
      if (
        id !== mainnet.id ||
        addresses[0]?.toLowerCase() !== review.wallet.toLowerCase()
      )
        throw new Error(
          "Wallet or network changed. Prepare the deployment again.",
        );
      const txHash = await wallet.sendTransaction({
        account: review.wallet,
        chain: mainnet,
        data: deploymentArtifact.creationCode as Hex,
        value: 0n,
        gas: review.gas,
      });
      setHash(txHash);
      setReview(null);
      await verify(txHash);
    } catch (e) {
      setReview(null);
      setError(e instanceof Error ? e.message : "Deployment failed.");
    } finally {
      setBusy(false);
    }
  }
  function download() {
    if (!manifest) return;
    const url = URL.createObjectURL(
      new Blob([JSON.stringify(manifest, null, 2) + "\n"], {
        type: "application/json",
      }),
    );
    const a = document.createElement("a");
    a.href = url;
    a.download = "oppor-mainnet-deployment.json";
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
  return (
    <main className="deploy-shell">
      <Link className="brand" to="/">
        oppor<span className="brand-dot">.</span>
      </Link>
      <section className="panel">
        <p className="eyebrow">Operator setup</p>
        <h1>Deploy the Oppor factory</h1>
        <p>
          Publish the release contract on Arc mainnet from your wallet. The
          factory is permissionless and has no platform owner or upgrade key.
        </p>
        <dl className="facts">
          <div>
            <dt>Network</dt>
            <dd>Arc mainnet · 5042</dd>
          </div>
          <div>
            <dt>Native value</dt>
            <dd>0 USDC</dd>
          </div>
          <div>
            <dt>Wallet</dt>
            <dd>
              {account.address ? shorten(account.address) : "Not connected"}
            </dd>
          </div>
        </dl>
        <p className="notice">
          Your wallet pays the network fee. This deploys a factory; it does not
          fund a campaign or grant platform administrator access.
        </p>
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        {hash && (
          <p>
            Transaction:{" "}
            <a
              href={`https://explorer.arc.io/tx/${hash}`}
              target="_blank"
              rel="noopener noreferrer"
            >
              {shorten(hash)}
            </a>
          </p>
        )}
        {manifest ? (
          <>
            <p className="notice" role="status">
              Factory deployed and runtime bytecode verified.
            </p>
            <p className="mono">{String(manifest.factory_address)}</p>
            <button className="button primary" onClick={download}>
              Download deployment manifest
            </button>
          </>
        ) : hash ? (
          <button
            className="button primary"
            disabled={busy}
            onClick={() => {
              setBusy(true);
              setError("");
              void verify(hash)
                .catch((e) =>
                  setError(
                    e instanceof Error ? e.message : "Verification failed.",
                  ),
                )
                .finally(() => setBusy(false));
            }}
          >
            {busy ? "Checking receipt…" : "Verify deployment receipt"}
          </button>
        ) : account.isConnected ? (
          <button
            className="button primary"
            disabled={busy}
            onClick={() => void prepare()}
          >
            {busy ? "Preparing…" : "Prepare deployment"}
          </button>
        ) : (
          <button className="button primary" onClick={openConnectModal}>
            Connect wallet
          </button>
        )}
        <p className="fineprint">
          Save the verified manifest for the server configuration. If
          confirmation times out after broadcast, verify the displayed
          transaction before sending another deployment.
        </p>
      </section>
      {review && (
        <Dialog
          close={() => {
            if (!busy) setReview(null);
          }}
        >
          <h2 id="dialog-title">Review factory deployment</h2>
          <p>Deploy Oppor to Arc mainnet from {shorten(review.wallet)}.</p>
          <dl className="facts">
            <div>
              <dt>Chain ID</dt>
              <dd>5042</dd>
            </div>
            <div>
              <dt>Native transfer</dt>
              <dd>0 USDC</dd>
            </div>
            <div>
              <dt>Estimated fee budget</dt>
              <dd>{formatUnits(review.gas * review.gasPrice, 18)} USDC</dd>
            </div>
          </dl>
          <p>
            The estimate includes a gas margin. Your wallet displays the final
            fee before you approve.
          </p>
          <button
            className="button primary wide"
            disabled={busy}
            onClick={() => void send()}
          >
            {busy ? "Waiting for wallet…" : "Deploy from wallet"}
          </button>
        </Dialog>
      )}
    </main>
  );
}
