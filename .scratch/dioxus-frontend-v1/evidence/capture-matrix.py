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
run('set','viewport','1440','900');run('select','select[aria-label="Theme"]','light');wait('document.documentElement.dataset.theme==="light"');run('screenshot',str(out/'desktop-light.png'))
(out/'axe-desktop-light.json').write_text(json.dumps(run('a11y'),indent=2))
run('select','select[aria-label="Theme"]','dark');wait('document.documentElement.dataset.theme==="dark"');run('screenshot',str(out/'desktop-dark.png'))
(out/'axe-desktop-dark.json').write_text(json.dumps(run('a11y'),indent=2))
run('set','viewport','390','844');run('screenshot',str(out/'compact-dark.png'))
run('click','.m1-project-menu summary')
r=probe('compact-menu','({selectorWidth:document.querySelector(".m1-workspace-select").getBoundingClientRect().width,viewport:innerWidth,pageWidth:document.documentElement.scrollWidth,menuOpen:document.querySelector(".m1-project-menu").open})');assert r['selectorWidth']>=90 and r['viewport']==r['pageWidth'] and r['menuOpen'],r
run('press','Escape');wait('!document.querySelector(".m1-project-menu").open')
run('click','button[aria-controls="m1-objects-panel"]');run('click','#m1-object-0');run('click','button[aria-controls="m1-inspector-panel"]');run('scrollintoview','#m1-position-x')
probe('compact-panels','({objectsExpanded:document.querySelector("button[aria-controls=m1-objects-panel]").getAttribute("aria-expanded"),inspectExpanded:document.querySelector("button[aria-controls=m1-inspector-panel]").getAttribute("aria-expanded"),inputVisible:document.querySelector("#m1-position-x").getBoundingClientRect().height>0,objectName:document.querySelector("#m1-object-0").getAttribute("aria-label")})')
run('scroll','up','2000');run('screenshot',str(out/'compact-panels-dark.png'),'--full')
run('click','button[aria-controls="m1-objects-panel"]');run('click','button[aria-controls="m1-inspector-panel"]');run('select','select[aria-label="Theme"]','light');wait('document.documentElement.dataset.theme==="light"');run('screenshot',str(out/'compact-light.png'))
(out/'axe-compact-light.json').write_text(json.dumps(run('a11y'),indent=2))
probe('compact-final','({viewport:innerWidth,pageWidth:document.documentElement.scrollWidth,selectorWidth:document.querySelector(".m1-workspace-select").getBoundingClientRect().width,headings:document.querySelectorAll("h1").length,mainCount:document.querySelectorAll("main").length,retryButtons:[...document.querySelectorAll("button")].filter(x=>x.textContent.includes("Retry save")).length})')
run('set','viewport','920','500');run('screenshot',str(out/'short-desktop.png'));probe('short-desktop','({width:innerWidth,pageWidth:document.documentElement.scrollWidth,height:innerHeight,pageHeight:document.documentElement.scrollHeight})')
run('set','viewport','1440','900');run('select','select[aria-label="Theme"]','dark')
print('PASS capture matrix and compact controls')
