// [INPUT]: line_check 命令结果（或启动 / 向导带回的检查结果）与窗口可见性。
// [OUTPUT]: useLineCheck 状态与顶部状态条：pass 绿、block 红、unknown 灰，显示原因、出口地区、分流模式与「重新检查」。
// [POS]: 全局线路状态的唯一呈现；出错、无结果或超过 3 分钟未更新都按 unknown 显示，绝不沿用旧绿灯。
import {useCallback,useEffect,useRef,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {ShieldAlert,ShieldCheck,ShieldQuestion,RefreshCw} from 'lucide-react';
import type {LineCheck,Verdict} from './contract';
import {modeLabel,shownVerdict,type Received} from './line-model';
import {Button} from './components/ui/button';
import {cn} from './components/ui/cn';
import {useI18n} from './i18n';
export type LineState={result:Received|null;error:string;busy:boolean;verdict:Verdict;run:()=>Promise<LineCheck|null>;accept:(check:LineCheck)=>void};
export function useLineCheck(active:boolean):LineState{
  const [result,setResult]=useState<Received|null>(null),[error,setError]=useState(''),[busy,setBusy]=useState(false),[now,setNow]=useState(Date.now());
  const pending=useRef(false);
  const accept=useCallback((check:LineCheck)=>{setResult({check,at:Date.now()});setError('');setNow(Date.now());},[]);
  const run=useCallback(async()=>{
    if(pending.current)return null;
    pending.current=true;setBusy(true);
    try{const check=await invoke<LineCheck>('line_check');accept(check);return check;}
    catch(e){setError(String(e));return null;}
    finally{pending.current=false;setBusy(false);}
  },[accept]);
  useEffect(()=>{
    if(!active)return;
    void run();
    const timer=window.setInterval(()=>{setNow(Date.now());void run();},60_000);
    return()=>window.clearInterval(timer);
  },[active,run]);
  return {result,error,busy,verdict:shownVerdict(result,error,now),run,accept};
}
const look:Record<Verdict,{box:string;icon:typeof ShieldCheck;title:string}>={
  pass:{box:'border-pass/40 bg-pass-bg text-pass',icon:ShieldCheck,title:'线路通过'},
  block:{box:'border-block/40 bg-block-bg text-block',icon:ShieldAlert,title:'已拦住'},
  unknown:{box:'border-unknown/40 bg-unknown-bg text-unknown',icon:ShieldQuestion,title:'无法确认'},
};
export function StatusBar({line}:{line:LineState}){
  const {t}=useI18n();
  const {verdict,result,error,busy}=line;
  const check=result?.check;
  const Icon=look[verdict].icon;
  const stale=!!check&&!error&&verdict==='unknown'&&check.verdict!=='unknown';
  const reason=error||(stale?'结果已过期，请重新检查。':'')||(check?check.reason:'')||(busy?'正在检查线路…':'还没有检查结果。');
  const facts=check&&!error&&!stale?[check.country&&t('出口地区 {v0}',{v0:[check.country,check.timezone].filter(Boolean).join(' · ')}),check.ip&&t('出口 IP {v0}',{v0:check.ip}),check.mode&&t('分流模式 {v0}',{v0:t(modeLabel(check.mode))})].filter(Boolean):[];
  return <div role="status" aria-live="polite" data-verdict={verdict} className={cn('status-bar flex items-center gap-3 rounded-xl border px-4 py-3',look[verdict].box)}>
    <Icon className="size-6 shrink-0" aria-hidden="true"/>
    <div className="min-w-0 flex-1">
      <div className="flex flex-wrap items-baseline gap-x-3"><strong className="text-sm">{t(look[verdict].title)}</strong><span className="text-xs text-fg/80">{facts.join(' · ')}</span></div>
      <p className="status-reason truncate text-xs text-fg/80" title={t(reason)}>{t(reason)}</p>
    </div>
    <Button size="sm" variant="secondary" loading={busy} onClick={()=>void line.run()}><RefreshCw className="size-3.5" aria-hidden="true"/>{t('重新检查')}</Button>
  </div>;
}
