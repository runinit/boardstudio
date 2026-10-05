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
    run('download','button.m1-project-copy-action',str(path))
    run('wait','--fn','!document.querySelector(".m1-project-menu[open]")')
    with zipfile.ZipFile(path) as z:
        return json.loads(z.read('project.json')), {n:hashlib.sha256(z.read(n)).hexdigest() for n in z.namelist() if n.startswith('assets/')}


report={'url':URL,'fixture':FIXTURE,'status':'incomplete'}
try:
    run('open',URL);run('set','viewport','1280','800')
    run('wait','--fn','Boolean(document.querySelector(\'input[type="file"][accept=".boardstudio"]\'))')
    run('upload','input[type="file"][accept=".boardstudio"]',str(Path(FIXTURE).resolve()))
    run('wait','--fn','Boolean(document.querySelector(\'button[aria-label="Expand keys"]\'))')
    doc,assets=archive('baseline');report['baselineRevision']=doc['revision']
    button('Expand keys');button('J1 mounting hole npth · 66.7, -47.6')
    run('wait','input[aria-label="X mm"]')
    run('focus','input[aria-label="X mm"]');run('fill','input[aria-label="X mm"]','60')
    report['xBeforeEnter']=evaluate('({label:document.activeElement?.getAttribute("aria-label"),value:document.activeElement?.value})')
    run('press','Enter')
    run('focus','input[aria-label="Y mm"]');run('fill','input[aria-label="Y mm"]','-40')
    report['yBeforeEnter']=evaluate('({label:document.activeElement?.getAttribute("aria-label"),value:document.activeElement?.value})')
    run('press','Enter')
    doc,assets=archive('edited');report['edited']={'revision':doc['revision'],'position':next(p['pose']['at'] for p in doc['parts'] if p['reference']=='J1')}
    button('Undo');doc,assets=archive('one-undo');report['oneUndo']={'revision':doc['revision'],'position':next(p['pose']['at'] for p in doc['parts'] if p['reference']=='J1')}
    button('Undo');doc,assets=archive('two-undos');report['twoUndos']={'revision':doc['revision'],'position':next(p['pose']['at'] for p in doc['parts'] if p['reference']=='J1')}
    report['status']='observed'
except Exception as e:
    report['error']=str(e);raise
finally:
    (OUTPUT/'report.json').write_text(json.dumps(report,indent=2)+'\n');run('close')
print(json.dumps(report))
