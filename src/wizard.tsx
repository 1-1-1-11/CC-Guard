// [INPUT]: settings_write / line_nodes / line_check 与当前设置。
// [OUTPUT]: 首次设置向导（controller 空或 port 为 0 时弹出）：① 连上 Clash ② 选端口和节点 ③ 检查并记住出口。
// [POS]: 小白第一次打开的入口；每一步都由后端真实读回确认，不预设成功；secret 只写不回显。
import {useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import type {LineCheck,Settings,SettingsPatch} from './contract';
import {AppDialog} from './components/ui/dialog';
import {Badge} from './components/ui/badge';
import {Button} from './components/ui/button';
import {Input} from './components/ui/input';
import {cn} from './components/ui/cn';
import {Field,NodeSelect,parsePort} from './settings';
import {useI18n} from './i18n';
const steps=['连上 Clash','选端口和节点','检查并记住出口'];
export function SetupWizard({open,settings,onSettings,onCheck,onClose}:{open:boolean;settings:Settings;onSettings:(settings:Settings)=>void;onCheck:(check:LineCheck)=>void;onClose:()=>void}){
  const {t}=useI18n();
  const [step,setStep]=useState(0),[busy,setBusy]=useState(false),[error,setError]=useState('');
  const [controller,setController]=useState(settings.line.controller||'127.0.0.1:9097'),[secret,setSecret]=useState('');
  const [nodes,setNodes]=useState<string[]>([]),[port,setPort]=useState(settings.line.port?String(settings.line.port):''),[node,setNode]=useState(settings.line.node);
  const [check,setCheck]=useState<LineCheck|null>(null);
  async function act(work:()=>Promise<void>){setBusy(true);setError('');try{await work();}catch(e){setError(e instanceof Error?e.message:String(e));}finally{setBusy(false);}}
  const write=async(line:NonNullable<SettingsPatch['line']>)=>{const next=await invoke<Settings>('settings_write',{patch:{line}});onSettings(next);return next;};
  const connect=()=>act(async()=>{
    if(!controller.trim())throw new Error('请填写 Clash 控制地址。');
    await write({controller:controller.trim(),...(secret?{secret}:{})});
    setSecret('');
    setNodes(await invoke<string[]>('line_nodes'));
    setStep(1);
  });
  const choose=()=>act(async()=>{
    const value=parsePort(port);
    if(value===null)throw new Error('专用端口要填 1–65535 的数字。');
    if(!node)throw new Error('请选择一个节点。');
    await write({port:value,node});
    setStep(2);
    const result=await invoke<LineCheck>('line_check');setCheck(result);onCheck(result);
  });
  const recheck=()=>act(async()=>{const result=await invoke<LineCheck>('line_check');setCheck(result);onCheck(result);});
  const remember=()=>act(async()=>{
    if(!check?.ip)throw new Error('还没有读到出口 IP，请先重新检查。');
    await write({expected_ip:check.ip});
    const result=await invoke<LineCheck>('line_check');setCheck(result);onCheck(result);
  });
  const passed=check?.verdict==='pass';
  return <AppDialog open={open} onOpenChange={value=>{if(!value&&!busy)onClose();}} title={t('第一次使用：三步配好 Claude 专用线路')}
    footer={<>
      <Button className={passed?'':'mr-auto'} variant={passed?'primary':'ghost'} disabled={busy} onClick={onClose}>{t(passed?'完成':'稍后再说')}</Button>
      {step>0&&!passed&&<Button disabled={busy} onClick={()=>{setError('');setStep(step-1);}}>{t('上一步')}</Button>}
      {step===0&&<Button variant="primary" loading={busy} onClick={()=>void connect()}>{t('连接并下一步')}</Button>}
      {step===1&&<Button variant="primary" loading={busy} onClick={()=>void choose()}>{t('保存并检查')}</Button>}
      {step===2&&!passed&&<Button loading={busy} onClick={()=>void recheck()}>{t('重新检查')}</Button>}
      {step===2&&!passed&&<Button variant="primary" disabled={busy||!check?.ip} onClick={()=>void remember()}>{t('记住这个出口')}</Button>}
    </>}>
    <ol className="mb-4 flex gap-2" aria-label={t('设置步骤')}>{steps.map((label,index)=><li key={label} aria-current={index===step?'step':undefined}
      className={cn('flex-1 rounded-lg border px-2 py-1.5 text-xs',index===step?'border-primary bg-primary/10 font-semibold text-primary':index<step?'border-border text-fg':'border-border text-muted')}>{index+1}. {t(label)}</li>)}</ol>
    <div className="space-y-3">
      {step===0&&<>
        <p className="text-muted">{t('打开 Clash Verge → 设置 → 外部控制，把地址和密钥填到这里。')}</p>
        <Field label={t('Clash 控制地址')} htmlFor="wizard-controller"><Input id="wizard-controller" value={controller} onChange={e=>setController(e.target.value)} spellCheck={false}/></Field>
        <Field label={t('控制密钥')} htmlFor="wizard-secret" hint={t('Clash 没设密钥时留空。')}><Input id="wizard-secret" type="password" autoComplete="off" value={secret} onChange={e=>setSecret(e.target.value)} placeholder={t(settings.line.secret_set?'已设置':'未设置')}/></Field>
      </>}
      {step===1&&<>
        <p className="text-muted">{t('在 Clash 里给 Claude 单独开一个入站端口，并把它指向下面选的节点。')}</p>
        <Field label={t('Claude 专用端口')} htmlFor="wizard-port"><Input id="wizard-port" inputMode="numeric" value={port} onChange={e=>setPort(e.target.value)} placeholder="7899"/></Field>
        <Field label={t('指定节点')} htmlFor="wizard-node"><NodeSelect id="wizard-node" value={node} onChange={setNode} nodes={nodes}/></Field>
      </>}
      {step===2&&<>
        {!check&&<p className="text-muted">{t('正在检查线路…')}</p>}
        {check&&<div className="space-y-2 rounded-lg border border-border p-3" data-verdict={check.verdict}>
          <Badge tone={check.verdict==='pass'?'pass':check.verdict==='block'?'block':'unknown'}>{t(check.verdict==='pass'?'通过':check.verdict==='block'?'拦住':'无法确认')}</Badge>
          <p>{t(check.reason)}</p>
          {check.ip&&<p className="text-muted">{t('出口 IP {v0}',{v0:check.ip})} · {[check.country,check.timezone].filter(Boolean).join(' · ')}</p>}
        </div>}
        {check&&!passed&&check.ip&&<p className="text-muted">{t('如果这就是你要的出口，点「记住这个出口」，以后出口变了就会拦住。')}</p>}
        {passed&&<p>{t('配好了。以后每次启动 Claude 前都会先检查这条线路。')}</p>}
      </>}
      {error&&<p role="alert" className="text-block">{t(error)}</p>}
    </div>
  </AppDialog>;
}
