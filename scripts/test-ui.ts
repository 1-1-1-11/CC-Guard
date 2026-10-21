/**
 * [INPUT]: Node.js、playwright-core、本机 Edge、Vite 与 tests/app.html 的契约假后端（mockIPC）。
 * [OUTPUT]: CC-Guard 界面验收：向导走完、secret 不回显、pass/block/unknown 三色、unknown 不显示绿色、启动被拦显示原因、
 *           账号切换确认、漏网置顶、只按域名提示、英文模式无中文、深色/浅色可读、只调用契约命令；每项一行 PASS/FAIL，末行汇总，失败非零退出。
 * [POS]: 前端发布前回归；截图写入系统临时目录，拦截全部外网请求，不启动应用、不碰真实账号。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {strict as assert} from 'node:assert';
import {join} from 'node:path';
import {mkdir} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {createServer} from 'vite';
import {chromium,type Page} from 'playwright-core';
import {KEPT,REMOVED} from '../src/contract.ts';
const edge='C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe';
const CONTRACT=['settings_read','settings_write','line_check','line_connections','line_nodes','launch','workspace_choose'];
const SECRET='fixture-secret-9f3a';
const shots=join(tmpdir(),'cc-guard-ui');
await mkdir(shots,{recursive:true});
const server=await createServer({logLevel:'error',server:{host:'127.0.0.1',port:0,strictPort:false,watch:{ignored:['**/src-tauri/**','**/browser-runtime/**','**/dist/**']}},optimizeDeps:{include:['@tauri-apps/api/mocks']}});
await server.listen();
const base=`http://127.0.0.1:${(server.httpServer!.address() as {port:number}).port}`;
const browser=await chromium.launch({executablePath:edge,headless:true});
const results:{name:string;ok:boolean;detail:string}[]=[];
const saved:string[]=[];
const errors:string[]=[];
let page:Page;
const fixture=<T=unknown>(code:string)=>page.evaluate(code) as Promise<T>;
const set=(code:string)=>page.evaluate(`(()=>{const s=window.appFixture.state;${code}})()`);
const shot=async(name:string)=>{const path=join(shots,`${name}.png`);await page.screenshot({path});saved.push(path);};
const bar=()=>page.locator('.status-bar');
const recheck=async(verdict:string)=>{await bar().getByRole('button',{name:/重新检查|Check again/}).click();await page.waitForFunction(v=>document.querySelector('.status-bar')?.getAttribute('data-verdict')===v&&!document.querySelector('.status-bar [data-loading]'),verdict);};
const nav=(name:string|RegExp)=>page.getByRole('navigation').or(page.locator('aside')).getByRole('button',{name}).first().click();
// ----- 用 canvas 把任意 CSS 颜色（含 oklab / color-mix）换算成 sRGB，判断是否偏绿 -----
async function colors(selector:string):Promise<number[][]>{
  return page.evaluate(selector=>{
    const canvas=document.createElement('canvas');canvas.width=canvas.height=1;
    const ctx=canvas.getContext('2d',{willReadFrequently:true})!;
    const rgba=(value:string)=>{ctx.clearRect(0,0,1,1);ctx.fillStyle='#000';ctx.fillStyle=value;ctx.fillRect(0,0,1,1);return [...ctx.getImageData(0,0,1,1).data];};
    const out:number[][]=[];
    for(const root of document.querySelectorAll(selector))for(const node of [root,...root.querySelectorAll('*')]){
      const style=getComputedStyle(node);
      for(const value of [style.color,style.backgroundColor,style.borderTopColor,style.fill,style.stroke])if(value&&value!=='none'&&!value.startsWith('url')){const c=rgba(value);if(c[3]>12)out.push(c);}
    }
    return out;
  },selector);
}
const greenish=([r,g,b]:number[])=>g>r+30&&g>b+20&&g>90;
const reddish=([r,g,b]:number[])=>r>g+60&&r>b+60;
async function contrast(selector:string):Promise<number>{
  return page.evaluate(selector=>{
    const canvas=document.createElement('canvas');canvas.width=canvas.height=1;
    const ctx=canvas.getContext('2d',{willReadFrequently:true})!;
    const rgb=(value:string)=>{ctx.clearRect(0,0,1,1);ctx.fillStyle=value;ctx.fillRect(0,0,1,1);return [...ctx.getImageData(0,0,1,1).data].slice(0,3);};
    const lum=(c:number[])=>{const [r,g,b]=c.map(v=>{v/=255;return v<=0.03928?v/12.92:((v+0.055)/1.055)**2.4;});return 0.2126*r+0.7152*g+0.0722*b;};
    const node=document.querySelector(selector)!;let bg=node as Element|null,back='rgba(0, 0, 0, 0)';
    while(bg&&(back=getComputedStyle(bg).backgroundColor)==='rgba(0, 0, 0, 0)')bg=bg.parentElement;
    const a=lum(rgb(getComputedStyle(node).color)),b=lum(rgb(back));
    return (Math.max(a,b)+0.05)/(Math.min(a,b)+0.05);
  },selector);
}
async function step(name:string,body:()=>Promise<void>){
  try{await body();results.push({name,ok:true,detail:''});console.log(`PASS ${name}`);}
  catch(e){const detail=e instanceof Error?e.message.split('\n')[0]:String(e);results.push({name,ok:false,detail});console.log(`FAIL ${name}: ${detail}`);await shot(`fail-${results.length}`).catch(()=>{});}
}
try{
  const context=await browser.newContext({viewport:{width:1040,height:820},colorScheme:'light'});
  await context.route('**/*',route=>route.request().url().startsWith(base+'/')?route.continue():route.abort());
  page=await context.newPage();page.setDefaultTimeout(8000);
  page.on('pageerror',error=>errors.push(error.message));
  page.on('console',message=>{if(message.type()==='error')errors.push(message.text());});
  await page.goto(base+'/tests/app.html');

  await step('首次打开弹出三步向导并走完，最后看到绿灯',async()=>{
    const wizard=page.getByRole('dialog',{name:/三步配好/});await wizard.waitFor();
    assert.equal(await bar().getAttribute('data-verdict'),'unknown');
    await wizard.getByLabel('Clash 控制地址').fill('127.0.0.1:9097');
    await wizard.getByLabel('控制密钥').fill(SECRET);
    await wizard.getByRole('button',{name:'连接并下一步'}).click();
    await wizard.getByLabel('Claude 专用端口').waitFor();
    await wizard.getByLabel('Claude 专用端口').fill('7899');
    await wizard.getByRole('combobox',{name:'指定节点'}).click();
    await page.getByRole('option',{name:'JP-HY2'}).click();
    await wizard.getByRole('button',{name:'保存并检查'}).click();
    await wizard.getByText('出口 IP 203.0.113.7').waitFor();
    assert.equal(await wizard.locator('[data-verdict]').getAttribute('data-verdict'),'unknown');
    await wizard.getByRole('button',{name:'记住这个出口'}).click();
    await wizard.getByText(/配好了/).waitFor();
    await shot('wizard-done');
    await wizard.getByRole('button',{name:'完成'}).click();
    await wizard.waitFor({state:'detached'});
    const line=await fixture<any>('window.appFixture.state.settings.line');
    assert.deepEqual([line.controller,line.port,line.node,line.expected_ip,line.secret_set],['127.0.0.1:9097',7899,'JP-HY2','203.0.113.7',true]);
    assert.equal(await bar().getAttribute('data-verdict'),'pass');
    assert.ok((await colors('.status-bar')).some(greenish),'pass 状态条应为绿色');
  });

  await step('secret 只写不回显（输入框、页面、localStorage）',async()=>{
    await nav('设置');
    const input=page.getByLabel('控制密钥');await input.waitFor();
    assert.equal(await input.inputValue(),'');
    await input.fill('second-secret-77');
    await page.getByRole('button',{name:'保存线路设置'}).click();
    await page.locator('[data-tone="info"]').filter({hasText:'线路设置已保存，正在重新检查。'}).waitFor();
    assert.equal(await input.inputValue(),'');
    assert.equal(await fixture('window.appFixture.state.secret'),'second-secret-77');
    const dump=await page.evaluate(()=>document.documentElement.outerHTML+[...document.querySelectorAll('input')].map(i=>i.value).join('|')+JSON.stringify({...localStorage}));
    for(const value of [SECRET,'second-secret-77'])assert.ok(!dump.includes(value),'secret 出现在页面或 localStorage');
    await nav('账号');
  });

  await step('pass 绿、block 红、unknown 灰',async()=>{
    await set("s.force='pass'");await recheck('pass');
    assert.ok((await colors('.status-bar')).some(greenish),'pass 无绿色');await shot('status-pass');
    await set("s.force='block'");await recheck('block');
    const blocked=await colors('.status-bar');
    assert.ok(blocked.some(reddish),'block 无红色');assert.ok(!blocked.some(greenish),'block 出现绿色');
    await bar().getByText(/与期望出口 203.0.113.7 不符/).waitFor();
    await bar().getByText(/出口地区 JP/).waitFor();await bar().getByText(/分流模式 规则分流/).waitFor();
    await shot('status-block');
    await set("s.force='unknown'");await recheck('unknown');
    const unknown=await colors('.status-bar');
    assert.ok(!unknown.some(reddish),'unknown 出现红色');assert.ok(!unknown.some(greenish),'unknown 出现绿色');
    await shot('status-unknown');
  });

  await step('unknown 不显示绿色（含读取失败）',async()=>{
    await set("s.force='unknown'");await recheck('unknown');
    assert.ok(!(await colors('.status-bar')).some(greenish),'unknown 状态条出现绿色');
    await set("s.force='pass'");await recheck('pass');
    await set('s.failCheck=true');
    await bar().getByRole('button',{name:'重新检查'}).click();
    await bar().getByText('读取 mihomo 控制接口超时。').waitFor();
    assert.equal(await bar().getAttribute('data-verdict'),'unknown');
    assert.ok(!(await colors('.status-bar')).some(greenish),'检查失败后仍显示绿色');
    await set('s.failCheck=false;s.force=null');await recheck('pass');
  });

  await step('启动被拦时不启动并用 Toast 显示原因',async()=>{
    await nav('账号');
    await set("s.force='block'");
    await page.getByRole('button',{name:'启动 Claude Code'}).click();
    const toast=page.locator('[data-tone="block"]').filter({hasText:'没有启动：启动 Claude Code'});
    await toast.waitFor();
    assert.match(await toast.innerText(),/实际出口 198.51.100.9 与期望出口 203.0.113.7 不符，已拦住。/);
    assert.equal(await bar().getAttribute('data-verdict'),'block');
    assert.deepEqual(await fixture('window.appFixture.state.launched'),[]);
    await shot('launch-blocked');
    await set('s.force=null');
    await page.getByRole('button',{name:'启动网页版'}).click();
    await page.locator('[data-tone="info"]').filter({hasText:'已启动：启动网页版'}).waitFor();
    assert.deepEqual(await fixture('window.appFixture.state.launched'),['web']);
  });

  await step('账号切换要先确认，取消不切换',async()=>{
    await nav('账号');
    const card=page.locator('.account-card').filter({has:page.getByRole('heading',{name:'Work account',exact:true})});
    await card.getByRole('button',{name:'切换',exact:true}).click();
    let dialog=page.getByRole('dialog',{name:'切换身份'});await dialog.waitFor();
    await dialog.getByRole('button',{name:'取消'}).click();
    assert.equal(await fixture("window.appFixture.commands.filter(r=>r.command==='identities_switch').length"),0);
    await card.getByRole('button',{name:'切换',exact:true}).click();
    dialog=page.getByRole('dialog',{name:'切换身份'});
    await dialog.getByRole('button',{name:'确认暂存并切换'}).click();
    await page.locator('.account-card[data-current]').getByRole('heading',{name:'Work account'}).waitFor();
    const order=await fixture<string[]>("window.appFixture.commands.map(r=>r.command).filter(c=>c.startsWith('identities_s'))");
    assert.deepEqual(order,['identities_stop','identities_switch']);
  });

  await step('漏网连接标红置顶，只按域名核对时有提示',async()=>{
    await set("s.leaks=[{host:'api.anthropic.com',process:'',inbound:'mixed',node:'DIRECT'}]");
    await nav('连接');
    await page.locator('.leak-row').first().waitFor();
    await page.getByRole('note').getByText(/只按域名核对/).waitFor();
    await set("s.leaks=[{host:'claude.ai',process:'',inbound:'mixed',node:'KR-HY2'}]");
    await page.waitForFunction(()=>document.querySelectorAll('.leak-row').length===2);
    const rows=page.locator('.leak-row');
    assert.match(await rows.first().innerText(),/claude\.ai[\s\S]*实际节点 KR-HY2/);
    assert.equal(await rows.first().getAttribute('data-leak'),'live');
    assert.ok((await colors('.leak-row:first-child')).some(reddish),'漏网行不是红色');
    await set('s.processMatching=true');
    await page.getByRole('note').waitFor({state:'detached'});
    await shot('connections-leak');
  });

  await step('英文模式没有中文',async()=>{
    await nav('设置');
    await page.getByRole('combobox',{name:'界面语言'}).click();
    await page.getByRole('option',{name:'English'}).click();
    await page.getByRole('heading',{name:'Settings'}).waitFor();
    assert.equal(await fixture('window.appFixture.state.language'),'en');
    const cjk=/[\u3400-\u9fff]/;
    for(const [target,ready] of [['Settings','Line'],['Accounts','Accounts'],['Connections','Connections']] as const){
      await nav(target);await page.getByRole('heading',{name:ready,exact:true}).waitFor();await page.waitForTimeout(300);
      const text=await page.evaluate(()=>document.body.innerText);
      const hit=text.split('\n').find(line=>cjk.test(line));
      assert.ok(!hit,`${target} 页出现中文：${hit}`);
    }
    await shot('english');
    await nav('Settings');
    await page.getByRole('combobox',{name:'Language'}).click();
    await page.getByRole('option',{name:'中文'}).click();
    await page.getByRole('heading',{name:'设置'}).waitFor();
  });

  await step('深色与浅色主题可读，主题只存在 localStorage',async()=>{
    await page.getByRole('combobox',{name:'主题'}).click();
    await page.getByRole('option',{name:'深色'}).click();
    await page.waitForFunction(()=>document.documentElement.classList.contains('dark'));
    assert.equal(await page.evaluate(()=>localStorage.getItem('cc-guard-theme')),'dark');
    for(const selector of ['body','main h1','.status-bar strong','.status-reason'])assert.ok(await contrast(selector)>=4.5,`深色 ${selector} 对比度不足`);
    await nav('账号');await page.locator('.account-card').first().waitFor();
    assert.ok(await contrast('.account-card h3')>=4.5,'深色账号卡片对比度不足');
    await shot('dark');
    await nav('设置');
    await page.getByRole('combobox',{name:'主题'}).click();
    await page.getByRole('option',{name:'浅色'}).click();
    await page.waitForFunction(()=>!document.documentElement.classList.contains('dark'));
    for(const selector of ['body','main h1','.status-bar strong','.status-reason'])assert.ok(await contrast(selector)>=4.5,`浅色 ${selector} 对比度不足`);
    await page.getByRole('combobox',{name:'主题'}).click();
    await page.getByRole('option',{name:'跟随系统'}).click();
    await page.emulateMedia({colorScheme:'dark'});
    await page.waitForFunction(()=>document.documentElement.classList.contains('dark'));
    await page.emulateMedia({colorScheme:'light'});
    await page.waitForFunction(()=>!document.documentElement.classList.contains('dark'));
  });

  await step('只调用契约命令，不出现 REMOVED 命令，无页面错误',async()=>{
    const used=[...new Set(await fixture<string[]>('window.appFixture.commands.map(r=>r.command)'))];
    const allowed=new Set<string>([...CONTRACT,...KEPT]);
    const outside=used.filter(command=>!allowed.has(command)&&!command.startsWith('plugin:'));
    assert.deepEqual(outside,[],'契约外命令');
    assert.deepEqual(used.filter(command=>(REMOVED as readonly string[]).includes(command)),[]);
    assert.deepEqual(errors,[],'页面错误');
  });
}finally{await browser.close();await server.close();}
const passed=results.filter(row=>row.ok).length;
console.log(`截图：\n${saved.map(path=>'  '+path).join('\n')}`);
console.log(`${passed}/${results.length} UI checks passed`);
process.exit(passed===results.length&&results.length>0?0:1);
