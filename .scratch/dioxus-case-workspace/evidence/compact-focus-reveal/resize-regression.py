import subprocess,json,sys
from pathlib import Path
session=sys.argv[1]; output=Path(sys.argv[2])
def run(*args):
 r=subprocess.run(['agent-browser','--session',session,*args],capture_output=True,text=True,timeout=30);assert r.returncode==0,r.stderr;return r.stdout
js='''JSON.stringify((()=>{const e=document.activeElement,r=e.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return{viewport:[innerWidth,innerHeight],tag:e.tagName,name:e.getAttribute('aria-label')||e.textContent,workspace:!!e.closest('.m1-workspace-content'),occluded:!!hit&&hit!==e&&!e.contains(hit),hit:hit?{tag:hit.tagName,cls:hit.className,text:hit.textContent.slice(0,120)}:null,panels:Array.from(document.querySelectorAll('[aria-label="Panel visibility"] button')).map(b=>[b.textContent,b.getAttribute('aria-expanded')]),rect:{x:r.x,y:r.y,width:r.width,height:r.height}}})())'''
def read():return json.loads(json.loads(run('eval',js)))
before=read();assert before['viewport'][0]>760 and before['workspace'] and not before['occluded'],before
assert any(v=='true' for _,v in before['panels']), 'Remembered open drawer required'
run('set','viewport','390','640')
after=read();output.write_text(json.dumps({'before':before,'after':after},indent=2)+'\n')
print(json.dumps(after));assert after['workspace'] and after['name']==before['name'],'Workspace focus lost on resize'
assert not after['occluded'],'Resize leaves the retained workspace focus behind compact drawers'
assert all(v=='false' for _,v in after['panels']),'Focused workspace did not close compact drawers on resize'
