import subprocess,json,sys,time
from pathlib import Path
session,tag=sys.argv[1:3]
out=Path(__file__).parent
js="""JSON.stringify({url:location.href,viewport:[innerWidth,innerHeight],document:[document.documentElement.scrollWidth,document.documentElement.scrollHeight],scroll:[scrollX,scrollY],panels:Array.from(document.querySelectorAll('[aria-label="Panel visibility"] button')).map(e=>({name:e.textContent,expanded:e.getAttribute('aria-expanded')})),elements:Array.from(document.querySelectorAll('body,body>*,main,main>*,.m1-editor,.m1-editor>*,.m1-workspace-content,.m1-case-panel,.m1-case-view,.m1-panel-slot,.m1-panel-content,footer')).map(e=>{const r=e.getBoundingClientRect(),s=getComputedStyle(e);return{tag:e.tagName,cls:e.className,role:e.getAttribute('role'),label:e.getAttribute('aria-label'),top:r.top,bottom:r.bottom,height:r.height,scrollHeight:e.scrollHeight,clientHeight:e.clientHeight,position:s.position,display:s.display,overflow:s.overflow,minHeight:s.minHeight,heightCss:s.height,gridTemplateRows:s.gridTemplateRows}})})"""
t=time.time();r=subprocess.run(['agent-browser','--session',session,'eval',js],text=True,capture_output=True,timeout=40)
assert r.returncode==0,r.stderr
data=json.loads(json.loads(r.stdout));data['observed_epoch']=t
(out/(tag+'.json')).write_text(json.dumps(data,indent=2)+'\n')
print(json.dumps({'tag':tag,'viewport':data['viewport'],'document':data['document'],'panels':data['panels']}))
if '--assert-fit' in sys.argv:assert data['document'][1]<=data['viewport'][1]+1, 'document vertically overflows viewport'
