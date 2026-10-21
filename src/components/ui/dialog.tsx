// [INPUT]: Radix Dialog / AlertDialog 受控状态、标题、说明、内容与底部按钮。
// [OUTPUT]: AppDialog；alert 模式默认聚焦 [data-dialog-cancel]，遮罩点击不关闭，关闭后焦点回到原处。
// [POS]: 身份切换、删除、CLI 维护与改名等全部确认窗的唯一外壳。
import {useId,useRef,type ReactNode} from 'react';
import {Dialog as DialogPrimitive,AlertDialog as AlertPrimitive} from 'radix-ui';
import {X} from 'lucide-react';
import {useI18n} from '../../i18n';
import {cn} from './cn';
type Props={open:boolean;onOpenChange:(open:boolean)=>void;title:string;description?:string;children?:ReactNode;footer?:ReactNode;className?:string;alert?:boolean};
export function AppDialog({open,onOpenChange,title,description,children,footer,className,alert=false}:Props){
  const {t}=useI18n();
  const titleId=useId(),descriptionId=useId();
  const Primitive=alert?AlertPrimitive:DialogPrimitive;
  const content=useRef<HTMLDivElement>(null);
  const previousFocus=useRef<HTMLElement|null>(null);
  return <Primitive.Root open={open} onOpenChange={onOpenChange}>
    <Primitive.Portal>
      <Primitive.Overlay className="fixed inset-0 z-40 bg-black/40"/>
      <Primitive.Content ref={content} tabIndex={-1} aria-labelledby={titleId} aria-describedby={description?descriptionId:undefined}
        className={cn('fixed left-1/2 top-1/2 z-50 flex max-h-[85vh] w-[min(560px,calc(100vw-32px))] -translate-x-1/2 -translate-y-1/2 flex-col rounded-2xl border border-border bg-surface text-fg shadow-xl outline-none',className)}
        onOpenAutoFocus={event=>{
          previousFocus.current=document.activeElement instanceof HTMLElement?document.activeElement:null;
          event.preventDefault();
          const safe=alert?content.current?.querySelector<HTMLElement>('[data-dialog-cancel]'):null;
          (safe||content.current)?.focus();
        }}
        onCloseAutoFocus={event=>{event.preventDefault();previousFocus.current?.focus();}}>
        <div className="flex items-start justify-between gap-4 px-5 pt-5">
          <div className="space-y-1">
            <Primitive.Title id={titleId} className="text-base font-semibold">{title}</Primitive.Title>
            {description&&<Primitive.Description id={descriptionId} className="text-sm text-muted">{description}</Primitive.Description>}
          </div>
          {!alert&&<button type="button" aria-label={t('关闭弹窗')} onClick={()=>onOpenChange(false)} className="rounded-md p-1 text-muted hover:bg-muted-bg"><X className="size-4" aria-hidden="true"/></button>}
        </div>
        <div className="min-h-0 overflow-y-auto px-5 py-4 text-sm">{children}</div>
        {footer&&<div className="flex justify-end gap-2 border-t border-border px-5 py-3">{footer}</div>}
      </Primitive.Content>
    </Primitive.Portal>
  </Primitive.Root>;
}
