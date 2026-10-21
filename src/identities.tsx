/**
 * [INPUT]: 原生身份命令（identities_*、browser_launch）、保留策略、共享确认窗与进度日志。
 * [OUTPUT]: 账号页：CC-Switch 式账号卡片列表（当前账号标「使用中」，其余「切换」，⋯ 里改名 / 删除），右上「+ 空账号」；
 *           切换 / 恢复确认、永久删除 Alert（默认聚焦取消、需勾选）、保留策略、联动浏览器与恢复提示。
 * [POS]: 身份管理 GUI；只呈现脱敏元数据，身份正文留在原生保险库。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {useEffect,useState,type ReactNode} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {MoreHorizontal,Plus,RefreshCw,UserRound} from 'lucide-react';
import {useI18n} from './i18n';
import {AppDialog} from './components/ui/dialog';
import {Badge} from './components/ui/badge';
import {Button} from './components/ui/button';
import {Card} from './components/ui/card';
import {Checkbox} from './components/ui/checkbox';
import {Input} from './components/ui/input';
import {Menu,type MenuItem} from './components/ui/dropdown-menu';
import {Switch} from './components/ui/switch';
import {cn} from './components/ui/cn';
import {CleanupActivity,useCleanupActivity} from './cleaner-activity';
import {formatBytes} from './cleaner-model';
import {defaultPolicy,policyRows,type Identity,type IdentityState} from './identity-model';
type Target=Identity|'empty'|'recovery';
export function IdentityPanel({onBusy}:{onBusy?:(busy:boolean)=>void}){
  const {t,locale}=useI18n();
  const [state,setState]=useState<IdentityState|null>(null);
  const [policy,setPolicy]=useState({...defaultPolicy});
  const [loading,setLoading]=useState(true),[busy,setBusy]=useState(false);
  const [error,setError]=useState(''),[warning,setWarning]=useState('');
  const [browserReview,setBrowserReview]=useState<boolean|null>(null),[useExisting,setUseExisting]=useState(false);
  const [review,setReview]=useState<Target|null>(null);
  const [name,setName]=useState('');
  const [rename,setRename]=useState<Identity|null>(null),[renameValue,setRenameValue]=useState('');
  const [deleteBrowser,setDeleteBrowser]=useState(false);
  const [deletion,setDeletion]=useState<Identity|'current'|null>(null),[accepted,setAccepted]=useState(false);
  const activity=useCleanupActivity();
  const locked=busy||!!review||!!rename||!!deletion||browserReview!==null;
  const active=state?.records.find(row=>row.id===state.active);
  const current=state?.current;
  const saved=state?.records.filter(row=>!current?.present||row.id!==state.active)??[];
  const currentName=current?.display_name||current?.email?.split('@')[0]||active?.name||t('当前身份');
  const blocked=locked||loading||!state||!!state?.recovery;
  const kept=policyRows.filter(([key])=>policy[key]).map(([,label])=>t(label));
  const removed=policyRows.filter(([key])=>!policy[key]).map(([,label])=>t(label));
  useEffect(()=>{void read();},[]);
  useEffect(()=>{onBusy?.(locked);return()=>onBusy?.(false);},[locked,onBusy]);
  async function read(){
    setLoading(true);
    try{setState(await invoke<IdentityState>('identities_read'));setError('');}
    catch(e){setError(String(e));}finally{setLoading(false);}
  }
  async function refreshAfterError(){
    try{setState(await invoke<IdentityState>('identities_read'));}catch{/* 保留操作错误，不掩盖恢复失败。 */}
  }
  async function configureBrowser(){
    if(browserReview===null||busy)return;
    const enabled=browserReview;setBusy(true);setError('');
    try{setState(await invoke<IdentityState>('identities_browser',{enabled,useExisting}));setBrowserReview(null);}
    catch(e){setError(String(e));await refreshAfterError();}finally{setBusy(false);}
  }
  async function retryBrowser(){
    if(busy)return;setBusy(true);setError('');
    try{await invoke('browser_launch',{onProgress:activity.start('browser')});activity.finish(true,'浏览器资料已与身份同步。');await read();}
    catch(e){setError(String(e));activity.finish(false,String(e));}finally{setBusy(false);}
  }
  function choose(target:Target){
    setName(target==='recovery'?t('恢复前状态'):currentName.slice(0,64));
    if(target==='recovery'){const previous=state?.records.find(row=>row.id===state.recovery);if(previous)setPolicy({...previous.policy});}
    setReview(target);setError('');
  }
  async function change(){
    if(!review||busy)return;
    const target=review;setReview(null);setBusy(true);setError('');setWarning('');
    const onProgress=activity.start('identity');
    try{
      await invoke('identities_stop');
      const next=await invoke<IdentityState>('identities_switch',{targetId:typeof target==='object'?target.id:'empty',currentName:name.trim(),policy,recover:target==='recovery',onProgress});
      setState(next);setWarning(next.network_warning||'');
      activity.finish(true,target==='recovery'?'原身份已恢复。':'身份切换完成，原身份已压缩暂存。');
    }catch(e){setError(String(e));activity.finish(false,String(e));await refreshAfterError();}
    finally{setBusy(false);}
  }
  function confirmDelete(target:Identity|'current'){setDeletion(target);setAccepted(false);setDeleteBrowser(false);setError('');}
  async function remove(){
    if(!deletion||!accepted||busy)return;
    const target=deletion;setDeletion(null);setBusy(true);setError('');setWarning('');
    const onProgress=activity.start('identity');
    try{
      if(target==='current')await invoke('identities_stop');
      const next=await invoke<IdentityState>('identities_delete',{id:target==='current'?'current':target.id,policy,deleteBrowser:target!=='current'&&deleteBrowser,onProgress});
      setState(next);setWarning(next.network_warning||'');
      activity.finish(true,'所选本地身份已直接删除。');
    }catch(e){setError(String(e));activity.finish(false,String(e));await refreshAfterError();}
    finally{setBusy(false);}
  }
  async function saveName(){
    if(!rename||busy)return;
    setBusy(true);
    try{setState(await invoke<IdentityState>('identities_rename',{id:rename.id,value:renameValue}));setRename(null);setError('');}
    catch(e){setError(String(e));}finally{setBusy(false);}
  }
  const time=(at:number)=>new Date(at).toLocaleString(locale==='zh'?'zh-CN':'en-US',{month:'short',day:'numeric',hour:'2-digit',minute:'2-digit'});
  return <section className="space-y-4">
    <header className="flex items-center gap-2">
      <div className="flex-1"><h1 className="text-lg font-semibold">{t('账号')}</h1><p className="text-xs text-muted">{t('每个账号的登录与本地数据分开保存，切换前自动暂存当前账号。')}</p></div>
      <Button size="sm" variant="ghost" disabled={locked||loading} onClick={()=>void read()} aria-label={t('刷新')}><RefreshCw className={cn('size-3.5',loading&&'animate-spin')} aria-hidden="true"/>{t('刷新')}</Button>
      <Button size="sm" variant="primary" disabled={blocked||current?.present===false} onClick={()=>choose('empty')}><Plus className="size-3.5" aria-hidden="true"/>{t('空账号')}</Button>
    </header>
    {error&&<p role="alert" className="rounded-lg border border-block/40 bg-block-bg px-3 py-2 text-sm text-block">{t(error)}</p>}
    {warning&&<p role="status" className="rounded-lg border border-border bg-muted-bg px-3 py-2 text-sm">{t(warning)}</p>}
    {state?.recovery&&<Card role="alert" className="flex items-center gap-3 border-block/40"><strong className="flex-1 text-sm">{t('有一次身份切换尚未完成')}</strong><Button size="sm" variant="primary" disabled={locked||loading} onClick={()=>choose('recovery')}>{t('恢复原身份')}</Button></Card>}
    <div className="account-list space-y-2" aria-label={t('账号列表')}>
      {current?.present===false&&<AccountCard name={t('空账号')} badge={<Badge tone="info">{t('使用中')}</Badge>} current detail={t('没有登录任何账号。新账号登录前请先切到这里。')}/>}
      {(loading&&!state||current?.present)&&<AccountCard name={currentName} current badge={<Badge tone="info">{loading&&!state?t('检查中'):t('使用中')}</Badge>}
        detail={current?.email?<><span>{current.email}</span>{current.organization&&<span className="text-muted"> · {current.organization}</span>}</>:t(current?.unavailable?'账号信息暂不可读':'账号信息未识别')}
        extra={<>{state&&<span>{state.running?t('Claude 运行中'):t('Claude 未运行')}</span>}{state?.browser.enabled&&<Badge>{t('独立浏览器')}</Badge>}</>}
        menu={[{label:t('直接删除'),danger:true,disabled:blocked,onSelect:()=>confirmDelete('current')}]} menuDisabled={blocked}/>}
      {saved.map(row=><AccountCard key={row.id} name={row.name} detail={`${formatBytes(row.bytes)} · ${time(row.saved_at)}`}
        extra={state?.browser.bound_ids.includes(row.id)&&<Badge>{t('独立浏览器')}</Badge>}
        action={<Button size="sm" disabled={blocked} onClick={()=>choose(row)}>{t('切换')}</Button>}
        menu={[{label:t('改名'),disabled:blocked,onSelect:()=>{setRename(row);setRenameValue(row.name);}},{label:t('删除'),danger:true,disabled:blocked,onSelect:()=>confirmDelete(row)}]} menuDisabled={blocked}/>)}
    </div>
    <Card className="space-y-3">
      <details className="identity-policy"><summary className="cursor-pointer text-sm"><strong>{t('保留在工作区')}</strong> <span className="text-xs text-muted">{kept.length} / {policyRows.length} · {t('切换账号时这些内容不跟着走')}</span></summary>
        <div className="mt-3 grid grid-cols-2 gap-2 sm:grid-cols-4">{policyRows.map(([key,label])=><Checkbox key={key} label={t(label)} checked={policy[key]} disabled={locked} onCheckedChange={checked=>setPolicy(old=>({...old,[key]:checked}))}/>)}</div>
      </details>
      <Switch label={t('联动浏览器')} checked={state?.browser.enabled??false} disabled={blocked} onCheckedChange={value=>{setUseExisting(false);setBrowserReview(value);setError('');}}/>
    </Card>
    {state?.browser_warning&&<Card role="alert" className="flex items-center gap-3 border-block/40"><span className="flex-1 text-sm">{t(state.browser_warning)}</span><Button size="sm" disabled={blocked} onClick={()=>void retryBrowser()}>{t('打开浏览器')}</Button></Card>}
    <CleanupActivity activity={activity.state}/>
    <AppDialog open={!!review} onOpenChange={open=>{if(!open&&!busy)setReview(null);}} title={review==='recovery'?t('恢复原身份'):review==='empty'?t('切换到空身份'):t('切换身份')} description={t('先压缩暂存当前 Claude Code 与 Desktop 状态，再切换。将结束已识别的 Claude 进程，未保存任务会中断。')}
      footer={<><Button data-dialog-cancel onClick={()=>setReview(null)}>{t('取消')}</Button><Button variant="primary" disabled={busy||loading||!state||!name.trim()} onClick={()=>void change()}>{review==='recovery'?t('确认恢复'):t('确认暂存并切换')}</Button></>}>
      <div className="space-y-3">
        <label className="block space-y-1"><span className="text-xs text-muted">{t('当前身份保存为')}</span><Input maxLength={64} value={name} onChange={e=>setName(e.target.value)}/></label>
        {typeof review==='object'&&review&&<p><span className="text-muted">{t('目标身份')}</span> <strong>{review.name}</strong></p>}
        <p><span className="text-muted">{t('留在工作区')}</span> <strong>{kept.join(' · ')||t('无')}</strong></p>
        {state?.browser.enabled&&<p>{t('将关闭并重开关联浏览器，请先保存网页任务。')}</p>}
        <p className="text-muted">{t('只管理本地状态；失效的登录可能需要重新认证。')}</p>
        <p className="text-muted">{t('网络代理设置始终留在本机，不随身份切换。')}</p>
      </div>
    </AppDialog>
    <AppDialog alert open={!!deletion} onOpenChange={open=>{if(!open&&!busy)setDeletion(null);}} title={deletion==='current'?t('直接删除当前身份'):t('删除暂存身份')} description={deletion==='current'?t('将结束 Claude 进程并删除所选本地数据。不会新增备份，已有暂存身份不受影响。'):t('永久删除这份暂存文件。当前工作区不受影响。')}
      footer={<><Button data-dialog-cancel onClick={()=>setDeletion(null)}>{t('取消')}</Button><Button variant="danger" disabled={busy||!accepted} onClick={()=>void remove()}>{t('确认直接删除')}</Button></>}>
      <div className="space-y-3">
        <p className="rounded-lg border border-block/40 bg-block-bg px-3 py-2 font-semibold text-block">{t('警告：此操作不可撤销，不会创建新备份。')}</p>
        {deletion==='current'?<><p><span className="text-muted">{t('删除范围')}</span> <strong>{[t('登录与身份数据'),...removed].join(' · ')}</strong></p><p><span className="text-muted">{t('留在工作区')}</span> <strong>{kept.join(' · ')||t('无')}</strong></p></>:<strong>{deletion?.name}</strong>}
        {typeof deletion==='object'&&deletion&&state?.browser.bound_ids.includes(deletion.id)&&<div><Checkbox label={t('同时永久删除关联浏览器资料')} checked={deleteBrowser} onCheckedChange={setDeleteBrowser}/></div>}
        <div><Checkbox checked={accepted} onCheckedChange={setAccepted} label={t('我知道此操作无法撤销')}/></div>
      </div>
    </AppDialog>
    <AppDialog open={browserReview!==null} onOpenChange={open=>{if(!open&&!busy)setBrowserReview(null);}} title={t(browserReview?'联动浏览器':'关闭浏览器联动')} description={t('将正常关闭并重开浏览器，请先保存网页任务。')}
      footer={<><Button disabled={busy} onClick={()=>setBrowserReview(null)}>{t('取消')}</Button><Button variant="primary" disabled={busy} loading={busy} onClick={()=>void configureBrowser()}>{t('确认')}</Button></>}>
      <div className="space-y-3">
        <p>{t(browserReview?'每个身份使用独立网页资料和稳定指纹，切换时自动恢复。首次仍需登录网站。':'恢复使用原有通用浏览器资料。各身份的独立资料会保留。')}</p>
        {browserReview&&<Checkbox label={t('将现有浏览器资料关联给当前身份')} checked={useExisting} disabled={busy} onCheckedChange={setUseExisting}/>}
        <p className="text-muted">{t('默认创建新资料，现有资料不会删除。资料隔离不会改变出口 IP。')}</p>
        {error&&<p role="alert" className="text-block">{t(error)}</p>}
      </div>
    </AppDialog>
    <AppDialog open={!!rename} onOpenChange={open=>{if(!open&&!busy)setRename(null);}} title={t('重命名身份')}
      footer={<><Button onClick={()=>setRename(null)}>{t('取消')}</Button><Button variant="primary" disabled={busy||!renameValue.trim()} onClick={()=>void saveName()}>{t('保存')}</Button></>}>
      <Input aria-label={t('身份名称')} maxLength={64} value={renameValue} onChange={e=>setRenameValue(e.target.value)}/>
      {error&&<p role="alert" className="mt-2 text-block">{t(error)}</p>}
    </AppDialog>
  </section>;
}
function AccountCard({name,detail,badge,extra,action,menu,menuDisabled,current=false}:{name:string;detail:ReactNode;badge?:ReactNode;extra?:ReactNode;action?:ReactNode;menu?:MenuItem[];menuDisabled?:boolean;current?:boolean}){
  const {t}=useI18n();
  return <Card role="group" aria-label={name} data-current={current||undefined} className={cn('account-card flex items-center gap-3 p-3',current&&'border-primary/60 ring-1 ring-primary/30')}>
    <div aria-hidden="true" className="grid size-9 shrink-0 place-items-center rounded-full bg-muted-bg text-sm font-semibold text-muted">{Array.from(name)[0]?.toUpperCase()||<UserRound className="size-4"/>}</div>
    <div className="min-w-0 flex-1">
      <div className="flex items-center gap-2"><h3 className="truncate text-sm font-semibold">{name}</h3>{badge}</div>
      <p className="truncate text-xs text-muted">{detail}</p>
      {extra&&<div className="mt-1 flex items-center gap-2 text-xs text-muted">{extra}</div>}
    </div>
    {action}
    {menu&&<Menu items={menu} trigger={<Button size="icon" variant="ghost" disabled={menuDisabled} aria-label={t('更多操作')}><MoreHorizontal className="size-4" aria-hidden="true"/></Button>}/>}
  </Card>;
}
