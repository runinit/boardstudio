import json, subprocess, sys
session = sys.argv[1]
js = "JSON.stringify({active:{tag:document.activeElement?.tagName,name:document.activeElement?.getAttribute('aria-label')||document.activeElement?.textContent?.trim(),expanded:document.activeElement?.getAttribute('aria-expanded')},panels:[...document.querySelectorAll('[aria-label=\"Panel visibility\"] button')].map(e=>({name:e.textContent.trim(),expanded:e.getAttribute('aria-expanded')})),hit:(()=>{let r=document.activeElement?.getBoundingClientRect();let h=r&&document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return h&&{tag:h.tagName,name:h.getAttribute('aria-label')||h.textContent?.trim().slice(0,100)}})()})"
def run(*args): return subprocess.run(['agent-browser','--session',session,*args],text=True,capture_output=True,check=True).stdout.strip()
def observe(label): return {'step':label,'state':json.loads(json.loads(run('eval',js)))}
results=[]
# Focus the actual public navigation buttons, then use real Enter/Space key presses.
run('focus','@e23')
run('press','Enter'); results.append(observe('Enter opens Objects'))
run('press','Enter'); results.append(observe('Enter closes Objects'))
run('focus','@e24')
run('press','Space'); results.append(observe('Space opens Inspector'))
run('press','Space'); results.append(observe('Space closes Inspector'))
open('.scratch/dioxus-case-workspace/evidence/compact-focus-resize-final-public/keyboard-toggle-states.json','w').write(json.dumps(results,indent=2)+'\n')
print(json.dumps(results,indent=2))
