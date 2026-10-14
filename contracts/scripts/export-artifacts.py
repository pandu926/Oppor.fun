#!/usr/bin/env python3
"""Export reproducible integration ABIs and runtime hashes, never deployment addresses."""
import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path
def _require(condition, message):
    if not condition:
        raise RuntimeError(message)

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser()
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
(root / 'abi').mkdir(exist_ok=True)

def write(path, value):
    data = json.dumps(value, indent=2) + '\n'
    if args.check:
        _require(path.read_text() == data, f'Stale exported artifact: {path}')
    else:
        path.write_text(data)
manifest = {'schema_version': 1, 'solidity': '0.8.30', 'evm_version': 'paris', 'optimizer_runs': 200, 'via_ir': True, 'bytecode_hash': 'none', 'dependencies': {'openzeppelin_contracts': {'version': '5.4.0', 'commit': 'c64a1edb67b6e3f4a15cca8909c9482ad33a02b0'}, 'forge_std': {'version': '1.9.7', 'commit': '77041d2ce690e692d6e03cc812b57d1ddaa4d505'}}, 'contracts': {}, 'source_sha256': {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((root / 'src').glob('*.sol'))}}
for name in ('CampaignFactory', 'CampaignEscrow'):
    artifact = json.loads((root / 'out' / f'{name}.sol' / f'{name}.json').read_text())
    settings = artifact['metadata']['settings']
    _require(artifact['metadata']['compiler']['version'].startswith('0.8.30+'), 'Verification failed')
    _require(settings['evmVersion'] == 'paris' and settings['viaIR'] is True, 'Verification failed')
    _require(settings['optimizer'] == {'enabled': True, 'runs': 200}, 'Verification failed')
    _require(settings['metadata']['bytecodeHash'] == 'none', 'Verification failed')
    runtime = artifact['deployedBytecode']['object']
    _require(not artifact['deployedBytecode'].get('immutableReferences'), 'Backend requires uniform runtime hashes')
    _require(not artifact['bytecode'].get('linkReferences'), 'External linking must be made explicit')
    size = len(bytes.fromhex(runtime.removeprefix('0x')))
    _require(size <= 24576, f'EIP-170 runtime size exceeded: {name}')
    digest = subprocess.check_output(['cast', 'keccak', runtime], text=True).strip()
    write(root / 'abi' / f'{name}.json', artifact['abi'])
    manifest['contracts'][name] = {'runtime_code_hash': digest, 'runtime_bytes': size, 'creation_bytes': len(bytes.fromhex(artifact['bytecode']['object'].removeprefix('0x')))}
write(root / 'deployment-artifacts.json', manifest)
print('Factory/escrow ABIs and uniform runtime code hashes verified.' if args.check else 'Exported factory/escrow ABIs and runtime code hashes.')
backend = (root.parent / 'backend/src/chain.rs').read_text()
structs = {}
for struct in ('CampaignConfig', 'Claim'):
    fields = re.search('struct ' + struct + ' \\{(.*?)\\}', backend, re.S).group(1)
    structs[struct] = '(' + ','.join((field.strip().split()[0] for field in fields.split(';') if field.strip())) + ')'

def canonical(item):
    if item['type'].startswith('tuple'):
        return '(' + ','.join((canonical(c) for c in item['components'])) + ')' + item['type'][5:]
    return item['type']

def rust_params(params):
    return [p.strip().split() for p in params.split(',') if p.strip()]
for client, name in (('Factory', 'CampaignFactory'), ('Escrow', 'CampaignEscrow')):
    block = re.search('interface ' + client + ' \\{(.*?)\\n    \\}', backend, re.S).group(1)
    abi = json.loads((root / 'abi' / f'{name}.json').read_text())
    for kind, method, params, suffix in re.findall('(function|event) (\\w+)\\(([^)]*)\\)([^;]*);', block):
        expected = rust_params(params)
        inputs = [structs.get(p[0], p[0]) for p in expected]
        actual = [a for a in abi if a['type'] == kind and a['name'] == method and ([canonical(i) for i in a['inputs']] == inputs)]
        _require(len(actual) == 1, f'Backend ABI mismatch: {client}.{method}')
        if kind == 'function':
            returns = re.search('returns\\(([^)]*)\\)', suffix)
            outputs = [structs.get(p[0], p[0]) for p in rust_params(returns.group(1))] if returns else []
            _require([canonical(o) for o in actual[0]['outputs']] == outputs, f'Backend return ABI mismatch: {method}')
        if kind == 'event':
            _require([i['indexed'] for i in actual[0]['inputs']] == ['indexed' in p for p in expected], 'Verification failed')
print('Every backend factory/escrow function and event matches the compiled ABI.')
