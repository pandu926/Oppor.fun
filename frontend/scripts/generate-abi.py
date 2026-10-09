"""Export the frontend function ABI from Foundry artifacts; compile contracts first."""
import json
from pathlib import Path
root = Path(__file__).resolve().parents[2]
parts = ['// Generated from Foundry artifacts. Run python3 scripts/generate-abi.py.']
for contract, export in [('CampaignFactory','factoryAbi'),('CampaignEscrow','escrowAbi')]:
    artifact = root / f'contracts/out/{contract}.sol/{contract}.json'
    names = {'createCampaign','fundERC20','fundERC721','fundERC1155','activate','finalize','claim','cancel','sweepRemaining'}
    abi = [a for a in json.loads(artifact.read_text())['abi'] if a['type']=='function' and a['name'] in names]
    parts.append(f'export const {export} = {json.dumps(abi, indent=2)} as const;')
(root/'frontend/src/lib/abi.ts').write_text('\n\n'.join(parts)+'\n')
