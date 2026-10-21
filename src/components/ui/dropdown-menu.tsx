// [INPUT]: Radix DropdownMenu 与菜单项。
// [OUTPUT]: Menu，「⋯」更多操作菜单。
// [POS]: 账号卡片的改名 / 删除入口。
import type {ReactNode} from 'react';
import {DropdownMenu as Primitive} from 'radix-ui';
import {cn} from './cn';
export type MenuItem={label:string;onSelect:()=>void;danger?:boolean;disabled?:boolean};
export function Menu({trigger,items}:{trigger:ReactNode;items:MenuItem[]}){
  return <Primitive.Root modal={false}><Primitive.Trigger asChild>{trigger}</Primitive.Trigger>
    <Primitive.Portal><Primitive.Content align="end" sideOffset={4} className="z-50 min-w-32 rounded-lg border border-border bg-surface p-1 shadow-lg">
      {items.map(item=><Primitive.Item key={item.label} disabled={item.disabled} onSelect={item.onSelect}
        className={cn('cursor-pointer rounded-md px-2 py-1.5 text-sm outline-none data-[highlighted]:bg-muted-bg data-[disabled]:opacity-50',item.danger?'text-block':'text-fg')}>{item.label}</Primitive.Item>)}
    </Primitive.Content></Primitive.Portal>
  </Primitive.Root>;
}
