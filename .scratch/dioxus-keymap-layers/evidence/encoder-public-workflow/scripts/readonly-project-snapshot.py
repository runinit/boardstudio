#!/usr/bin/env python3
"""Read the current BoardStudio project and Keymap encoder values via readonly IDB."""
import json
import subprocess
import sys
from pathlib import Path

if len(sys.argv) != 3:
    raise SystemExit('usage: readonly-project-snapshot.py SESSION OUTPUT.json')
session, output = sys.argv[1:]
js = r'''(async()=>{
  const dbNames=(await indexedDB.databases()).map(x=>x.name).filter(Boolean);
  const activeKeys=Object.keys(localStorage).filter(k=>k.includes('active-project'));
  const opened=[];
  for(const name of dbNames){
    const q=indexedDB.open(name);
    const db=await new Promise((resolve,reject)=>{q.onsuccess=()=>resolve(q.result);q.onerror=()=>reject(q.error)});
    if(!db.objectStoreNames.contains('projects')){db.close();continue}
    const keys=activeKeys.filter(k=>k.endsWith(name));
    const activeId=keys.map(k=>localStorage.getItem(k)).find(Boolean);
    if(!activeId){db.close();continue}
    const tx=db.transaction('projects','readonly');
    const req=tx.objectStore('projects').get(activeId);
    const record=await new Promise((resolve,reject)=>{req.onsuccess=()=>resolve(req.result);req.onerror=()=>reject(req.error)});
    await new Promise((resolve,reject)=>{tx.oncomplete=resolve;tx.onerror=()=>reject(tx.error);tx.onabort=()=>reject(tx.error)});
    db.close();
    if(!record)continue;
    const doc=record.document??record;
    const layers=(doc.keymap?.layers??[]).map(layer=>({id:layer.id,name:layer.name,sensors:layer.sensors??{},bindings:layer.bindings??{}}));
    opened.push({database:name,activeKey:keys[0],activeProjectId:activeId,recordKeys:Object.keys(record),projectId:doc.id,name:doc.name,revision:doc.revision,layers});
  }
  return JSON.stringify({url:location.href,databaseNames:dbNames,activeProjectKeys:activeKeys,projects:opened});
})()'''
run=subprocess.run(['agent-browser','--session',session,'eval',js],text=True,capture_output=True)
if run.returncode:
    raise SystemExit(run.stderr+run.stdout)
raw=run.stdout.strip()
try:
    value=json.loads(raw)
    if isinstance(value,str):value=json.loads(value)
except json.JSONDecodeError as exc:
    raise SystemExit(f'Could not decode read-only page result: {exc}\n{raw}')
Path(output).write_text(json.dumps(value,indent=2)+'\n')
print(json.dumps({'url':value.get('url'),'databases':value.get('databaseNames'),'projects':[(x['name'],x['projectId'],x['revision']) for x in value.get('projects',[])]},indent=2))
