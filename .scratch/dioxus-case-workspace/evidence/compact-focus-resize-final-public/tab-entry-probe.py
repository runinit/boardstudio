import json,subprocess,sys
session=sys.argv[1]
js="JSON.stringify({active:{tag:document.activeElement?.tagName,name:document.activeElement?.getAttribute('aria-label')||document.activeElement?.textContent?.trim().slice(0,120),tabindex:document.activeElement?.tabIndex,rect:(()=>{let r=document.activeElement?.getBoundingClientRect();return r&&{x:r.x,y:r.y,w:r.width,h:r.height}})(),workspace:!!document.activeElement?.closest('.m1-workspace-content')},panels:[...document.querySelectorAll('[aria-label=\"Panel visibility\"] button')].map(e=>({name:e.textContent.trim(),expanded:e.getAttribute('aria-expanded')})),hit:(()=>{let r=document.activeElement?.getBoundingClientRect();let h=r&&document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return h&&{tag:h.tagName,name:h.getAttribute('aria-label')||h.textContent?.trim().slice(0,100)}})()})"
def run(*args):return subprocess.run(['agent-browser','--session',session,*args],text=True,capture_output=True,check=True).stdout.strip()
def observe():return json.loads(json.loads(run('eval',js)))
run('focus','@e48')
run('press','Tab'); entered=observe()
run('press','Shift+Tab'); returned=observe()
data={'sequence':['focus last Objects row','press Tab','press Shift+Tab'],'workspace_entry':entered,'shift_tab_return':returned}
open('.scratch/dioxus-case-workspace/evidence/compact-focus-resize-final-public/tab-entry-states.json','w').write(json.dumps(data,indent=2)+'\n')
print(json.dumps(data,indent=2))
