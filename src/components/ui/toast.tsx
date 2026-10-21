// [INPUT]: Radix Toast 与调用方给出的标题、原因和色调。
// [OUTPUT]: ToastProvider、useToast()；右下角堆叠，block 红、info 蓝，无绿色；保存文案键，渲染时按当前语言翻译。
// [POS]: 启动失败、设置保存等一次性结果的提示通道；不持久化、不写日志。
import {createContext,useCallback,useContext,useState,type ReactNode} from 'react';
import {Toast as Primitive} from 'radix-ui';
import {cn} from './cn';
import {useI18n} from '../../i18n';
type Values=Record<string,string|number>;
type Item={id:number;title:string;values?:Values;description?:string;tone:'block'|'info'};
type Show=(item:Omit<Item,'id'>)=>void;
const Context=createContext<Show>(()=>{});
let sequence=0;
export function ToastProvider({children}:{children:ReactNode}){
  const {t}=useI18n();
  const [items,setItems]=useState<Item[]>([]);
  const show=useCallback<Show>(item=>setItems(old=>[...old.slice(-3),{...item,id:++sequence}]),[]);
  return <Context.Provider value={show}><Primitive.Provider duration={8000}>
    {children}
    {items.map(item=><Primitive.Root key={item.id} role={item.tone==='block'?'alert':'status'} data-tone={item.tone}
      onOpenChange={open=>{if(!open)setItems(old=>old.filter(row=>row.id!==item.id));}}
      className={cn('rounded-xl border bg-surface p-3 shadow-lg',item.tone==='block'?'border-block/40':'border-primary/40')}>
      <Primitive.Title className={cn('text-sm font-semibold',item.tone==='block'?'text-block':'text-primary')}>{t(item.title,item.values)}</Primitive.Title>
      {item.description&&<Primitive.Description className="mt-1 text-xs text-fg">{t(item.description)}</Primitive.Description>}
    </Primitive.Root>)}
    <Primitive.Viewport className="fixed bottom-4 right-4 z-[60] flex w-80 flex-col gap-2 outline-none"/>
  </Primitive.Provider></Context.Provider>;
}
export const useToast=()=>useContext(Context);
