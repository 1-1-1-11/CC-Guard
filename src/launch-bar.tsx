// [INPUT]: launch 命令（后端启动前重新检查线路）与 Toast。
// [OUTPUT]: 账号页底部「启动 Claude Code / 桌面版 / 网页版」；被拦或出错用 Toast 显示原因，并把本次检查结果交给状态条。
// [POS]: 唯一启动入口；网页版照旧走 launch('web')。
import {useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {Globe,Monitor,Terminal} from 'lucide-react';
import type {LaunchEntry,LaunchResult,LineCheck} from './contract';
import {Button} from './components/ui/button';
import {Card} from './components/ui/card';
import {useToast} from './components/ui/toast';
import {useI18n} from './i18n';
const entries:{entry:LaunchEntry;label:string;icon:typeof Terminal}[]=[
  {entry:'cli',label:'启动 Claude Code',icon:Terminal},
  {entry:'desktop',label:'启动桌面版',icon:Monitor},
  {entry:'web',label:'启动网页版',icon:Globe},
];
export function LaunchBar({disabled,onCheck}:{disabled?:boolean;onCheck:(check:LineCheck)=>void}){
  const {t}=useI18n();
  const toast=useToast();
  const [pending,setPending]=useState<LaunchEntry|null>(null);
  async function launch(entry:LaunchEntry,label:string){
    setPending(entry);
    try{
      const result=await invoke<LaunchResult>('launch',{entry});
      onCheck(result.check);
      if(result.launched)toast({tone:'info',title:'已启动：{v0}',values:{v0:label}});
      else toast({tone:'block',title:'没有启动：{v0}',values:{v0:label},description:result.reason||result.check.reason});
    }catch(e){toast({tone:'block',title:'没有启动：{v0}',values:{v0:label},description:String(e)});}
    finally{setPending(null);}
  }
  return <Card className="flex flex-wrap items-center gap-2 p-3">
    <span className="mr-auto text-xs text-muted">{t('每次启动前都会重新检查线路，没通过就不启动。')}</span>
    {entries.map(({entry,label,icon:Icon})=><Button key={entry} variant={entry==='cli'?'primary':'secondary'} loading={pending===entry} disabled={disabled||!!pending} onClick={()=>void launch(entry,label)}>
      {pending!==entry&&<Icon className="size-4" aria-hidden="true"/>}{t(label)}
    </Button>)}
  </Card>;
}
