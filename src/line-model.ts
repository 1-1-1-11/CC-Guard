// [INPUT]: contract.ts 的 Verdict / LineCheck / Settings / Leak。
// [OUTPUT]: 显示用判定（错误、无结果、过期一律 unknown）、首次设置判断、分流模式文案、漏网记录合并。
// [POS]: 状态条、向导与连接页共用的纯函数；只有后端给出 pass 且结果新鲜时才可能显示绿色。
import type {Leak,LineCheck,Settings,Verdict} from './contract';
export const STALE_MS=180_000;
export type Received={check:LineCheck;at:number};
export function shownVerdict(result:Received|null,error:string,now:number):Verdict{
  if(error||!result)return 'unknown';
  if(now-result.at>STALE_MS)return 'unknown';
  return result.check.verdict==='pass'||result.check.verdict==='block'?result.check.verdict:'unknown';
}
export const needsSetup=(settings:Settings)=>!settings.line.controller.trim()||!settings.line.port;
const modes:Record<string,string>={rule:'规则分流',global:'全局',direct:'直连'};
export const modeLabel=(mode:string)=>modes[mode.toLowerCase()]??mode;
export type SeenLeak=Leak&{first:number;last:number;live:boolean};
const key=(leak:Leak)=>`${leak.host}|${leak.process}|${leak.inbound}|${leak.node}`;
// 每个漏网目标只列一次；本次快照仍在的排最前，其余按最近出现排序。
export function mergeLeaks(seen:SeenLeak[],leaks:Leak[],at:number,limit=200):SeenLeak[]{
  const rows=new Map(seen.map(row=>[key(row),{...row,live:false}]));
  for(const leak of leaks){const old=rows.get(key(leak));rows.set(key(leak),old?{...old,last:at,live:true}:{...leak,first:at,last:at,live:true});}
  return [...rows.values()].sort((a,b)=>Number(b.live)-Number(a.live)||b.last-a.last).slice(0,limit);
}
