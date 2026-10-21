// [INPUT]: 原生 button 属性、视觉变体与加载状态。
// [OUTPUT]: Button，主按钮 / 次按钮 / 幽灵 / 危险四种变体。
// [POS]: 全部页面的按钮；绿色不出现在按钮上，绿色只属于 pass。
import type {ButtonHTMLAttributes} from 'react';
import {cn} from './cn';
type Variant='primary'|'secondary'|'ghost'|'danger';
type Size='sm'|'md'|'icon';
const variants:Record<Variant,string>={
  primary:'bg-primary text-primary-fg hover:bg-primary/90',
  secondary:'border border-border bg-surface text-fg hover:bg-muted-bg',
  ghost:'text-fg hover:bg-muted-bg',
  danger:'bg-block text-white hover:bg-block/90',
};
const sizes:Record<Size,string>={sm:'h-8 px-3 text-xs gap-1.5',md:'h-9 px-4 text-sm gap-2',icon:'h-8 w-8 justify-center'};
export function Button({variant='secondary',size='md',loading=false,className,disabled,children,...props}:ButtonHTMLAttributes<HTMLButtonElement>&{variant?:Variant;size?:Size;loading?:boolean}){
  return <button type="button" {...props} disabled={disabled||loading} data-loading={loading||undefined}
    className={cn('inline-flex shrink-0 items-center rounded-lg font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary disabled:pointer-events-none disabled:opacity-50',variants[variant],sizes[size],className)}>
    {loading&&<span aria-hidden="true" className="size-3.5 animate-spin rounded-full border-2 border-current border-r-transparent"/>}
    {children}
  </button>;
}
