#!/usr/bin/env python3
"""Public desktop replay of the published coordinate correction; isolated Chrome."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time
import zipfile

URL, FIXTURE, OUTPUT = sys.argv[1:]
OUTPUT = Path(OUTPUT).resolve()
OUTPUT.mkdir(parents=True, exist_ok=False)
SESSION = 'coordinate-desktop-' + str(time.time_ns())
steps = []

def run(*args):
    result = subprocess.run(['agent-browser', '--session', SESSION, *args, '--json'], capture_output=True, text=True, timeout=90)
    steps.append({'args':args,'exit':result.returncode,'stdout':result.stdout,'stderr':result.stderr})
    (OUTPUT/'steps.json').write_text(json.dumps(steps,indent=2)+'\n')
    if result.returncode: raise RuntimeError(result.stdout + result.stderr)
    value = json.loads(result.stdout)
    if not value.get('success',True): raise RuntimeError(str(value))
    return value.get('data',value)

def evaluate(expression):
    return run('eval',expression).get('result')

def button(name):
    run('find','role','button','click','--name',name,'--exact')

def archive(name):
    button('Project')
    path=OUTPUT/(name+'.boardstudio')
    run('download','button[title="Save project copy…"]',str(path))

    with zipfile.ZipFile(path) as z:
        return json.loads(z.read('project.json')), {n:hashlib.sha256(z.read(n)).hexdigest() for n in z.namelist() if n.startswith('assets/')}


report={'url':URL,'fixture':FIXTURE,'checks':{},'status':'incomplete'}
try:
    run('open',URL)
    run('set','viewport','1280','800')
    run('wait','--fn','Boolean(document.querySelector(".wb-project-trigger") || document.querySelector(".wb-open-project"))')
    if evaluate('Boolean(document.querySelector(".wb-project-trigger"))'): run('click','.wb-project-trigger')
    run('click','.wb-open-project')
    run('upload','input[type="file"].wb-project-file-input',str(Path(FIXTURE).resolve()))
    run('wait','[role="button"][aria-label="J1, mounting hole npth, X 66.675 Y -47.625"]')
    run('focus','[role="button"][aria-label="J1, mounting hole npth, X 66.675 Y -47.625"]')
    run('press','Enter')
    report['selection']=evaluate('({active:document.activeElement?.outerHTML.slice(0,300),fields:[...document.querySelectorAll("input[type=number]")].map(e=>({label:e.getAttribute("aria-label"),value:e.value})),j1:[...document.querySelectorAll("button,[role=treeitem]")].filter(e=>(e.getAttribute("aria-label")||e.textContent||"").includes("J1")).map(e=>({role:e.getAttribute("role"),name:e.getAttribute("aria-label"),text:e.textContent?.trim().slice(0,90)})).slice(-8)})')
    run('wait','.wb-coordinate-grid > label:nth-child(1) input')
    run('focus','.wb-coordinate-grid > label:nth-child(1) input')
    run('fill','.wb-coordinate-grid > label:nth-child(1) input','60')
    report['xBeforeEnter']=evaluate('({label:document.activeElement?.getAttribute("aria-label"),value:document.activeElement?.value})')
    run('press','Enter')
    run('focus','.wb-coordinate-grid > label:nth-child(2) input')
    run('fill','.wb-coordinate-grid > label:nth-child(2) input','-40')
    report['yBeforeEnter']=evaluate('({label:document.activeElement?.getAttribute("aria-label"),value:document.activeElement?.value})')
    run('press','Enter')
    doc,assets=archive('edited')
    report['edited']={'revision':doc['revision'],'position':next(p['pose']['at'] for p in doc['parts'] if p['reference']=='J1')}
    button('Undo');button('Undo')
    doc,assets=archive('two-undos')
    report['twoUndos']={'revision':doc['revision'],'position':next(p['pose']['at'] for p in doc['parts'] if p['reference']=='J1')}
    report['status']='observed'
except Exception as e:
    report['error']=str(e)
    raise
finally:
    (OUTPUT/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    run('close')
print(json.dumps(report))
