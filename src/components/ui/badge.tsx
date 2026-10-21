// [INPUT]: 色调与文字。
// [OUTPUT]: Badge；pass 是唯一绿色色调，block 红、unknown 灰、info 蓝。
// [POS]: 状态与标签的小徽标。
import type {HTMLAttributes} from 'react';
import {cn} from './cn';
export type Tone='pass'|'block'|'unknown'|'info'|'neutral';
const tones:Record<Tone,string>={
  pass:'bg-pass-bg text-pass border-pass/30',
  block:'bg-block-bg text-block border-block/30',
  unknown:'bg-unknown-bg text-unknown border-unknown/30',
  info:'bg-primary/10 text-primary border-primary/30',
  neutral:'bg-muted-bg text-muted border-border',
};
export function Badge({tone='neutral',className,...props}:HTMLAttributes<HTMLSpanElement>&{tone?:Tone}){
  return <span {...props} data-tone={tone} className={cn('inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs font-medium',tones[tone],className)}/>;
}
