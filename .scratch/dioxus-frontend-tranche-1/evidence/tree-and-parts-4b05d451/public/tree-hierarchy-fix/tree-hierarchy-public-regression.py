#!/usr/bin/env python3
"""Public same-archive REVIUNG tree trace for the React reference or Dioxus candidate."""
import hashlib
import json
import subprocess
import sys
from pathlib import Path

if len(sys.argv) != 5:
    raise SystemExit('usage: tree-hierarchy-public-regression.py URL SESSION reference|candidate OUTPUT.json')
url, session, side, output = sys.argv[1:]
if side not in {'reference', 'candidate'}:
    raise SystemExit('side must be reference or candidate')
out = Path(output)
out.parent.mkdir(parents=True, exist_ok=True)
fixture = Path('/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target/builds/frontend-tree-8ea71a84-20261002/site-root/assets/fixtures/reviung41.boardstudio')
sha = hashlib.sha256(fixture.read_bytes()).hexdigest()

def ab(*args, check=True):
    p = subprocess.run(['agent-browser', '--session', session, *args], capture_output=True, text=True)
    if check and p.returncode:
        raise RuntimeError(p.stdout + p.stderr)
    return p.stdout.strip(), p.returncode

def ev(js):
    raw, code = ab('eval', js)
    if code:
        raise RuntimeError(raw)
    value = json.loads(raw)
    if isinstance(value, str):
        value = json.loads(value)
    return value

ab('open', url)
ab('wait', '250')
ab('upload', 'input[type=file]', str(fixture))
ab('wait', '1000')
# This read-only IndexedDB probe verifies that the public import opened the archive's saved document.
identity = ev(r'''(async()=>{
 const key=Object.keys(localStorage).find(k=>k==='boardstudio-v2-active-project'||k.startsWith('boardstudio-m1-active-project:'));
 const id=localStorage.getItem(key); const dbName=(await indexedDB.databases())[0].name;
 const request=indexedDB.open(dbName); const db=await new Promise((a,b)=>{request.onsuccess=()=>a(request.result);request.onerror=()=>b(request.error)});
 const store=db.transaction('projects','readonly').objectStore('projects');
 const record=await new Promise((a,b)=>{const q=store.get(id);q.onsuccess=()=>a(q.result);q.onerror=()=>b(q.error)}); db.close();
 return JSON.stringify({dbName,id,name:record?.name,revision:record?.revision,format:record?.format,parts:record?.parts?.length,matrices:record?.matrices?.length,layouts:record?.layouts?.length});
})()''')

def tree_state():
    return ev(r'''(()=>({
      root:[...document.querySelectorAll('[role=treeitem]')].find(e=>e.getAttribute('aria-level')==='1')&&(()=>{const e=[...document.querySelectorAll('[role=treeitem]')].find(e=>e.getAttribute('aria-level')==='1');return {text:e.innerText.replace(/\n/g,' '),expanded:e.getAttribute('aria-expanded')}})(),
      items:[...document.querySelectorAll('[role=treeitem]')].map(e=>({text:e.innerText.replace(/\n/g,' '),level:Number(e.getAttribute('aria-level')),expanded:e.getAttribute('aria-expanded'),selected:e.getAttribute('aria-selected')})),
      disclosureButtons:[...document.querySelectorAll('button[aria-label*=Expand],button[aria-label*=Collapse]')].map(e=>({label:e.getAttribute('aria-label'),expanded:e.getAttribute('aria-expanded')}))
    }))()''')

initial = tree_state()
if side == 'candidate' and initial['root'] and initial['root']['expanded'] != 'true':
    ab('find', 'role', 'button', 'click', '--name', 'Expand Keyboard PCB')
    ab('wait', '100')
if not any(b['label'] == 'Expand right keys' for b in tree_state()['disclosureButtons']):
    # Reference can have a known already-expanded matrix only in a reused browser; this harness uses fresh sessions.
    raise AssertionError('Expected a collapsed right keys matrix in fresh imported fixture')
ab('find', 'role', 'button', 'click', '--name', 'Expand right keys')
ab('wait', '100')
right_open = tree_state()
# Candidate currently adds an owned-layout node between the matrix and Columns; open it for a full descendant trace.
if side == 'candidate' and any(x['text'].startswith('right keys 18 keys') and x['expanded'] == 'false' for x in right_open['items']):
    ab('find', 'role', 'button', 'click', '--name', 'Expand right keys')
    ab('wait', '100')
    nested_open = tree_state()
else:
    nested_open = right_open

def direct_right_children(state):
    items = state['items']
    parent = next((i for i in items if i['level'] == 2 and i['text'].startswith('right keys ')), None)
    if not parent:
        return []
    start = items.index(parent)
    return [item['text'] for item in items[start + 1:] if item['level'] == parent['level'] + 1 and item['level'] <= 3]

expected_direct = { 'Column 1 3 keys', 'Column 2 3 keys', 'Column 3 3 keys', 'Column 4 3 keys', 'Column 5 3 keys', 'Column 6 3 keys', 'main-U1 mcu nice nano · 273.2, -15.0', 'main-RST reset switch tht top · 273.2, -45.0'}
all_direct_children = direct_right_children(right_open)
direct_children = [label for label in all_direct_children if not label.startswith('Bridge ')]
components_group = any(x['text'].startswith('Components 2 parts') and x['level'] == 2 for x in right_open['items'])
result = {
    'url': url,
    'side': side,
    'fixture': str(fixture),
    'fixture_sha256': sha,
    'project_identity': identity,
    'initial_root': initial['root'],
    'root_after_required_expand': tree_state()['root'],
    'right_matrix_state': right_open,
    'right_matrix_direct_children': direct_children,
    'all_direct_children_including_t1_11': all_direct_children,
    'nested_matrix_open_state': nested_open,
    'candidate_separate_components_group': components_group,
    'expected_react_direct_children': sorted(expected_direct),
    'direct_children_match_reference': set(direct_children) == expected_direct,
    'excluded': 'Outline and Bridge rows/activation belong to T1-11',
}
out.write_text(json.dumps(result, indent=2, ensure_ascii=False) + '\n')
print(json.dumps({k: result[k] for k in ['side','url','fixture_sha256','project_identity','initial_root','right_matrix_direct_children','candidate_separate_components_group','direct_children_match_reference']}, indent=2, ensure_ascii=False))
if result['initial_root'].get('expanded') != 'true' or not result['direct_children_match_reference'] or components_group:
    print('RED: candidate tree hierarchy differs from pinned React on same imported document', file=sys.stderr)
    sys.exit(1)
