// [INPUT]: Radix Tooltip 与提示文字。
// [OUTPUT]: TooltipProvider、Tooltip，悬停或聚焦时显示简短说明。
// [POS]: 图标按钮与状态条的补充说明。
import type {ReactNode} from 'react';
import {Tooltip as Primitive} from 'radix-ui';
export const TooltipProvider=Primitive.Provider;
export function Tooltip({content,children}:{content:ReactNode;children:ReactNode}){
  return <Primitive.Root><Primitive.Trigger asChild>{children}</Primitive.Trigger>
    <Primitive.Portal><Primitive.Content sideOffset={6} className="z-50 max-w-xs rounded-md bg-fg px-2 py-1 text-xs text-bg shadow">{content}</Primitive.Content></Primitive.Portal>
  </Primitive.Root>;
}
