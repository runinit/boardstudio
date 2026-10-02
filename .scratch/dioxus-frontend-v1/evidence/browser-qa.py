import json,subprocess,sys,pathlib
session=sys.argv[1];url=sys.argv[2];out=pathlib.Path(sys.argv[3]);out.mkdir(parents=True,exist_ok=True)
steps=[]
def run(*args):
 p=subprocess.run(['agent-browser','--session',session,*args,'--json'],text=True,capture_output=True)
 try:r=json.loads(p.stdout)
 except ValueError:r={'raw':p.stdout,'error':p.stderr}
 steps.append({'args':list(args),'exit':p.returncode,'result':r});(out/'browser-steps.json').write_text(json.dumps(steps,indent=2))
 if p.returncode or not r.get('success',True):raise RuntimeError(str(steps[-1]))
 return r.get('data',r)
def probe(name,expr):
 r=run('eval',f'JSON.stringify({expr})');v=r.get('result',r)
 if isinstance(v,str):v=json.loads(v)
 (out/(name+'.json')).write_text(json.dumps(v,indent=2));return v
def wait(expr):run('wait','--fn',expr)
run('open',url);run('set','viewport','1440','900');run('wait','.m1-library')
run('click','.m1-library-landing .m1-library button:first-child');run('wait','.m1-canvas')
wait('document.querySelector(".m1-save-state").textContent.includes("Saved")')
r=probe('shell','({tabs:[...document.querySelectorAll("[role=tab]")].map(x=>x.textContent.trim()),heading:document.querySelector("h1")?.textContent,menuOpen:document.querySelector(".m1-project-menu").open,mainCount:document.querySelectorAll("main").length,title:document.title})')
assert r['tabs']==['Layout','PCB','Keymap','Keycaps','Case','Parts'] and r['heading'] and r['mainCount']==1 and not r['menuOpen'],r
# Theme preference, live system updates and independence from workspace state.
run('select','select[aria-label="Theme"]','dark');wait('document.documentElement.dataset.theme === "dark"')
run('set','media','light');wait('document.documentElement.dataset.theme === "dark"')
run('select','select[aria-label="Theme"]','system');wait('document.documentElement.dataset.theme === "light"')
run('set','media','dark');wait('document.documentElement.dataset.theme === "dark"')
run('select','select[aria-label="Theme"]','light');wait('document.documentElement.dataset.theme === "light"')
run('reload');run('wait','.m1-canvas');wait('document.documentElement.dataset.theme === "light"')
r=probe('theme-persistence','({preference:localStorage.getItem("boardstudio:v2:theme"),effective:document.documentElement.dataset.theme,workspace:document.querySelector("[role=tab][aria-selected=true]").textContent.trim()})');assert r['preference']=='light' and r['workspace']=='Layout',r
# Keyboard tabs and placeholders.
run('focus','#m1-tab-Layout');run('press','ArrowRight');wait('document.querySelector("#m1-tab-PCB").getAttribute("aria-selected")==="true"')
run('press','End');wait('document.activeElement.id==="m1-tab-Parts"')
run('press','Home');wait('document.activeElement.id==="m1-tab-Layout"')
for name in ['PCB','Keymap','Keycaps','Parts']:
 run('click','#m1-tab-'+name);run('wait','.m1-placeholder-workspace');run('click','.m1-placeholder-workspace button');run('wait','.m1-canvas')
# Numeric preview is cancelled by workspace unmount; committed edit and history survive.
run('click','#m1-object-0');run('wait','#m1-position-x')
base=probe('position-before','({x:document.querySelector("#m1-position-x").value,y:document.querySelector("#m1-position-y").value})')['x']
changed=str(float(base)+3)
run('fill','#m1-position-x',changed);run('click','#m1-tab-PCB');run('click','#m1-tab-Layout');run('wait','#m1-position-x')
wait('Number(document.querySelector("#m1-position-x").value)==='+str(float(base)))
run('fill','#m1-position-x',changed);run('press','Enter');wait('document.querySelector(".m1-save-state").textContent.includes("Saved")')
run('click','#m1-tab-Keycaps');run('click','#m1-tab-Layout');wait('Number(document.querySelector("#m1-position-x").value)==='+changed)
run('click','button[aria-label="Undo"]');wait('Number(document.querySelector("#m1-position-x").value)==='+str(float(base)))
probe('position-after-undo','({x:document.querySelector("#m1-position-x").value,save:document.querySelector(".m1-save-state").textContent})')
# Project menu selection must close it.
run('click','.m1-project-menu summary');run('click','.m1-project-menu .m1-library button:first-child');run('wait','.m1-canvas');wait('!document.querySelector(".m1-project-menu").open')
print('PASS shell, themes, live OS theme, reload, keyboard, placeholders, numeric cancellation/history, menu dismissal')
