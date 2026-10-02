import json,subprocess,sys
session=sys.argv[1]
js="JSON.stringify({url:location.href,viewport:[innerWidth,innerHeight],document:[document.documentElement.scrollWidth,document.documentElement.scrollHeight],active:{tag:document.activeElement?.tagName,name:document.activeElement?.getAttribute('aria-label')||document.activeElement?.textContent?.trim().slice(0,100),isConnected:document.activeElement?.isConnected,rect:(()=>{let r=document.activeElement?.getBoundingClientRect();return r&&{x:r.x,y:r.y,w:r.width,h:r.height}})()},panels:[...document.querySelectorAll('[aria-label=\"Panel visibility\"] button')].map(e=>({name:e.textContent.trim(),expanded:e.getAttribute('aria-expanded')})),slots:[...document.querySelectorAll('.m1-panel-slot')].map(e=>({cls:e.className,display:getComputedStyle(e).display,rect:(()=>{let r=e.getBoundingClientRect();return{x:r.x,y:r.y,w:r.width,h:r.height}})()})),hit:(()=>{let r=document.activeElement?.getBoundingClientRect();let h=r&&document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return h&&{tag:h.tagName,name:h.getAttribute('aria-label')||h.textContent?.trim().slice(0,100)}})(),previewReady:document.body.innerText.includes('Exact case geometry ready.'),canvas:[...document.querySelectorAll('canvas')].map(e=>{let r=e.getBoundingClientRect();return{x:r.x,y:r.y,w:r.width,h:r.height}})})"
def run(*args):return subprocess.run(['agent-browser','--session',session,*args],text=True,capture_output=True,check=True).stdout.strip()
def observe(label):return {'step':label,'state':json.loads(json.loads(run('eval',js)))}
# Presently on the scoped route with both drawers closed at 390x640.
run('click','@e74');run('click','@e75'); states=[observe('compact with both drawers open')]
run('set','viewport','1280','577'); states.append(observe('desktop after resize'))
run('focus','@e81'); states.append(observe('desktop Generate case focus'))
run('set','viewport','390','640'); states.append(observe('immediate compact resize observation'))
run('wait','600'); states.append(observe('compact after resize settles for 600ms'))
open('.scratch/dioxus-case-workspace/evidence/compact-focus-resize-final-public/desktop-focus-return-compact-states.json','w').write(json.dumps(states,indent=2)+'\n')
print(json.dumps(states,indent=2))
