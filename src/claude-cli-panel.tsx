/**
 * [INPUT]: claude_cli_read / claude_cli_manage / cleaner_stop 命令、共享确认窗与进度日志。
 * [OUTPUT]: CLI 安装状态与确认后的卸载 / 重装。
 * [POS]: 设置页「关于」里的程序维护区；保留身份暂存与个人资料。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {useI18n} from './i18n';
import {AppDialog} from './components/ui/dialog';
import {Button} from './components/ui/button';
import {CleanupActivity,useCleanupActivity} from './cleaner-activity';
import type {Scan} from './cleaner-model';
type Status={method:string;path:string;version:string};
type Action='uninstall'|'reinstall';
export function ClaudeCliPanel({blocked=false,onBusy}:{blocked?:boolean;onBusy?:(busy:boolean)=>void}){
  const {t}=useI18n();
  const [status,setStatus]=useState<Status|null>(null),[loading,setLoading]=useState(true),[action,setAction]=useState<Action|null>(null),[review,setReview]=useState<Action|null>(null),[error,setError]=useState('');
  const activity=useCleanupActivity();
  useEffect(()=>{void read();},[]);
  useEffect(()=>{onBusy?.(!!action||!!review);return()=>onBusy?.(false);},[action,review,onBusy]);
  async function read(){setLoading(true);try{setStatus(await invoke<Status>('claude_cli_read'));}catch(e){setError(String(e));}finally{setLoading(false);}}
  async function run(){
    if(!review)return;
    const selected=review;setReview(null);setAction(selected);setError('');
    const onProgress=activity.start('cli');
    try{
      const stopped=await invoke<Scan>('cleaner_stop');if(stopped.running)throw new Error('仍有 Claude 进程在运行。');
      setStatus(await invoke<Status>('claude_cli_manage',{action:selected,onProgress}));
      activity.finish(true,selected==='uninstall'?'Claude Code CLI 已卸载。':'Claude Code CLI 已重装，版本核验通过。');
    }catch(e){setError(String(e));activity.finish(false,String(e));await read();}
    finally{setAction(null);}
  }
  return <section className="space-y-3">
    <div className="flex items-center gap-3">
      <div className="flex-1"><h3 className="text-sm font-semibold">{t('Claude Code CLI')}</h3><p className="text-xs text-muted">{loading?t('正在检查安装状态…'):status?.method?status.version||t('已安装'):t('未安装')}</p></div>
      <Button size="sm" disabled={blocked||loading||!!action||!status?.method} onClick={()=>setReview('uninstall')}>{t('一键卸载')}</Button>
      <Button size="sm" disabled={blocked||loading||!!action} onClick={()=>setReview('reinstall')}>{t('一键重装')}</Button>
    </div>
    {error&&<p role="alert" className="text-xs text-block">{t(error)}</p>}
    <CleanupActivity activity={activity.state}/>
    <AppDialog open={!!review} onOpenChange={open=>{if(!open)setReview(null);}} title={review==='uninstall'?t('卸载 Claude Code CLI'):t('重装 Claude Code CLI')} description={t('将结束已识别的 Claude 进程；程序维护保留身份暂存与个人资料。')}
      footer={<><Button onClick={()=>setReview(null)}>{t('取消')}</Button><Button variant="primary" onClick={()=>void run()}>{t('确认')}</Button></>}/>
  </section>;
}
