import subprocess,json,sys
from pathlib import Path
session,output=sys.argv[1:3]
js='''JSON.stringify(Array.from(document.querySelectorAll('.m1-keymap-binding-editor select[aria-label$=" behavior"]')).map(e=>({label:e.getAttribute('aria-label'),value:e.value,valueAttribute:e.getAttribute('value'),selected:Array.from(e.selectedOptions).map(o=>o.value),fields:Array.from(e.closest('.m1-keymap-binding-editor').querySelectorAll('input')).map(i=>({label:i.getAttribute('aria-label'),value:i.value}))})))'''
r=subprocess.run(['agent-browser','--session',session,'eval',js],capture_output=True,text=True,check=True);data=json.loads(json.loads(r.stdout));Path(output).write_text(json.dumps(data,indent=2)+'\n');print(json.dumps(data))
assert len(data)>=2,'Expected fresh Sofle rotation editors'
assert all(row['value']=='none' and row['selected']==['none'] and not row['fields'] for row in data),'Fresh accepted unassigned encoder must display Unassigned, not Key press'
