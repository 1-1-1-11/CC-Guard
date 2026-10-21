// [INPUT]: line-model 纯函数。
// [OUTPUT]: 只有新鲜的 pass 显示为 pass；错误、过期、缺结果为 unknown；首次设置判断；漏网合并去重与置顶。
// [POS]: 状态显示不骗人的离线回归。
import {test,expect} from 'bun:test';
import type {LineCheck,Settings} from './contract';
import {mergeLeaks,modeLabel,needsSetup,shownVerdict,STALE_MS} from './line-model';
const check=(verdict:LineCheck['verdict']):LineCheck=>({verdict,label:'',reason:'',mode:'rule',ip:'203.0.113.7',country:'US',timezone:'America/Los_Angeles',checked_at:1});
test('只有新鲜且无错误的 pass 才显示为 pass',()=>{
  expect(shownVerdict({check:check('pass'),at:1000},'',1000)).toBe('pass');
  expect(shownVerdict({check:check('block'),at:1000},'',1000)).toBe('block');
  expect(shownVerdict({check:check('unknown'),at:1000},'',1000)).toBe('unknown');
  expect(shownVerdict({check:check('pass'),at:1000},'超时',1000)).toBe('unknown');
  expect(shownVerdict({check:check('pass'),at:1000},'',1000+STALE_MS+1)).toBe('unknown');
  expect(shownVerdict(null,'',1000)).toBe('unknown');
  expect(shownVerdict({check:{...check('pass'),verdict:'PASS' as never},at:1000},'',1000)).toBe('unknown');
});
test('controller 空或端口为 0 时需要向导',()=>{
  const line={controller:'127.0.0.1:9097',secret_set:true,port:7899,node:'JP',expected_ip:'',services:[],claude_hosts:[],claude_processes:[]};
  const settings=(patch:Partial<typeof line>):Settings=>({line:{...line,...patch},workspace:''});
  expect(needsSetup(settings({}))).toBe(false);
  expect(needsSetup(settings({controller:' '}))).toBe(true);
  expect(needsSetup(settings({port:0}))).toBe(true);
});
test('分流模式文案与未知原样',()=>{
  expect(modeLabel('Rule')).toBe('规则分流');expect(modeLabel('custom')).toBe('custom');
});
test('漏网记录去重，当前仍在的置顶',()=>{
  const a={host:'api.anthropic.com',process:'claude.exe',inbound:'mixed',node:'DIRECT'},b={...a,host:'claude.ai'};
  let seen=mergeLeaks([],[a,b],1);
  seen=mergeLeaks(seen,[a],2);
  expect(seen.map(row=>[row.host,row.live,row.first,row.last])).toEqual([['api.anthropic.com',true,1,2],['claude.ai',false,1,1]]);
  seen=mergeLeaks(seen,[b],3);
  expect(seen[0].host).toBe('claude.ai');expect(seen.length).toBe(2);
});
