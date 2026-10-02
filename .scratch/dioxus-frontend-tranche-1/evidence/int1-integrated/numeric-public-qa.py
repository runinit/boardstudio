#!/usr/bin/env python3
"""Public-UI INT1 numeric Inspector behavior trace; usage: run_numeric.py SESSION URL OUTDIR"""
import json, pathlib, subprocess, sys
session, url, out_arg = sys.argv[1:4]
out = pathlib.Path(out_arg); out.mkdir(parents=True, exist_ok=True)
steps=[]
def run(*args):
    p=subprocess.run(['agent-browser','--session',session,*args,'--json'],text=True,capture_output=True)
    try: result=json.loads(p.stdout)
    except ValueError: result={'raw':p.stdout,'stderr':p.stderr}
    row={'args':list(args),'exit':p.returncode,'result':result}
    steps.append(row); (out/'steps.json').write_text(json.dumps(steps,indent=2))
    if p.returncode or (isinstance(result,dict) and result.get('success') is False):
        raise RuntimeError(json.dumps(row))
    return result.get('data',result) if isinstance(result,dict) else result
def probe(name, expr):
    value=run('eval',f'JSON.stringify({expr})')
    data=value.get('result',value) if isinstance(value,dict) else value
    if isinstance(data,str): data=json.loads(data)
    (out/(name+'.json')).write_text(json.dumps(data,indent=2)); return data
def wait(expr): run('wait','--fn',expr,'--timeout','8000')
def pos_state(part_id):
    return '({x:document.querySelector("#m1-position-x")?.value,y:document.querySelector("#m1-position-y")?.value,transform:document.querySelector('+json.dumps('[data-part-id="'+part_id+'"]')+')?.getAttribute("transform"),revision:Number(document.querySelector(".m1-editor-footer").textContent.match(/Revision (\\d+)/)?.[1]),save:document.querySelector(".m1-save-state")?.textContent,status:[...document.querySelectorAll("[role=status]")].map(e=>e.textContent)})'
def transform_expr(part_id):
    return 'document.querySelector('+json.dumps('[data-part-id="'+part_id+'"]')+')?.getAttribute("transform")'
run('open',url); run('wait','.m1-library'); run('click','.m1-library-landing .m1-library button:first-child'); run('wait','.m1-canvas')
wait('document.querySelector(".m1-save-state").textContent.includes("Saved")')
target=probe('target','({id:document.querySelector(".m1-keycap-overlay").parentElement.dataset.partId,transform:document.querySelector(".m1-keycap-overlay").parentElement.getAttribute("transform")})')
a=target['id']; selector='[data-part-id='+json.dumps(a)+']'
run('click',selector+' .m1-part-hit-area'); run('wait','#m1-position-x')
start=probe('initial',pos_state(a)); x=float(start['x']); rev=int(start['revision'])
# Escape cancels a visible preview and restores both field and selected geometry.
run('fill','#m1-position-x',str(x+3)); wait(transform_expr(a)+'!=='+json.dumps(start['transform']))
preview=probe('escape-preview',pos_state(a)); assert preview['revision']==rev and float(preview['x'])==x+3,preview
run('press','Escape'); wait('Math.abs(Number(document.querySelector("#m1-position-x").value)-'+str(x)+')<0.0001')
wait(transform_expr(a)+'==='+json.dumps(start['transform']))
probe('escape-restored',pos_state(a))
# Enter commits one history step; Undo restores the original field and geometry.
run('fill','#m1-position-x',str(x+3)); wait(transform_expr(a)+'!=='+json.dumps(start['transform']))
run('press','Enter'); wait('Number(document.querySelector(".m1-editor-footer").textContent.match(/Revision (\\d+)/)?.[1])>'+str(rev))
committed=probe('enter-committed',pos_state(a)); assert abs(float(committed['x'])-(x+3))<0.001,committed
run('click','button[aria-label="Undo"]'); wait('Math.abs(Number(document.querySelector("#m1-position-x").value)-'+str(x)+')<0.0001'); wait(transform_expr(a)+'==='+json.dumps(start['transform']))
undo1=probe('enter-undo',pos_state(a))
# Apply has the same commit / Undo path.
run('fill','#m1-position-x',str(x+3)); wait(transform_expr(a)+'!=='+json.dumps(start['transform']))
run('find','role','button','click','--name','Apply position'); wait('Number(document.querySelector(".m1-editor-footer").textContent.match(/Revision (\\d+)/)?.[1])>'+str(undo1['revision']))
apply=probe('apply-committed',pos_state(a)); assert abs(float(apply['x'])-(x+3))<0.001,apply
run('click','button[aria-label="Undo"]'); wait('Math.abs(Number(document.querySelector("#m1-position-x").value)-'+str(x)+')<0.0001'); wait(transform_expr(a)+'==='+json.dumps(start['transform']))
probe('apply-undo',pos_state(a))
# Changing the selected visible part discards the old draft and restores its preview.
b=probe('second-target','([...document.querySelectorAll(".m1-keycap-overlay")].map(e=>e.parentElement.dataset.partId).find(id=>id!=='+json.dumps(a)+'))')
bid=b if isinstance(b,str) else b
bsel='[data-part-id='+json.dumps(bid)+']'
probe('second-initial','({id:'+json.dumps(bid)+',transform:'+transform_expr(bid)+'})')
run('fill','#m1-position-x',str(x+3)); wait(transform_expr(a)+'!=='+json.dumps(start['transform']))
run('click',bsel+' .m1-part-hit-area'); run('wait','#m1-position-x'); wait(transform_expr(a)+'==='+json.dumps(start['transform']))
probe('selection-changed','({selected:[...document.querySelectorAll(".m1-part-hit-area")].find(e=>e.closest("[data-part-id]")?.classList.contains("is-selected"))?.closest("[data-part-id]")?.dataset.partId,x:document.querySelector("#m1-position-x").value,oldTransform:'+transform_expr(a)+',status:[...document.querySelectorAll("[role=status]")].map(e=>e.textContent)})')
# Return to first visible part; switching workspace with a pending draft must restore it on unmount.
run('click',selector+' .m1-part-hit-area'); run('wait','#m1-position-x')
start_again=probe('layout-return-start',pos_state(a)); xa=float(start_again['x']); ra=int(start_again['revision'])
run('fill','#m1-position-x',str(xa+3)); wait(transform_expr(a)+'!=='+json.dumps(start_again['transform']))
run('click','#m1-tab-PCB'); run('wait','#m1-tab-PCB[aria-selected="true"]')
probe('after-inspector-unmount','({x:document.querySelector("#m1-position-x"),transform:'+transform_expr(a)+',revision:Number(document.querySelector(".m1-editor-footer").textContent.match(/Revision (\\d+)/)?.[1])})')
run('click','#m1-tab-Layout'); run('wait','#m1-position-x'); wait('Math.abs(Number(document.querySelector("#m1-position-x").value)-'+str(xa)+')<0.0001'); wait(transform_expr(a)+'==='+json.dumps(start_again['transform']))
probe('layout-return-restored',pos_state(a))
# Observe, do not assume, handling of empty number draft. Escape is the cleanup action.
run('fill','#m1-position-x',''); invalid=probe('invalid-empty',pos_state(a)); run('press','Escape'); wait('Math.abs(Number(document.querySelector("#m1-position-x").value)-'+str(xa)+')<0.0001'); probe('invalid-escape',pos_state(a))
print(json.dumps({'target':a,'secondTarget':bid,'baselineRevision':rev,'resultDirectory':str(out)}))
