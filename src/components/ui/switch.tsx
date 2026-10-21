// [INPUT]: Radix Switch 受控状态与标签。
// [OUTPUT]: Switch，带可点击文字标签的开关。
// [POS]: 布尔选项（如联动浏览器）。
import {useId} from 'react';
import {Switch as Primitive} from 'radix-ui';
export function Switch({label,checked,onCheckedChange,disabled}:{label:string;checked:boolean;onCheckedChange:(value:boolean)=>void;disabled?:boolean}){
  const id=useId();
  return <span className="inline-flex items-center gap-2 text-sm">
    <Primitive.Root id={id} checked={checked} onCheckedChange={onCheckedChange} disabled={disabled}
      className="relative h-5 w-9 shrink-0 rounded-full bg-border transition-colors data-[state=checked]:bg-primary disabled:opacity-50 focus-visible:outline-2 focus-visible:outline-primary">
      <Primitive.Thumb className="block size-4 translate-x-0.5 rounded-full bg-white shadow transition-transform data-[state=checked]:translate-x-[18px]"/>
    </Primitive.Root>
    <label htmlFor={id} className="cursor-pointer select-none">{label}</label>
  </span>;
}
