from pathlib import Path
import subprocess, socket, json, time
p = Path(__file__).resolve().parent
session = "boardstudio-p1-large-integer-20261001"
sock=socket.socket();sock.bind(("127.0.0.1",0));port=sock.getsockname()[1];sock.close()
log=(p/"large-integer-runtime-server.log").open("w")
server=subprocess.Popen(["python3","-m","http.server",str(port),"--bind","127.0.0.1","--directory",str(p/"site")],stdout=log,stderr=subprocess.STDOUT)
commands=[]
def browser(args):
    cmd=["agent-browser","--session",session,"--executable-path","/usr/bin/chromium","--json",*args]
    r=subprocess.run(cmd,capture_output=True,text=True,timeout=25)
    commands.append({"command":cmd,"exit_code":r.returncode,"stdout":r.stdout,"stderr":r.stderr})
    return r
input_line=json.loads((p/"large-integer-oracle-input.jsonl").read_text().splitlines()[0])
script="""new Promise(resolve => {
 const request = INPUT;
 request.document.revision = 9007199254740993n;
 const worker = new Worker('/worker/worker-entry.js', {type:'module'});
 let posted = false;
 const finish = result => { clearTimeout(timer); worker.terminate(); resolve(result); };
 const timer = setTimeout(() => finish({outcome:'deadline'}), 15000);
 worker.onerror = event => finish({outcome:'worker-error', message:event.message, request_posted:posted});
 worker.onmessage = event => {
  if(event.data === 'ready') { posted=true; worker.postMessage({frame:{id:{epoch:9,operation:1}, action:{Core:request}}}); }
  else finish({outcome:'reply',kind:event.data?.frame?.payload?.Core?.kind});
 };
})""".replace("INPUT",json.dumps(input_line))
result={"candidate":"3f25e9d9a6679d551acd6e9439e2f19ca844e0ee","expected":"valid u64 Open must return Scene preserving provider contract","status":"unverified"}
try:
 time.sleep(.5)
 assert browser(["open",f"http://127.0.0.1:{port}/"]).returncode==0
 r=browser(["eval",script]);assert r.returncode==0,r.stderr
 observed=json.loads(r.stdout).get("data",{}).get("result");result["observed"]=observed
 assert observed["outcome"]=="worker-error" and observed["request_posted"] and "9007199254740993 can't be represented" in observed["message"],observed
 result["status"]="confirmed-contract-regression"
finally:
 try: browser(["close"])
 finally: server.terminate();server.wait(timeout=10);log.close()
 result["commands"]=commands
 (p/"large-integer-runtime.json").write_text(json.dumps(result,indent=2)+"\n")
print(json.dumps({k:v for k,v in result.items() if k!="commands"},indent=2))
