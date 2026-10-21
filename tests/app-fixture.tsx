/**
 * [INPUT]: Tauri 官方 mockIPC / mockWindows、真实 App 根组件与 contract.ts 的命令形状。
 * [OUTPUT]: 按契约造的内存假后端：设置（secret 只写不读）、线路检查三态、节点、连接快照、启动拦截、身份与语言；记录每条命令供审计。
 * [POS]: scripts/test-ui.ts 的离线入口；没有原生命令、真实账号、Clash 或网络，生产 rollup 不包含此页。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {createRoot} from 'react-dom/client';
import {mockIPC,mockWindows} from '@tauri-apps/api/mocks';
import type {LaunchEntry,LineCheck,LineConnections,Settings,Verdict} from '../src/contract';
import {defaultPolicy,type IdentityState} from '../src/identity-model';
import {App,Root} from '../src/main';
const EXIT='203.0.113.7';
const state={
  settings:{line:{controller:'',secret_set:false,port:0,node:'',expected_ip:'',services:['https://a.example.invalid','https://b.example.invalid'],claude_hosts:['anthropic.com','claude.ai','claude.com'],claude_processes:['claude.exe']},workspace:'C:\\Fixture'} as Settings,
  secret:'',
  force:null as Verdict|null,
  failCheck:false,
  nodes:['JP-HY2','KR-HY2'],
  leaks:[] as LineConnections['leaks'],
  processMatching:false,
  language:'zh' as 'zh'|'en',
  launched:[] as LaunchEntry[],
  identities:{browser:{enabled:false,bound_ids:[]},browser_warning:null,active:'a',recovery:null,running:false,
    current:{present:true,display_name:'Fixture user',email:'fixture@example.invalid',organization:null,unavailable:false},
    records:[{id:'a',name:'Fixture user',saved_at:1780000000000,bytes:2048,policy:{...defaultPolicy}},{id:'b',name:'Work account',saved_at:1780000001000,bytes:4096,policy:{...defaultPolicy}}]} as IdentityState,
};
const commands:{command:string;args:any}[]=[];
function check():LineCheck{
  const line=state.settings.line;
  const base={mode:'rule',country:'',timezone:'',ip:'',checked_at:Date.now()};
  if(!line.controller||!line.port)return {...base,verdict:'unknown',label:'无法确认',reason:'controller 地址为空。'};
  const verdict=state.force??(line.expected_ip?'pass':'unknown');
  if(verdict==='pass')return {...base,verdict,label:'通过',reason:`专用线路出口为 ${EXIT}，与期望一致。`,ip:EXIT,country:'US',timezone:'America/Los_Angeles'};
  if(verdict==='block')return {...base,verdict,label:'拦住',reason:`实际出口 198.51.100.9 与期望出口 ${EXIT} 不符，已拦住。`,ip:'198.51.100.9',country:'JP',timezone:'Asia/Tokyo'};
  return {...base,verdict,label:'无法确认',reason:'出口查询服务少于 2 个，无法确认。',ip:line.expected_ip?'':EXIT,country:line.expected_ip?'':'US',timezone:line.expected_ip?'':'America/Los_Angeles'};
}
mockWindows('main');
mockIPC(async(command,args:any)=>{
  commands.push({command,args:structuredClone(args)});
  await new Promise(resolve=>setTimeout(resolve,30));
  switch(command){
    case 'settings_read':return structuredClone(state.settings);
    case 'settings_write':{
      const {secret,...line}=args.patch.line??{};
      if(typeof secret==='string'){state.secret=secret;state.settings.line.secret_set=secret.length>0;}
      Object.assign(state.settings.line,line);
      return structuredClone(state.settings);
    }
    case 'line_check':if(state.failCheck)throw '读取 mihomo 控制接口超时。';return check();
    case 'line_nodes':if(!state.settings.line.controller)throw '连不上 mihomo 控制接口。';return [...state.nodes];
    case 'line_connections':{
      const verdict:Verdict=state.leaks.length?'block':'pass';
      return {verdict,label:verdict==='pass'?'通过':'拦住',reason:verdict==='pass'?`专用线路出口为 ${EXIT}，与期望一致。`:`实际出口 198.51.100.9 与期望出口 ${EXIT} 不符，已拦住。`,total:12,claude:3,leaks:structuredClone(state.leaks),process_matching:state.processMatching,checked_at:Date.now()} satisfies LineConnections;
    }
    case 'launch':{const result=check();if(result.verdict==='pass')state.launched.push(args.entry);return {launched:result.verdict==='pass',reason:result.verdict==='pass'?'':result.reason,check:result};}
    case 'workspace_choose':state.settings.workspace='D:\\Projects\\demo';return state.settings.workspace;
    case 'ui_language_read':return state.language;
    case 'ui_language_write':state.language=args.language;return null;
    case 'identities_read':return structuredClone(state.identities);
    case 'identities_stop':return {running:false};
    case 'identities_switch':{
      const target=state.identities.records.find(row=>row.id===args.targetId)!;
      state.identities.active=target.id;state.identities.current={...state.identities.current,display_name:target.name};
      return structuredClone(state.identities);
    }
    case 'claude_cli_read':return {method:'native',path:'C:\\Fixture\\claude.exe',version:'2.1.0'};
    case 'plugin:window|is_visible':return true;
  }
  throw `Unexpected fixture command: ${command}`;
},{shouldMockEvents:true});
Object.assign(window,{appFixture:{state,commands}});
createRoot(document.getElementById('fixture')!).render(<Root><App/></Root>);
