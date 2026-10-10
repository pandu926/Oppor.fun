import { readFile, writeFile } from "node:fs/promises";
import { keccak256 } from "viem";
const root = new URL("../../", import.meta.url);
const metadata = JSON.parse(
  await readFile(new URL("contracts/deployment-artifacts.json", root), "utf8"),
);
const artifact = async (name) =>
  JSON.parse(
    await readFile(
      new URL(`contracts/out/${name}.sol/${name}.json`, root),
      "utf8",
    ),
  );
const factory = await artifact("CampaignFactory");
const escrow = await artifact("CampaignEscrow");
for (const [name, build] of [
  ["CampaignFactory", factory],
  ["CampaignEscrow", escrow],
]) {
  if (
    keccak256(build.deployedBytecode.object) !==
    metadata.contracts[name].runtime_code_hash
  )
    throw new Error(
      `${name} runtime does not match the release metadata. Re-export contract artifacts first.`,
    );
}
const output = {
  creationCode: factory.bytecode.object,
  factoryCodeHash: metadata.contracts.CampaignFactory.runtime_code_hash,
  escrowCodeHash: metadata.contracts.CampaignEscrow.runtime_code_hash,
};
await writeFile(
  new URL("frontend/src/lib/deployment-artifact.ts", root),
  "// Generated from the pinned Foundry build. Run node scripts/generate-deployment-artifact.mjs.\n" +
    "export const deploymentArtifact = " +
    JSON.stringify(output, null, 2) +
    " as const;\n",
);
