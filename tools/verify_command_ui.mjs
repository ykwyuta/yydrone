// Exercise the actual WASM/canvas UI in an isolated, installed headless Chromium.
// Usage: node tools/verify_command_ui.mjs [path/to/chrome.exe] [--tutorial | --proposals]
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { readFile, writeFile, mkdir, readdir, access } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
let browserPath = process.argv[2];
if (!browserPath) {
  const cache = path.join(process.env.LOCALAPPDATA, 'ms-playwright');
  const versions = (await readdir(cache)).filter(n => /^chromium-\d+$/.test(n)).sort((a,b) => Number(b.split('-')[1])-Number(a.split('-')[1]));
  for (const version of versions) {
    const candidate = path.join(cache,version,'chrome-win64','chrome.exe');
    try { await access(candidate); browserPath=candidate; break; } catch {}
  }
}
if (!browserPath) throw new Error('An installed Chromium executable is required');
const profile = path.join(root,'target',`command-ui-qa-${process.pid}`);
await mkdir(profile,{recursive:true});
const dist = path.join(root,'dist');
const server = createServer(async (req,res) => {
  try {
    const pathname = decodeURIComponent(new URL(req.url,'http://localhost').pathname);
    const file = path.resolve(dist,'.'+(pathname==='/'?'/index.html':pathname));
    if (!file.startsWith(dist+path.sep)) {res.writeHead(403).end();return;}
    const mime={'.html':'text/html; charset=utf-8','.js':'text/javascript; charset=utf-8','.wasm':'application/wasm','.png':'image/png','.webmanifest':'application/manifest+json'};
    res.writeHead(200,{'Content-Type':mime[path.extname(file)]||'application/octet-stream','Cache-Control':'no-store'});res.end(await readFile(file));
  } catch {res.writeHead(404).end();}
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const origin=`http://127.0.0.1:${server.address().port}`;
const browser=spawn(browserPath,['--headless=new','--remote-debugging-port=0',`--user-data-dir=${profile}`,'--no-first-run','--no-default-browser-check','--disable-background-networking','--disable-extensions','--use-angle=swiftshader','about:blank'],{windowsHide:true,stdio:['ignore','ignore','pipe']});
let browserLog='';browser.stderr.on('data',chunk=>{browserLog=(browserLog+chunk.toString()).slice(-10000);});
let socket;
try {
  let port;
  for (let i=0;i<100;i++){try{port=Number((await readFile(path.join(profile,'DevToolsActivePort'),'utf8')).split('\n')[0]);break;}catch{await sleep(100);}}
  if (!port) throw new Error('Chromium did not expose its local debugging port: '+browserLog);
  const targets=await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  const target=targets.find(t=>t.type==='page');if(!target)throw new Error('Missing isolated browser page');
  socket=new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  let seq=0;const pending=new Map();const exceptions=[];
  socket.addEventListener('message',event=>{const msg=JSON.parse(event.data);if(msg.id&&pending.has(msg.id)){const {resolve,reject,timer}=pending.get(msg.id);clearTimeout(timer);pending.delete(msg.id);msg.error?reject(new Error(JSON.stringify(msg.error))):resolve(msg.result);}if(msg.method==='Runtime.exceptionThrown')exceptions.push(msg.params.exceptionDetails);});
  const send=(method,params={})=>new Promise((resolve,reject)=>{const id=++seq;const timer=setTimeout(()=>{pending.delete(id);reject(new Error('CDP timeout: '+method));},20000);pending.set(id,{resolve,reject,timer});socket.send(JSON.stringify({id,method,params}));});
  const evaluate=async expression=>(await send('Runtime.evaluate',{expression,returnByValue:true})).result.value;
  const waitFor=async(expression,label,seconds=20)=>{for(let i=0;i<seconds*10;i++){if(await evaluate(expression))return;await sleep(100);}throw new Error('UI timeout: '+label);};
  await send('Runtime.enable');await send('Page.enable');
  await send('Page.addScriptToEvaluateOnNewDocument',{source:`
    window.__commandTexts=[];
    const clear=CanvasRenderingContext2D.prototype.clearRect;
    CanvasRenderingContext2D.prototype.clearRect=function(...args){if(this.canvas.id==='ui')window.__commandTexts=[];return clear.apply(this,args);};
    const fill=CanvasRenderingContext2D.prototype.fillText;
    CanvasRenderingContext2D.prototype.fillText=function(text,x,y,...rest){if(this.canvas.id==='ui'){const t=this.getTransform();window.__commandTexts.push({text:String(text),x:t.a*x+t.c*y+t.e,y:t.b*x+t.d*y+t.f});}return fill.call(this,text,x,y,...rest);};
  `});
  const key=async(name,code,vk)=>{await send('Input.dispatchKeyEvent',{type:'keyDown',key:name,code,windowsVirtualKeyCode:vk,nativeVirtualKeyCode:vk});await send('Input.dispatchKeyEvent',{type:'keyUp',key:name,code,windowsVirtualKeyCode:vk,nativeVirtualKeyCode:vk});await sleep(180);};
  const click=async(x,y)=>{await send('Input.dispatchMouseEvent',{type:'mouseMoved',x,y});await send('Input.dispatchMouseEvent',{type:'mousePressed',x,y,button:'left',clickCount:1});await send('Input.dispatchMouseEvent',{type:'mouseReleased',x,y,button:'left',clickCount:1});await sleep(180);};
  const clickText=async text=>{await waitFor(`window.__commandTexts.some(t=>t.text===${JSON.stringify(text)})`,text);const item=await evaluate(`window.__commandTexts.find(t=>t.text===${JSON.stringify(text)})`);await click(item.x+3,item.y+7);};
  const screenshot=async name=>{const data=await send('Page.captureScreenshot',{format:'png'});await writeFile(path.join(root,'docs',`command-ui-${name}.png`),Buffer.from(data.data,'base64'));};
  const start=async(compact,width,height)=>{await send('Emulation.setDeviceMetricsOverride',{width,height,deviceScaleFactor:1,mobile:false});await send('Page.navigate',{url:origin+'/?compact='+Number(compact)});await waitFor(`document.getElementById('loading')?.classList.contains('hidden')`,'WASM load');await key('Enter','Enter',13);await key(' ','Space',32);await key('F3','F3',114);await waitFor(`window.__commandTexts.some(t=>t.text.includes('司令部 —'))`,'headquarters');};
  if (process.argv.includes('--tutorial')) {
    const has = value => `window.__commandTexts.some(t=>t.text.includes(${JSON.stringify(value)}))`;
    const shot = async name => {const data=await send('Page.captureScreenshot',{format:'png'});await writeFile(path.join(root,'docs',`tutorial-${name}.png`),Buffer.from(data.data,'base64'));};
    const right = async (x,y) => {await send('Input.dispatchMouseEvent',{type:'mouseMoved',x,y});await send('Input.dispatchMouseEvent',{type:'mousePressed',x,y,button:'right',clickCount:1});await send('Input.dispatchMouseEvent',{type:'mouseReleased',x,y,button:'right',clickCount:1});await sleep(200);};
    const fast = async () => {for(let i=0;i<3;i++)await key('+','Equal',187);};
    const done = async () => {console.log('Waiting:',await evaluate(`window.__commandTexts.filter(t=>t.text.startsWith('訓練 ')).map(t=>t.text)`));await waitFor(has('達成！'), 'tutorial goal',35);await clickText('次へ');};
    const training = async (compact,width,height) => {await send('Emulation.setDeviceMetricsOverride',{width,height,deviceScaleFactor:1,mobile:false});await send('Page.navigate',{url:origin+'/?compact='+Number(compact)});await waitFor(`document.getElementById('loading')?.classList.contains('hidden')`,'WASM load');await clickText('チュートリアル');await waitFor(has('訓練へようこそ'),'tutorial welcome');};
    await training(false,1600,900);await key('F5','F5',116);await waitFor(has('訓練は保存されません'),'training save guard');await clickText('訓練へ戻る');await shot('welcome');await clickText('訓練を始める');
    await send('Input.dispatchKeyEvent',{type:'keyDown',key:'ArrowRight',code:'ArrowRight',windowsVirtualKeyCode:39});await sleep(300);await send('Input.dispatchKeyEvent',{type:'keyUp',key:'ArrowRight',code:'ArrowRight',windowsVirtualKeyCode:39});
    await send('Input.dispatchMouseEvent',{type:'mouseWheel',x:900,y:350,deltaX:0,deltaY:-120});await done();
    const z=Math.min(1600/90,(900-44-190)/42), wx=x=>800+(x-108)*z, wy=y=>450+(y-94)*z;
    await click(wx(90),wy(100));await shot('selection');await done();
    await click(wx(90),wy(100));await right(wx(98),wy(100));await fast();await done();
    await click(wx(98),wy(100));await right(wx(110),wy(100));await fast();await done();
    await click(wx(112),wy(96));await right(wx(120),wy(100));await fast();await done();
    await clickText('歩兵');await fast();await done();
    await clickText('建設');await clickText('補給所');await click(wx(94),wy(106));await fast();await done();
    await clickText('ドローン');await clickText('偵察機');await fast();await done();
    await click(wx(88),wy(106));await right(wx(126),wy(86));await fast();await done();
    await clickText('FPV班');await fast();await sleep(3500);
    // Drag-select the newly produced team around the military base, then move it forward.
    await send('Input.dispatchMouseEvent',{type:'mousePressed',x:wx(92),y:wy(90),button:'left',clickCount:1});await send('Input.dispatchMouseEvent',{type:'mouseMoved',x:wx(100),y:wy(98),button:'left',buttons:1});await send('Input.dispatchMouseEvent',{type:'mouseReleased',x:wx(100),y:wy(98),button:'left',clickCount:1});await sleep(200);
    await right(wx(108),wy(104));await done();
    await click(wx(110),wy(100));await key('F3','F3',114);await clickText('全力防御');await clickText('予算 +100');await clickText('地域と部隊');await clickText('地図で中心を指定');await click(wx(108),wy(94));await clickText('選択 1 部隊を委任');await clickText('閉じる');await done();
    await key('F3','F3',114);await clickText('参謀本部');await shot('staff');await clickText('閉じる');await done();
    await fast();await sleep(2200);await key('F3','F3',114);await clickText('参謀本部');await clickText('戦線レポート');await shot('report');await clickText('閉じる');await done();
    await waitFor(has('訓練終了'),'tutorial completion');await shot('complete');
    await clickText('通常作戦へ');await waitFor(has('作戦開始'),'normal briefing');
    console.log('Tutorial: all 13 actual goals, completion and normal campaign transition passed');
    await training(true,1000,480);await shot('compact-welcome');await clickText('訓練を始める');await clickText('スキップ');await shot('compact-selection');
    await clickText('目標へ');await click(650,292);await waitFor(has('達成！'),'compact selection');await clickText('やり直す');
    if(await evaluate(has('達成！')))throw new Error('Retry failed to reset completed goal');
    for(let i=0;i<12;i++)await clickText('スキップ');await shot('compact-complete');await clickText('タイトルへ');await waitFor(has('チュートリアル'),'return title');
    console.log('Compact tutorial: selection, retry, skip, completion and exit passed');
  } else if (process.argv.includes('--proposals')) {
    const has = value => `window.__commandTexts.some(t=>t.text.includes(${JSON.stringify(value)}))`;
    const prepare = async (compact,width,height) => {
      await start(compact,width,height);await key('F3','F3',114);for(let i=0;i<3;i++)await key('+','Equal',187);
      await waitFor(has('人から提案'),'proactive proposals');await key('F3','F3',114);await clickText('将軍からの提案');
      await waitFor(has('追加予算'),'budget and theater request');
    };
    const appointAll = async () => {
      const names=['ヴェレス / 突破重視','ミラ / 兵力温存','オリク / 均衡調整','ルネ / 機甲集中','エダ / 火力優先','セイラ / 偵察重視','マルク / 守備重視','ノヴァ / 機動襲撃','イリス / 補給重視'];
      for(let gi=0;gi<9;gi++) {
        if(gi>0&&gi%3===0)await clickText('次の3人');
        await clickText(names[gi]);await clickText('戦地を承認');
      }
    };
    await prepare(false,1600,900);await screenshot('proposals');
    await clickText('地図で見る');await waitFor(has('破線は未承認'),'proposal preview');await screenshot('proposal-preview');await clickText('破線は未承認・提案へ戻る');
    await clickText('予算を承認');await waitFor(has('追加予算 0'),'budget accepted');
    await appointAll();await clickText('閉じる');await waitFor(has('9 イリス'),'all nine assigned');await screenshot('territories');
    await clickText('担当領域 ON');await waitFor(has('担当領域 OFF'),'hide territory overlay');
    if(await evaluate(has('9 イリス')))throw new Error('Hidden regions still expose legend');
    await clickText('担当領域 OFF');const firstLegend=await evaluate(`window.__commandTexts.find(t=>t.text==='1 ヴェレス')`);if(!firstLegend)throw new Error('Missing first general in legend');await click(firstLegend.x+4,firstLegend.y+7);await waitFor(has('担当地域：'),'legend opens selected general');
    console.log('Desktop: proactive requests, preview, budget/theater approval, nine regions, legend and visibility toggle passed');
    await prepare(true,1000,480);await screenshot('compact-proposals');
    await appointAll();await clickText('閉じる');await waitFor(has('担当 9人'),'compact nine territories');await screenshot('compact-territories');await clickText('次の担当');await clickText('次の担当');await waitFor(has('9 イリス'),'compact final portrait page');await screenshot('compact-territories-last');
    await clickText('担当領域 ON');await clickText('担当領域 OFF');
    console.log('Compact: proposal controls and all nine labeled territories passed');
  } else {
  await start(false,1600,900);await screenshot('generals');
  await clickText('次の3人');await clickText('次の3人');await clickText('イリス / 補給重視');
  await clickText('予算 +100');
  await waitFor(`window.__commandTexts.some(t=>t.text.includes('予算 100'))`,'allocated budget');
  await clickText('地域と部隊');await clickText('地図で中心を指定');await click(800,340);
  await waitFor(`window.__commandTexts.some(t=>t.text.includes('担当地域：'))`,'region selected');await screenshot('region');
  await clickText('参謀本部');await screenshot('staff-plan');
  await clickText('戦線レポート');await screenshot('fronts');
  await key('F3','F3',114);for(let i=0;i<3;i++)await key('+','Equal',187);
  await waitFor(`window.__commandTexts.some(t=>t.text.includes('直近戦果'))`,'periodic report',35);
  await key('F3','F3',114);await screenshot('periodic-report');
  console.log('Desktop: headquarters, pagination, budget, region, staff plan and periodic report passed');
  await start(true,1000,480);await screenshot('compact-generals');
  await clickText('地域と部隊');await screenshot('compact-region');
  await clickText('参謀本部');await screenshot('compact-staff');
  await clickText('戦線レポート');await screenshot('compact-fronts');
  }
  if(exceptions.length)throw new Error('Browser runtime exceptions: '+JSON.stringify(exceptions));
  console.log('Browser UI: no runtime exceptions');
  await send('Browser.close').catch(()=>{});
} catch(error) {
  if(socket?.readyState===1) {
    socket.send(JSON.stringify({id:999999,method:'Page.captureScreenshot',params:{format:'png'}}));
    await new Promise(resolve=>{const timer=setTimeout(resolve,1000);socket.addEventListener('message',async event=>{const m=JSON.parse(event.data);if(m.id===999999&&m.result?.data){await writeFile(path.join(root,'docs','tutorial-failure.png'),Buffer.from(m.result.data,'base64'));clearTimeout(timer);resolve();}});});
  }
  throw error;
} finally {
  socket?.close();browser.kill();server.close();
}
