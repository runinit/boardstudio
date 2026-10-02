import subprocess,json,sys
from pathlib import Path
session=sys.argv[1]
out=Path(sys.argv[2]); trace=[]
def run(*args):
 r=subprocess.run(['agent-browser','--session',session,*args],text=True,capture_output=True,timeout=30);assert r.returncode==0,r.stderr;return r.stdout
js='''JSON.stringify((()=>{const e=document.activeElement,r=e.getBoundingClientRect(),x=r.x+r.width/2,y=r.y+r.height/2,hit=document.elementFromPoint(x,y);return {tag:e.tagName,name:e.getAttribute('aria-label')||e.textContent,workspace:!!e.closest('.m1-workspace-content'),rect:{x:r.x,y:r.y,width:r.width,height:r.height},hit:hit?{tag:hit.tagName,cls:hit.className,text:hit.textContent.slice(0,120)}:null,occluded:!!hit&&hit!==e&&!e.contains(hit),panels:Array.from(document.querySelectorAll('[aria-label="Panel visibility"] button')).map(b=>[b.textContent,b.getAttribute('aria-expanded')])}})())'''
for i in range(45):
 run('press','Tab')
 row=json.loads(json.loads(run('eval',js)));row['tab']=i+1;trace.append(row)
 out.write_text(json.dumps(trace,indent=2)+'\n')
 if row['workspace'] and row['occluded']:
  print(json.dumps(row));raise AssertionError('Real Tab focused a Case control covered by an open drawer')
 if row['workspace']:
  assert row['tag']!='BODY' and row['name'], 'Workspace control focus was lost'
  assert all(value=='false' for _,value in row['panels']), 'Drawer remained open on workspace focus'
  retained=json.loads(json.loads(run('eval',js)))
  assert retained['workspace'] and retained['tag']==row['tag'] and retained['name']==row['name'], 'Focused child was not retained'
  assert not retained['occluded'], 'Focused child remains covered after settlement'
  print('PASS: actual workspace control reached, both drawers closed, child focus retained and visible')
  break
else:
 raise AssertionError('No actual workspace control reached in45Tab presses')
