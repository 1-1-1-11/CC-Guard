// [INPUT]: 任意内容。
// [OUTPUT]: Card，CC-Switch 式圆角卡片。
// [POS]: 账号、连接、设置分组的容器。
import type {HTMLAttributes} from 'react';
import {cn} from './cn';
export function Card({className,...props}:HTMLAttributes<HTMLDivElement>){
  return <div {...props} className={cn('rounded-xl border border-border bg-surface p-4 shadow-sm',className)}/>;
}
