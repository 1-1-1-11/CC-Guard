/**
 * [INPUT]: Tauri 单次调用的有序进度流，身份、CLI 与浏览器操作结果，共享 YAML 文案。
 * [OUTPUT]: 本次活动状态、紧凑真实进度与默认收起的日志；原始消息保留，切换语言时重新翻译。
 * [POS]: 本地事务的观察层；调用通道隔离不同操作，不制造进度。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {useEffect,useRef,useState} from 'react';
import {Channel} from '@tauri-apps/api/core';
import {RefreshCw} from 'lucide-react';
import {useI18n} from './i18n';
import {Button} from './components/ui/button';
import {Card} from './components/ui/card';
import {cn} from './components/ui/cn';
export type Progress={stage:string;status:'running'|'ok'|'warning'|'error';message:string;current:number;total:number|null;elapsed_ms:number};
type Kind='browser'|'cli'|'identity';
type Activity={kind:Kind;started:number;entries:Progress[];running:boolean;failed:boolean};
const starts:Record<Kind,string>={browser:'开始浏览器操作。',identity:'开始本地身份操作。',cli:'开始 Claude Code CLI 安装管理。'};
const titles:Record<Kind,string>={browser:'浏览器操作记录',identity:'身份操作记录',cli:'CLI 安装记录'};
export function useCleanupActivity(){
  const [state,setState]=useState<Activity|null>(null);
  const sequence=useRef(0),began=useRef(0);
  function append(event:Progress,token=sequence.current){
    if(token!==sequence.current)return;
    setState(old=>{
      if(!old)return old;
      const entries=[...old.entries];
      const done=entries.at(-1)?.stage==='done'?entries.pop():undefined;
      entries.push(event);if(done)entries.push(done);
      return {...old,entries:entries.slice(-2000)};
    });
  }
  const elapsed=()=>Math.round(performance.now()-began.current);
  function start(kind:Kind){
    sequence.current++;began.current=performance.now();
    setState({kind,started:began.current,entries:[],running:true,failed:false});
    append({stage:kind,status:'running',message:starts[kind],current:0,total:null,elapsed_ms:0});
    const token=sequence.current;
    return new Channel<Progress>(event=>append({...event,elapsed_ms:elapsed()},token));
  }
  function finish(success:boolean,message:string){
    append({stage:'done',status:success?'ok':'error',message,current:0,total:null,elapsed_ms:elapsed()});
    setState(old=>old?{...old,running:false,failed:!success}:old);
  }
  return {state,start,finish};
}
export function CleanupActivity({activity}:{activity:Activity|null}){
  const {t}=useI18n();
  const [open,setOpen]=useState(false);
  const logBox=useRef<HTMLDivElement>(null);
  useEffect(()=>{setOpen(false);},[activity?.started]);
  useEffect(()=>{if(logBox.current)logBox.current.scrollTop=logBox.current.scrollHeight;},[activity?.entries.length,open]);
  if(!activity)return null;
  const last=activity.entries.at(-1);
  const count=last?.total&&last.stage!=='done'?`${last.current} / ${last.total}`:null;
  return <Card aria-label={t('操作活动日志')} className={cn('space-y-2 p-3',activity.failed&&'border-block/40')}>
    <div className="flex items-center gap-2 text-sm">
      {activity.running?<RefreshCw className="size-4 animate-spin text-primary" aria-hidden="true"/>:<span aria-hidden="true" className={activity.failed?'text-block':'text-muted'}>{activity.failed?'!':'·'}</span>}
      <strong>{t(titles[activity.kind])}</strong>
      <span className="text-xs text-muted">{activity.running?t('运行中'):activity.failed?t('已停止 · 请检查'):t('已完成')}</span>
      <Button className="ml-auto" variant="ghost" size="sm" aria-expanded={open} onClick={()=>setOpen(!open)}>{open?t('收起日志'):t('展开日志')} [{activity.entries.length}]</Button>
    </div>
    <div role="status" aria-live="polite" className="flex justify-between text-xs text-muted"><span>{t(last?.message||'准备中…')}</span>{count&&<strong>{count}</strong>}</div>
    {open&&<div ref={logBox} role="log" aria-label={t('本次操作日志')} className="max-h-48 overflow-y-auto rounded-lg bg-muted-bg p-2 font-mono text-xs">
      {activity.entries.map((entry,i)=><div key={i} className={cn('flex gap-2',entry.status==='error'&&'text-block')}><time className="text-muted">+{(entry.elapsed_ms/1000).toFixed(2)}s</time><span>{t(entry.message)}</span></div>)}
    </div>}
  </Card>;
}
