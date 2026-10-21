/**
 * [INPUT]: Node.js 标准管道、已锁定 playwright-core、本机 Edge、Vite 与 tests/ 的虚构 Tauri IPC。
 * [OUTPUT]: 账号卡片（顺序、使用中、⋯ 菜单）、联动浏览器确认/取消、保留选项、暂存切换、永久删除 Alert（默认聚焦取消、需勾选）、
 *           改名、失败回读、未完成切换的恢复与双语的真实 DOM 交互验收；外加两套持久浏览器资料隔离。
 * [POS]: 发布前无头行为回归；无截图、无真实账号/文件修改，不加载原生宿主，不允许外网请求。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {strict as assert} from 'node:assert';
import {resolve,join} from 'node:path';
import {mkdtemp,mkdir,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {createServer} from 'vite';
import {chromium} from 'playwright-core';
import {testBrowserProfiles} from './test-browser-profiles.ts';
const edge='C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe';
const server=await createServer({server:{host:'127.0.0.1',port:0,strictPort:false,watch:{ignored:['**/src-tauri/**','**/browser-runtime/**','**/releases/**','**/dist/**']}},optimizeDeps:{include:['@tauri-apps/api/mocks']}});
await server.listen();
const address=server.httpServer!.address() as {port:number};
const base=`http://127.0.0.1:${address.port}`;
const env=Object.fromEntries(Object.entries(process.env).filter(([key])=>!key.startsWith('APPLE_')&&!key.startsWith('TAURI_SIGNING_PRIVATE_KEY'))) as Record<string,string>;
// ----- 临时 HOME/XDG 避免真实浏览器资料 -----
const temporary=await mkdtemp(join(tmpdir(),'cc-guard-ui-'));
for(const name of ['config','cache'])await mkdir(join(temporary,name));
Object.assign(env,{HOME:temporary,XDG_CONFIG_HOME:join(temporary,'config'),XDG_CACHE_HOME:join(temporary,'cache')});
let browser;
try{browser=await chromium.launch({executablePath:edge,headless:true,env});}
catch(error){await server.close();await rm(temporary,{recursive:true,force:true});throw error;}
let checks=0;
try{
 const context=await browser.newContext({viewport:{width:1040,height:820}});
 await context.route('**/*',route=>route.request().url().startsWith(base+'/')?route.continue():route.abort());
 const page=await context.newPage();page.setDefaultTimeout(10000);
 const errors:string[]=[];page.on('console',message=>{if(message.type()==='error')console.error(message.text());});page.on('pageerror',error=>{errors.push(error.message);console.error(error.message);});
 const fixture=<T>(code:string)=>page.evaluate(code) as Promise<T>;
 const sent=(command:string)=>fixture<number>(`window.identityFixture.commands.filter(r=>r.command===${JSON.stringify(command)}).length`);
 await page.goto(base+'/tests/identities.html');
 const cards=page.locator('.account-card');
 const card=(name:string)=>cards.filter({has:page.getByRole('heading',{name,exact:true})});
 await page.getByText('fixture@example.invalid',{exact:true}).waitFor();
 // 当前账号排第一并标「使用中」；已是当前的暂存副本不重复出现。
 assert.equal(await cards.count(),40);
 assert.equal(await cards.first().getAttribute('data-current'),'true');
 assert.match(await cards.first().innerText(),/Fixture user[\s\S]*使用中/);
 assert.equal(await card('Identity 1').count(),0);checks++;
 // 暂存账号只有「切换」按钮，改名 / 删除收在 ⋯ 菜单里。
 assert.equal(await card('Identity 2').getByRole('button',{name:'切换',exact:true}).count(),1);
 assert.equal(await card('Identity 2').getByRole('button',{name:'删除',exact:true}).count(),0);
 await card('Identity 2').getByRole('button',{name:'更多操作'}).click();
 assert.deepEqual(await page.getByRole('menuitem').allInnerTexts(),['改名','删除']);
 await page.keyboard.press('Escape');checks++;
 // 联动浏览器：取消不改状态，确认才调用；默认不把现有资料关联给当前身份。
 const linked=page.getByRole('switch',{name:'联动浏览器'});
 assert.equal(await linked.getAttribute('aria-checked'),'true');
 await linked.click();
 await page.getByRole('dialog').getByRole('button',{name:'取消',exact:true}).click();
 assert.equal(await linked.getAttribute('aria-checked'),'true');assert.equal(await sent('identities_browser'),0);
 await linked.click();
 await page.getByRole('dialog').getByRole('button',{name:'确认',exact:true}).click();
 await page.waitForFunction(()=>(window as any).identityFixture.state.browser.enabled===false);
 await linked.click();
 assert.equal(await page.getByRole('dialog').getByRole('checkbox',{name:'将现有浏览器资料关联给当前身份'}).getAttribute('aria-checked'),'false');
 await page.getByRole('dialog').getByRole('button',{name:'确认',exact:true}).click();
 await page.waitForFunction(()=>(window as any).identityFixture.state.browser.enabled===true);checks++;
 // 直接删除当前账号：Alert 默认聚焦取消、未勾选不可确认、点遮罩不关闭，取消不发命令。
 await cards.first().getByRole('button',{name:'更多操作'}).click();
 await page.getByRole('menuitem',{name:'直接删除'}).click();
 let dialog=page.getByRole('alertdialog');await dialog.waitFor();
 assert.match(await dialog.innerText(),/不可撤销/);
 assert.equal(await dialog.getByRole('button',{name:'确认直接删除'}).isEnabled(),false);
 await page.waitForFunction(()=>document.activeElement?.textContent==='取消');
 await page.mouse.click(3,3);assert.equal(await dialog.count(),1);
 await dialog.getByRole('button',{name:'取消'}).click();
 assert.equal(await sent('identities_delete'),0);checks++;
 // 保留策略：取消勾选的项目随切换请求一起发出。
 await page.getByText('保留在工作区',{exact:true}).click();
 await page.getByRole('checkbox',{name:'Skills 与命令'}).click();
 assert.equal(await page.getByRole('checkbox',{name:'Skills 与命令'}).getAttribute('aria-checked'),'false');checks++;
 // 「+ 空账号」：确认窗填写当前账号名，先结束进程再暂存切换。
 await page.getByRole('button',{name:'空账号'}).click();
 dialog=page.getByRole('dialog');await dialog.waitFor();
 assert.match(await dialog.innerText(),/将结束已识别的 Claude 进程/);
 await dialog.getByLabel('当前身份保存为').fill('First archive');
 await dialog.getByRole('button',{name:'确认暂存并切换'}).click();
 await page.waitForFunction(()=>!(window as any).identityFixture.state.current.present);
 const order=await fixture<string[]>("window.identityFixture.commands.map(r=>r.command).filter(c=>c==='identities_stop'||c==='identities_switch')");
 assert.deepEqual(order,['identities_stop','identities_switch']);
 const request=await fixture<any>("window.identityFixture.commands.find(r=>r.command==='identities_switch').args");
 assert.equal(request.policy.skills,false);assert.equal(request.targetId,'empty');assert.equal(request.currentName,'First archive');checks++;
 // 空账号成为使用中，「+ 空账号」不可再点。
 await page.getByRole('heading',{name:'空账号',exact:true}).waitFor();
 assert.match(await cards.first().innerText(),/使用中/);
 assert.equal(await page.getByRole('button',{name:'空账号'}).isEnabled(),false);checks++;
 // 切换到暂存账号：取消不发命令，确认后目标成为当前账号。
 await card('Identity 3').getByRole('button',{name:'切换',exact:true}).click();
 await page.getByRole('dialog').getByRole('button',{name:'取消',exact:true}).click();
 assert.equal(await sent('identities_switch'),1);checks++;
 await card('Identity 3').getByRole('button',{name:'切换',exact:true}).click();
 dialog=page.getByRole('dialog');
 assert.match(await dialog.innerText(),/目标身份\s*Identity 3/);
 await dialog.getByRole('button',{name:'确认暂存并切换'}).click();
 await page.getByText('Fixture team').first().waitFor();
 assert.equal(await cards.first().getByRole('heading').innerText(),'Identity 3');
 assert.equal(await card('Identity 3').getByRole('button',{name:'切换',exact:true}).count(),0);checks++;
 // 删除绑定了浏览器资料的暂存账号：关联资料默认不删。
 await card('Identity 1').getByRole('button',{name:'更多操作'}).click();
 await page.getByRole('menuitem',{name:'删除'}).click();
 dialog=page.getByRole('alertdialog');
 assert.equal(await dialog.getByRole('checkbox',{name:'同时永久删除关联浏览器资料'}).getAttribute('aria-checked'),'false');
 await dialog.getByRole('button',{name:'取消',exact:true}).click();checks++;
 // 改名后再永久删除：勾选确认才执行，当前账号不受影响。
 await card('Identity 4').getByRole('button',{name:'更多操作'}).click();
 await page.getByRole('menuitem',{name:'改名'}).click();
 await page.getByRole('dialog').getByRole('textbox',{name:'身份名称'}).fill('Renamed fixture');
 await page.getByRole('dialog').getByRole('button',{name:'保存',exact:true}).click();
 await card('Renamed fixture').waitFor();checks++;
 await card('Renamed fixture').getByRole('button',{name:'更多操作'}).click();
 await page.getByRole('menuitem',{name:'删除'}).click();
 dialog=page.getByRole('alertdialog');
 await dialog.getByRole('checkbox',{name:'我知道此操作无法撤销'}).click();
 await dialog.getByRole('button',{name:'确认直接删除'}).click();await card('Renamed fixture').waitFor({state:'detached'});
 assert.equal(await fixture('window.identityFixture.state.current.present'),true);checks++;
 // 英文界面。
 await page.evaluate(()=>{(window as any).identityFixture.language('en');});
 await page.getByRole('button',{name:'Empty account'}).waitFor();
 assert.match(await cards.first().innerText(),/In use/);checks++;
 // 切换失败：显示原因并回读状态，当前账号保持不变。
 await page.evaluate(()=>{(window as any).identityFixture.setFailure('identities_switch');});
 await page.getByRole('button',{name:'Empty account'}).click();
 await page.getByRole('dialog').getByRole('button',{name:'Archive & switch'}).click();
 await page.getByText(/fixture operation failed/).first().waitFor();
 assert.equal(await fixture('window.identityFixture.state.current.present'),true);checks++;
 // 未完成的切换：锁住其他操作，只能恢复原身份。
 await page.evaluate(()=>{(window as any).identityFixture.recover();});
 await page.getByRole('button',{name:'Refresh',exact:true}).click();
 await page.getByRole('button',{name:'Restore previous identity',exact:true}).waitFor();
 assert.equal(await page.getByRole('button',{name:'Empty account'}).isEnabled(),false);
 await page.getByRole('button',{name:'Restore previous identity',exact:true}).click();
 await page.getByRole('dialog').getByRole('button',{name:'Restore'}).click();
 await page.waitForFunction(()=>(window as any).identityFixture.state.recovery===null);checks++;
 // 直接删除当前账号：不新增暂存。
 await cards.first().getByRole('button',{name:'More actions'}).click();
 await page.getByRole('menuitem',{name:'Delete permanently'}).click();
 dialog=page.getByRole('alertdialog');
 await dialog.getByRole('checkbox',{name:'I understand this cannot be undone'}).click();
 const count=await fixture<number>('window.identityFixture.state.records.length');
 await dialog.getByRole('button',{name:'Confirm deletion',exact:true}).click();
 await page.getByRole('heading',{name:'Empty account',exact:true}).waitFor();
 assert.equal(await fixture('window.identityFixture.state.records.length'),count);checks++;
 assert.equal(errors.length,0,errors.join('\n'));
 checks+=await testBrowserProfiles(edge,base,temporary,env);
 console.log(`${checks} identity and browser profile UI simulation checks passed; no native commands or user data used.`);
}finally{await browser.close();await server.close();await rm(temporary,{recursive:true,force:true});}
