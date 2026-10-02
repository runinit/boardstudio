import json,subprocess,sys
session=sys.argv[1]
js="JSON.stringify({viewport:[innerWidth,innerHeight],document:[document.documentElement.scrollWidth,document.documentElement.scrollHeight],active:{tag:document.activeElement?.tagName,name:document.activeElement?.getAttribute('aria-label')||document.activeElement?.textContent?.trim().slice(0,100),isConnected:document.activeElement?.isConnected,rect:(()=>{let r=document.activeElement?.getBoundingClientRect();return r&&{x:r.x,y:r.y,w:r.width,h:r.height}})()},panels:[...document.querySelectorAll('[aria-label=\"Panel visibility\"] button')].map(e=>({name:e.textContent.trim(),expanded:e.getAttribute('aria-expanded')})),hit:(()=>{let r=document.activeElement?.getBoundingClientRect();let h=r&&document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return h&&{tag:h.tagName,name:h.getAttribute('aria-label')||h.textContent?.trim().slice(0,100)}})(),canvas:[...document.querySelectorAll('canvas')].map(e=>{let r=e.getBoundingClientRect();return{x:r.x,y:r.y,w:r.width,h:r.height}}),previewReady:document.body.innerText.includes('Exact case geometry ready.')})"
def run(*args):return subprocess.run(['agent-browser','--session',session,*args],text=True,capture_output=True,check=True).stdout.strip()
def observe(label): return {'step':label,'state':json.loads(json.loads(run('eval',js)))}
run('focus','@e44')
states=[observe('Objects tree row focused at 390x640')]
run('set','viewport','390','844')
states.append(observe('retained focus after resize to 390x844'))
run('set','viewport','1280','577')
states.append(observe('retained focus after resize to 1280x577'))
open('.scratch/dioxus-case-workspace/evidence/compact-focus-resize-final-public/retained-focus-resize-states.json','w').write(json.dumps(states,indent=2)+'\n')
print(json.dumps(states,indent=2))
