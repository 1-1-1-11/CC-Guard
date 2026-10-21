// [INPUT]: Radix Select 受控值与选项列表。
// [OUTPUT]: Select，下拉选择（节点、语言、主题）。
// [POS]: 有限选项的表单控件；选项值不能为空串（Radix 约束）。
import {Select as Primitive} from 'radix-ui';
import {ChevronDown} from 'lucide-react';
import {cn} from './cn';
export type Option={value:string;label:string};
type Props={value:string;onValueChange:(value:string)=>void;options:Option[];placeholder?:string;disabled?:boolean;className?:string;id?:string;'aria-label'?:string};
export function Select({value,onValueChange,options,placeholder,disabled,className,...aria}:Props){
  return <Primitive.Root value={value||undefined} onValueChange={onValueChange} disabled={disabled}>
    <Primitive.Trigger {...aria} className={cn('inline-flex h-9 w-full items-center justify-between gap-2 rounded-lg border border-border bg-surface px-3 text-sm text-fg data-[placeholder]:text-muted disabled:opacity-50 focus-visible:outline-2 focus-visible:outline-primary',className)}>
      <Primitive.Value placeholder={placeholder}/>
      <Primitive.Icon><ChevronDown className="size-4 text-muted" aria-hidden="true"/></Primitive.Icon>
    </Primitive.Trigger>
    <Primitive.Portal><Primitive.Content position="popper" sideOffset={4} className="z-50 max-h-72 min-w-[var(--radix-select-trigger-width)] overflow-hidden rounded-lg border border-border bg-surface shadow-lg">
      <Primitive.Viewport className="p-1">{options.map(option=>
        <Primitive.Item key={option.value} value={option.value} className="cursor-pointer rounded-md px-2 py-1.5 text-sm text-fg outline-none data-[highlighted]:bg-muted-bg data-[state=checked]:font-semibold">
          <Primitive.ItemText>{option.label}</Primitive.ItemText>
        </Primitive.Item>)}</Primitive.Viewport>
    </Primitive.Content></Primitive.Portal>
  </Primitive.Root>;
}
