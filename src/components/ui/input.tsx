// [INPUT]: 原生 input 属性。
// [OUTPUT]: Input，统一边框、焦点与禁用样式。
// [POS]: 表单输入；secret 输入框由调用方保证只写不回显。
import type {InputHTMLAttributes} from 'react';
import {cn} from './cn';
export function Input({className,...props}:InputHTMLAttributes<HTMLInputElement>){
  return <input {...props} className={cn('h-9 w-full rounded-lg border border-border bg-surface px-3 text-sm text-fg placeholder:text-muted focus-visible:outline-2 focus-visible:outline-primary disabled:opacity-50',className)}/>;
}
