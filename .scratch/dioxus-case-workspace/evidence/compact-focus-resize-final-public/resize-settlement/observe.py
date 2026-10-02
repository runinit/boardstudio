import subprocess,json,re,sys
from pathlib import Path
session,output=sys.argv[1:3]
def run(*args):
 r=subprocess.run(['agent-browser','--session',session,*args],text=True,capture_output=True,timeout=40);assert r.returncode==0,r.stderr;return r.stdout
run('set','viewport','390','640')
state=json.loads(json.loads(run('eval','JSON.stringify(Array.from(document.querySelectorAll(\'[aria-label="Panel visibility"] button\')).map(e=>[e.textContent,e.getAttribute("aria-expanded")==="true"]))')))
for name,opened in state:
 if not opened:run('find','role','button','click','--name',name,'--exact')
run('set','viewport','1280','577')
snapshot=run('snapshot','-i');ref=re.search(r'button "Generate case" \[ref=(e\d+)\]',snapshot).group(1)
run('focus','@'+ref)
js='''(()=>{
 const result={rows:[],done:false,started:performance.now(),url:location.href};window.__caseResizeObservation=result;
 const query=matchMedia('(max-width:760px)');let compactFrames=0;
 function observe(kind){const e=document.activeElement,r=e.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);result.rows.push({kind,t:performance.now(),width:innerWidth,compact:query.matches,focused:e.getAttribute('aria-label')||e.textContent,workspace:!!e.closest('.m1-workspace-content'),covered:!!hit&&hit!==e&&!e.contains(hit),expanded:Array.from(document.querySelectorAll('[aria-label="Panel visibility"] button')).map(b=>b.getAttribute('aria-expanded')),hit:hit?.className});}
 const media=()=>observe('media-change');query.addEventListener('change',media);const resize=()=>observe('resize-event');window.addEventListener('resize',resize);
 const mutation=new MutationObserver(()=>observe('panel-attribute-change'));document.querySelector('[aria-label="Panel visibility"]').querySelectorAll('button').forEach(b=>mutation.observe(b,{attributes:true,attributeFilter:['aria-expanded']}));
 function finish(){result.done=true;query.removeEventListener('change',media);window.removeEventListener('resize',resize);mutation.disconnect();}
 function frame(){if(result.done)return;observe('animation-frame');if(query.matches)compactFrames++;if(compactFrames>=30||performance.now()-result.started>5000)finish();else requestAnimationFrame(frame);}
 observe('initial-desktop');requestAnimationFrame(frame);return 'read-only resize observation installed';})()'''
run('eval',js)
run('set','viewport','390','640')
raw=run('eval',"new Promise(resolve=>{function check(){if(window.__caseResizeObservation.done){resolve(JSON.stringify(window.__caseResizeObservation));}else setTimeout(check,20)}check()})")
data=json.loads(json.loads(raw));Path(output).write_text(json.dumps(data,indent=2)+'\n')
run('eval','delete window.__caseResizeObservation')
rows=data['rows'];compact=[r for r in rows if r['compact']];first=compact[0];closed=next((r for r in compact if r['expanded']==['false','false'] and not r['covered']),None)
summary={'first_compact':first,'first_uncovered_closed':closed,'elapsed_ms':closed['t']-first['t'] if closed else None,'covered_compact_animation_frames':sum(r['kind']=='animation-frame' and r['covered'] for r in compact),'total_compact_animation_frames':sum(r['kind']=='animation-frame' for r in compact),'final':rows[-1]}
print(json.dumps(summary,indent=2))
