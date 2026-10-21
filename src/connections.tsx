// [INPUT]: line_connections 只读快照，窗口可见时每 2 秒读取一次。
// [OUTPUT]: 连接页：判定徽标、Claude 连接计数；漏网连接标红置顶（本次仍在的最前）；process_matching=false 时提示只按域名核对。
// [POS]: 只观察，不拦截、不修复、不断开任何连接；启动决定在后端 launch。
import {useEffect,useRef,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {Info} from 'lucide-react';
import type {LineConnections,Verdict} from './contract';
import {mergeLeaks,type SeenLeak} from './line-model';
import {Badge,type Tone} from './components/ui/badge';
import {Button} from './components/ui/button';
import {Card} from './components/ui/card';
import {cn} from './components/ui/cn';
import {useI18n} from './i18n';
const tones:Record<Verdict,Tone>={pass:'pass',block:'block',unknown:'unknown'};
export function ConnectionsPanel({active=true}:{active?:boolean}){
  const {t,locale}=useI18n();
  const [latest,setLatest]=useState<LineConnections|null>(null),[seen,setSeen]=useState<SeenLeak[]>([]),[error,setError]=useState('');
  const live=useRef(true);
  useEffect(()=>{live.current=true;return()=>{live.current=false;};},[]);
  useEffect(()=>{
    if(!active)return;
    let pending=false;
    const look=async()=>{
      if(pending||document.visibilityState==='hidden')return;
      pending=true;
      try{
        const next=await invoke<LineConnections>('line_connections');
        if(live.current){setLatest(next);setSeen(old=>mergeLeaks(old,next.leaks,Date.now()));setError('');}
      }catch(e){if(live.current)setError(String(e));}
      finally{pending=false;}
    };
    void look();
    const timer=window.setInterval(()=>void look(),2000);
    return()=>window.clearInterval(timer);
  },[active]);
  const verdict:Verdict=error||!latest?'unknown':latest.verdict==='pass'||latest.verdict==='block'?latest.verdict:'unknown';
  const time=(at:number)=>new Date(at).toLocaleTimeString(locale==='zh'?'zh-CN':'en-US');
  return <section className="space-y-4">
    <header className="flex items-center gap-2">
      <div className="flex-1"><h1 className="text-lg font-semibold">{t('连接')}</h1><p className="text-xs text-muted">{t('只读：每 2 秒看一次 Claude 的连接走了哪个节点，不会断开或修改任何东西。')}</p></div>
      <Button size="sm" disabled={!seen.length} onClick={()=>setSeen([])}>{t('清空记录')}</Button>
    </header>
    <Card className="flex flex-wrap items-center gap-3">
      <Badge tone={tones[verdict]} data-verdict={verdict}>{t(verdict==='pass'?'通过':verdict==='block'?'拦住':'无法确认')}</Badge>
      <span className="flex-1 text-sm">{t(error||latest?.reason||'正在读取连接…')}</span>
      {latest&&<span className="text-xs text-muted">{t('Claude 连接 {v0} / 全部 {v1}',{v0:latest.claude,v1:latest.total})}</span>}
    </Card>
    {latest&&!latest.process_matching&&<p role="note" className="flex items-start gap-2 rounded-lg border border-border bg-muted-bg px-3 py-2 text-xs"><Info className="mt-0.5 size-3.5 shrink-0" aria-hidden="true"/>{t('只按域名核对：Clash 没有开启 find-process-mode: always，认不出发起连接的程序。')}</p>}
    <div className="leak-list space-y-2" aria-label={t('漏网连接')}>
      {seen.map(row=><Card key={`${row.host}|${row.process}|${row.inbound}|${row.node}`} data-leak={row.live?'live':'seen'} className={cn('leak-row flex items-center gap-3 border-block/50 p-3',row.live&&'bg-block-bg')}>
        <Badge tone="block">{t('漏网')}</Badge>
        <div className="min-w-0 flex-1"><strong className="block truncate text-sm">{row.host||'—'}</strong>
          <span className="text-xs text-muted">{[row.process||t('未知程序'),t('入口 {v0}',{v0:row.inbound||'—'}),t('实际节点 {v0}',{v0:row.node||'—'})].join(' · ')}</span></div>
        <time className="text-xs text-muted">{t(row.live?'现在':'最近 {v0}',{v0:time(row.last)})}</time>
      </Card>)}
      {!seen.length&&<p className="px-1 text-sm text-muted">{t(latest?'没有看到漏网连接。':'还没有连接记录。')}</p>}
    </div>
  </section>;
}
