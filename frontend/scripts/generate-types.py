"""Generate domain types from the repository's authoritative OpenAPI schemas."""
import json
from pathlib import Path
root = Path(__file__).resolve().parents[2]
schemas = json.loads((root / 'backend/openapi.json').read_text())['components']['schemas']
def typ(s):
    if '$ref' in s: return s['$ref'].split('/')[-1]
    if 'anyOf' in s: return ' | '.join(typ(v) for v in s['anyOf'])
    if 'enum' in s: return ' | '.join(json.dumps(v) for v in s['enum'])
    t = s.get('type')
    if t == 'null': return 'null'
    if t in ('number', 'integer'): return 'number'
    if t in ('string', 'boolean'): return t
    if t == 'array': return '('+typ(s['items'])+')[]'
    if t == 'object':
        if not s.get('properties'): return 'Record<string, unknown>'
        return '{\n'+'\n'.join(f'  {json.dumps(k)}{"" if k in s.get("required", []) else "?"}: {typ(v)};' for k,v in s['properties'].items())+'\n}'
    return 'unknown'
text = '// Generated from backend/openapi.json. Run python3 scripts/generate-types.py.\n'
text += '\n\n'.join('export type '+k+' = '+typ(v)+';' for k,v in schemas.items())+'\n'
(root/'frontend/src/lib/types.ts').write_text(text)
