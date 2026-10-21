/**
 * [INPUT]: settings_read、line_check（经 useLineCheck）、ui_language_read、窗口可见与托盘拦截事件、各页面组件。
 * [OUTPUT]: CC-Guard 主窗口：左侧栏（账号、连接，底部设置），顶部线路状态条，首次设置向导，Toast。
 * [POS]: 单窗口编排层；隐藏到托盘时停止轮询；身份事务进行中锁定导航。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {useCallback,useEffect,useState,type ReactNode} from 'react';
import {createRoot} from 'react-dom/client';
import {invoke} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
import {getCurrentWindow} from '@tauri-apps/api/window';
import {Activity,Settings as SettingsIcon,UsersRound} from 'lucide-react';
import type {Settings} from './contract';
import {needsSetup} from './line-model';
import {StatusBar,useLineCheck} from './status-bar';
import {IdentityPanel} from './identities';
import {LaunchBar} from './launch-bar';
import {ConnectionsPanel} from './connections';
import {SettingsPanel} from './settings';
import {SetupWizard} from './wizard';
import {ToastProvider,useToast} from './components/ui/toast';
import {TooltipProvider} from './components/ui/tooltip';
import {cn} from './components/ui/cn';
import {setLocale,useI18n,type Locale} from './i18n';
import {applyTheme} from './theme';
import logo from '../assets/cc-guard.svg';
import './index.css';
type Page='accounts'|'connections'|'settings';
const pages:{page:Page;label:string;icon:typeof Activity}[]=[{page:'accounts',label:'账号',icon:UsersRound},{page:'connections',label:'连接',icon:Activity}];
export function App(){
  const {t}=useI18n();
  const toast=useToast();
  const [page,setPage]=useState<Page>('accounts'),[busy,setBusy]=useState(false);
  const [settings,setSettings]=useState<Settings|null>(null),[settingsError,setSettingsError]=useState('');
  const [wizard,setWizard]=useState(false);
  const [visible,setVisible]=useState(document.visibilityState!=='hidden');
  const line=useLineCheck(visible);
  useEffect(()=>{
    void getCurrentWindow().isVisible().then(setVisible).catch(()=>{});
    const off=listen<boolean>('window-visible',event=>setVisible(event.payload));
    return()=>{void off.then(stop=>stop());};
  },[]);
  useEffect(()=>{
    const off=listen<string>('proxy-launch-blocked',event=>toast({tone:'block',title:'没有启动',description:event.payload}));
    return()=>{void off.then(stop=>stop());};
  },[toast]);
  useEffect(()=>{void invoke<Locale>('ui_language_read').then(setLocale).catch(()=>{});},[]);
  useEffect(()=>{
    invoke<Settings>('settings_read').then(next=>{setSettings(next);setSettingsError('');if(needsSetup(next))setWizard(true);}).catch(e=>setSettingsError(String(e)));
  },[]);
  const recheck=useCallback(()=>{void line.run();},[line.run]);
  const nav=(target:Page,label:string,Icon:typeof Activity)=><button key={target} type="button" disabled={busy} aria-current={page===target?'page':undefined} onClick={()=>setPage(target)}
    className={cn('flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm transition-colors disabled:opacity-50',page===target?'bg-primary/10 font-semibold text-primary':'text-fg hover:bg-muted-bg')}>
    <Icon className="size-4" aria-hidden="true"/>{t(label)}</button>;
  return <div className="flex h-full">
    <aside className="flex w-48 shrink-0 flex-col border-r border-border bg-surface p-3">
      <div className="mb-4 flex items-center gap-2 px-2 py-1"><img src={logo} alt="" width={26} height={26} draggable={false}/><strong className="text-sm">CC-Guard</strong><span className="text-xs text-muted">v{__APP_VERSION__}</span></div>
      <nav aria-label={t('主要页面')} className="space-y-1">{pages.map(item=>nav(item.page,item.label,item.icon))}</nav>
      <div className="mt-auto">{nav('settings','设置',SettingsIcon)}</div>
    </aside>
    <div className="flex min-w-0 flex-1 flex-col">
      <div className="p-4 pb-0"><StatusBar line={line}/></div>
      <main className="min-h-0 flex-1 overflow-y-auto p-4">
        {settingsError&&<p role="alert" className="mb-3 rounded-lg border border-block/40 bg-block-bg px-3 py-2 text-sm text-block">{t(settingsError)}</p>}
        {settings&&needsSetup(settings)&&!wizard&&<div role="note" className="mb-3 flex items-center gap-3 rounded-lg border border-primary/40 bg-primary/10 px-3 py-2 text-sm"><span className="flex-1">{t('线路还没设置好，Claude 无法启动。')}</span><button type="button" className="font-semibold text-primary" onClick={()=>setWizard(true)}>{t('开始设置')}</button></div>}
        {page==='accounts'&&<div className="space-y-4"><IdentityPanel onBusy={setBusy}/><LaunchBar disabled={busy} onCheck={line.accept}/></div>}
        {page==='connections'&&<ConnectionsPanel active={visible}/>}
        {page==='settings'&&<SettingsPanel settings={settings} onSettings={setSettings} onCheck={line.accept} onRecheck={recheck}/>}
      </main>
    </div>
    {settings&&wizard&&<SetupWizard open settings={settings} onSettings={setSettings} onCheck={line.accept} onClose={()=>setWizard(false)}/>}
  </div>;
}
export function Root({children}:{children:ReactNode}){
  return <TooltipProvider delayDuration={500}><ToastProvider>{children}</ToastProvider></TooltipProvider>;
}
const mount=document.getElementById('app');
if(mount){applyTheme();createRoot(mount).render(<Root><App/></Root>);}
